//! `tracedraw --mcp`: a Model Context Protocol server on stdin/stdout.
//!
//! The editor runs headless and answers JSON-RPC 2.0 messages (one per
//! line) with the MCP methods `initialize`, `ping`, `tools/list` and
//! `tools/call`. The tools open and save files in every format the
//! editor knows, export pages, run automation scripts (the same
//! JavaScript object model as Tools > Scripts), describe the document
//! and undo or redo. An agent connected to this server edits documents
//! the way a user does: every change is a command with undo.

use crate::app::App;
use crate::describe::document as describe;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the server until stdin closes.
pub fn serve() {
    let mut app = App::headless();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(reply) = handle_line(&mut app, line) else {
            continue;
        };
        let mut out = stdout.lock();
        if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}

/// One request line to one response line (`None` for notifications).
pub fn handle_line(app: &mut App, line: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Some(
                json!({"jsonrpc": "2.0", "id": Value::Null,
                       "error": {"code": -32700, "message": format!("parse error: {e}")}})
                .to_string(),
            )
        }
    };
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    // Notifications carry no id and get no reply.
    let id = id?;
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "tracedraw", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tool_list()})),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call_tool(app, name, &args) {
                Ok(text) => {
                    Ok(json!({"content": [{"type": "text", "text": text}], "isError": false}))
                }
                Err(e) => Ok(json!({"content": [{"type": "text", "text": e}], "isError": true})),
            }
        }
        other => Err((-32601, format!("method not found: {other}"))),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}).to_string(),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
                .to_string()
        }
    })
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": props, "required": required},
    })
}

pub fn tool_list() -> Vec<Value> {
    vec![
        tool(
            "open",
            "Open a file as the current document: .cdr, .tdraw, .svg, .svgz, .pdf, .ai, .eps, .ps, .dxf, .psd, .psb, .emf, .wmf, .plt, .txt, .rtf, .docx, or an image.",
            json!({"path": {"type": "string"}}),
            &["path"],
        ),
        tool(
            "new_document",
            "Start a new document with a page of the given size in millimetres (default A4).",
            json!({"width_mm": {"type": "number"}, "height_mm": {"type": "number"}, "title": {"type": "string"}}),
            &[],
        ),
        tool(
            "save",
            "Save the document to a path: .tdraw (native) or .cdr. For .cdr, version picks the file version (7 to 27; default: the version of the .cdr file the document came from, else the one set in the options).",
            json!({"path": {"type": "string"}, "version": {"type": "integer"}}),
            &["path"],
        ),
        tool(
            "export",
            "Export the first page (or page_index) to a path; the extension picks the format: svg, pdf, eps, dxf, emf, wmf, plt, psd, html, png. dpi applies to png and psd (default 96).",
            json!({"path": {"type": "string"}, "page_index": {"type": "integer"}, "dpi": {"type": "number"}}),
            &["path"],
        ),
        tool(
            "run_script",
            "Run a JavaScript automation script against the document (Application.ActiveDocument, ActivePage, ActiveLayer, Shapes, CreateRectangle, CreateEllipse, CreateArtisticText, CreateCurve, Fill, Outline, Move, Rotate, ...). Returns what the script printed. The script's changes are one undo step.",
            json!({"source": {"type": "string"}}),
            &["source"],
        ),
        tool(
            "document_info",
            "Describe the document: title, pages, layers and every object with its id, type, name, bounds in millimetres, fill and outline.",
            json!({}),
            &[],
        ),
        tool("undo", "Undo the last change.", json!({}), &[]),
        tool("redo", "Redo the last undone change.", json!({}), &[]),
    ]
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing argument: {key}"))
}

pub fn call_tool(app: &mut App, name: &str, args: &Value) -> Result<String, String> {
    match name {
        "open" => {
            let path = std::path::PathBuf::from(str_arg(args, "path")?);
            if !path.exists() {
                return Err(format!("no such file: {}", path.display()));
            }
            app.open_path(path);
            if app.status.contains("failed") || app.status.contains("error") {
                return Err(app.status.clone());
            }
            Ok(describe(app).to_string())
        }
        "new_document" => {
            let w = args
                .get("width_mm")
                .and_then(|v| v.as_f64())
                .unwrap_or(210.0);
            let h = args
                .get("height_mm")
                .and_then(|v| v.as_f64())
                .unwrap_or(297.0);
            if !(w > 0.0 && h > 0.0 && w < 10_000.0 && h < 10_000.0) {
                return Err("page size out of range".into());
            }
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| app.next_untitled_name());
            let doc = App::localized_document(title, tracedraw_core::geometry::Size::new(w, h));
            app.open_document(doc, None);
            Ok(describe(app).to_string())
        }
        "save" => {
            let path = std::path::PathBuf::from(str_arg(args, "path")?);
            let is_cdr = crate::app::is_cdr_path(&path);
            let version = match args.get("version") {
                None | Some(Value::Null) => app.cdr_save_version(),
                Some(v) => v
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .filter(|v| tracedraw_cdr::write::checked_versions().contains(v))
                    .ok_or_else(|| {
                        format!(
                            "version must be {} to {}",
                            tracedraw_cdr::write::CHECKED_SINCE,
                            tracedraw_cdr::write::CHECKED_UNTIL
                        )
                    })?,
            };
            let r = if is_cdr {
                std::fs::write(
                    &path,
                    tracedraw_cdr::write::document_to_cdr_version_with(
                        app.engine.document(),
                        version,
                        &crate::app::cdr_previews(app.engine.document()),
                    ),
                )
                .map_err(|e| e.to_string())
            } else {
                tracedraw_io::save_native(
                    &tracedraw_text::fonts().with_carried_fonts(app.engine.document()),
                    &path,
                )
                .map_err(|e| e.to_string())
            };
            r?;
            app.engine.mark_saved();
            app.file = Some(path.clone());
            if is_cdr {
                app.cdr_version = Some(version);
                app.cdr_unsaved_version = None;
                return Ok(format!("saved {} (version {version})", path.display()));
            }
            Ok(format!("saved {}", path.display()))
        }
        "export" => {
            let path = std::path::PathBuf::from(str_arg(args, "path")?);
            let page_index = args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let dpi = args.get("dpi").and_then(|v| v.as_f64()).unwrap_or(96.0);
            export_to(app, &path, page_index, dpi)?;
            Ok(format!("exported {}", path.display()))
        }
        "run_script" => {
            let src = str_arg(args, "source")?;
            let out = crate::scripting::run(app, src)?;
            Ok(if out.is_empty() {
                "ok".to_string()
            } else {
                out.join("\n")
            })
        }
        "document_info" => Ok(describe(app).to_string()),
        "undo" => {
            app.undo();
            Ok(describe(app).to_string())
        }
        "redo" => {
            app.redo();
            Ok(describe(app).to_string())
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

fn export_to(app: &App, path: &std::path::Path, page_index: usize, dpi: f64) -> Result<(), String> {
    let doc = app.engine.document();
    if page_index >= doc.pages.len() {
        return Err(format!("page_index {page_index} out of range"));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let e = |r: tracedraw_io::Result<()>| r.map_err(|e| e.to_string());
    match ext.as_str() {
        "svg" => e(tracedraw_io::save_svg(doc, page_index, path)),
        "pdf" => e(tracedraw_io::save_pdf(doc, path)),
        "eps" => std::fs::write(path, tracedraw_io::eps::page_to_eps(doc, page_index))
            .map_err(|e| e.to_string()),
        "ps" | "prn" => {
            std::fs::write(path, tracedraw_io::eps::document_to_ps(doc)).map_err(|e| e.to_string())
        }
        "dxf" => e(tracedraw_io::save_dxf(doc, page_index, path)),
        "emf" => e(tracedraw_io::save_emf(doc, page_index, path)),
        "wmf" => e(tracedraw_io::save_wmf(doc, page_index, path)),
        "plt" => e(tracedraw_io::save_plt(doc, page_index, path)),
        "psd" => e(tracedraw_io::save_psd(
            doc,
            page_index,
            dpi.clamp(1.0, 2400.0),
            path,
        )),
        "html" => std::fs::write(path, tracedraw_io::html::document_to_html(doc))
            .map_err(|e| e.to_string()),
        "png" => {
            let page = doc.pages[page_index].id;
            let pm = tracedraw_render::render_page_image(doc, page, dpi.clamp(1.0, 2400.0))
                .ok_or("render failed")?;
            pm.save_png(path).map_err(|e| e.to_string())
        }
        other => Err(format!("unsupported export format: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(app: &mut App, id: u64, method: &str, params: Value) -> Value {
        let line =
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
        let reply = handle_line(app, &line).expect("reply");
        serde_json::from_str(&reply).expect("json")
    }

    #[test]
    fn initialize_list_and_call_tools() {
        let mut app = App::headless();
        let init = call(&mut app, 1, "initialize", json!({}));
        assert_eq!(init["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(init["result"]["serverInfo"]["name"], "tracedraw");
        // A notification gets no reply.
        assert!(handle_line(
            &mut app,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string()
        )
        .is_none());
        let list = call(&mut app, 2, "tools/list", json!({}));
        let names: Vec<&str> = list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"run_script") && names.contains(&"export"));
        // New document, draw through a script, inspect, undo.
        let nd = call(
            &mut app,
            3,
            "tools/call",
            json!({"name": "new_document", "arguments": {"width_mm": 100, "height_mm": 50}}),
        );
        assert_eq!(nd["result"]["isError"], false);
        let run = call(
            &mut app,
            4,
            "tools/call",
            json!({"name": "run_script", "arguments": {"source":
                "var s = ActiveLayer.CreateRectangle(10, 10, 40, 30); s.Fill.ApplyUniformFill(Color.RGB(255, 0, 0)); print(ActivePage.Shapes.Count);"}}),
        );
        assert_eq!(run["result"]["isError"], false, "{run}");
        assert_eq!(run["result"]["content"][0]["text"], "1");
        let info = call(
            &mut app,
            5,
            "tools/call",
            json!({"name": "document_info", "arguments": {}}),
        );
        let text = info["result"]["content"][0]["text"].as_str().unwrap();
        let d: Value = serde_json::from_str(text).unwrap();
        assert_eq!(d["pages"][0]["size_mm"]["width"], 100.0);
        let objs = &d["pages"][0]["layers"][0]["objects"];
        assert_eq!(objs.as_array().unwrap().len(), 1);
        assert_eq!(objs[0]["type"], "rectangle");
        assert_eq!(objs[0]["fill"]["solid"], "R:255 G:0 B:0");
        let undo = call(
            &mut app,
            6,
            "tools/call",
            json!({"name": "undo", "arguments": {}}),
        );
        let d: Value =
            serde_json::from_str(undo["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            d["pages"][0]["layers"][0]["objects"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        // Unknown tool and unknown method.
        let bad = call(
            &mut app,
            7,
            "tools/call",
            json!({"name": "nope", "arguments": {}}),
        );
        assert_eq!(bad["result"]["isError"], true);
        let nm = call(&mut app, 8, "resources/list", json!({}));
        assert_eq!(nm["error"]["code"], -32601);
    }

    #[test]
    fn export_and_save_write_files() {
        let mut app = App::headless();
        let dir = std::env::temp_dir().join(format!("tracedraw-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        call(
            &mut app,
            1,
            "tools/call",
            json!({"name": "run_script", "arguments": {"source": "ActiveLayer.CreateEllipse(10, 10, 60, 40);"}}),
        );
        for ext in ["svg", "pdf", "png", "emf", "cdr", "tdraw"] {
            let path = dir.join(format!("out.{ext}"));
            let tool = if ext == "cdr" || ext == "tdraw" {
                "save"
            } else {
                "export"
            };
            let r = call(
                &mut app,
                2,
                "tools/call",
                json!({"name": tool, "arguments": {"path": path.display().to_string()}}),
            );
            assert_eq!(r["result"]["isError"], false, "{ext}: {r}");
            assert!(
                std::fs::metadata(&path)
                    .map(|m| m.len() > 0)
                    .unwrap_or(false),
                "{ext}"
            );
        }
        // The saved .cdr opens again.
        let r = call(
            &mut app,
            3,
            "tools/call",
            json!({"name": "open", "arguments": {"path": dir.join("out.cdr").display().to_string()}}),
        );
        assert_eq!(r["result"]["isError"], false, "{r}");
        let d: Value =
            serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(d["pages"][0]["layers"][0]["objects"][0]["type"], "ellipse");
        // A version of choice, and one that cannot be written.
        let v21 = dir.join("v21.cdr");
        let r = call(
            &mut app,
            4,
            "tools/call",
            json!({"name": "save", "arguments": {"path": v21.display().to_string(), "version": 21}}),
        );
        assert_eq!(r["result"]["isError"], false, "{r}");
        let bytes = std::fs::read(&v21).unwrap_or_default();
        let (_, report) = tracedraw_cdr::open_bytes(&bytes, "v21").expect("opens");
        assert_eq!(report.version.map(|v| v.0), Some(21));
        assert_eq!(app.cdr_save_version(), 21);
        for bad in [json!(3), json!(6), json!(28), json!("x"), json!(-1)] {
            let r = call(
                &mut app,
                5,
                "tools/call",
                json!({"name": "save", "arguments": {"path": dir.join("bad.cdr").display().to_string(), "version": bad}}),
            );
            assert_eq!(r["result"]["isError"], true, "{bad}: {r}");
        }
        assert!(!dir.join("bad.cdr").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
