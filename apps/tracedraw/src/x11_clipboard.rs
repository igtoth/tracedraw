//! The picture on the X11 clipboard (#1488).
//!
//! A screenshot tool keeps its picture on the clipboard itself and hands
//! it over when asked, a large one in parts (the INCR transfer of the
//! X11 clipboard conventions), each part when the reader has taken the
//! one before. The clipboard library gave each part 10 ms to come and
//! asked for PNG alone: a busy program missed that time and the paste
//! came out empty, so a picture took two or three pastes to arrive. Here
//! the clipboard's list of types (TARGETS) is read first, the best
//! picture type it offers is asked for (PNG first, then BMP, TIFF, WebP,
//! GIF, JPEG), each part may take up to two seconds, and a read that
//! comes back empty is tried again briefly.

use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, Property, Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

/// The picture types asked for, best first: lossless ones before JPEG.
pub const IMAGE_TYPES: [&str; 10] = [
    "image/png",
    "image/bmp",
    "image/x-bmp",
    "image/x-ms-bmp",
    "image/tiff",
    "image/webp",
    "image/gif",
    "image/jpeg",
    "image/jpg",
    "image/pjpeg",
];

/// How long the owner of the clipboard may take to answer a request (it
/// may encode a large picture first).
const ANSWER_WAIT: Duration = Duration::from_secs(3);

/// How long each part of a picture sent in parts may take.
const PART_WAIT: Duration = Duration::from_secs(2);

/// How many times a read that found no picture is made.
const TRIES: u32 = 3;

/// The pause between two tries.
const RETRY_PAUSE: Duration = Duration::from_millis(150);

/// The picture types `offered` holds (the clipboard's targets), best
/// first.
pub fn image_types(offered: &[String]) -> Vec<&'static str> {
    IMAGE_TYPES
        .iter()
        .copied()
        .filter(|t| offered.iter().any(|o| o.trim().eq_ignore_ascii_case(t)))
        .collect()
}

/// The picture in `bytes`, sent as `mime`; its own signature decides
/// when the type does not fit it.
pub fn decode(mime: &str, bytes: &[u8]) -> Option<image::RgbaImage> {
    use image::ImageFormat as F;
    let format = match mime.to_ascii_lowercase().as_str() {
        "image/png" => Some(F::Png),
        "image/bmp" | "image/x-bmp" | "image/x-ms-bmp" => Some(F::Bmp),
        "image/tiff" => Some(F::Tiff),
        "image/webp" => Some(F::WebP),
        "image/gif" => Some(F::Gif),
        "image/jpeg" | "image/jpg" | "image/pjpeg" => Some(F::Jpeg),
        _ => None,
    };
    format
        .and_then(|f| image::load_from_memory_with_format(bytes, f).ok())
        .or_else(|| image::load_from_memory(bytes).ok())
        .map(|img| img.into_rgba8())
}

/// The atoms of a TARGETS answer (32-bit values in the connection's
/// byte order).
fn atoms_of(bytes: &[u8]) -> Vec<Atom> {
    bytes
        .chunks_exact(4)
        .map(|c| Atom::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// A connection to the X server with a window of its own that the
/// clipboard's owner writes into.
struct Reader {
    conn: RustConnection,
    window: Window,
    clipboard: Atom,
    targets: Atom,
    incr: Atom,
    /// The window's property the owner writes the answer to.
    slot: Atom,
}

impl Reader {
    /// A reader on the display of `DISPLAY` (`None`: no X server).
    fn open(display: Option<&str>) -> Option<Self> {
        let (conn, screen) = x11rb::connect(display).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let window = conn.generate_id().ok()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .ok()?;
        let intern = |name: &str| -> Option<Atom> {
            Some(
                conn.intern_atom(false, name.as_bytes())
                    .ok()?
                    .reply()
                    .ok()?
                    .atom,
            )
        };
        let clipboard = intern("CLIPBOARD")?;
        let targets = intern("TARGETS")?;
        let incr = intern("INCR")?;
        let slot = intern("TRACEDRAW_CLIPBOARD")?;
        Some(Reader {
            conn,
            window,
            clipboard,
            targets,
            incr,
            slot,
        })
    }

    /// The first event `pick` accepts before `until`.
    fn wait<T>(&self, until: Instant, mut pick: impl FnMut(&Event) -> Option<T>) -> Option<T> {
        loop {
            match self.conn.poll_for_event() {
                Ok(Some(event)) => {
                    if let Some(found) = pick(&event) {
                        return Some(found);
                    }
                }
                Ok(None) => {
                    if Instant::now() >= until {
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => return None,
            }
        }
    }

    /// The clipboard's content as `target`: `None` when its owner does
    /// not answer, refuses, or stops sending parts.
    fn convert(&self, target: Atom) -> Option<Vec<u8>> {
        let c = &self.conn;
        c.delete_property(self.window, self.slot).ok()?;
        c.convert_selection(
            self.window,
            self.clipboard,
            target,
            self.slot,
            x11rb::CURRENT_TIME,
        )
        .ok()?;
        c.flush().ok()?;
        let window = self.window;
        let property = self.wait(Instant::now() + ANSWER_WAIT, |event| match event {
            Event::SelectionNotify(e) if e.requestor == window => Some(e.property),
            _ => None,
        })?;
        if property == x11rb::NONE {
            return None;
        }
        let answer = c
            .get_property(false, window, self.slot, AtomEnum::ANY, 0, u32::MAX / 4)
            .ok()?
            .reply()
            .ok()?;
        // Deleting the answer tells the owner it was read; for a transfer
        // in parts it asks for the first part.
        c.delete_property(window, self.slot).ok()?;
        c.flush().ok()?;
        if answer.type_ != self.incr {
            return Some(answer.value);
        }
        let mut data = Vec::new();
        loop {
            let slot = self.slot;
            let part_came = self.wait(Instant::now() + PART_WAIT, |event| match event {
                Event::PropertyNotify(e)
                    if e.window == window && e.atom == slot && e.state == Property::NEW_VALUE =>
                {
                    Some(())
                }
                _ => None,
            });
            if part_came.is_none() {
                // No closing empty part (the owner went away): what came
                // is kept, and decoding tells whether it is whole.
                return (!data.is_empty()).then_some(data);
            }
            let part = c
                .get_property(true, window, slot, AtomEnum::ANY, 0, u32::MAX / 4)
                .ok()?
                .reply()
                .ok()?;
            c.flush().ok()?;
            // An empty part ends the transfer.
            if part.value.is_empty() {
                return Some(data);
            }
            data.extend_from_slice(&part.value);
        }
    }

    /// The names of the types the clipboard offers; empty when it has
    /// no owner or the owner does not answer.
    fn offered(&self) -> Vec<String> {
        let Some(bytes) = self.convert(self.targets) else {
            return Vec::new();
        };
        let cookies: Vec<_> = atoms_of(&bytes)
            .into_iter()
            .filter_map(|a| self.conn.get_atom_name(a).ok())
            .collect();
        cookies
            .into_iter()
            .filter_map(|cookie| cookie.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.name).into_owned())
            .collect()
    }

    /// The best picture the clipboard offers, decoded.
    fn image(&self) -> Option<image::RgbaImage> {
        for attempt in 0..TRIES {
            if attempt > 0 {
                std::thread::sleep(RETRY_PAUSE);
            }
            let offered = self.offered();
            if offered.is_empty() {
                // No answer yet: the owner may be busy.
                continue;
            }
            let types = image_types(&offered);
            if types.is_empty() {
                // Text or files, not a picture.
                return None;
            }
            for mime in types {
                let Ok(cookie) = self.conn.intern_atom(false, mime.as_bytes()) else {
                    continue;
                };
                let Some(atom) = cookie.reply().ok().map(|r| r.atom) else {
                    continue;
                };
                if let Some(img) = self.convert(atom).and_then(|b| decode(mime, &b)) {
                    return Some(img);
                }
                log::info!("the clipboard's {mime} picture did not arrive whole");
            }
        }
        None
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}

/// The picture on the clipboard of the X server of `DISPLAY`; `None`
/// when there is no X server or no picture.
pub fn read_image() -> Option<image::RgbaImage> {
    std::env::var_os("DISPLAY")?;
    Reader::open(None)?.image()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picture_types_go_best_first() {
        let offered: Vec<String> = [
            "TARGETS",
            "image/jpeg",
            "image/bmp",
            "application/x-qt-image",
            "IMAGE/PNG",
            "text/plain",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            image_types(&offered),
            vec!["image/png", "image/bmp", "image/jpeg"]
        );
        assert!(image_types(&["UTF8_STRING".to_string()]).is_empty());
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&7u32.to_ne_bytes());
        bytes.extend_from_slice(&300u32.to_ne_bytes());
        bytes.push(9);
        assert_eq!(atoms_of(&bytes), vec![7, 300]);
    }

    fn png_of(img: &image::RgbaImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)
            .expect("png");
        out.into_inner()
    }

    #[test]
    fn pictures_decode_by_type_or_by_signature() {
        let img = image::RgbaImage::from_fn(5, 3, |x, y| {
            image::Rgba([x as u8 * 40, y as u8 * 80, 7, 255])
        });
        let png = png_of(&img);
        assert_eq!(decode("image/png", &png).as_ref(), Some(&img));
        // A type that does not fit: the signature decides.
        assert_eq!(decode("image/bmp", &png).as_ref(), Some(&img));
        let mut bmp = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img.clone())
            .to_rgb8()
            .write_to(&mut bmp, image::ImageFormat::Bmp)
            .expect("bmp");
        let back = decode("image/x-MS-bmp", bmp.get_ref()).expect("bmp");
        assert_eq!(back.dimensions(), (5, 3));
        assert_eq!(back.get_pixel(4, 2)[0], 160);
        assert!(decode("image/png", b"not a picture").is_none());
    }

    /// An X server for the tests: the one of `DISPLAY`, else an Xvfb
    /// started for them; `None` when neither is there.
    struct Server {
        display: String,
        child: Option<std::process::Child>,
    }

    impl Drop for Server {
        fn drop(&mut self) {
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn x_server() -> Option<Server> {
        if let Some(d) = std::env::var("DISPLAY").ok().filter(|d| !d.is_empty()) {
            if x11rb::connect(Some(&d)).is_ok() {
                return Some(Server {
                    display: d,
                    child: None,
                });
            }
        }
        for n in 0..20 {
            let num = 200 + std::process::id() % 300 + n;
            if std::path::Path::new(&format!("/tmp/.X11-unix/X{num}")).exists() {
                continue;
            }
            let display = format!(":{num}");
            let child = std::process::Command::new("Xvfb")
                .arg(&display)
                .args(["-nolisten", "tcp", "-screen", "0", "64x64x24"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok()?;
            let server = Server {
                display,
                child: Some(child),
            };
            for _ in 0..100 {
                if x11rb::connect(Some(&server.display)).is_ok() {
                    return Some(server);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        None
    }

    /// A busy screenshot tool: it owns the clipboard on `display` with
    /// `png` as image/png, answers TARGETS, and sends the picture in
    /// parts of `part` bytes, waiting `pause` before each one, once.
    fn serve(
        display: String,
        png: Vec<u8>,
        part: usize,
        pause: Duration,
        ready: std::sync::mpsc::Sender<()>,
    ) -> std::thread::JoinHandle<()> {
        use x11rb::protocol::xproto::{
            ChangeWindowAttributesAux, PropMode, SelectionNotifyEvent, SELECTION_NOTIFY_EVENT,
        };
        use x11rb::wrapper::ConnectionExt as _;
        std::thread::spawn(move || {
            let (conn, screen) = x11rb::connect(Some(&display)).expect("owner's connection");
            let root = conn.setup().roots[screen].root;
            let win = conn.generate_id().expect("id");
            conn.create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                win,
                root,
                0,
                0,
                1,
                1,
                0,
                WindowClass::INPUT_OUTPUT,
                x11rb::COPY_FROM_PARENT,
                &CreateWindowAux::new(),
            )
            .expect("window");
            let atom = |name: &str| {
                conn.intern_atom(false, name.as_bytes())
                    .expect("atom")
                    .reply()
                    .expect("atom")
                    .atom
            };
            let (clipboard, targets, incr, png_type) = (
                atom("CLIPBOARD"),
                atom("TARGETS"),
                atom("INCR"),
                atom("image/png"),
            );
            conn.set_selection_owner(win, clipboard, x11rb::CURRENT_TIME)
                .expect("owner");
            conn.flush().expect("flush");
            ready.send(()).expect("ready");
            let mut transfer: Option<(Window, Atom, usize)> = None;
            let until = Instant::now() + Duration::from_secs(30);
            while Instant::now() < until {
                let Some(event) = conn.poll_for_event().expect("event") else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                match event {
                    Event::SelectionRequest(e) => {
                        let mut property = e.property;
                        if e.target == targets {
                            conn.change_property32(
                                PropMode::REPLACE,
                                e.requestor,
                                e.property,
                                AtomEnum::ATOM,
                                &[targets, png_type],
                            )
                            .expect("targets");
                        } else if e.target == png_type {
                            conn.change_window_attributes(
                                e.requestor,
                                &ChangeWindowAttributesAux::new()
                                    .event_mask(EventMask::PROPERTY_CHANGE),
                            )
                            .expect("watch");
                            conn.change_property32(
                                PropMode::REPLACE,
                                e.requestor,
                                e.property,
                                incr,
                                &[png.len() as u32],
                            )
                            .expect("incr");
                            transfer = Some((e.requestor, e.property, 0));
                        } else {
                            property = x11rb::NONE;
                        }
                        let notify = SelectionNotifyEvent {
                            response_type: SELECTION_NOTIFY_EVENT,
                            sequence: 0,
                            time: e.time,
                            requestor: e.requestor,
                            selection: e.selection,
                            target: e.target,
                            property,
                        };
                        conn.send_event(false, e.requestor, EventMask::NO_EVENT, notify)
                            .expect("notify");
                        conn.flush().expect("flush");
                    }
                    Event::PropertyNotify(e) if e.state == Property::DELETE => {
                        let Some((w, p, at)) = transfer else {
                            continue;
                        };
                        if e.window != w || e.atom != p {
                            continue;
                        }
                        std::thread::sleep(pause);
                        let end = (at + part).min(png.len());
                        conn.change_property8(PropMode::REPLACE, w, p, png_type, &png[at..end])
                            .expect("part");
                        conn.flush().expect("flush");
                        if at == end {
                            // The closing empty part reaches the server
                            // before the connection ends.
                            let _ = conn.sync();
                            return;
                        }
                        transfer = Some((w, p, end));
                    }
                    _ => {}
                }
            }
        })
    }

    /// A screenshot tool that takes 30 ms over each part of its picture
    /// (the clipboard library gave each part 10 ms): the picture arrives
    /// whole on the first paste (#1488).
    #[test]
    fn a_picture_sent_slowly_in_parts_arrives_whole() {
        let Some(server) = x_server() else {
            eprintln!("no X server (DISPLAY or Xvfb): skipped");
            return;
        };
        let img = image::RgbaImage::from_fn(96, 64, |x, y| {
            let v = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)) as u8;
            image::Rgba([v, v.wrapping_mul(3), x as u8, 255])
        });
        let png = png_of(&img);
        assert!(png.len() > 8 * 1024, "the picture comes in parts");
        let (tx, rx) = std::sync::mpsc::channel();
        let owner = serve(
            server.display.clone(),
            png,
            2048,
            Duration::from_millis(30),
            tx,
        );
        rx.recv_timeout(Duration::from_secs(10))
            .expect("owner ready");
        let reader = Reader::open(Some(&server.display)).expect("reader");
        assert!(reader.offered().iter().any(|t| t == "image/png"));
        let got = reader.image().expect("the picture");
        assert_eq!(got, img);
        owner.join().expect("owner");
        // No owner any more: nothing, without waiting long.
        let started = Instant::now();
        assert!(reader.image().is_none());
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
