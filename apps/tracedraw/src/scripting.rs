//! JavaScript automation (Scripts panel, Tools > Scripts). The object
//! model: `Application.ActiveDocument`,
//! `ActivePage`, `ActiveLayer`, `Shapes`, `Shape` with `Fill`, `Outline`,
//! `Move`, `Rotate`, `Stretch`, `Delete`, `Duplicate`, `ConvertToCurves`,
//! plus `CreateRectangle`, `CreateEllipse`, `CreateArtisticText`,
//! `CreateCurve`, `CreatePolygon`. Recorded macros are plain scripts.
//!
//! Implementation: one native dispatcher `__td(method, argsJson)` works on
//! a snapshot of the document and records commands; the JS prelude builds
//! the object model on top. After the script the commands run as one
//! undo step.

use crate::app::App;
use boa_engine::{
    js_string, native_function::NativeFunction, object::ObjectInitializer, Context, JsData,
    JsResult, JsValue, Source,
};
use serde_json::{json, Value};
use std::cell::RefCell;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, Point, Rect},
    Color, Command, Document, Fill, LayerId, PageId, ShapeId, Stroke, TextSpan,
};

struct HostInner {
    doc: Document,
    page: PageId,
    layer: Option<LayerId>,
    selection: Vec<ShapeId>,
    commands: Vec<Command>,
    output: Vec<String>,
    next_id: u64,
}

struct Host(RefCell<HostInner>);

impl boa_engine::gc::Finalize for Host {}
// The host holds no garbage-collected values.
unsafe impl boa_engine::gc::Trace for Host {
    boa_engine::gc::empty_trace!();
}
impl JsData for Host {}

const PRELUDE: &str = r#"
function __call(m, a) { return JSON.parse(__td(m, JSON.stringify(a || {}))); }
class Color {
  constructor(v) { this.v = v || {model:'rgb', r:0, g:0, b:0}; }
  static RGB(r, g, b) { return new Color({model:'rgb', r:r/255, g:g/255, b:b/255}); }
  static CMYK(c, m, y, k) { return new Color({model:'cmyk', c:c/100, m:m/100, y:y/100, k:k/100}); }
  static Gray(v) { return new Color({model:'gray', v:v/100}); }
  get Hex() { return __call('color_hex', {color:this.v}).hex; }
}
class Fill {
  constructor(id) { this.id = id; }
  get Type() { return __call('fill_type', {id:this.id}).type; }
  ApplyNoFill() { __call('set_fill', {id:this.id, fill:{fill:'none'}}); }
  ApplyUniformFill(color) { __call('set_fill', {id:this.id, fill:{fill:'solid', ...color.v}}); }
  ApplyFountainFill(from, to, angle) { __call('set_fountain', {id:this.id, from:from.v, to:to.v, angle:angle||0}); }
  get UniformColor() { return new Color(__call('fill_color', {id:this.id}).color); }
}
class Outline {
  constructor(id) { this.id = id; }
  SetNoOutline() { __call('set_outline', {id:this.id, outline:null}); }
  SetProperties(width, color) { __call('set_outline_props', {id:this.id, width:width, color:color ? color.v : null}); }
  get Width() { return __call('outline_width', {id:this.id}).width; }
  set Width(w) { __call('set_outline_props', {id:this.id, width:w, color:null}); }
  get Color() { return new Color(__call('outline_color', {id:this.id}).color); }
  set Color(c) { __call('set_outline_props', {id:this.id, width:null, color:c.v}); }
}
class Shape {
  constructor(id) { this.id = id; this.Fill = new Fill(id); this.Outline = new Outline(id); }
  get Name() { return __call('shape_info', {id:this.id}).name; }
  set Name(n) { __call('set_name', {id:this.id, name:n}); }
  get Type() { return __call('shape_info', {id:this.id}).type; }
  get PositionX() { return __call('shape_info', {id:this.id}).x; }
  get PositionY() { return __call('shape_info', {id:this.id}).y; }
  get SizeWidth() { return __call('shape_info', {id:this.id}).w; }
  get SizeHeight() { return __call('shape_info', {id:this.id}).h; }
  get CenterX() { const i = __call('shape_info', {id:this.id}); return i.x + i.w/2; }
  get CenterY() { const i = __call('shape_info', {id:this.id}); return i.y + i.h/2; }
  get Text() { return __call('shape_info', {id:this.id}).text; }
  set Text(t) { __call('set_text', {id:this.id, text:t}); }
  SetPosition(x, y) { __call('set_position', {id:this.id, x:x, y:y}); }
  SetSize(w, h) { __call('set_size', {id:this.id, w:w, h:h}); }
  Move(dx, dy) { __call('move', {id:this.id, dx:dx, dy:dy}); }
  Rotate(deg) { __call('rotate', {id:this.id, deg:deg}); }
  Stretch(sx, sy) { __call('stretch', {id:this.id, sx:sx, sy:sy===undefined?sx:sy}); }
  Skew(ax, ay) { __call('skew', {id:this.id, ax:ax, ay:ay||0}); }
  Delete() { __call('delete', {id:this.id}); }
  Duplicate(dx, dy) { return new Shape(__call('duplicate', {id:this.id, dx:dx||0, dy:dy||0}).id); }
  ConvertToCurves() { __call('convert_to_curves', {id:this.id}); return this; }
  AddToSelection() { __call('select', {ids:[this.id], add:true}); }
  CreateSelection() { __call('select', {ids:[this.id], add:false}); }
  OrderToFront() { __call('order', {id:this.id, op:'front'}); }
  OrderToBack() { __call('order', {id:this.id, op:'back'}); }
  SetOpacity(pct) { __call('set_opacity', {id:this.id, opacity:1 - pct/100}); }
  get Transparency() { return __call('shape_info', {id:this.id}).transparency; }
  AddDropShadow(dx, dy, opacity) { __call('drop_shadow', {id:this.id, dx:dx, dy:dy, opacity:opacity===undefined?50:opacity}); }
  get Curve() { const id = this.id; return { Nodes: { get Count() { return __call('node_count', {id:id}).count; } } }; }
}
class ShapeRange {
  constructor(ids) { this.ids = ids; }
  get Count() { return this.ids.length; }
  Item(i) { return new Shape(this.ids[i-1]); }
  get Shapes() { return this.ids.map(id => new Shape(id)); }
  forEach(f) { this.Shapes.forEach(f); }
  Delete() { this.ids.forEach(id => __call('delete', {id:id})); }
  Move(dx, dy) { this.ids.forEach(id => __call('move', {id:id, dx:dx, dy:dy})); }
  Group() { return new Shape(__call('group', {ids:this.ids}).id); }
  AlignLeft() { __call('align', {ids:this.ids, how:'left'}); }
  AlignRight() { __call('align', {ids:this.ids, how:'right'}); }
  AlignTop() { __call('align', {ids:this.ids, how:'top'}); }
  AlignBottom() { __call('align', {ids:this.ids, how:'bottom'}); }
  AlignCentersHorizontally() { __call('align', {ids:this.ids, how:'center_h'}); }
  AlignCentersVertically() { __call('align', {ids:this.ids, how:'center_v'}); }
  CreateSelection() { __call('select', {ids:this.ids, add:false}); }
}
class Shapes {
  constructor(layer) { this.layer = layer; }
  get ids() { return __call('shapes', {layer:this.layer}).ids; }
  get Count() { return this.ids.length; }
  Item(i) { return new Shape(this.ids[i-1]); }
  get All() { return new ShapeRange(this.ids); }
  FindShape(name) { const id = __call('find', {layer:this.layer, name:name}).id; return id ? new Shape(id) : null; }
  forEach(f) { this.ids.forEach(id => f(new Shape(id))); }
}
class Layer {
  constructor(id) { this.id = id; this.Shapes = new Shapes(id); }
  get Name() { return __call('layer_info', {layer:this.id}).name; }
  set Name(n) { __call('set_layer_name', {layer:this.id, name:n}); }
  get Visible() { return __call('layer_info', {layer:this.id}).visible; }
  set Visible(v) { __call('set_layer_visible', {layer:this.id, visible:v}); }
  Activate() { __call('activate_layer', {layer:this.id}); }
  CreateRectangle(x, y, w, h, radius) { return new Shape(__call('create_rect', {layer:this.id, x:x, y:y, w:w, h:h, radius:radius||0}).id); }
  CreateEllipse(x, y, w, h) { return new Shape(__call('create_ellipse', {layer:this.id, x:x, y:y, w:w, h:h}).id); }
  CreatePolygon(x, y, w, h, points) { return new Shape(__call('create_polygon', {layer:this.id, x:x, y:y, w:w, h:h, points:points||5, sharpness:0}).id); }
  CreateStar(x, y, w, h, points, sharpness) { return new Shape(__call('create_polygon', {layer:this.id, x:x, y:y, w:w, h:h, points:points||5, sharpness:sharpness===undefined?0.5:sharpness}).id); }
  CreateArtisticText(x, y, text, font, size) { return new Shape(__call('create_text', {layer:this.id, x:x, y:y, text:text, font:font||'', size:size||24, frame:null}).id); }
  CreateParagraphText(x, y, w, h, text, font, size) { return new Shape(__call('create_text', {layer:this.id, x:x, y:y, text:text, font:font||'', size:size||12, frame:[w, h]}).id); }
  CreateLineSegment(x1, y1, x2, y2) { return new Shape(__call('create_curve', {layer:this.id, points:[[x1,y1],[x2,y2]], closed:false}).id); }
  CreateCurve(points, closed) { return new Shape(__call('create_curve', {layer:this.id, points:points, closed:!!closed}).id); }
  CreateCurveSegment(points) { return this.CreateCurve(points, false); }
  ImportBitmap(path) { return new Shape(__call('import_bitmap', {layer:this.id, path:path}).id); }
}
class Layers {
  constructor(page) { this.page = page; }
  get ids() { return __call('layers', {page:this.page}).ids; }
  get Count() { return this.ids.length; }
  Item(i) { return new Layer(this.ids[i-1]); }
  forEach(f) { this.ids.forEach(id => f(new Layer(id))); }
  Find(name) { const id = __call('find_layer', {page:this.page, name:name}).id; return id ? new Layer(id) : null; }
}
class Page {
  constructor(id) { this.id = id; this.Layers = new Layers(id); }
  get Name() { return __call('page_info', {page:this.id}).name; }
  get SizeWidth() { return __call('page_info', {page:this.id}).w; }
  get SizeHeight() { return __call('page_info', {page:this.id}).h; }
  SetSize(w, h) { __call('set_page_size', {page:this.id, w:w, h:h}); }
  get ActiveLayer() { return new Layer(__call('active_layer', {page:this.id}).id); }
  get Shapes() { const page = this.id; return { get All() { return new ShapeRange(__call('page_shapes', {page:page}).ids); }, get Count() { return __call('page_shapes', {page:page}).ids.length; } }; }
  CreateLayer(name) { return new Layer(__call('create_layer', {page:this.id, name:name}).id); }
  Activate() { __call('activate_page', {page:this.id}); }
  get Index() { return __call('page_info', {page:this.id}).index; }
}
class Pages {
  get ids() { return __call('pages', {}).ids; }
  get Count() { return this.ids.length; }
  Item(i) { return new Page(this.ids[i-1]); }
  forEach(f) { this.ids.forEach(id => f(new Page(id))); }
}
class Document {
  constructor() { this.Pages = new Pages(); }
  get Name() { return __call('doc_info', {}).title; }
  get ActivePage() { return new Page(__call('doc_info', {}).page); }
  get ActiveLayer() { return this.ActivePage.ActiveLayer; }
  get Selection() { return new ShapeRange(__call('selection', {}).ids); }
  get SelectionRange() { return this.Selection; }
  ClearSelection() { __call('select', {ids:[], add:false}); }
  SelectAll() { __call('select_all', {}); }
  AddPages(n) { for (let i = 0; i < (n||1); i++) __call('add_page', {}); return this.Pages.Item(this.Pages.Count); }
  CreateRectangle(x, y, w, h, r) { return this.ActiveLayer.CreateRectangle(x, y, w, h, r); }
  CreateEllipse(x, y, w, h) { return this.ActiveLayer.CreateEllipse(x, y, w, h); }
  CreateArtisticText(x, y, t, f, s) { return this.ActiveLayer.CreateArtisticText(x, y, t, f, s); }
  Export(path, format) { __call('export', {path:path, format:format||''}); }
  BeginCommandGroup(name) { }
  EndCommandGroup() { }
}
const Application = {
  get ActiveDocument() { return new Document(); },
  get ActivePage() { return new Document().ActivePage; },
  get ActiveLayer() { return new Document().ActiveLayer; },
  get ActiveShape() { const s = new Document().Selection; return s.Count ? s.Item(1) : null; },
  get ActiveSelection() { return new Document().Selection; },
  get Version() { return __call('version', {}).version; },
  get Name() { return 'TraceDraw'; },
  CreateColor: Color,
};
const ActiveDocument = Application.ActiveDocument;
const ActivePage = Application.ActivePage;
const ActiveLayer = Application.ActiveLayer;
const ActiveSelection = Application.ActiveSelection;
function print(...a) { __call('log', {text: a.map(String).join(' ')}); }
const console = { log: print, error: print, warn: print, info: print };
function MsgBox(t) { print(t); }
"#;

/// Run a script against the app; returns the printed lines.
pub fn run(app: &mut App, src: &str) -> Result<Vec<String>, String> {
    let host = Host(RefCell::new(HostInner {
        doc: app.engine.document().clone(),
        page: app.page,
        layer: app.active_layer(),
        selection: app.selection.clone(),
        commands: Vec::new(),
        output: Vec::new(),
        next_id: app.engine.document().ids().peek_next(),
    }));
    let mut ctx = Context::default();
    ctx.insert_data(host);
    let dispatcher = NativeFunction::from_fn_ptr(dispatch);
    let global = ObjectInitializer::new(&mut ctx).build();
    let _ = global;
    ctx.register_global_callable(js_string!("__td"), 2, dispatcher)
        .map_err(|e| e.to_string())?;
    ctx.eval(Source::from_bytes(PRELUDE))
        .map_err(|e| format!("prelude: {e}"))?;
    let result = ctx.eval(Source::from_bytes(src));
    let host = ctx
        .remove_data::<Host>()
        .ok_or_else(|| "script host lost".to_string())?;
    let inner = host.0.into_inner();
    let mut output = inner.output;
    match result {
        Ok(v) => {
            // Echo the completion value only when nothing was printed,
            // so one-liners such as `ActiveDocument.Pages.Count` show a result.
            if output.is_empty() && !v.is_undefined() {
                output.push(v.display().to_string());
            }
        }
        Err(e) => {
            // Apply what ran before the error.
            apply(app, inner.commands, inner.selection, inner.page);
            return Err(e.to_string());
        }
    }
    apply(app, inner.commands, inner.selection, inner.page);
    Ok(output)
}

fn apply(app: &mut App, commands: Vec<Command>, selection: Vec<ShapeId>, page: PageId) {
    if !commands.is_empty() {
        if let Err(e) = app.engine.run_batch("Script", &commands) {
            app.status = e.to_string();
        }
    }
    app.page = page;
    app.selection = selection
        .into_iter()
        .filter(|id| app.doc().find_shape(*id).is_some())
        .collect();
}

fn dispatch(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let method = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let arg_json = args
        .get(1)
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "{}".into());
    let a: Value = serde_json::from_str(&arg_json).unwrap_or(Value::Null);
    let Some(host) = ctx.get_data::<Host>() else {
        return Err(boa_engine::JsNativeError::error()
            .with_message("no host")
            .into());
    };
    let mut h = host.0.borrow_mut();
    let out = h.call(&method, &a).map_err(|e| {
        boa_engine::JsError::from(boa_engine::JsNativeError::error().with_message(e))
    })?;
    Ok(JsValue::from(js_string!(out.to_string())))
}

fn id_arg(a: &Value, key: &str) -> Result<u64, String> {
    a.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing {key}"))
}

fn num(a: &Value, key: &str, default: f64) -> f64 {
    a.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
}

fn color_from(v: &Value) -> Option<Color> {
    serde_json::from_value(v.clone()).ok()
}

impl HostInner {
    fn run(&mut self, cmd: Command) -> Result<(), String> {
        cmd.apply(&mut self.doc).map_err(|e| e.to_string())?;
        self.commands.push(cmd);
        Ok(())
    }

    fn new_id(&mut self) -> ShapeId {
        // Ids must match what the engine will allocate when the batch replays:
        // the engine allocates from the same source in the same order.
        let id = ShapeId(self.next_id);
        self.next_id += 1;
        id
    }

    fn shape(&self, id: u64) -> Result<Shape, String> {
        self.doc
            .find_shape(ShapeId(id))
            .cloned()
            .ok_or_else(|| format!("no shape {id}"))
    }

    fn layer_for(&self, a: &Value) -> Result<LayerId, String> {
        if let Some(l) = a.get("layer").and_then(|v| v.as_u64()) {
            return Ok(LayerId(l));
        }
        self.layer.ok_or_else(|| "no active layer".to_string())
    }

    fn add(&mut self, layer: LayerId, mut shape: Shape) -> Result<Value, String> {
        shape.id = self.new_id();
        let id = shape.id;
        self.run(Command::AddShape { layer, shape })?;
        Ok(json!({"id": id.raw()}))
    }

    fn call(&mut self, method: &str, a: &Value) -> Result<Value, String> {
        Ok(match method {
            "version" => json!({"version": env!("CARGO_PKG_VERSION")}),
            "log" => {
                self.output.push(
                    a.get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string(),
                );
                json!({})
            }
            "doc_info" => json!({"title": self.doc.title, "page": self.page.raw()}),
            "pages" => {
                json!({"ids": self.doc.pages.iter().map(|p| p.id.raw()).collect::<Vec<_>>()})
            }
            "page_info" => {
                let pid = PageId(id_arg(a, "page")?);
                let (i, p) = self
                    .doc
                    .pages
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.id == pid)
                    .ok_or("no page")?;
                json!({"name": p.name, "w": p.size.width, "h": p.size.height, "index": i + 1})
            }
            "set_page_size" => {
                let page = PageId(id_arg(a, "page")?);
                let size = tracedraw_core::Size::new(num(a, "w", 210.0), num(a, "h", 297.0));
                self.run(Command::ResizePage { page, size })?;
                json!({})
            }
            "activate_page" => {
                self.page = PageId(id_arg(a, "page")?);
                json!({})
            }
            "add_page" => {
                let size = self
                    .doc
                    .page(self.page)
                    .map(|p| p.size)
                    .unwrap_or(tracedraw_core::document::paper::A4);
                self.run(Command::AddPage { name: None, size })?;
                json!({})
            }
            "layers" => {
                let p = self
                    .doc
                    .page(PageId(id_arg(a, "page")?))
                    .map_err(|e| e.to_string())?;
                json!({"ids": p.layers.iter().map(|l| l.id.raw()).collect::<Vec<_>>()})
            }
            "layer_info" => {
                let l = self
                    .doc
                    .layer(LayerId(id_arg(a, "layer")?))
                    .map_err(|e| e.to_string())?;
                json!({"name": l.name, "visible": l.visible, "locked": l.locked})
            }
            "set_layer_name" => {
                let layer = LayerId(id_arg(a, "layer")?);
                let name = a
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                self.run(Command::RenameLayer { layer, name })?;
                json!({})
            }
            "set_layer_visible" => {
                let layer = LayerId(id_arg(a, "layer")?);
                let visible = a.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                self.run(Command::SetLayerVisible { layer, visible })?;
                json!({})
            }
            "activate_layer" => {
                self.layer = Some(LayerId(id_arg(a, "layer")?));
                json!({})
            }
            "active_layer" => {
                let p = self
                    .doc
                    .page(PageId(id_arg(a, "page")?))
                    .map_err(|e| e.to_string())?;
                let id = self
                    .layer
                    .filter(|l| p.layers.iter().any(|x| x.id == *l))
                    .or_else(|| p.layers.last().map(|l| l.id))
                    .ok_or("no layer")?;
                json!({"id": id.raw()})
            }
            "create_layer" => {
                let page = PageId(id_arg(a, "page")?);
                let name = a
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Layer")
                    .to_string();
                self.run(Command::AddLayer { page, name })?;
                let id = self
                    .doc
                    .page(page)
                    .ok()
                    .and_then(|p| p.layers.last().map(|l| l.id))
                    .ok_or("no layer")?;
                json!({"id": id.raw()})
            }
            "find_layer" => {
                let p = self
                    .doc
                    .page(PageId(id_arg(a, "page")?))
                    .map_err(|e| e.to_string())?;
                let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                json!({"id": p.layers.iter().find(|l| l.name == name).map(|l| l.id.raw())})
            }
            "shapes" => {
                let l = self
                    .doc
                    .layer(LayerId(id_arg(a, "layer")?))
                    .map_err(|e| e.to_string())?;
                json!({"ids": l.shapes.iter().map(|s| s.id.raw()).collect::<Vec<_>>()})
            }
            "page_shapes" => {
                let p = self
                    .doc
                    .page(PageId(id_arg(a, "page")?))
                    .map_err(|e| e.to_string())?;
                json!({"ids": p.layers.iter().flat_map(|l| l.shapes.iter().map(|s| s.id.raw())).collect::<Vec<_>>()})
            }
            "find" => {
                let l = self
                    .doc
                    .layer(LayerId(id_arg(a, "layer")?))
                    .map_err(|e| e.to_string())?;
                let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                json!({"id": l.shapes.iter().find(|s| s.name.as_deref() == Some(name)).map(|s| s.id.raw())})
            }
            "selection" => {
                json!({"ids": self.selection.iter().map(|s| s.raw()).collect::<Vec<_>>()})
            }
            "select" => {
                let ids: Vec<ShapeId> = a
                    .get("ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64()).map(ShapeId).collect())
                    .unwrap_or_default();
                let add = a.get("add").and_then(|v| v.as_bool()).unwrap_or(false);
                if add {
                    for id in ids {
                        if !self.selection.contains(&id) {
                            self.selection.push(id);
                        }
                    }
                } else {
                    self.selection = ids;
                }
                json!({})
            }
            "select_all" => {
                let p = self.doc.page(self.page).map_err(|e| e.to_string())?;
                self.selection = p
                    .layers
                    .iter()
                    .flat_map(|l| l.shapes.iter().map(|s| s.id))
                    .collect();
                json!({})
            }
            "shape_info" => {
                let s = self.shape(id_arg(a, "id")?)?;
                let b = s.bounds();
                let text = match &s.kind {
                    ShapeKind::Text { spans, .. } => {
                        spans.iter().map(|x| x.text.as_str()).collect::<String>()
                    }
                    _ => String::new(),
                };
                json!({
                    "name": s.name.clone().unwrap_or_default(),
                    "type": crate::ui::dockers::kind_id(&s.kind),
                    "x": b.x0, "y": b.y0, "w": b.width(), "h": b.height(),
                    "text": text,
                    "transparency": ((1.0 - s.opacity) * 100.0).round(),
                })
            }
            "node_count" => {
                let s = self.shape(id_arg(a, "id")?)?;
                let n = s
                    .local_path()
                    .elements()
                    .iter()
                    .filter(|e| !matches!(e, kurbo::PathEl::ClosePath))
                    .count();
                json!({"count": n})
            }
            "set_name" => {
                let shape = ShapeId(id_arg(a, "id")?);
                let name = a
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
                self.run(Command::SetShapeName { shape, name })?;
                json!({})
            }
            "set_text" => {
                let id = id_arg(a, "id")?;
                let s = self.shape(id)?;
                if let ShapeKind::Text {
                    spans,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                } = s.kind
                {
                    let style = spans
                        .first()
                        .cloned()
                        .unwrap_or_else(|| TextSpan::new("", "Arial", 24.0));
                    let text = a
                        .get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.run(Command::SetShapeKind {
                        shape: ShapeId(id),
                        kind: ShapeKind::Text {
                            spans: vec![TextSpan { text, ..style }],
                            origin,
                            frame,
                            align,
                            para,
                            on_path,
                        },
                    })?;
                }
                json!({})
            }
            "set_position" => {
                let id = id_arg(a, "id")?;
                let s = self.shape(id)?;
                let b = s.bounds();
                let d = Point::new(num(a, "x", b.x0), num(a, "y", b.y0)) - b.origin();
                self.run(Command::TransformShapes {
                    shapes: vec![ShapeId(id)],
                    transform: Affine::translate(d),
                })?;
                json!({})
            }
            "set_size" => {
                let id = id_arg(a, "id")?;
                let s = self.shape(id)?;
                let b = s.bounds();
                let sx = num(a, "w", b.width()) / b.width().max(1e-9);
                let sy = num(a, "h", b.height()) / b.height().max(1e-9);
                let t = Affine::translate(b.origin().to_vec2())
                    * Affine::scale_non_uniform(sx, sy)
                    * Affine::translate(-b.origin().to_vec2());
                self.run(Command::TransformShapes {
                    shapes: vec![ShapeId(id)],
                    transform: t,
                })?;
                json!({})
            }
            "move" => {
                let id = ShapeId(id_arg(a, "id")?);
                self.run(Command::TransformShapes {
                    shapes: vec![id],
                    transform: Affine::translate((num(a, "dx", 0.0), num(a, "dy", 0.0))),
                })?;
                json!({})
            }
            "rotate" => {
                let id = id_arg(a, "id")?;
                let c = self.shape(id)?.bounds().center();
                self.run(Command::TransformShapes {
                    shapes: vec![ShapeId(id)],
                    transform: Affine::rotate_about(num(a, "deg", 0.0).to_radians(), c),
                })?;
                json!({})
            }
            "stretch" => {
                let id = id_arg(a, "id")?;
                let c = self.shape(id)?.bounds().center();
                let t = Affine::translate(c.to_vec2())
                    * Affine::scale_non_uniform(num(a, "sx", 1.0), num(a, "sy", 1.0))
                    * Affine::translate(-c.to_vec2());
                self.run(Command::TransformShapes {
                    shapes: vec![ShapeId(id)],
                    transform: t,
                })?;
                json!({})
            }
            "skew" => {
                let id = id_arg(a, "id")?;
                let c = self.shape(id)?.bounds().center();
                let t = Affine::translate(c.to_vec2())
                    * Affine::skew(
                        num(a, "ax", 0.0).to_radians().tan(),
                        num(a, "ay", 0.0).to_radians().tan(),
                    )
                    * Affine::translate(-c.to_vec2());
                self.run(Command::TransformShapes {
                    shapes: vec![ShapeId(id)],
                    transform: t,
                })?;
                json!({})
            }
            "delete" => {
                let id = ShapeId(id_arg(a, "id")?);
                self.run(Command::DeleteShapes { shapes: vec![id] })?;
                self.selection.retain(|s| *s != id);
                json!({})
            }
            "duplicate" => {
                let id = id_arg(a, "id")?;
                let mut s = self.shape(id)?;
                let (layer, _) = self.doc.locate(ShapeId(id)).map_err(|e| e.to_string())?;
                s.transform =
                    Affine::translate((num(a, "dx", 0.0), num(a, "dy", 0.0))) * s.transform;
                self.add(layer, s)?
            }
            "convert_to_curves" => {
                let id = id_arg(a, "id")?;
                let s = self.shape(id)?;
                let path = s.local_path();
                let closed = path
                    .elements()
                    .iter()
                    .any(|e| matches!(e, kurbo::PathEl::ClosePath));
                self.run(Command::SetShapeKind {
                    shape: ShapeId(id),
                    kind: ShapeKind::Path { path, closed },
                })?;
                json!({})
            }
            "order" => {
                let id = ShapeId(id_arg(a, "id")?);
                let (layer, _) = self.doc.locate(id).map_err(|e| e.to_string())?;
                let n = self.doc.layer(layer).map(|l| l.shapes.len()).unwrap_or(1);
                let index = if a.get("op").and_then(|o| o.as_str()) == Some("front") {
                    n - 1
                } else {
                    0
                };
                self.run(Command::Reorder {
                    shape: id,
                    layer,
                    index,
                })?;
                json!({})
            }
            "set_opacity" => {
                let id = ShapeId(id_arg(a, "id")?);
                self.run(Command::SetOpacity {
                    shapes: vec![id],
                    opacity: num(a, "opacity", 1.0).clamp(0.0, 1.0),
                })?;
                json!({})
            }
            "drop_shadow" => {
                let id = ShapeId(id_arg(a, "id")?);
                let shadow = tracedraw_core::Shadow {
                    offset: tracedraw_core::Vec2::new(num(a, "dx", 3.0), num(a, "dy", -3.0)),
                    opacity: num(a, "opacity", 50.0) / 100.0,
                    ..Default::default()
                };
                self.run(Command::SetShadow {
                    shapes: vec![id],
                    shadow: Some(shadow),
                })?;
                json!({})
            }
            "group" => {
                let ids: Vec<ShapeId> = a
                    .get("ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64()).map(ShapeId).collect())
                    .unwrap_or_default();
                if ids.len() < 2 {
                    return Err("group needs two objects".into());
                }
                let gid = self.new_id();
                self.run(Command::Group { shapes: ids })?;
                json!({"id": gid.raw()})
            }
            "align" => {
                let ids: Vec<ShapeId> = a
                    .get("ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64()).map(ShapeId).collect())
                    .unwrap_or_default();
                let shapes: Vec<Shape> = ids
                    .iter()
                    .filter_map(|id| self.doc.find_shape(*id).cloned())
                    .collect();
                let Some(target) = shapes.last().map(|s| s.bounds()) else {
                    return Ok(json!({}));
                };
                let how = a.get("how").and_then(|h| h.as_str()).unwrap_or("left");
                for s in &shapes {
                    let b = s.bounds();
                    let d = match how {
                        "left" => (target.x0 - b.x0, 0.0),
                        "right" => (target.x1 - b.x1, 0.0),
                        "top" => (0.0, target.y1 - b.y1),
                        "bottom" => (0.0, target.y0 - b.y0),
                        "center_h" => (target.center().x - b.center().x, 0.0),
                        _ => (0.0, target.center().y - b.center().y),
                    };
                    self.run(Command::TransformShapes {
                        shapes: vec![s.id],
                        transform: Affine::translate(d),
                    })?;
                }
                json!({})
            }
            "fill_type" => {
                let s = self.shape(id_arg(a, "id")?)?;
                json!({"type": match s.fill { Fill::None => "none", Fill::Solid(_) => "uniform", Fill::Fountain(_) => "fountain", Fill::Pattern(_) => "pattern", Fill::Texture(_) => "texture", Fill::Mesh(_) => "mesh" }})
            }
            "fill_color" => {
                let s = self.shape(id_arg(a, "id")?)?;
                json!({"color": s.fill.preview_color().unwrap_or(Color::BLACK)})
            }
            "set_fill" => {
                let id = ShapeId(id_arg(a, "id")?);
                let fill: Fill =
                    serde_json::from_value(a.get("fill").cloned().unwrap_or(Value::Null))
                        .map_err(|e| e.to_string())?;
                self.run(Command::SetFill {
                    shapes: vec![id],
                    fill,
                })?;
                json!({})
            }
            "set_fountain" => {
                let id = ShapeId(id_arg(a, "id")?);
                let from = a.get("from").and_then(color_from).unwrap_or(Color::BLACK);
                let to = a.get("to").and_then(color_from).unwrap_or(Color::WHITE);
                self.run(Command::SetFill {
                    shapes: vec![id],
                    fill: Fill::linear(from, to, num(a, "angle", 0.0)),
                })?;
                json!({})
            }
            "outline_width" => {
                let s = self.shape(id_arg(a, "id")?)?;
                json!({"width": s.stroke.as_ref().map(|st| st.width).unwrap_or(0.0)})
            }
            "outline_color" => {
                let s = self.shape(id_arg(a, "id")?)?;
                json!({"color": s.stroke.as_ref().map(|st| st.color).unwrap_or(Color::BLACK)})
            }
            "set_outline" => {
                let id = ShapeId(id_arg(a, "id")?);
                self.run(Command::SetStroke {
                    shapes: vec![id],
                    stroke: None,
                })?;
                json!({})
            }
            "set_outline_props" => {
                let id = id_arg(a, "id")?;
                let s = self.shape(id)?;
                let mut st = s.stroke.clone().unwrap_or_default();
                if let Some(w) = a.get("width").and_then(|w| w.as_f64()) {
                    st.width = w;
                }
                if let Some(c) = a.get("color").and_then(color_from) {
                    st.color = c;
                }
                self.run(Command::SetStroke {
                    shapes: vec![ShapeId(id)],
                    stroke: Some(st),
                })?;
                json!({})
            }
            "color_hex" => {
                json!({"hex": a.get("color").and_then(color_from).unwrap_or(Color::BLACK).to_hex()})
            }
            "create_rect" => {
                let layer = self.layer_for(a)?;
                let (x, y, w, h) = (
                    num(a, "x", 0.0),
                    num(a, "y", 0.0),
                    num(a, "w", 10.0),
                    num(a, "h", 10.0),
                );
                let mut s = Shape::new(
                    ShapeId(0),
                    ShapeKind::Rect {
                        rect: Rect::new(x, y, x + w, y + h),
                        radius: num(a, "radius", 0.0),
                        corners: None,
                    },
                );
                s.stroke = Some(Stroke::default());
                self.add(layer, s)?
            }
            "create_ellipse" => {
                let layer = self.layer_for(a)?;
                let (x, y, w, h) = (
                    num(a, "x", 0.0),
                    num(a, "y", 0.0),
                    num(a, "w", 10.0),
                    num(a, "h", 10.0),
                );
                let s = Shape::new(
                    ShapeId(0),
                    ShapeKind::Ellipse {
                        rect: Rect::new(x, y, x + w, y + h),
                        arc: None,
                    },
                );
                self.add(layer, s)?
            }
            "create_polygon" => {
                let layer = self.layer_for(a)?;
                let (x, y, w, h) = (
                    num(a, "x", 0.0),
                    num(a, "y", 0.0),
                    num(a, "w", 10.0),
                    num(a, "h", 10.0),
                );
                let s = Shape::new(
                    ShapeId(0),
                    ShapeKind::Polygon {
                        rect: Rect::new(x, y, x + w, y + h),
                        points: num(a, "points", 5.0) as u32,
                        sharpness: num(a, "sharpness", 0.0),
                        complex: None,
                    },
                );
                self.add(layer, s)?
            }
            "create_text" => {
                let layer = self.layer_for(a)?;
                let text = a
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let font = a
                    .get("font")
                    .and_then(|f| f.as_str())
                    .filter(|f| !f.is_empty())
                    .unwrap_or("Arial")
                    .to_string();
                let frame = a.get("frame").and_then(|f| f.as_array()).and_then(|f| {
                    Some(tracedraw_core::Size::new(
                        f.first()?.as_f64()?,
                        f.get(1)?.as_f64()?,
                    ))
                });
                let mut s = Shape::new(
                    ShapeId(0),
                    ShapeKind::Text {
                        spans: vec![TextSpan::new(text, font, num(a, "size", 24.0))],
                        origin: Point::new(num(a, "x", 0.0), num(a, "y", 0.0)),
                        frame,
                        align: tracedraw_core::TextAlign::Left,
                        para: Default::default(),
                        on_path: None,
                    },
                );
                s.fill = Fill::Solid(Color::BLACK);
                s.stroke = None;
                self.add(layer, s)?
            }
            "create_curve" => {
                let layer = self.layer_for(a)?;
                let pts: Vec<Point> = a
                    .get("points")
                    .and_then(|p| p.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| {
                                let xy = p.as_array()?;
                                Some(Point::new(xy.first()?.as_f64()?, xy.get(1)?.as_f64()?))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                if pts.len() < 2 {
                    return Err("a curve needs at least two points".into());
                }
                let closed = a.get("closed").and_then(|c| c.as_bool()).unwrap_or(false);
                let path = tracedraw_core::geometry::polyline_path(&pts, closed);
                let s = Shape::new(ShapeId(0), ShapeKind::Path { path, closed });
                self.add(layer, s)?
            }
            "import_bitmap" => {
                let layer = self.layer_for(a)?;
                let path = a
                    .get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("missing path")?;
                let img = crate::files::open_image(path)
                    .map_err(|e| e.to_string())?
                    .to_rgba8();
                let (w, h) = img.dimensions();
                let png = crate::bitmap_fx::encode(&img).ok_or("encode failed")?;
                let mm_w = w as f64 * 25.4 / 96.0;
                let mm_h = h as f64 * 25.4 / 96.0;
                let s = Shape::new(
                    ShapeId(0),
                    ShapeKind::Bitmap {
                        rect: Rect::new(0.0, 0.0, mm_w, mm_h),
                        width_px: w,
                        height_px: h,
                        png,
                        fx: None,
                    },
                );
                self.add(layer, s)?
            }
            "export" => {
                let path = a
                    .get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("missing path")?;
                let p = std::path::Path::new(path);
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let idx = self
                    .doc
                    .pages
                    .iter()
                    .position(|pg| pg.id == self.page)
                    .unwrap_or(0);
                let data = match ext.as_str() {
                    "svg" => tracedraw_io::svg::page_to_svg(&self.doc, idx).into_bytes(),
                    "pdf" => tracedraw_io::pdf::document_to_pdf(&self.doc),
                    "tdraw" => self.doc.to_json().map_err(|e| e.to_string())?.into_bytes(),
                    "png" => {
                        let pm = tracedraw_render::render_page_image(&self.doc, self.page, 300.0)
                            .ok_or("render failed")?;
                        pm.encode_png().map_err(|e| e.to_string())?
                    }
                    other => return Err(format!("unsupported export format: {other}")),
                };
                crate::files::write(p, data).map_err(|e| e.to_string())?;
                json!({})
            }
            other => return Err(format!("unknown method {other}")),
        })
    }
}

/// Turn recorded commands into a script the user can edit and replay.
pub fn commands_to_script(cmds: &[Command]) -> String {
    let mut out = String::from("// Recorded macro\nconst doc = Application.ActiveDocument;\nconst layer = doc.ActiveLayer;\n");
    for c in cmds {
        match c {
            Command::AddShape { shape, .. } => {
                let b = shape.bounds();
                match &shape.kind {
                    ShapeKind::Rect { radius, .. } => out.push_str(&format!(
                        "layer.CreateRectangle({:.3}, {:.3}, {:.3}, {:.3}, {:.3});\n",
                        b.x0,
                        b.y0,
                        b.width(),
                        b.height(),
                        radius
                    )),
                    ShapeKind::Ellipse { .. } => out.push_str(&format!(
                        "layer.CreateEllipse({:.3}, {:.3}, {:.3}, {:.3});\n",
                        b.x0,
                        b.y0,
                        b.width(),
                        b.height()
                    )),
                    ShapeKind::Polygon {
                        points, sharpness, ..
                    } => out.push_str(&format!(
                        "layer.CreateStar({:.3}, {:.3}, {:.3}, {:.3}, {}, {:.2});\n",
                        b.x0,
                        b.y0,
                        b.width(),
                        b.height(),
                        points,
                        sharpness
                    )),
                    ShapeKind::Text { spans, origin, .. } => {
                        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                        let font = spans
                            .first()
                            .map(|s| s.font_family.clone())
                            .unwrap_or_default();
                        let size = spans.first().map(|s| s.size_pt).unwrap_or(24.0);
                        out.push_str(&format!(
                            "layer.CreateArtisticText({:.3}, {:.3}, {}, {}, {});\n",
                            origin.x,
                            origin.y,
                            serde_json::to_string(&text).unwrap_or_default(),
                            serde_json::to_string(&font).unwrap_or_default(),
                            size
                        ));
                    }
                    _ => out.push_str(&format!("// {} (not scriptable yet)\n", c.label())),
                }
            }
            Command::TransformShapes { shapes, transform } => {
                let [a, b, _, _, e, f] = transform.as_coeffs();
                let deg = b.atan2(a).to_degrees();
                for s in shapes {
                    if deg.abs() > 1e-6 {
                        out.push_str(&format!("doc.Shape({}).Rotate({:.3});\n", s.raw(), deg));
                    } else {
                        out.push_str(&format!(
                            "new Shape({}).Move({:.3}, {:.3});\n",
                            s.raw(),
                            e,
                            f
                        ));
                    }
                }
            }
            Command::SetFill { shapes, fill } => {
                let j = serde_json::to_string(fill).unwrap_or_default();
                for s in shapes {
                    out.push_str(&format!(
                        "__call('set_fill', {{id:{}, fill:{j}}});\n",
                        s.raw()
                    ));
                }
            }
            Command::SetStroke { shapes, stroke } => {
                for s in shapes {
                    match stroke {
                        None => out
                            .push_str(&format!("new Shape({}).Outline.SetNoOutline();\n", s.raw())),
                        Some(st) => out.push_str(&format!(
                            "new Shape({}).Outline.SetProperties({:.3}, new Color({}));\n",
                            s.raw(),
                            st.width,
                            serde_json::to_string(&st.color).unwrap_or_default()
                        )),
                    }
                }
            }
            Command::DeleteShapes { shapes } => {
                for s in shapes {
                    out.push_str(&format!("new Shape({}).Delete();\n", s.raw()));
                }
            }
            other => out.push_str(&format!("// {}\n", other.label())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::headless()
    }

    #[test]
    fn script_creates_and_moves_a_rectangle() {
        let mut a = app();
        let out = run(
            &mut a,
            "const r = ActiveLayer.CreateRectangle(10, 20, 30, 40); r.Move(5, 5); r.Fill.ApplyUniformFill(Color.RGB(255,0,0)); print('w=' + r.SizeWidth); r.Name = 'box';",
        )
        .unwrap();
        assert_eq!(out, vec!["w=30".to_string()]);
        let shapes = a.doc().pages[0].layers[0].shapes.clone();
        assert_eq!(shapes.len(), 1);
        let b = shapes[0].bounds();
        assert!((b.x0 - 15.0).abs() < 1e-6 && (b.y0 - 25.0).abs() < 1e-6);
        assert_eq!(shapes[0].name.as_deref(), Some("box"));
        assert_eq!(a.engine.undo_label(), Some("Script"));
    }

    #[test]
    fn script_error_is_reported() {
        let mut a = app();
        let r = run(&mut a, "nosuchthing.foo();");
        assert!(r.is_err());
    }

    #[test]
    fn selection_round_trip() {
        let mut a = app();
        run(
            &mut a,
            "const e = ActiveLayer.CreateEllipse(0,0,10,10); e.CreateSelection();",
        )
        .unwrap();
        assert_eq!(a.selection.len(), 1);
        let out = run(
            &mut a,
            "print(ActiveSelection.Count + ' ' + ActiveSelection.Item(1).Type);",
        )
        .unwrap();
        assert_eq!(out[0], "1 Ellipse");
    }
}
