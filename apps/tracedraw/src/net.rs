//! HTTP for the services TraceDraw talks to (AI Drawing, GitHub): one
//! request at a time, on a thread on the desktop and as a `fetch` in a
//! browser, its answer delivered on a channel.

/// One HTTP request.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    /// `GET`, `POST` or `PUT`.
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// A binary body (multipart uploads); sent instead of `body` when
    /// not empty.
    pub bytes: Vec<u8>,
}

/// What the service answered.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// A response, or why none came (network, address, certificate).
pub type HttpResult = Result<HttpResponse, String>;

/// Send `req` and deliver the answer (at most `limit` bytes of body) on
/// `tx`: on a thread on the desktop, as a fetch in a browser.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn(req: HttpRequest, limit: u64, tx: std::sync::mpsc::Sender<HttpResult>) {
    std::thread::spawn(move || {
        let _ = tx.send(send(&req, limit));
    });
}

#[cfg(target_arch = "wasm32")]
pub fn spawn(req: HttpRequest, limit: u64, tx: std::sync::mpsc::Sender<HttpResult>) {
    let _ = limit;
    wasm_bindgen_futures::spawn_local(async move {
        let _ = tx.send(send(req).await);
        crate::web::request_repaint();
    });
}

/// The TLS set-up of every HTTPS request: certificates are checked
/// against the system's trusted ones.
#[cfg(not(target_arch = "wasm32"))]
pub fn tls_config() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(root_certs())
        .build()
}

/// The system's verifier (Windows, macOS, Linux).
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn root_certs() -> ureq::tls::RootCerts {
    ureq::tls::RootCerts::PlatformVerifier
}

/// Android: the system's verifier needs a Java part the app does not
/// carry (and set-up the app never did), so every HTTPS request failed
/// its certificate check (#1396). The certificates the system trusts are
/// read from its store instead, the files its own TLS reads; the bundled
/// Mozilla roots stand in when the store cannot be read. Read once.
#[cfg(target_os = "android")]
fn root_certs() -> ureq::tls::RootCerts {
    static ROOTS: std::sync::OnceLock<ureq::tls::RootCerts> = std::sync::OnceLock::new();
    ROOTS
        .get_or_init(|| {
            let dirs: Vec<&std::path::Path> = ANDROID_CA_DIRS
                .iter()
                .map(std::path::Path::new)
                .collect();
            match system_store(&dirs) {
                Some((dir, certs)) => {
                    log::info!(
                        "HTTPS: {} trusted certificates from {}",
                        certs.len(),
                        dir.display()
                    );
                    ureq::tls::RootCerts::Specific(std::sync::Arc::new(certs))
                }
                None => {
                    log::warn!("HTTPS: the system's certificate store cannot be read, the bundled roots are used");
                    ureq::tls::RootCerts::WebPki
                }
            }
        })
        .clone()
}

/// The folders of Android's certificate store, the one updated with the
/// system's TLS module (Android 14 and later) first.
#[cfg(any(target_os = "android", test))]
const ANDROID_CA_DIRS: [&str; 2] = [
    "/apex/com.android.conscrypt/cacerts",
    "/system/etc/security/cacerts",
];

/// The certificates of the first of `dirs` that holds any, with that
/// folder: each file there is one certificate in PEM form (with its
/// description as text before it).
#[cfg(any(target_os = "android", test))]
fn system_store(
    dirs: &[&std::path::Path],
) -> Option<(std::path::PathBuf, Vec<ureq::tls::Certificate<'static>>)> {
    use ureq::tls::PemItem;
    dirs.iter().find_map(|dir| {
        let mut certs = Vec::new();
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            certs.extend(ureq::tls::parse_pem(&bytes).filter_map(|item| match item {
                Ok(PemItem::Certificate(c)) => Some(c),
                _ => None,
            }));
        }
        (!certs.is_empty()).then(|| (dir.to_path_buf(), certs))
    })
}

/// One request over HTTPS (or HTTP for local servers), waiting up to
/// five minutes. Certificates are checked against the system's trusted
/// ones ([`tls_config`]); proxies come from the usual environment
/// variables.
#[cfg(not(target_arch = "wasm32"))]
pub fn send(req: &HttpRequest, limit: u64) -> HttpResult {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(300)))
        .http_status_as_error(false)
        .tls_config(tls_config())
        .build()
        .into();
    let mut resp = match req.method {
        "GET" => {
            let mut r = agent.get(&req.url);
            for (name, value) in &req.headers {
                r = r.header(name, value);
            }
            r.call()
        }
        "PUT" => {
            let mut r = agent.put(&req.url);
            for (name, value) in &req.headers {
                r = r.header(name, value);
            }
            r.send(req.body.as_str())
        }
        _ => {
            let mut r = agent.post(&req.url);
            for (name, value) in &req.headers {
                r = r.header(name, value);
            }
            if req.bytes.is_empty() {
                r.send(req.body.as_str())
            } else {
                r.send(&req.bytes[..])
            }
        }
    }
    .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body = resp
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(HttpResponse { status, body })
}

#[cfg(target_arch = "wasm32")]
pub async fn send(req: HttpRequest) -> HttpResult {
    use wasm_bindgen::JsCast;
    let js = |e: wasm_bindgen::JsValue| {
        e.as_string()
            .or_else(|| {
                e.dyn_ref::<js_sys::Error>()
                    .map(|e| String::from(e.message()))
            })
            .unwrap_or_else(|| "the request failed".to_string())
    };
    let init = web_sys::RequestInit::new();
    init.set_method(req.method);
    init.set_mode(web_sys::RequestMode::Cors);
    if req.method != "GET" {
        if req.bytes.is_empty() {
            init.set_body(&wasm_bindgen::JsValue::from_str(&req.body));
        } else {
            let array = js_sys::Uint8Array::from(&req.bytes[..]);
            init.set_body(&array);
        }
    }
    let headers = web_sys::Headers::new().map_err(js)?;
    for (name, value) in &req.headers {
        headers.set(name, value).map_err(js)?;
    }
    init.set_headers(headers.as_ref());
    let request = web_sys::Request::new_with_str_and_init(&req.url, &init).map_err(js)?;
    let window = web_sys::window().ok_or("no browser window")?;
    let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(js)?;
    let resp: web_sys::Response = resp.dyn_into().map_err(js)?;
    let status = resp.status();
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(js)?)
        .await
        .map_err(js)?;
    Ok(HttpResponse {
        status,
        body: text.as_string().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Android's certificate store: the files of the first folder that
    /// has certificates, each read with the text before its PEM block;
    /// files that hold none are skipped (#1396).
    #[test]
    fn the_android_store_is_read_from_its_first_folder_with_certificates() {
        let root = std::env::temp_dir().join(format!("tracedraw-cacerts-{}", std::process::id()));
        let empty = root.join("apex");
        let system = root.join("system");
        std::fs::create_dir_all(&empty).expect("folder");
        std::fs::create_dir_all(&system).expect("folder");
        let pem = "Certificate:\n    Data: a description\n-----BEGIN CERTIFICATE-----\nMIIBszCCAVmgAwIBAgI=\n-----END CERTIFICATE-----\n";
        std::fs::write(system.join("1a2b3c4d.0"), pem).expect("write");
        std::fs::write(system.join("5e6f7a8b.0"), pem).expect("write");
        std::fs::write(system.join("notes.txt"), "no certificate here").expect("write");
        std::fs::write(empty.join("readme"), "nothing").expect("write");
        let missing = root.join("missing");
        let (dir, certs) =
            system_store(&[missing.as_path(), empty.as_path(), system.as_path()]).expect("store");
        assert_eq!(dir, system);
        assert_eq!(certs.len(), 2);
        assert_eq!(certs[0].der().len(), 14);
        assert!(system_store(&[missing.as_path(), empty.as_path()]).is_none());
        let _ = std::fs::remove_dir_all(&root);
        // The store's places on the device, the updatable one first.
        assert!(ANDROID_CA_DIRS[0].starts_with("/apex/"));
        assert_eq!(ANDROID_CA_DIRS[1], "/system/etc/security/cacerts");
    }

    /// Every request is set up the same way, with the system's
    /// certificates.
    #[test]
    fn requests_check_certificates_against_the_system() {
        assert!(matches!(
            tls_config().root_certs(),
            ureq::tls::RootCerts::PlatformVerifier
        ));
    }
}
