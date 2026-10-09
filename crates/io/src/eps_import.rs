//! EPS import: a small PostScript interpreter that runs the file and
//! records what it paints.
//!
//! The language core (stacks, dictionaries, procedures, control flow,
//! arithmetic, strings, arrays) is complete enough to run the prologs that
//! drawing programs emit, so their private operators (defined with `def`
//! in the file) work without special cases. The graphics operators build
//! paths, fills, outlines, images, text and shadings into TraceDraw
//! objects. Fonts are not rasterised: `show` makes a text object with the
//! font's name and size, and embedded Type 1 programs (`eexec`) are
//! skipped. Everything is bounded (operations, recursion, sizes) and no
//! input can panic the importer.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracedraw_core::{
    document::{ParagraphStyle, Shape, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _, Size, Vec2},
    id::IdSource,
    style::{Fountain, FountainKind, LineCap, LineJoin, Stop},
    Color, Fill, ShapeId, Stroke, TextAlign,
};

const PT_MM: f64 = 25.4 / 72.0;
const MAX_OPS: u64 = 6_000_000;
const MAX_DEPTH: usize = 200;
const MAX_STACK: usize = 50_000;
const MAX_STRING: usize = 64 << 20;

#[derive(Debug, Clone)]
pub struct Imported {
    pub shapes: Vec<Shape>,
    pub size: Size,
    pub warnings: Vec<String>,
}

/// Does the buffer look like PostScript / EPS?
pub fn is_eps(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(64)];
    head.starts_with(b"%!PS")
        || head.starts_with(&[0xc5, 0xd0, 0xd3, 0xc6])
        || head.starts_with(b"%!")
}

/// Parse an EPS (or plain PostScript) file into shapes.
pub fn parse(bytes: &[u8], ids: &mut IdSource) -> Result<Imported, String> {
    // DOS EPS binary header: PostScript section offset and length.
    let ps: &[u8] = if bytes.starts_with(&[0xc5, 0xd0, 0xd3, 0xc6]) && bytes.len() >= 12 {
        let off = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        bytes
            .get(off..off.saturating_add(len).min(bytes.len()))
            .ok_or("DOS EPS header points outside the file")?
    } else {
        bytes
    };
    if !ps.starts_with(b"%!") && !ps.windows(2).take(1024).any(|w| w == b"%!") {
        return Err("not a PostScript file".into());
    }
    let bbox = bounding_box(ps);
    let mut interp = Interp::new(ps, ids, bbox);
    interp.install_systemdict();
    interp.run_main();
    let warnings = std::mem::take(&mut interp.warnings);
    let shapes = std::mem::take(&mut interp.shapes);
    // Page: the bounding box, or the content bounds.
    let (size, shift) = match bbox {
        Some(b) => (
            Size::new(b.width() * PT_MM, b.height() * PT_MM),
            Vec2::new(-b.x0 * PT_MM, -b.y0 * PT_MM),
        ),
        None => {
            let b = shapes
                .iter()
                .map(|s| s.bounds())
                .reduce(|a, b| a.union(b))
                .unwrap_or(Rect::new(0.0, 0.0, 210.0, 297.0));
            (
                Size::new(b.width().max(1.0), b.height().max(1.0)),
                Vec2::new(-b.x0, -b.y0),
            )
        }
    };
    let mut shapes = shapes;
    if shift != Vec2::ZERO {
        let t = Affine::translate(shift);
        for s in shapes.iter_mut() {
            s.transform = t * s.transform;
        }
    }
    Ok(Imported {
        shapes,
        size,
        warnings,
    })
}

/// `%%HiResBoundingBox` or `%%BoundingBox` in points.
fn bounding_box(ps: &[u8]) -> Option<Rect> {
    let head = String::from_utf8_lossy(&ps[..ps.len().min(64 << 10)]);
    let mut lo: Option<Rect> = None;
    for line in head.lines() {
        let (key, rest) = if let Some(r) = line.strip_prefix("%%HiResBoundingBox:") {
            ("hi", r)
        } else if let Some(r) = line.strip_prefix("%%BoundingBox:") {
            ("lo", r)
        } else {
            continue;
        };
        let v: Vec<f64> = rest
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        if v.len() == 4 && v[2] > v[0] && v[3] > v[1] && v[2] - v[0] < 1e6 {
            let r = Rect::new(v[0], v[1], v[2], v[3]);
            if key == "hi" {
                return Some(r);
            }
            lo = Some(r);
        }
    }
    lo
}

// ----- objects --------------------------------------------------------------------

type Str = Rc<RefCell<Vec<u8>>>;
type Arr = Rc<RefCell<Vec<Obj>>>;
type Dict = Rc<RefCell<DictData>>;

#[derive(Debug, Default)]
struct DictData {
    map: HashMap<Rc<str>, Obj>,
}

#[derive(Debug, Clone)]
enum Obj {
    Null,
    Mark,
    Bool(bool),
    Int(i64),
    Real(f64),
    /// Name; `exec` true = executable (bare), false = literal (/name).
    Name(Rc<str>, bool),
    Str(Str),
    /// Array; `exec` true = procedure.
    Array(Arr, bool),
    Dict(Dict),
    Op(&'static str),
    Save(u32),
    /// A file: the main program (`currentfile`) or a decoding filter on it.
    File(Rc<RefCell<FileObj>>),
    Font(Dict),
}

impl Obj {
    fn type_name(&self) -> &'static str {
        match self {
            Obj::Null => "nulltype",
            Obj::Mark => "marktype",
            Obj::Bool(_) => "booleantype",
            Obj::Int(_) => "integertype",
            Obj::Real(_) => "realtype",
            Obj::Name(..) => "nametype",
            Obj::Str(_) => "stringtype",
            Obj::Array(..) => "arraytype",
            Obj::Dict(_) => "dicttype",
            Obj::Op(_) => "operatortype",
            Obj::Save(_) => "savetype",
            Obj::File(_) => "filetype",
            Obj::Font(_) => "dicttype",
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Obj::Int(i) => Some(*i as f64),
            Obj::Real(r) => Some(*r),
            _ => None,
        }
    }
    fn is_exec(&self) -> bool {
        match self {
            Obj::Name(_, e) => *e,
            Obj::Array(_, e) => *e,
            Obj::Op(_) => true,
            _ => false,
        }
    }
    fn equals(&self, other: &Obj) -> bool {
        match (self, other) {
            (Obj::Null, Obj::Null) | (Obj::Mark, Obj::Mark) => true,
            (Obj::Bool(a), Obj::Bool(b)) => a == b,
            (a, b) if a.num().is_some() && b.num().is_some() => a.num() == b.num(),
            (Obj::Name(a, _), Obj::Name(b, _)) => a == b,
            (Obj::Name(a, _), Obj::Str(b)) | (Obj::Str(b), Obj::Name(a, _)) => {
                a.as_bytes() == b.borrow().as_slice()
            }
            (Obj::Str(a), Obj::Str(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Obj::Array(a, _), Obj::Array(b, _)) => Rc::ptr_eq(a, b),
            (Obj::Dict(a), Obj::Dict(b)) | (Obj::Font(a), Obj::Font(b)) => Rc::ptr_eq(a, b),
            (Obj::Op(a), Obj::Op(b)) => a == b,
            (Obj::Save(a), Obj::Save(b)) => a == b,
            _ => false,
        }
    }
}

fn mk_str(b: &[u8]) -> Obj {
    Obj::Str(Rc::new(RefCell::new(b.to_vec())))
}

fn mk_array(v: Vec<Obj>, exec: bool) -> Obj {
    Obj::Array(Rc::new(RefCell::new(v)), exec)
}

fn mk_dict() -> Dict {
    Rc::new(RefCell::new(DictData::default()))
}

/// A readable byte source: the program text past the current scan point,
/// optionally through a decoding filter.
#[derive(Debug)]
struct FileObj {
    /// Decoded bytes still to read (for filters and string sources).
    buf: Vec<u8>,
    pos: usize,
    /// Reads straight from the main program when true.
    main: bool,
    closed: bool,
    /// A filter on the main file that has not read its data yet: it
    /// decodes from the program position at first use, which is after the
    /// operator that consumes it (as PostScript's lazy filters do).
    pending_filter: Option<Rc<str>>,
}

// ----- scanner -----------------------------------------------------------------

struct Scanner<'a> {
    src: &'a [u8],
    pos: usize,
}

enum Token {
    Obj(Obj),
    ProcStart,
    ProcEnd,
    Eof,
}

impl<'a> Scanner<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() || c == 0 {
                self.pos += 1;
            } else if c == b'%' {
                while let Some(c) = self.peek() {
                    if c == b'\n' || c == b'\r' {
                        break;
                    }
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn is_delim(c: u8) -> bool {
        matches!(
            c,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        ) || c.is_ascii_whitespace()
            || c == 0
    }

    fn next(&mut self) -> Token {
        self.skip_ws();
        let Some(c) = self.peek() else {
            return Token::Eof;
        };
        match c {
            b'{' => {
                self.pos += 1;
                Token::ProcStart
            }
            b'}' => {
                self.pos += 1;
                Token::ProcEnd
            }
            b'[' | b']' => {
                self.pos += 1;
                Token::Obj(Obj::Name(Rc::from(if c == b'[' { "[" } else { "]" }), true))
            }
            b'(' => Token::Obj(self.string()),
            b'<' => {
                if self.src.get(self.pos + 1) == Some(&b'<') {
                    self.pos += 2;
                    Token::Obj(Obj::Name(Rc::from("<<"), true))
                } else if self.src.get(self.pos + 1) == Some(&b'~') {
                    self.pos += 2;
                    Token::Obj(self.ascii85())
                } else {
                    self.pos += 1;
                    Token::Obj(self.hex_string())
                }
            }
            b'>' => {
                if self.src.get(self.pos + 1) == Some(&b'>') {
                    self.pos += 2;
                    Token::Obj(Obj::Name(Rc::from(">>"), true))
                } else {
                    self.pos += 1;
                    self.next()
                }
            }
            b'/' => {
                self.pos += 1;
                let imm = if self.peek() == Some(b'/') {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if Self::is_delim(c) {
                        break;
                    }
                    self.pos += 1;
                }
                let name = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
                let _ = imm;
                Token::Obj(Obj::Name(Rc::from(name.as_str()), false))
            }
            b')' => {
                self.pos += 1;
                self.next()
            }
            _ => {
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if Self::is_delim(c) {
                        break;
                    }
                    self.pos += 1;
                }
                let text = &self.src[start..self.pos];
                let s = String::from_utf8_lossy(text);
                if let Some(n) = parse_number(&s) {
                    Token::Obj(n)
                } else {
                    Token::Obj(Obj::Name(Rc::from(s.as_ref()), true))
                }
            }
        }
    }

    fn string(&mut self) -> Obj {
        // At '('.
        self.pos += 1;
        let mut depth = 1;
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            match c {
                b'\\' => {
                    let Some(e) = self.peek() else { break };
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'\n' => {}
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        v = v * 8 + (d - b'0') as u32;
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(v as u8);
                        }
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(c);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(c);
                }
                _ => out.push(c),
            }
            if out.len() > MAX_STRING {
                break;
            }
        }
        mk_str(&out)
    }

    fn hex_string(&mut self) -> Obj {
        let mut out = Vec::new();
        let mut hi: Option<u8> = None;
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == b'>' {
                break;
            }
            let v = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => continue,
            };
            match hi.take() {
                Some(h) => out.push(h * 16 + v),
                None => hi = Some(v),
            }
        }
        if let Some(h) = hi {
            out.push(h * 16);
        }
        mk_str(&out)
    }

    fn ascii85(&mut self) -> Obj {
        let start = self.pos;
        let mut end = self.src.len();
        let mut i = self.pos;
        while i + 1 < self.src.len() {
            if self.src[i] == b'~' && self.src[i + 1] == b'>' {
                end = i;
                break;
            }
            i += 1;
        }
        self.pos = (end + 2).min(self.src.len());
        mk_str(&decode_ascii85(&self.src[start..end]))
    }

    /// Raw bytes after the current token (for `currentfile` readers).
    fn take_raw(&mut self, n: usize) -> &'a [u8] {
        let start = self.pos.min(self.src.len());
        let end = (start + n).min(self.src.len());
        self.pos = end;
        &self.src[start..end]
    }

    fn read_line(&mut self) -> &'a [u8] {
        let start = self.pos.min(self.src.len());
        let mut end = start;
        while end < self.src.len() && self.src[end] != b'\n' && self.src[end] != b'\r' {
            end += 1;
        }
        let line = &self.src[start..end];
        if end < self.src.len() {
            end += 1;
            if self.src.get(end - 1) == Some(&b'\r') && self.src.get(end) == Some(&b'\n') {
                end += 1;
            }
        }
        self.pos = end;
        line
    }
}

fn parse_number(s: &str) -> Option<Obj> {
    if s.is_empty() {
        return None;
    }
    let first = s.as_bytes()[0];
    if !(first.is_ascii_digit() || matches!(first, b'+' | b'-' | b'.')) {
        return None;
    }
    if let Ok(i) = s.parse::<i64>() {
        return Some(Obj::Int(i));
    }
    if let Some((radix, digits)) = s.split_once('#') {
        if let Ok(r) = radix.parse::<u32>() {
            if (2..=36).contains(&r) {
                if let Ok(v) = i64::from_str_radix(digits, r) {
                    return Some(Obj::Int(v));
                }
            }
        }
        return None;
    }
    let t = s.replace(['e', 'E'], "e");
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(Obj::Real)
}

fn decode_ascii85(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4 / 5);
    let mut tuple = [0u8; 5];
    let mut n = 0;
    for &c in data {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'z' && n == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !(b'!'..=b'u').contains(&c) {
            continue;
        }
        tuple[n] = c - b'!';
        n += 1;
        if n == 5 {
            let v = tuple
                .iter()
                .fold(0u32, |a, d| a.wrapping_mul(85).wrapping_add(*d as u32));
            out.extend_from_slice(&v.to_be_bytes());
            n = 0;
        }
    }
    if n > 0 {
        for t in tuple.iter_mut().skip(n) {
            *t = 84;
        }
        let v = tuple
            .iter()
            .fold(0u32, |a, d| a.wrapping_mul(85).wrapping_add(*d as u32));
        out.extend_from_slice(&v.to_be_bytes()[..n - 1]);
    }
    out
}

fn decode_asciihex(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut hi = None;
    for &c in data {
        let v = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            b'>' => break,
            _ => continue,
        };
        match hi.take() {
            Some(h) => out.push(h * 16 + v),
            None => hi = Some(v),
        }
    }
    if let Some(h) = hi {
        out.push(h * 16);
    }
    out
}

fn decode_runlength(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let l = data[i] as usize;
        i += 1;
        if l == 128 {
            break;
        } else if l < 128 {
            let end = (i + l + 1).min(data.len());
            out.extend_from_slice(&data[i..end]);
            i = end;
        } else {
            if let Some(&b) = data.get(i) {
                out.extend(std::iter::repeat_n(b, 257 - l));
            }
            i += 1;
        }
        if out.len() > MAX_STRING {
            break;
        }
    }
    out
}

fn inflate(data: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut out = Vec::new();
    let mut d = flate2::read::ZlibDecoder::new(data);
    let _ = d.read_to_end(&mut out);
    if out.is_empty() {
        let mut d = flate2::read::DeflateDecoder::new(data);
        let _ = d.read_to_end(&mut out);
    }
    out
}

// ----- graphics state ------------------------------------------------------------

#[derive(Clone)]
struct GState {
    ctm: Affine,
    color: Color,
    /// Pattern fill set with `setpattern`.
    pattern: Option<Fill>,
    line_width: f64,
    cap: LineCap,
    join: LineJoin,
    dash: Vec<f64>,
    clip: Option<BezPath>,
    font: Option<FontInfo>,
    /// Current path in device space (points) and the current point in user space.
    path: BezPath,
    current: Option<Point>,
    start: Option<Point>,
}

#[derive(Clone, Debug)]
struct FontInfo {
    family: String,
    bold: bool,
    italic: bool,
    /// Font matrix (maps glyph space to user space).
    matrix: Affine,
    /// Width per character in glyph units of the matrix when known (Type 3
    /// or Metrics); else 0.5 em.
    widths: Option<Rc<HashMap<u8, f64>>>,
}

impl Default for GState {
    fn default() -> Self {
        GState {
            ctm: Affine::IDENTITY,
            color: Color::BLACK,
            pattern: None,
            line_width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: Vec::new(),
            clip: None,
            font: None,
            path: BezPath::new(),
            current: None,
            start: None,
        }
    }
}

// ----- interpreter -----------------------------------------------------------------

enum Flow {
    Normal,
    Exit,
    Stop,
}

struct Interp<'a> {
    scanner: Scanner<'a>,
    ids: &'a mut IdSource,
    stack: Vec<Obj>,
    dicts: Vec<Dict>,
    systemdict: Dict,
    userdict: Dict,
    globaldict: Dict,
    gs: GState,
    gstack: Vec<GState>,
    save_stack: Vec<(u32, Vec<GState>, GState)>,
    next_save: u32,
    ops: u64,
    depth: usize,
    shapes: Vec<Shape>,
    warnings: Vec<String>,
    fonts: HashMap<Rc<str>, Dict>,
    rand_state: u64,
    /// Device transform: PostScript points, y up (same as ours) to mm.
    base: Affine,
    bbox: Option<Rect>,
    /// Components of the current colour space (setcolor operands).
    color_space: usize,
    color_space_obj: Option<Obj>,
    /// Indexed colour space lookup (RGB triples) when one is current.
    indexed_palette: Option<Vec<u8>>,
    /// Separation / DeviceN tint: 1 = full ink.
    separation: bool,
    /// Files created by a DCTDecode filter (whole-stream JPEG sources).
    jpeg_files: Vec<usize>,
}

impl<'a> Interp<'a> {
    fn new(src: &'a [u8], ids: &'a mut IdSource, bbox: Option<Rect>) -> Self {
        let systemdict = mk_dict();
        let userdict = mk_dict();
        let globaldict = mk_dict();
        let base = Affine::scale(PT_MM);
        let mut gs = GState::default();
        gs.ctm = base;
        Interp {
            scanner: Scanner { src, pos: 0 },
            ids,
            stack: Vec::new(),
            dicts: vec![systemdict.clone(), globaldict.clone(), userdict.clone()],
            systemdict,
            userdict,
            globaldict,
            gs,
            gstack: Vec::new(),
            save_stack: Vec::new(),
            next_save: 1,
            ops: 0,
            depth: 0,
            shapes: Vec::new(),
            warnings: Vec::new(),
            fonts: HashMap::new(),
            rand_state: 0x2545F4914F6CDD1D,
            base,
            bbox,
            color_space: 1,
            color_space_obj: None,
            indexed_palette: None,
            separation: false,
            jpeg_files: Vec::new(),
        }
    }

    fn warn(&mut self, m: impl Into<String>) {
        let m = m.into();
        if self.warnings.len() < 60 && !self.warnings.contains(&m) {
            self.warnings.push(m);
        }
    }

    fn install_systemdict(&mut self) {
        let names: &[&'static str] = &[
            // stack
            "pop",
            "dup",
            "exch",
            "copy",
            "index",
            "roll",
            "clear",
            "count",
            "mark",
            "cleartomark",
            "counttomark",
            "[",
            "]",
            "<<",
            ">>",
            // arithmetic
            "add",
            "sub",
            "mul",
            "div",
            "idiv",
            "mod",
            "neg",
            "abs",
            "sqrt",
            "sin",
            "cos",
            "atan",
            "exp",
            "ln",
            "log",
            "cvi",
            "cvr",
            "round",
            "truncate",
            "floor",
            "ceiling",
            "rand",
            "srand",
            "rrand",
            "bitshift",
            // relational and boolean
            "eq",
            "ne",
            "gt",
            "ge",
            "lt",
            "le",
            "and",
            "or",
            "not",
            "xor",
            "true",
            "false",
            "null",
            // control
            "exec",
            "if",
            "ifelse",
            "for",
            "repeat",
            "loop",
            "exit",
            "stop",
            "stopped",
            "countexecstack",
            "execstack",
            "quit",
            "forall",
            // arrays, dicts, strings
            "array",
            "length",
            "get",
            "put",
            "getinterval",
            "putinterval",
            "aload",
            "astore",
            "dict",
            "begin",
            "end",
            "def",
            "load",
            "store",
            "known",
            "where",
            "currentdict",
            "systemdict",
            "userdict",
            "globaldict",
            "statusdict",
            "errordict",
            "maxlength",
            "undef",
            "string",
            "cvs",
            "cvn",
            "cvx",
            "cvlit",
            "xcheck",
            "readonly",
            "executeonly",
            "noaccess",
            "bind",
            "type",
            "search",
            "anchorsearch",
            "token",
            "cvrs",
            // vm and misc
            "save",
            "restore",
            "gsave",
            "grestore",
            "grestoreall",
            "initgraphics",
            "setglobal",
            "currentglobal",
            "setpacking",
            "currentpacking",
            "usertime",
            "realtime",
            "version",
            "product",
            "revision",
            "languagelevel",
            "serialnumber",
            "vmstatus",
            "flush",
            "print",
            "=",
            "==",
            "pstack",
            "stack",
            "currentfile",
            "closefile",
            "flushfile",
            "readstring",
            "readhexstring",
            "readline",
            "read",
            "bytesavailable",
            "filter",
            "eexec",
            "findresource",
            "defineresource",
            "resourcestatus",
            "setpagedevice",
            "currentpagedevice",
            "nulldevice",
            "echo",
            "setuserparams",
            "currentuserparams",
            "setsystemparams",
            "currentsystemparams",
            "cshow",
            // graphics state
            "newpath",
            "moveto",
            "lineto",
            "curveto",
            "rmoveto",
            "rlineto",
            "rcurveto",
            "closepath",
            "arc",
            "arcn",
            "arct",
            "arcto",
            "rectfill",
            "rectstroke",
            "rectclip",
            "currentpoint",
            "flattenpath",
            "reversepath",
            "pathbbox",
            "clip",
            "eoclip",
            "initclip",
            "clippath",
            "fill",
            "eofill",
            "stroke",
            "strokepath",
            "charpath",
            "setgray",
            "setrgbcolor",
            "sethsbcolor",
            "setcmykcolor",
            "setcolor",
            "setcolorspace",
            "currentgray",
            "currentrgbcolor",
            "currentcmykcolor",
            "currenthsbcolor",
            "currentcolor",
            "currentcolorspace",
            "setlinewidth",
            "currentlinewidth",
            "setlinecap",
            "currentlinecap",
            "setlinejoin",
            "currentlinejoin",
            "setmiterlimit",
            "currentmiterlimit",
            "setdash",
            "currentdash",
            "setflat",
            "currentflat",
            "setstrokeadjust",
            "currentstrokeadjust",
            "setoverprint",
            "currentoverprint",
            "setsmoothness",
            "setblackgeneration",
            "setundercolorremoval",
            "settransfer",
            "setscreen",
            "setcolorscreen",
            "sethalftone",
            "currenthalftone",
            "setcolortransfer",
            "setcolorrendering",
            "currentcolorrendering",
            // matrices
            "translate",
            "scale",
            "rotate",
            "concat",
            "matrix",
            "currentmatrix",
            "setmatrix",
            "defaultmatrix",
            "initmatrix",
            "identmatrix",
            "invertmatrix",
            "concatmatrix",
            "transform",
            "itransform",
            "dtransform",
            "idtransform",
            // painting extras
            "image",
            "imagemask",
            "colorimage",
            "shfill",
            "setpattern",
            "makepattern",
            "showpage",
            "copypage",
            "erasepage",
            // fonts
            "findfont",
            "scalefont",
            "makefont",
            "setfont",
            "currentfont",
            "selectfont",
            "definefont",
            "show",
            "ashow",
            "widthshow",
            "awidthshow",
            "kshow",
            "xshow",
            "yshow",
            "xyshow",
            "stringwidth",
            "undefinefont",
            "rootfont",
            "setcachedevice",
            "setcachedevice2",
            "setcharwidth",
            "glyphshow",
        ];
        let mut d = self.systemdict.borrow_mut();
        for n in names {
            d.map.insert(Rc::from(*n), Obj::Op(n));
        }
        // Standard dictionaries and fonts.
        d.map
            .insert(Rc::from("FontDirectory"), Obj::Dict(mk_dict()));
        d.map
            .insert(Rc::from("StandardEncoding"), standard_encoding_array());
        d.map
            .insert(Rc::from("ISOLatin1Encoding"), standard_encoding_array());
    }

    // ----- lookup and stack helpers -----

    fn lookup(&self, name: &str) -> Option<Obj> {
        for d in self.dicts.iter().rev() {
            if let Some(v) = d.borrow().map.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn def(&mut self, name: Rc<str>, value: Obj) {
        if let Some(d) = self.dicts.last() {
            d.borrow_mut().map.insert(name, value);
        }
    }

    fn pop(&mut self) -> Option<Obj> {
        self.stack.pop()
    }

    fn pop_num(&mut self) -> Option<f64> {
        let o = self.stack.pop()?;
        o.num()
    }

    fn pop_int(&mut self) -> Option<i64> {
        let o = self.stack.pop()?;
        match o {
            Obj::Int(i) => Some(i),
            Obj::Real(r) if r.is_finite() => Some(r as i64),
            _ => None,
        }
    }

    fn push(&mut self, o: Obj) {
        if self.stack.len() < MAX_STACK {
            self.stack.push(o);
        }
    }

    fn pop_matrix(&mut self) -> Option<Affine> {
        let o = self.stack.pop()?;
        matrix_of(&o)
    }

    fn pop_proc(&mut self) -> Option<Obj> {
        let o = self.stack.pop()?;
        match &o {
            Obj::Array(..) | Obj::Op(_) | Obj::Name(..) => Some(o),
            _ => None,
        }
    }

    // ----- execution -----

    /// Run the main program token by token.
    fn run_main(&mut self) {
        loop {
            if self.ops > MAX_OPS {
                self.warn("program too long; stopped");
                break;
            }
            match self.scanner.next() {
                Token::Eof => break,
                Token::ProcStart => {
                    let p = self.read_proc();
                    self.push(p);
                }
                Token::ProcEnd => {}
                Token::Obj(o) => match self.execute_token(o) {
                    Flow::Normal => {}
                    Flow::Exit => {}
                    Flow::Stop => {}
                },
            }
        }
    }

    /// Read a `{ ... }` body from the scanner into a procedure object.
    fn read_proc(&mut self) -> Obj {
        let mut items = Vec::new();
        loop {
            match self.scanner.next() {
                Token::Eof | Token::ProcEnd => break,
                Token::ProcStart => {
                    let inner = self.read_proc();
                    items.push(inner);
                }
                Token::Obj(o) => items.push(o),
            }
            if items.len() > 1 << 20 {
                break;
            }
        }
        mk_array(items, true)
    }

    /// A token from the program: executable names run, everything else
    /// is pushed.
    fn execute_token(&mut self, o: Obj) -> Flow {
        match o {
            Obj::Name(ref n, true) => self.exec_name(n.clone()),
            other => {
                self.push(other);
                Flow::Normal
            }
        }
    }

    fn exec_name(&mut self, n: Rc<str>) -> Flow {
        match self.lookup(&n) {
            Some(v) => self.exec_obj(v),
            None => {
                self.warn(format!("undefined: {n}"));
                Flow::Normal
            }
        }
    }

    /// Execute an object: procedures run their elements, operators run,
    /// names look up, everything else is pushed.
    fn exec_obj(&mut self, o: Obj) -> Flow {
        self.ops += 1;
        if self.ops > MAX_OPS {
            return Flow::Stop;
        }
        match o {
            Obj::Array(items, true) => {
                if self.depth >= MAX_DEPTH {
                    self.warn("procedures nested too deeply");
                    return Flow::Stop;
                }
                self.depth += 1;
                let flow = self.run_proc(&items);
                self.depth -= 1;
                flow
            }
            Obj::Name(n, true) => self.exec_name(n),
            Obj::Op(op) => self.operator(op),
            other => {
                self.push(other);
                Flow::Normal
            }
        }
    }

    fn run_proc(&mut self, items: &Arr) -> Flow {
        let n = items.borrow().len();
        for i in 0..n {
            let item = {
                let b = items.borrow();
                match b.get(i) {
                    Some(o) => o.clone(),
                    None => break,
                }
            };
            let flow = match item {
                // Inside a procedure, a nested procedure is data.
                Obj::Array(_, true) => {
                    self.push(item);
                    Flow::Normal
                }
                Obj::Name(n, true) => self.exec_name(n),
                Obj::Op(op) => self.operator(op),
                other => {
                    self.push(other);
                    Flow::Normal
                }
            };
            if !matches!(flow, Flow::Normal) {
                return flow;
            }
            if self.ops > MAX_OPS {
                return Flow::Stop;
            }
        }
        Flow::Normal
    }

    /// Run a procedure given as an operand (proc, operator or name).
    fn call(&mut self, p: &Obj) -> Flow {
        match p {
            Obj::Array(_, true) | Obj::Op(_) | Obj::Name(_, true) => self.exec_obj(p.clone()),
            Obj::Array(items, false) => {
                // A literal array used as a procedure (after cvx is lost).
                self.exec_obj(Obj::Array(items.clone(), true))
            }
            other => {
                self.push(other.clone());
                Flow::Normal
            }
        }
    }

    // ----- shapes -----

    fn new_shape(&mut self, kind: ShapeKind) -> Shape {
        Shape::new(ShapeId(self.ids.shape().0), kind)
    }

    fn emit(&mut self, mut shape: Shape) {
        if let Some(clip) = &self.gs.clip {
            let cb = clip.bounding_box();
            let sb = shape.bounds();
            if cb.width() <= 0.0 || cb.height() <= 0.0 || sb.intersect(cb).area() <= 0.0 {
                return;
            }
            let inside = cb.x0 <= sb.x0 + 1e-6
                && cb.y0 <= sb.y0 + 1e-6
                && cb.x1 >= sb.x1 - 1e-6
                && cb.y1 >= sb.y1 - 1e-6;
            let is_text = matches!(shape.kind, ShapeKind::Text { .. });
            if !inside && !(is_text && sb.intersect(cb).area() >= 0.6 * sb.area()) {
                let mut frame = self.new_shape(ShapeKind::Path {
                    path: clip.clone(),
                    closed: true,
                });
                frame.fill = Fill::None;
                frame.stroke = None;
                let mut pc = self.new_shape(ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents: vec![shape],
                });
                pc.fill = Fill::None;
                pc.stroke = None;
                shape = pc;
            }
        }
        self.shapes.push(shape);
    }

    fn device_scale(&self) -> f64 {
        let c = self.gs.ctm.as_coeffs();
        (c[0] * c[3] - c[1] * c[2]).abs().sqrt()
    }

    fn paint(&mut self, fill: bool, stroke: bool, even_odd: bool) {
        let path = std::mem::take(&mut self.gs.path);
        self.gs.current = None;
        self.gs.start = None;
        if path.elements().is_empty() {
            return;
        }
        let closed = path
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::ClosePath));
        let mut shape = self.new_shape(ShapeKind::Path {
            path,
            closed: closed || fill,
        });
        shape.fill = if fill {
            match &self.gs.pattern {
                Some(p) => p.clone(),
                None => Fill::Solid(self.gs.color),
            }
        } else {
            Fill::None
        };
        if fill && even_odd {
            shape.data.push(("fill.rule".into(), "evenodd".into()));
        }
        shape.stroke = if stroke {
            let scale = self.device_scale();
            let mut s = Stroke::new(self.gs.color, (self.gs.line_width * scale).max(0.0));
            if s.width < Stroke::HAIRLINE {
                s.width = Stroke::HAIRLINE;
            }
            s.cap = self.gs.cap;
            s.join = self.gs.join;
            if !self.gs.dash.is_empty() && s.width > 0.0 {
                s.dash = self.gs.dash.iter().map(|d| d * scale / s.width).collect();
            }
            Some(s)
        } else {
            None
        };
        self.emit(shape);
    }

    fn user_to_device(&self, p: Point) -> Point {
        self.gs.ctm * p
    }

    fn set_clip(&mut self, path: BezPath) {
        if path.elements().is_empty() {
            return;
        }
        let nb = path.bounding_box();
        self.gs.clip = Some(match &self.gs.clip {
            Some(old) => {
                let ob = old.bounding_box();
                if nb.x0 >= ob.x0 - 1e-6
                    && nb.y0 >= ob.y0 - 1e-6
                    && nb.x1 <= ob.x1 + 1e-6
                    && nb.y1 <= ob.y1 + 1e-6
                {
                    path
                } else if ob.x0 >= nb.x0 - 1e-6
                    && ob.y0 >= nb.y0 - 1e-6
                    && ob.x1 <= nb.x1 + 1e-6
                    && ob.y1 <= nb.y1 + 1e-6
                {
                    old.clone()
                } else {
                    ob.intersect(nb).to_path(0.01)
                }
            }
            None => path,
        });
    }

    // ----- operators -----

    fn operator(&mut self, op: &'static str) -> Flow {
        macro_rules! need {
            ($e:expr) => {
                match $e {
                    Some(v) => v,
                    None => {
                        self.warn(format!("{op}: wrong or missing operands"));
                        return Flow::Normal;
                    }
                }
            };
        }
        match op {
            // ----- stack -----
            "pop" => {
                self.pop();
            }
            "dup" => {
                if let Some(o) = self.stack.last().cloned() {
                    self.push(o);
                }
            }
            "exch" => {
                let n = self.stack.len();
                if n >= 2 {
                    self.stack.swap(n - 1, n - 2);
                }
            }
            "copy" => {
                let top = need!(self.pop());
                match top {
                    Obj::Int(n) => {
                        let n = n.max(0) as usize;
                        let len = self.stack.len();
                        if n <= len {
                            for i in 0..n {
                                let o = self.stack[len - n + i].clone();
                                self.push(o);
                            }
                        }
                    }
                    Obj::Array(dst, e) => {
                        if let Some(Obj::Array(src, _)) = self.pop() {
                            let s = src.borrow().clone();
                            let n = s.len().min(dst.borrow().len());
                            dst.borrow_mut()[..n].clone_from_slice(&s[..n]);
                            let sub: Vec<Obj> = dst.borrow()[..n].to_vec();
                            self.push(mk_array(sub, e));
                        }
                    }
                    Obj::Str(dst) => {
                        if let Some(Obj::Str(src)) = self.pop() {
                            let s = src.borrow().clone();
                            let n = s.len().min(dst.borrow().len());
                            dst.borrow_mut()[..n].copy_from_slice(&s[..n]);
                            self.push(mk_str(&dst.borrow()[..n]));
                        }
                    }
                    Obj::Dict(dst) => {
                        if let Some(Obj::Dict(src)) = self.pop() {
                            let s: Vec<(Rc<str>, Obj)> = src
                                .borrow()
                                .map
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                            dst.borrow_mut().map.extend(s);
                            self.push(Obj::Dict(dst));
                        }
                    }
                    _ => {}
                }
            }
            "index" => {
                let n = need!(self.pop_int()).max(0) as usize;
                let len = self.stack.len();
                if n < len {
                    let o = self.stack[len - 1 - n].clone();
                    self.push(o);
                }
            }
            "roll" => {
                let j = need!(self.pop_int());
                let n = need!(self.pop_int()).max(0) as usize;
                let len = self.stack.len();
                if n > 0 && n <= len {
                    let slice = &mut self.stack[len - n..];
                    let j = j.rem_euclid(n as i64) as usize;
                    slice.rotate_right(j);
                }
            }
            "clear" => self.stack.clear(),
            "count" => {
                let n = self.stack.len() as i64;
                self.push(Obj::Int(n));
            }
            "mark" | "[" | "<<" => self.push(Obj::Mark),
            "cleartomark" => {
                while let Some(o) = self.pop() {
                    if matches!(o, Obj::Mark) {
                        break;
                    }
                }
            }
            "counttomark" => {
                let n = self
                    .stack
                    .iter()
                    .rev()
                    .position(|o| matches!(o, Obj::Mark))
                    .unwrap_or(self.stack.len());
                self.push(Obj::Int(n as i64));
            }
            "]" => {
                let mut items = Vec::new();
                while let Some(o) = self.pop() {
                    if matches!(o, Obj::Mark) {
                        break;
                    }
                    items.push(o);
                }
                items.reverse();
                self.push(mk_array(items, false));
            }
            ">>" => {
                let mut items = Vec::new();
                while let Some(o) = self.pop() {
                    if matches!(o, Obj::Mark) {
                        break;
                    }
                    items.push(o);
                }
                items.reverse();
                let d = mk_dict();
                for pair in items.chunks_exact(2) {
                    if let Some(k) = key_of(&pair[0]) {
                        d.borrow_mut().map.insert(k, pair[1].clone());
                    }
                }
                self.push(Obj::Dict(d));
            }
            // ----- arithmetic -----
            "add" | "sub" | "mul" | "div" | "idiv" | "mod" | "atan" | "exp" | "bitshift" => {
                let b = need!(self.pop());
                let a = need!(self.pop());
                let (x, y) = (need!(a.num()), need!(b.num()));
                let both_int = matches!((&a, &b), (Obj::Int(_), Obj::Int(_)));
                let r = match op {
                    "add" => {
                        if both_int {
                            Obj::Int((x as i64).wrapping_add(y as i64))
                        } else {
                            Obj::Real(x + y)
                        }
                    }
                    "sub" => {
                        if both_int {
                            Obj::Int((x as i64).wrapping_sub(y as i64))
                        } else {
                            Obj::Real(x - y)
                        }
                    }
                    "mul" => {
                        if both_int {
                            Obj::Int((x as i64).wrapping_mul(y as i64))
                        } else {
                            Obj::Real(x * y)
                        }
                    }
                    "div" => Obj::Real(if y == 0.0 { 0.0 } else { x / y }),
                    "idiv" => Obj::Int(if y as i64 == 0 {
                        0
                    } else {
                        (x as i64).wrapping_div(y as i64)
                    }),
                    "mod" => Obj::Int(if y as i64 == 0 {
                        0
                    } else {
                        (x as i64).wrapping_rem(y as i64)
                    }),
                    "atan" => Obj::Real(x.atan2(y).to_degrees().rem_euclid(360.0)),
                    "exp" => Obj::Real(x.powf(y)),
                    _ => {
                        let s = y as i64;
                        let v = x as i64;
                        Obj::Int(if s >= 0 {
                            v.checked_shl(s as u32).unwrap_or(0)
                        } else {
                            v.checked_shr((-s) as u32).unwrap_or(0)
                        })
                    }
                };
                self.push(r);
            }
            "neg" | "abs" | "sqrt" | "sin" | "cos" | "ln" | "log" | "cvi" | "cvr" | "round"
            | "truncate" | "floor" | "ceiling" => {
                let a = need!(self.pop());
                let x = match (&a, op) {
                    (Obj::Str(s), "cvi") | (Obj::Str(s), "cvr") => {
                        let t = String::from_utf8_lossy(&s.borrow()).trim().to_string();
                        need!(parse_number(&t).and_then(|o| o.num()))
                    }
                    _ => need!(a.num()),
                };
                let is_int = matches!(a, Obj::Int(_));
                let r = match op {
                    "neg" => {
                        if is_int {
                            Obj::Int(-(x as i64))
                        } else {
                            Obj::Real(-x)
                        }
                    }
                    "abs" => {
                        if is_int {
                            Obj::Int((x as i64).abs())
                        } else {
                            Obj::Real(x.abs())
                        }
                    }
                    "sqrt" => Obj::Real(x.max(0.0).sqrt()),
                    "sin" => Obj::Real(x.to_radians().sin()),
                    "cos" => Obj::Real(x.to_radians().cos()),
                    "ln" => Obj::Real(if x > 0.0 { x.ln() } else { 0.0 }),
                    "log" => Obj::Real(if x > 0.0 { x.log10() } else { 0.0 }),
                    "cvi" => Obj::Int(x.trunc() as i64),
                    "cvr" => Obj::Real(x),
                    "round" => {
                        if is_int {
                            a.clone()
                        } else {
                            Obj::Real((x + 0.5).floor())
                        }
                    }
                    "truncate" => {
                        if is_int {
                            a.clone()
                        } else {
                            Obj::Real(x.trunc())
                        }
                    }
                    "floor" => {
                        if is_int {
                            a.clone()
                        } else {
                            Obj::Real(x.floor())
                        }
                    }
                    _ => {
                        if is_int {
                            a.clone()
                        } else {
                            Obj::Real(x.ceil())
                        }
                    }
                };
                self.push(r);
            }
            "rand" => {
                self.rand_state ^= self.rand_state << 13;
                self.rand_state ^= self.rand_state >> 7;
                self.rand_state ^= self.rand_state << 17;
                self.push(Obj::Int((self.rand_state >> 33) as i64));
            }
            "srand" => {
                let s = need!(self.pop_int());
                self.rand_state = (s as u64) | 1;
            }
            "rrand" => self.push(Obj::Int((self.rand_state & 0x7fffffff) as i64)),
            // ----- relational, boolean -----
            "eq" | "ne" => {
                let b = need!(self.pop());
                let a = need!(self.pop());
                let r = a.equals(&b);
                self.push(Obj::Bool(if op == "eq" { r } else { !r }));
            }
            "gt" | "ge" | "lt" | "le" => {
                let b = need!(self.pop());
                let a = need!(self.pop());
                let ord = match (&a, &b) {
                    (Obj::Str(x), Obj::Str(y)) => x.borrow().cmp(&y.borrow()),
                    _ => need!(a.num()).total_cmp(&need!(b.num())),
                };
                let r = match op {
                    "gt" => ord.is_gt(),
                    "ge" => ord.is_ge(),
                    "lt" => ord.is_lt(),
                    _ => ord.is_le(),
                };
                self.push(Obj::Bool(r));
            }
            "and" | "or" | "xor" => {
                let b = need!(self.pop());
                let a = need!(self.pop());
                let r = match (&a, &b) {
                    (Obj::Bool(x), Obj::Bool(y)) => Obj::Bool(match op {
                        "and" => *x && *y,
                        "or" => *x || *y,
                        _ => *x ^ *y,
                    }),
                    _ => {
                        let (x, y) = (need!(a.num()) as i64, need!(b.num()) as i64);
                        Obj::Int(match op {
                            "and" => x & y,
                            "or" => x | y,
                            _ => x ^ y,
                        })
                    }
                };
                self.push(r);
            }
            "not" => {
                let a = need!(self.pop());
                let r = match a {
                    Obj::Bool(b) => Obj::Bool(!b),
                    other => Obj::Int(!(need!(other.num()) as i64)),
                };
                self.push(r);
            }
            "true" => self.push(Obj::Bool(true)),
            "false" => self.push(Obj::Bool(false)),
            "null" => self.push(Obj::Null),
            // ----- control -----
            "exec" => {
                let p = need!(self.pop());
                return self.exec_obj(p);
            }
            "if" => {
                let p = need!(self.pop_proc());
                let c = need!(self.pop());
                if matches!(c, Obj::Bool(true)) {
                    return self.call(&p);
                }
            }
            "ifelse" => {
                let p2 = need!(self.pop_proc());
                let p1 = need!(self.pop_proc());
                let c = need!(self.pop());
                return if matches!(c, Obj::Bool(true)) {
                    self.call(&p1)
                } else {
                    self.call(&p2)
                };
            }
            "for" => {
                let p = need!(self.pop_proc());
                let limit = need!(self.pop());
                let inc = need!(self.pop());
                let init = need!(self.pop());
                let ints = matches!(
                    (&init, &inc, &limit),
                    (Obj::Int(_), Obj::Int(_), Obj::Int(_))
                );
                let (mut i, step, lim) = (need!(init.num()), need!(inc.num()), need!(limit.num()));
                if step == 0.0 {
                    return Flow::Normal;
                }
                let mut guard = 0u64;
                while (step > 0.0 && i <= lim) || (step < 0.0 && i >= lim) {
                    self.push(if ints {
                        Obj::Int(i as i64)
                    } else {
                        Obj::Real(i)
                    });
                    match self.call(&p) {
                        Flow::Normal => {}
                        Flow::Exit => break,
                        Flow::Stop => return Flow::Stop,
                    }
                    i += step;
                    guard += 1;
                    if guard > 10_000_000 || self.ops > MAX_OPS {
                        break;
                    }
                }
            }
            "repeat" => {
                let p = need!(self.pop_proc());
                let n = need!(self.pop_int()).max(0);
                for _ in 0..n.min(10_000_000) {
                    match self.call(&p) {
                        Flow::Normal => {}
                        Flow::Exit => break,
                        Flow::Stop => return Flow::Stop,
                    }
                    if self.ops > MAX_OPS {
                        break;
                    }
                }
            }
            "loop" => {
                let p = need!(self.pop_proc());
                loop {
                    match self.call(&p) {
                        Flow::Normal => {}
                        Flow::Exit => break,
                        Flow::Stop => return Flow::Stop,
                    }
                    if self.ops > MAX_OPS {
                        break;
                    }
                }
            }
            "exit" => return Flow::Exit,
            "stop" => return Flow::Stop,
            "stopped" => {
                let p = need!(self.pop());
                let depth = self.stack.len();
                let flow = self.call(&p);
                let stopped = matches!(flow, Flow::Stop);
                if stopped {
                    self.stack.truncate(depth.min(self.stack.len()));
                }
                self.push(Obj::Bool(stopped));
            }
            "forall" => {
                let p = need!(self.pop_proc());
                let c = need!(self.pop());
                match c {
                    Obj::Array(items, _) => {
                        let v = items.borrow().clone();
                        for o in v {
                            self.push(o);
                            match self.call(&p) {
                                Flow::Normal => {}
                                Flow::Exit => break,
                                Flow::Stop => return Flow::Stop,
                            }
                        }
                    }
                    Obj::Str(s) => {
                        let v = s.borrow().clone();
                        for b in v {
                            self.push(Obj::Int(b as i64));
                            match self.call(&p) {
                                Flow::Normal => {}
                                Flow::Exit => break,
                                Flow::Stop => return Flow::Stop,
                            }
                        }
                    }
                    Obj::Dict(d) | Obj::Font(d) => {
                        let v: Vec<(Rc<str>, Obj)> = d
                            .borrow()
                            .map
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        for (k, val) in v {
                            self.push(Obj::Name(k, false));
                            self.push(val);
                            match self.call(&p) {
                                Flow::Normal => {}
                                Flow::Exit => break,
                                Flow::Stop => return Flow::Stop,
                            }
                        }
                    }
                    _ => {}
                }
            }
            "countexecstack" => self.push(Obj::Int(self.depth as i64)),
            "execstack" => {
                let a = need!(self.pop());
                self.push(a);
            }
            "quit" => return Flow::Stop,
            // ----- arrays, strings, dicts -----
            "array" => {
                let n = need!(self.pop_int()).clamp(0, 1 << 20) as usize;
                self.push(mk_array(vec![Obj::Null; n], false));
            }
            "string" => {
                let n = need!(self.pop_int()).clamp(0, MAX_STRING as i64) as usize;
                self.push(mk_str(&vec![0u8; n]));
            }
            "dict" => {
                let _ = self.pop_int();
                self.push(Obj::Dict(mk_dict()));
            }
            "length" => {
                let o = need!(self.pop());
                let n = match &o {
                    Obj::Array(a, _) => a.borrow().len(),
                    Obj::Str(s) => s.borrow().len(),
                    Obj::Dict(d) | Obj::Font(d) => d.borrow().map.len(),
                    Obj::Name(n, _) => n.len(),
                    _ => 0,
                };
                self.push(Obj::Int(n as i64));
            }
            "maxlength" => {
                let _ = self.pop();
                self.push(Obj::Int(65535));
            }
            "get" => {
                let k = need!(self.pop());
                let c = need!(self.pop());
                let v = match (&c, &k) {
                    (Obj::Array(a, _), _) => {
                        let i = need!(k.num()) as i64;
                        a.borrow()
                            .get(i.max(0) as usize)
                            .cloned()
                            .unwrap_or(Obj::Null)
                    }
                    (Obj::Str(s), _) => {
                        let i = need!(k.num()) as i64;
                        Obj::Int(*s.borrow().get(i.max(0) as usize).unwrap_or(&0) as i64)
                    }
                    (Obj::Dict(d), _) | (Obj::Font(d), _) => {
                        let key = need!(key_of(&k));
                        match d.borrow().map.get(&key) {
                            Some(v) => v.clone(),
                            None => {
                                self.warn(format!("get: undefined key {key}"));
                                Obj::Null
                            }
                        }
                    }
                    _ => Obj::Null,
                };
                self.push(v);
            }
            "put" => {
                let v = need!(self.pop());
                let k = need!(self.pop());
                let c = need!(self.pop());
                match &c {
                    Obj::Array(a, _) => {
                        let i = need!(k.num()) as i64;
                        if i >= 0 {
                            let mut b = a.borrow_mut();
                            if (i as usize) < b.len() {
                                b[i as usize] = v;
                            }
                        }
                    }
                    Obj::Str(s) => {
                        let i = need!(k.num()) as i64;
                        if i >= 0 {
                            let mut b = s.borrow_mut();
                            if (i as usize) < b.len() {
                                b[i as usize] = need!(v.num()) as u8;
                            }
                        }
                    }
                    Obj::Dict(d) | Obj::Font(d) => {
                        if let Some(key) = key_of(&k) {
                            d.borrow_mut().map.insert(key, v);
                        }
                    }
                    _ => {}
                }
            }
            "getinterval" => {
                let n = need!(self.pop_int()).max(0) as usize;
                let i = need!(self.pop_int()).max(0) as usize;
                let c = need!(self.pop());
                match c {
                    Obj::Array(a, e) => {
                        let b = a.borrow();
                        let end = (i + n).min(b.len());
                        let sub = if i <= end {
                            b[i..end].to_vec()
                        } else {
                            Vec::new()
                        };
                        drop(b);
                        self.push(mk_array(sub, e));
                    }
                    Obj::Str(s) => {
                        let b = s.borrow();
                        let end = (i + n).min(b.len());
                        let sub = if i <= end {
                            b[i..end].to_vec()
                        } else {
                            Vec::new()
                        };
                        drop(b);
                        self.push(mk_str(&sub));
                    }
                    _ => {}
                }
            }
            "putinterval" => {
                let src = need!(self.pop());
                let i = need!(self.pop_int()).max(0) as usize;
                let dst = need!(self.pop());
                match (&dst, &src) {
                    (Obj::Array(d, _), Obj::Array(s, _)) => {
                        let s = s.borrow().clone();
                        let mut d = d.borrow_mut();
                        for (k, v) in s.into_iter().enumerate() {
                            if i + k < d.len() {
                                d[i + k] = v;
                            }
                        }
                    }
                    (Obj::Str(d), Obj::Str(s)) => {
                        let s = s.borrow().clone();
                        let mut d = d.borrow_mut();
                        for (k, v) in s.into_iter().enumerate() {
                            if i + k < d.len() {
                                d[i + k] = v;
                            }
                        }
                    }
                    _ => {}
                }
            }
            "aload" => {
                let a = need!(self.pop());
                if let Obj::Array(items, _) = &a {
                    let v = items.borrow().clone();
                    for o in v {
                        self.push(o);
                    }
                }
                self.push(a);
            }
            "astore" => {
                let a = need!(self.pop());
                if let Obj::Array(items, _) = &a {
                    let n = items.borrow().len();
                    let len = self.stack.len();
                    if n <= len {
                        let v: Vec<Obj> = self.stack.drain(len - n..).collect();
                        *items.borrow_mut() = v;
                    }
                }
                self.push(a);
            }
            "begin" => {
                let d = need!(self.pop());
                match d {
                    Obj::Dict(d) | Obj::Font(d) => {
                        if self.dicts.len() < 1000 {
                            self.dicts.push(d);
                        }
                    }
                    _ => {}
                }
            }
            "end" => {
                if self.dicts.len() > 3 {
                    self.dicts.pop();
                }
            }
            "def" => {
                let v = need!(self.pop());
                let k = need!(self.pop());
                if let Some(key) = key_of(&k) {
                    self.def(key, v);
                }
            }
            "load" => {
                let k = need!(self.pop());
                let key = need!(key_of(&k));
                match self.lookup(&key) {
                    Some(v) => self.push(v),
                    None => self.warn(format!("load: undefined {key}")),
                }
            }
            "store" => {
                let v = need!(self.pop());
                let k = need!(self.pop());
                let key = need!(key_of(&k));
                let mut done = false;
                for d in self.dicts.iter().rev() {
                    if d.borrow().map.contains_key(&key) {
                        d.borrow_mut().map.insert(key.clone(), v.clone());
                        done = true;
                        break;
                    }
                }
                if !done {
                    self.def(key, v);
                }
            }
            "known" => {
                let k = need!(self.pop());
                let d = need!(self.pop());
                let key = need!(key_of(&k));
                let r = match d {
                    Obj::Dict(d) | Obj::Font(d) => d.borrow().map.contains_key(&key),
                    _ => false,
                };
                self.push(Obj::Bool(r));
            }
            "where" => {
                let k = need!(self.pop());
                let key = need!(key_of(&k));
                let mut found = None;
                for d in self.dicts.iter().rev() {
                    if d.borrow().map.contains_key(&key) {
                        found = Some(d.clone());
                        break;
                    }
                }
                match found {
                    Some(d) => {
                        self.push(Obj::Dict(d));
                        self.push(Obj::Bool(true));
                    }
                    None => self.push(Obj::Bool(false)),
                }
            }
            "undef" => {
                let k = need!(self.pop());
                let d = need!(self.pop());
                if let (Obj::Dict(d), Some(key)) = (d, key_of(&k)) {
                    d.borrow_mut().map.remove(&key);
                }
            }
            "currentdict" => {
                let d = self.dicts.last().cloned().unwrap_or_else(mk_dict);
                self.push(Obj::Dict(d));
            }
            "systemdict" => self.push(Obj::Dict(self.systemdict.clone())),
            "userdict" | "statusdict" | "errordict" => self.push(Obj::Dict(self.userdict.clone())),
            "globaldict" => self.push(Obj::Dict(self.globaldict.clone())),
            "cvs" => {
                let s = need!(self.pop());
                let v = need!(self.pop());
                let text = to_text(&v);
                if let Obj::Str(buf) = &s {
                    let mut b = buf.borrow_mut();
                    let n = text.len().min(b.len());
                    b[..n].copy_from_slice(&text.as_bytes()[..n]);
                    let sub = b[..n].to_vec();
                    drop(b);
                    self.push(mk_str(&sub));
                } else {
                    self.push(mk_str(text.as_bytes()));
                }
            }
            "cvrs" => {
                let s = need!(self.pop());
                let radix = need!(self.pop_int()).clamp(2, 36) as u32;
                let v = need!(self.pop_num()) as i64;
                let text = to_radix(v, radix);
                if let Obj::Str(buf) = &s {
                    let mut b = buf.borrow_mut();
                    let n = text.len().min(b.len());
                    b[..n].copy_from_slice(&text.as_bytes()[..n]);
                    let sub = b[..n].to_vec();
                    drop(b);
                    self.push(mk_str(&sub));
                }
            }
            "cvn" => {
                let s = need!(self.pop());
                match &s {
                    Obj::Str(b) => {
                        let name = String::from_utf8_lossy(&b.borrow()).into_owned();
                        self.push(Obj::Name(Rc::from(name.as_str()), false));
                    }
                    _ => self.push(s),
                }
            }
            "cvx" => {
                let o = need!(self.pop());
                let r = match o {
                    Obj::Name(n, _) => Obj::Name(n, true),
                    Obj::Array(a, _) => Obj::Array(a, true),
                    Obj::Str(s) => {
                        // Executable string: parse it into a procedure.
                        let text = s.borrow().clone();
                        let mut sub = Interp::scan_only(&text);
                        mk_array(std::mem::take(&mut sub), true)
                    }
                    other => other,
                };
                self.push(r);
            }
            "cvlit" => {
                let o = need!(self.pop());
                let r = match o {
                    Obj::Name(n, _) => Obj::Name(n, false),
                    Obj::Array(a, _) => Obj::Array(a, false),
                    other => other,
                };
                self.push(r);
            }
            "xcheck" => {
                let o = need!(self.pop());
                self.push(Obj::Bool(o.is_exec()));
            }
            "readonly" | "executeonly" | "noaccess" => {}
            "bind" => {
                let o = need!(self.pop());
                if let Obj::Array(items, _) = &o {
                    self.bind(items, 0);
                }
                self.push(o);
            }
            "type" => {
                let o = need!(self.pop());
                self.push(Obj::Name(Rc::from(o.type_name()), true));
            }
            "search" | "anchorsearch" => {
                let seek = need!(self.pop());
                let s = need!(self.pop());
                let (Obj::Str(hay), Obj::Str(needle)) = (&s, &seek) else {
                    return Flow::Normal;
                };
                let h = hay.borrow().clone();
                let n = needle.borrow().clone();
                let pos = if n.is_empty() {
                    Some(0)
                } else if op == "anchorsearch" {
                    h.starts_with(&n).then_some(0)
                } else {
                    h.windows(n.len()).position(|w| w == n.as_slice())
                };
                match pos {
                    Some(i) => {
                        let post = h[i + n.len()..].to_vec();
                        let pre = h[..i].to_vec();
                        self.push(mk_str(&post));
                        self.push(mk_str(&n));
                        if op == "search" {
                            self.push(mk_str(&pre));
                        }
                        self.push(Obj::Bool(true));
                    }
                    None => {
                        self.push(s);
                        self.push(Obj::Bool(false));
                    }
                }
            }
            "token" => {
                let s = need!(self.pop());
                match &s {
                    Obj::Str(buf) => {
                        let text = buf.borrow().clone();
                        let mut sc = Scanner { src: &text, pos: 0 };
                        match sc.next() {
                            Token::Eof => self.push(Obj::Bool(false)),
                            Token::ProcStart => {
                                let rest = text[sc.pos.min(text.len())..].to_vec();
                                self.push(mk_str(&rest));
                                self.push(mk_array(Vec::new(), true));
                                self.push(Obj::Bool(true));
                            }
                            Token::ProcEnd => self.push(Obj::Bool(false)),
                            Token::Obj(o) => {
                                let rest = text[sc.pos.min(text.len())..].to_vec();
                                self.push(mk_str(&rest));
                                self.push(o);
                                self.push(Obj::Bool(true));
                            }
                        }
                    }
                    Obj::File(f) => {
                        let o = self.file_token(f);
                        match o {
                            Some(o) => {
                                self.push(o);
                                self.push(Obj::Bool(true));
                            }
                            None => self.push(Obj::Bool(false)),
                        }
                    }
                    _ => self.push(Obj::Bool(false)),
                }
            }
            // ----- vm, misc -----
            "save" => {
                let id = self.next_save;
                self.next_save += 1;
                self.save_stack
                    .push((id, self.gstack.clone(), self.gs.clone()));
                if self.save_stack.len() > 64 {
                    self.save_stack.remove(0);
                }
                self.push(Obj::Save(id));
            }
            "restore" => {
                let o = need!(self.pop());
                if let Obj::Save(id) = o {
                    if let Some(pos) = self.save_stack.iter().position(|(i, _, _)| *i == id) {
                        let (_, gstack, gs) = self.save_stack.remove(pos);
                        self.save_stack.truncate(pos);
                        self.gstack = gstack;
                        self.gs = gs;
                    }
                }
            }
            "gsave" => {
                self.gstack.push(self.gs.clone());
                if self.gstack.len() > 512 {
                    self.gstack.remove(0);
                }
            }
            "grestore" => {
                // The current path is part of the graphics state, so
                // `gsave fill grestore stroke` strokes the same path.
                if let Some(g) = self.gstack.pop() {
                    self.gs = g;
                }
            }
            "grestoreall" => {
                if let Some(g) = self.gstack.first().cloned() {
                    self.gstack.clear();
                    self.gs = g;
                }
            }
            "initgraphics" => {
                let path = std::mem::take(&mut self.gs.path);
                self.gs = GState::default();
                self.gs.ctm = self.base;
                self.gs.path = path;
            }
            "setglobal"
            | "setpacking"
            | "setuserparams"
            | "setsystemparams"
            | "flush"
            | "setpagedevice"
            | "nulldevice"
            | "echo"
            | "showpage"
            | "copypage"
            | "erasepage"
            | "setflat"
            | "setstrokeadjust"
            | "setoverprint"
            | "setsmoothness"
            | "setmiterlimit"
            | "setblackgeneration"
            | "setundercolorremoval"
            | "settransfer"
            | "sethalftone"
            | "setcolorrendering"
            | "setcachedevice"
            | "setcachedevice2"
            | "setcharwidth" => {
                // One operand consumed (or more for the screen operators).
                let n = match op {
                    "setcachedevice" => 6,
                    "setcachedevice2" => 10,
                    "setcharwidth" => 2,
                    "showpage" | "copypage" | "erasepage" | "nulldevice" | "flush" => 0,
                    _ => 1,
                };
                for _ in 0..n {
                    self.pop();
                }
            }
            "setscreen" => {
                for _ in 0..3 {
                    self.pop();
                }
            }
            "setcolorscreen" => {
                for _ in 0..12 {
                    self.pop();
                }
            }
            "setcolortransfer" => {
                for _ in 0..4 {
                    self.pop();
                }
            }
            "currentglobal" | "currentpacking" | "currentoverprint" | "currentstrokeadjust" => {
                self.push(Obj::Bool(false))
            }
            "currentuserparams"
            | "currentsystemparams"
            | "currentpagedevice"
            | "currenthalftone"
            | "currentcolorrendering" => self.push(Obj::Dict(mk_dict())),
            "usertime" | "realtime" => self.push(Obj::Int((self.ops % 1_000_000) as i64)),
            "version" => self.push(mk_str(b"3010")),
            "product" => self.push(mk_str(b"TraceDraw")),
            "revision" | "serialnumber" => self.push(Obj::Int(1)),
            "languagelevel" => self.push(Obj::Int(3)),
            "vmstatus" => {
                self.push(Obj::Int(0));
                self.push(Obj::Int(1 << 20));
                self.push(Obj::Int(1 << 24));
            }
            "print" | "=" | "==" => {
                self.pop();
            }
            "pstack" | "stack" => {}
            "currentfile" => {
                let f = Rc::new(RefCell::new(FileObj {
                    buf: Vec::new(),
                    pos: 0,
                    main: true,
                    closed: false,
                    pending_filter: None,
                }));
                self.push(Obj::File(f));
            }
            "closefile" => {
                if let Some(Obj::File(f)) = self.pop() {
                    f.borrow_mut().closed = true;
                }
            }
            "flushfile" => {
                if let Some(Obj::File(f)) = self.pop() {
                    if f.borrow().main {
                        // Skip to the end of the current line.
                        self.scanner.read_line();
                    }
                }
            }
            "bytesavailable" => {
                let _ = self.pop();
                self.push(Obj::Int(-1));
            }
            "read" => {
                let f = need!(self.pop());
                if let Obj::File(f) = f {
                    match self.file_read(&f, 1).first().copied() {
                        Some(b) => {
                            self.push(Obj::Int(b as i64));
                            self.push(Obj::Bool(true));
                        }
                        None => self.push(Obj::Bool(false)),
                    }
                }
            }
            "readstring" | "readhexstring" => {
                let s = need!(self.pop());
                let f = need!(self.pop());
                if let (Obj::File(f), Obj::Str(buf)) = (f, &s) {
                    let n = buf.borrow().len();
                    let data = if op == "readhexstring" {
                        self.file_read_hex(&f, n)
                    } else {
                        self.file_read(&f, n)
                    };
                    let got = data.len();
                    buf.borrow_mut()[..got].copy_from_slice(&data);
                    let sub = buf.borrow()[..got].to_vec();
                    self.push(mk_str(&sub));
                    self.push(Obj::Bool(got == n));
                }
            }
            "readline" => {
                let s = need!(self.pop());
                let f = need!(self.pop());
                if let (Obj::File(f), Obj::Str(buf)) = (f, &s) {
                    self.materialize(&f);
                    let line = if f.borrow().main {
                        self.scanner.read_line().to_vec()
                    } else {
                        let mut fb = f.borrow_mut();
                        let start = fb.pos.min(fb.buf.len());
                        let mut end = start;
                        while end < fb.buf.len() && fb.buf[end] != b'\n' {
                            end += 1;
                        }
                        let l = fb.buf[start..end].to_vec();
                        fb.pos = (end + 1).min(fb.buf.len());
                        l
                    };
                    let n = line.len().min(buf.borrow().len());
                    buf.borrow_mut()[..n].copy_from_slice(&line[..n]);
                    let sub = buf.borrow()[..n].to_vec();
                    self.push(mk_str(&sub));
                    self.push(Obj::Bool(true));
                }
            }
            "filter" => {
                let name = need!(self.pop());
                let Some(fname) = key_of(&name) else {
                    return Flow::Normal;
                };
                // Optional parameter dictionary before the source.
                let mut src = need!(self.pop());
                if let Obj::Dict(_) = src {
                    src = need!(self.pop());
                }
                if let Obj::File(f) = &src {
                    if f.borrow().main && f.borrow().pending_filter.is_none() {
                        // Lazy: decode when first read.
                        let nf = Rc::new(RefCell::new(FileObj {
                            buf: Vec::new(),
                            pos: 0,
                            main: true,
                            closed: false,
                            pending_filter: Some(fname.clone()),
                        }));
                        if fname.as_ref() == "DCTDecode" {
                            self.jpeg_files.push(Rc::as_ptr(&nf) as usize);
                        }
                        self.push(Obj::File(nf));
                        return Flow::Normal;
                    }
                }
                let raw: Vec<u8> = match &src {
                    Obj::File(f) => {
                        self.materialize(f);
                        let fb = f.borrow();
                        fb.buf[fb.pos.min(fb.buf.len())..].to_vec()
                    }
                    Obj::Str(s) => s.borrow().clone(),
                    Obj::Array(_, true) => {
                        // Procedure source: call it until it returns an empty string.
                        let mut out = Vec::new();
                        for _ in 0..100_000 {
                            self.call(&src);
                            match self.pop() {
                                Some(Obj::Str(s)) if !s.borrow().is_empty() => {
                                    out.extend_from_slice(&s.borrow());
                                }
                                _ => break,
                            }
                        }
                        out
                    }
                    _ => Vec::new(),
                };
                let decoded = match &*fname {
                    "ASCIIHexDecode" => decode_asciihex(&raw),
                    "ASCII85Decode" => decode_ascii85(&raw),
                    "RunLengthDecode" => decode_runlength(&raw),
                    "FlateDecode" => inflate(&raw),
                    "LZWDecode" => {
                        self.warn("LZWDecode filter is not supported; image skipped");
                        Vec::new()
                    }
                    "DCTDecode" => raw,
                    "SubFileDecode" | "NullEncode" | "ReusableStreamDecode" => raw,
                    other => {
                        self.warn(format!("filter {other} not supported"));
                        raw
                    }
                };
                let f = Rc::new(RefCell::new(FileObj {
                    buf: decoded,
                    pos: 0,
                    main: false,
                    closed: false,
                    pending_filter: None,
                }));
                if fname.as_ref() == "DCTDecode" {
                    // JPEG data is decoded whole by `image`.
                    self.jpeg_files.push(Rc::as_ptr(&f) as usize);
                }
                self.push(Obj::File(f));
            }
            "eexec" => {
                // Encrypted font program: skip to its `cleartomark`.
                let _ = self.pop();
                self.skip_eexec();
            }
            "findresource" => {
                let cat = need!(self.pop());
                let key = need!(self.pop());
                let c = key_of(&cat).unwrap_or_else(|| Rc::from(""));
                if c.as_ref() == "Font" {
                    self.push(key);
                    return self.operator("findfont");
                }
                let d = mk_dict();
                if let Some(k) = key_of(&key) {
                    d.borrow_mut()
                        .map
                        .insert(Rc::from("Name"), Obj::Name(k, false));
                }
                self.push(Obj::Dict(d));
            }
            "defineresource" => {
                let _cat = self.pop();
                let inst = need!(self.pop());
                let _key = self.pop();
                self.push(inst);
            }
            "resourcestatus" => {
                let _cat = self.pop();
                let _key = self.pop();
                self.push(Obj::Bool(false));
            }
            // ----- path construction -----
            "newpath" => {
                self.gs.path = BezPath::new();
                self.gs.current = None;
                self.gs.start = None;
            }
            "moveto" | "rmoveto" => {
                let y = need!(self.pop_num());
                let x = need!(self.pop_num());
                let p = if op == "rmoveto" {
                    need!(self.gs.current) + Vec2::new(x, y)
                } else {
                    Point::new(x, y)
                };
                let d = self.user_to_device(p);
                self.gs.path.move_to(d);
                self.gs.current = Some(p);
                self.gs.start = Some(p);
            }
            "lineto" | "rlineto" => {
                let y = need!(self.pop_num());
                let x = need!(self.pop_num());
                let p = if op == "rlineto" {
                    need!(self.gs.current) + Vec2::new(x, y)
                } else {
                    Point::new(x, y)
                };
                let d = self.user_to_device(p);
                if self.gs.current.is_none() {
                    self.gs.path.move_to(d);
                    self.gs.start = Some(p);
                } else {
                    self.gs.path.line_to(d);
                }
                self.gs.current = Some(p);
            }
            "curveto" | "rcurveto" => {
                let y3 = need!(self.pop_num());
                let x3 = need!(self.pop_num());
                let y2 = need!(self.pop_num());
                let x2 = need!(self.pop_num());
                let y1 = need!(self.pop_num());
                let x1 = need!(self.pop_num());
                let base = if op == "rcurveto" {
                    need!(self.gs.current).to_vec2()
                } else {
                    Vec2::ZERO
                };
                let p1 = Point::new(x1, y1) + base;
                let p2 = Point::new(x2, y2) + base;
                let p3 = Point::new(x3, y3) + base;
                if self.gs.current.is_none() {
                    let d = self.user_to_device(p1);
                    self.gs.path.move_to(d);
                    self.gs.start = Some(p1);
                }
                self.gs.path.curve_to(
                    self.user_to_device(p1),
                    self.user_to_device(p2),
                    self.user_to_device(p3),
                );
                self.gs.current = Some(p3);
            }
            "closepath" => {
                if self.gs.current.is_some() && !self.gs.path.elements().is_empty() {
                    self.gs.path.close_path();
                    self.gs.current = self.gs.start;
                }
            }
            "arc" | "arcn" => {
                let a1 = need!(self.pop_num());
                let a0 = need!(self.pop_num());
                let r = need!(self.pop_num());
                let y = need!(self.pop_num());
                let x = need!(self.pop_num());
                self.arc(Point::new(x, y), r, a0, a1, op == "arcn");
            }
            "arct" | "arcto" => {
                let r = need!(self.pop_num());
                let y2 = need!(self.pop_num());
                let x2 = need!(self.pop_num());
                let y1 = need!(self.pop_num());
                let x1 = need!(self.pop_num());
                let p0 = need!(self.gs.current);
                let (p1, p2) = (Point::new(x1, y1), Point::new(x2, y2));
                let (t1, t2) = self.arct(p0, p1, p2, r);
                if op == "arcto" {
                    for v in [t1.x, t1.y, t2.x, t2.y] {
                        self.push(Obj::Real(v));
                    }
                }
            }
            "rectfill" | "rectstroke" | "rectclip" => {
                let rects = self.pop_rects();
                let saved = std::mem::take(&mut self.gs.path);
                let (sc, ss) = (self.gs.current, self.gs.start);
                let mut clip = BezPath::new();
                for r in rects {
                    let pts = [
                        Point::new(r.x0, r.y0),
                        Point::new(r.x1, r.y0),
                        Point::new(r.x1, r.y1),
                        Point::new(r.x0, r.y1),
                    ];
                    self.gs.path.move_to(self.user_to_device(pts[0]));
                    for p in &pts[1..] {
                        self.gs.path.line_to(self.user_to_device(*p));
                    }
                    self.gs.path.close_path();
                }
                match op {
                    "rectfill" => self.paint(true, false, false),
                    "rectstroke" => self.paint(false, true, false),
                    _ => {
                        clip = std::mem::take(&mut self.gs.path);
                    }
                }
                self.gs.path = saved;
                self.gs.current = sc;
                self.gs.start = ss;
                if op == "rectclip" {
                    self.set_clip(clip);
                }
            }
            "currentpoint" => {
                let p = need!(self.gs.current);
                self.push(Obj::Real(p.x));
                self.push(Obj::Real(p.y));
            }
            "flattenpath" | "strokepath" => {}
            "reversepath" => {
                let p = std::mem::take(&mut self.gs.path);
                self.gs.path = tracedraw_core::nodes::reverse(&p);
            }
            "pathbbox" => {
                let b = self.gs.path.bounding_box();
                let inv = self.gs.ctm.inverse();
                let (a, c) = (inv * Point::new(b.x0, b.y0), inv * Point::new(b.x1, b.y1));
                for v in [a.x.min(c.x), a.y.min(c.y), a.x.max(c.x), a.y.max(c.y)] {
                    self.push(Obj::Real(v));
                }
            }
            "clip" | "eoclip" => {
                let p = self.gs.path.clone();
                self.set_clip(p);
            }
            "initclip" => self.gs.clip = None,
            "clippath" => {
                self.gs.path = match &self.gs.clip {
                    Some(c) => c.clone(),
                    None => {
                        let b = self
                            .bbox
                            .map(|b| Affine::scale(PT_MM).transform_rect_bbox(b))
                            .unwrap_or(Rect::new(0.0, 0.0, 210.0, 297.0));
                        b.to_path(0.01)
                    }
                };
                self.gs.current = Some(Point::ZERO);
                self.gs.start = Some(Point::ZERO);
            }
            "fill" => self.paint(true, false, false),
            "eofill" => self.paint(true, false, true),
            "stroke" => self.paint(false, true, false),
            "charpath" => {
                // Glyph outlines are not available: the string is shown as text
                // and the path is left empty.
                let _ = self.pop();
                if let Some(Obj::Str(s)) = self.pop() {
                    let text = s.borrow().clone();
                    self.show_text(&text, None);
                }
            }
            // ----- colour -----
            "setgray" => {
                let v = need!(self.pop_num());
                self.gs.color = Color::Gray {
                    v: v.clamp(0.0, 1.0) as f32,
                };
                self.gs.pattern = None;
            }
            "setrgbcolor" => {
                let b = need!(self.pop_num());
                let g = need!(self.pop_num());
                let r = need!(self.pop_num());
                self.gs.color = Color::Rgb {
                    r: r.clamp(0.0, 1.0) as f32,
                    g: g.clamp(0.0, 1.0) as f32,
                    b: b.clamp(0.0, 1.0) as f32,
                };
                self.gs.pattern = None;
            }
            "sethsbcolor" => {
                let b = need!(self.pop_num());
                let s = need!(self.pop_num());
                let h = need!(self.pop_num());
                self.gs.color = Color::Hsb {
                    h: (h.clamp(0.0, 1.0) * 360.0) as f32,
                    s: s.clamp(0.0, 1.0) as f32,
                    b: b.clamp(0.0, 1.0) as f32,
                };
                self.gs.pattern = None;
            }
            "setcmykcolor" => {
                let k = need!(self.pop_num());
                let y = need!(self.pop_num());
                let m = need!(self.pop_num());
                let c = need!(self.pop_num());
                self.gs.color = Color::Cmyk {
                    c: c.clamp(0.0, 1.0) as f32,
                    m: m.clamp(0.0, 1.0) as f32,
                    y: y.clamp(0.0, 1.0) as f32,
                    k: k.clamp(0.0, 1.0) as f32,
                };
                self.gs.pattern = None;
            }
            "setcolorspace" => {
                let cs = need!(self.pop());
                self.color_space = color_space_components(&cs);
                self.indexed_palette = None;
                self.separation = false;
                if let Obj::Array(a, _) = &cs {
                    let items = a.borrow().clone();
                    let family = items
                        .first()
                        .and_then(key_of)
                        .unwrap_or_else(|| Rc::from(""));
                    match family.as_ref() {
                        "Indexed" | "I" => {
                            // [/Indexed base hival lookup]
                            let base_n = items.get(1).map(color_space_components).unwrap_or(3);
                            let lookup: Vec<u8> = match items.get(3) {
                                Some(Obj::Str(s)) => s.borrow().clone(),
                                Some(p @ Obj::Array(_, true)) => {
                                    // Procedure: index -> components; sample it.
                                    let hival = items.get(2).and_then(|o| o.num()).unwrap_or(255.0)
                                        as usize;
                                    let mut out = Vec::new();
                                    for i in 0..=hival.min(4095) {
                                        self.push(Obj::Int(i as i64));
                                        let before = self.stack.len() - 1;
                                        let _ = self.call(p);
                                        let comps: Vec<f64> = self
                                            .stack
                                            .drain(before.min(self.stack.len())..)
                                            .filter_map(|o| o.num())
                                            .collect();
                                        if let Some(c) = color_from(&comps, base_n) {
                                            out.extend_from_slice(&c.to_rgb8());
                                        }
                                    }
                                    self.indexed_palette = Some(out);
                                    Vec::new()
                                }
                                _ => Vec::new(),
                            };
                            if !lookup.is_empty() {
                                let mut pal = Vec::with_capacity(lookup.len() / base_n.max(1) * 3);
                                for e in lookup.chunks_exact(base_n.max(1)) {
                                    let comps: Vec<f64> =
                                        e.iter().map(|b| *b as f64 / 255.0).collect();
                                    if let Some(c) = color_from(&comps, base_n) {
                                        pal.extend_from_slice(&c.to_rgb8());
                                    }
                                }
                                self.indexed_palette = Some(pal);
                            }
                        }
                        "Separation" | "DeviceN" => self.separation = true,
                        _ => {}
                    }
                }
                self.color_space_obj = Some(cs);
            }
            "setcolor" => {
                // Pattern dictionaries, else components per the colour space.
                if let Some(Obj::Dict(p)) = self.stack.last().cloned() {
                    if p.borrow().map.contains_key("PaintProc")
                        || p.borrow().map.contains_key("Shading")
                    {
                        self.pop();
                        self.apply_pattern(&p);
                        return Flow::Normal;
                    }
                }
                let n = self.color_space;
                let mut v = Vec::new();
                for _ in 0..n {
                    v.push(need!(self.pop_num()));
                }
                v.reverse();
                self.gs.color = match (n, v.as_slice()) {
                    (1, [g]) => self.indexed_or_gray(*g),
                    (3, [r, g, b]) => Color::Rgb {
                        r: r.clamp(0.0, 1.0) as f32,
                        g: g.clamp(0.0, 1.0) as f32,
                        b: b.clamp(0.0, 1.0) as f32,
                    },
                    (4, [c, m, y, k]) => Color::Cmyk {
                        c: c.clamp(0.0, 1.0) as f32,
                        m: m.clamp(0.0, 1.0) as f32,
                        y: y.clamp(0.0, 1.0) as f32,
                        k: k.clamp(0.0, 1.0) as f32,
                    },
                    _ => self.gs.color,
                };
                self.gs.pattern = None;
            }
            "currentgray" => {
                let [r, g, b] = self.gs.color.to_rgb_f32();
                self.push(Obj::Real((0.3 * r + 0.59 * g + 0.11 * b) as f64));
            }
            "currentrgbcolor" => {
                let [r, g, b] = self.gs.color.to_rgb_f32();
                for v in [r, g, b] {
                    self.push(Obj::Real(v as f64));
                }
            }
            "currenthsbcolor" => {
                let [r, g, b] = self.gs.color.to_rgb_f32();
                let (h, s, v) = tracedraw_core::color::rgb_to_hsb(r, g, b);
                for x in [h / 360.0, s, v] {
                    self.push(Obj::Real(x as f64));
                }
            }
            "currentcmykcolor" => {
                if let Color::Cmyk { c, m, y, k } = self.gs.color.convert_to("CMYK") {
                    for v in [c, m, y, k] {
                        self.push(Obj::Real(v as f64));
                    }
                }
            }
            "currentcolor" => match self.gs.color {
                Color::Gray { v } => self.push(Obj::Real(v as f64)),
                Color::Cmyk { c, m, y, k } => {
                    for v in [c, m, y, k] {
                        self.push(Obj::Real(v as f64));
                    }
                }
                other => {
                    let [r, g, b] = other.to_rgb_f32();
                    for v in [r, g, b] {
                        self.push(Obj::Real(v as f64));
                    }
                }
            },
            "currentcolorspace" => {
                let cs = self.color_space_obj.clone().unwrap_or_else(|| {
                    mk_array(vec![Obj::Name(Rc::from("DeviceGray"), false)], false)
                });
                self.push(cs);
            }
            "setlinewidth" => self.gs.line_width = need!(self.pop_num()).max(0.0),
            "currentlinewidth" => self.push(Obj::Real(self.gs.line_width)),
            "setlinecap" => {
                self.gs.cap = match need!(self.pop_int()) {
                    1 => LineCap::Round,
                    2 => LineCap::Square,
                    _ => LineCap::Butt,
                }
            }
            "currentlinecap" => self.push(Obj::Int(match self.gs.cap {
                LineCap::Butt => 0,
                LineCap::Round => 1,
                LineCap::Square => 2,
            })),
            "setlinejoin" => {
                self.gs.join = match need!(self.pop_int()) {
                    1 => LineJoin::Round,
                    2 => LineJoin::Bevel,
                    _ => LineJoin::Miter,
                }
            }
            "currentlinejoin" => self.push(Obj::Int(match self.gs.join {
                LineJoin::Miter => 0,
                LineJoin::Round => 1,
                LineJoin::Bevel => 2,
            })),
            "currentmiterlimit" => self.push(Obj::Real(10.0)),
            "currentflat" => self.push(Obj::Real(1.0)),
            "setdash" => {
                let _offset = self.pop_num();
                let a = need!(self.pop());
                self.gs.dash = match a {
                    Obj::Array(items, _) => items
                        .borrow()
                        .iter()
                        .filter_map(|o| o.num())
                        .filter(|v| v.is_finite() && *v >= 0.0)
                        .collect(),
                    _ => Vec::new(),
                };
                if self.gs.dash.iter().all(|v| *v == 0.0) {
                    self.gs.dash.clear();
                }
            }
            "currentdash" => {
                let d: Vec<Obj> = self.gs.dash.iter().map(|v| Obj::Real(*v)).collect();
                self.push(mk_array(d, false));
                self.push(Obj::Real(0.0));
            }
            // ----- matrices -----
            "translate" | "scale" => {
                // With a matrix operand: [tx ty matrix] -> matrix; else modify the CTM.
                if let Some(Obj::Array(m, _)) = self.stack.last().cloned() {
                    if m.borrow().len() == 6 {
                        self.pop();
                        let y = need!(self.pop_num());
                        let x = need!(self.pop_num());
                        let a = if op == "translate" {
                            Affine::translate((x, y))
                        } else {
                            Affine::scale_non_uniform(x, y)
                        };
                        self.push(matrix_obj(&m, a));
                        return Flow::Normal;
                    }
                }
                let y = need!(self.pop_num());
                let x = need!(self.pop_num());
                self.gs.ctm *= if op == "translate" {
                    Affine::translate((x, y))
                } else {
                    Affine::scale_non_uniform(x, y)
                };
            }
            "rotate" => {
                if let Some(Obj::Array(m, _)) = self.stack.last().cloned() {
                    if m.borrow().len() == 6 {
                        self.pop();
                        let a = need!(self.pop_num());
                        self.push(matrix_obj(&m, Affine::rotate(a.to_radians())));
                        return Flow::Normal;
                    }
                }
                let a = need!(self.pop_num());
                self.gs.ctm *= Affine::rotate(a.to_radians());
            }
            "concat" => {
                let m = need!(self.pop_matrix());
                self.gs.ctm *= m;
            }
            "matrix" | "identmatrix" => {
                if op == "identmatrix" {
                    let _ = self.pop();
                }
                self.push(affine_to_obj(Affine::IDENTITY));
            }
            "currentmatrix" | "defaultmatrix" | "initmatrix" => {
                if op == "initmatrix" {
                    self.gs.ctm = self.base;
                    return Flow::Normal;
                }
                let m = need!(self.pop());
                let a = if op == "currentmatrix" {
                    self.gs.ctm
                } else {
                    self.base
                };
                if let Obj::Array(arr, _) = &m {
                    self.push(matrix_obj(arr, a));
                } else {
                    self.push(affine_to_obj(a));
                }
            }
            "setmatrix" => {
                let m = need!(self.pop_matrix());
                self.gs.ctm = m;
            }
            "invertmatrix" => {
                let out = need!(self.pop());
                let m = need!(self.pop_matrix());
                let inv = m.inverse();
                if let Obj::Array(arr, _) = &out {
                    self.push(matrix_obj(arr, inv));
                } else {
                    self.push(affine_to_obj(inv));
                }
            }
            "concatmatrix" => {
                let out = need!(self.pop());
                let b = need!(self.pop_matrix());
                let a = need!(self.pop_matrix());
                let r = b * a;
                if let Obj::Array(arr, _) = &out {
                    self.push(matrix_obj(arr, r));
                } else {
                    self.push(affine_to_obj(r));
                }
            }
            "transform" | "itransform" | "dtransform" | "idtransform" => {
                // Optional matrix operand.
                let m = if let Some(Obj::Array(a, _)) = self.stack.last().cloned() {
                    if a.borrow().len() == 6 {
                        self.pop();
                        need!(matrix_of(&Obj::Array(a, false)))
                    } else {
                        self.gs.ctm
                    }
                } else {
                    self.gs.ctm
                };
                let y = need!(self.pop_num());
                let x = need!(self.pop_num());
                let m = if op.starts_with('i') { m.inverse() } else { m };
                let r = if op.contains("dtransform") {
                    let c = m.as_coeffs();
                    Point::new(c[0] * x + c[2] * y, c[1] * x + c[3] * y)
                } else {
                    m * Point::new(x, y)
                };
                self.push(Obj::Real(r.x));
                self.push(Obj::Real(r.y));
            }
            // ----- images and shadings -----
            "image" | "imagemask" | "colorimage" => self.image_op(op),
            "shfill" => {
                let d = need!(self.pop());
                if let Obj::Dict(d) = d {
                    self.shfill(&d);
                }
            }
            "makepattern" => {
                let m = need!(self.pop_matrix());
                let d = need!(self.pop());
                if let Obj::Dict(d) = &d {
                    d.borrow_mut()
                        .map
                        .insert(Rc::from("__matrix"), affine_to_obj(m));
                }
                self.push(d);
            }
            "setpattern" => {
                let d = need!(self.pop());
                if let Obj::Dict(d) = d {
                    self.apply_pattern(&d);
                }
            }
            // ----- fonts and text -----
            "findfont" => {
                let k = need!(self.pop());
                let name = key_of(&k).unwrap_or_else(|| Rc::from("Helvetica"));
                let d = match self.fonts.get(&name) {
                    Some(d) => d.clone(),
                    None => {
                        let d = mk_dict();
                        d.borrow_mut()
                            .map
                            .insert(Rc::from("FontName"), Obj::Name(name.clone(), false));
                        d.borrow_mut()
                            .map
                            .insert(Rc::from("FontMatrix"), affine_to_obj(Affine::scale(0.001)));
                        d.borrow_mut().map.insert(Rc::from("FontType"), Obj::Int(1));
                        d
                    }
                };
                self.push(Obj::Font(d));
            }
            "scalefont" | "makefont" => {
                let m = need!(self.pop());
                let f = need!(self.pop());
                let scale = match (&m, op) {
                    (_, "scalefont") => Affine::scale(need!(m.num())),
                    _ => need!(matrix_of(&m)),
                };
                if let Obj::Font(d) | Obj::Dict(d) = f {
                    let nd = mk_dict();
                    nd.borrow_mut().map = d.borrow().map.clone();
                    let fm = d
                        .borrow()
                        .map
                        .get("FontMatrix")
                        .and_then(matrix_of)
                        .unwrap_or(Affine::scale(0.001));
                    nd.borrow_mut()
                        .map
                        .insert(Rc::from("FontMatrix"), affine_to_obj(scale * fm));
                    self.push(Obj::Font(nd));
                }
            }
            "setfont" => {
                let f = need!(self.pop());
                if let Obj::Font(d) | Obj::Dict(d) = f {
                    self.gs.font = Some(font_info(&d));
                }
            }
            "selectfont" => {
                let m = need!(self.pop());
                let k = need!(self.pop());
                self.push(k);
                self.operator("findfont");
                self.push(m);
                if m_is_num(&self.stack) {
                    self.operator("scalefont");
                } else {
                    self.operator("makefont");
                }
                self.operator("setfont");
            }
            "currentfont" | "rootfont" => {
                let d = mk_dict();
                if let Some(f) = &self.gs.font {
                    d.borrow_mut().map.insert(
                        Rc::from("FontName"),
                        Obj::Name(Rc::from(f.family.as_str()), false),
                    );
                    d.borrow_mut()
                        .map
                        .insert(Rc::from("FontMatrix"), affine_to_obj(f.matrix));
                }
                self.push(Obj::Font(d));
            }
            "definefont" => {
                let f = need!(self.pop());
                let k = need!(self.pop());
                if let (Some(name), Obj::Dict(d) | Obj::Font(d)) = (key_of(&k), &f) {
                    self.fonts.insert(name, d.clone());
                    self.push(Obj::Font(d.clone()));
                }
            }
            "undefinefont" => {
                let _ = self.pop();
            }
            "show" => {
                let s = need!(self.pop());
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, None);
                }
            }
            "ashow" => {
                let s = need!(self.pop());
                let ay = need!(self.pop_num());
                let ax = need!(self.pop_num());
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, Some((ax, ay, 0.0, 0.0, 0)));
                }
            }
            "widthshow" => {
                let s = need!(self.pop());
                let ch = need!(self.pop_int());
                let cy = need!(self.pop_num());
                let cx = need!(self.pop_num());
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, Some((0.0, 0.0, cx, cy, ch)));
                }
            }
            "awidthshow" => {
                let s = need!(self.pop());
                let ay = need!(self.pop_num());
                let ax = need!(self.pop_num());
                let ch = need!(self.pop_int());
                let cy = need!(self.pop_num());
                let cx = need!(self.pop_num());
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, Some((ax, ay, cx, cy, ch)));
                }
            }
            "kshow" => {
                let s = need!(self.pop());
                let _p = self.pop();
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, None);
                }
            }
            "xshow" | "yshow" | "xyshow" => {
                let _nums = self.pop();
                let s = need!(self.pop());
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, None);
                }
            }
            "cshow" => {
                let s = need!(self.pop());
                let _p = self.pop();
                if let Obj::Str(s) = s {
                    let text = s.borrow().clone();
                    self.show_text(&text, None);
                }
            }
            "glyphshow" => {
                let g = need!(self.pop());
                if let Some(name) = key_of(&g) {
                    let ch = glyph_char(&name);
                    let mut b = [0u8; 4];
                    let text = ch.encode_utf8(&mut b).as_bytes().to_vec();
                    self.show_text(&text, None);
                }
            }
            "stringwidth" => {
                let s = need!(self.pop());
                let n = match &s {
                    Obj::Str(b) => b.borrow().len(),
                    _ => 0,
                };
                let (w, _) = self.text_advance(n);
                self.push(Obj::Real(w));
                self.push(Obj::Real(0.0));
            }
            other => self.warn(format!("operator {other} not implemented")),
        }
        Flow::Normal
    }

    /// Replace executable names by operators inside a procedure (bind).
    fn bind(&mut self, items: &Arr, depth: usize) {
        if depth > 32 {
            return;
        }
        let n = items.borrow().len();
        for i in 0..n {
            let item = items.borrow()[i].clone();
            match item {
                Obj::Name(ref name, true) => {
                    if let Some(Obj::Op(op)) = self.systemdict.borrow().map.get(name.as_ref()) {
                        items.borrow_mut()[i] = Obj::Op(op);
                    }
                }
                Obj::Array(inner, true) => self.bind(&inner, depth + 1),
                _ => {}
            }
        }
    }

    fn scan_only(text: &[u8]) -> Vec<Obj> {
        let mut sc = Scanner { src: text, pos: 0 };
        let mut out = Vec::new();
        let mut stack: Vec<Vec<Obj>> = Vec::new();
        loop {
            match sc.next() {
                Token::Eof => break,
                Token::ProcStart => stack.push(Vec::new()),
                Token::ProcEnd => {
                    if let Some(items) = stack.pop() {
                        let p = mk_array(items, true);
                        match stack.last_mut() {
                            Some(parent) => parent.push(p),
                            None => out.push(p),
                        }
                    }
                }
                Token::Obj(o) => match stack.last_mut() {
                    Some(parent) => parent.push(o),
                    None => out.push(o),
                },
            }
            if out.len() > 1 << 20 {
                break;
            }
        }
        while let Some(items) = stack.pop() {
            out.push(mk_array(items, true));
        }
        out
    }

    fn pop_rects(&mut self) -> Vec<Rect> {
        // Either x y w h, or an array/string of them.
        if let Some(Obj::Array(a, _)) = self.stack.last().cloned() {
            self.pop();
            let v: Vec<f64> = a.borrow().iter().filter_map(|o| o.num()).collect();
            return v
                .chunks_exact(4)
                .map(|c| {
                    Rect::new(
                        c[0].min(c[0] + c[2]),
                        c[1].min(c[1] + c[3]),
                        c[0].max(c[0] + c[2]),
                        c[1].max(c[1] + c[3]),
                    )
                })
                .collect();
        }
        let h = self.pop_num().unwrap_or(0.0);
        let w = self.pop_num().unwrap_or(0.0);
        let y = self.pop_num().unwrap_or(0.0);
        let x = self.pop_num().unwrap_or(0.0);
        vec![Rect::new(
            x.min(x + w),
            y.min(y + h),
            x.max(x + w),
            y.max(y + h),
        )]
    }

    fn arc(&mut self, c: Point, r: f64, a0: f64, a1: f64, clockwise: bool) {
        if !(r.is_finite() && a0.is_finite() && a1.is_finite()) || r < 0.0 {
            return;
        }
        let mut sweep = if clockwise {
            -(a0 - a1).rem_euclid(360.0)
        } else {
            (a1 - a0).rem_euclid(360.0)
        };
        if sweep == 0.0 && (a1 - a0).abs() >= 360.0 - 1e-9 {
            sweep = if clockwise { -360.0 } else { 360.0 };
        }
        let start = c + Vec2::from_angle(a0.to_radians()) * r;
        match self.gs.current {
            Some(_) => {
                let d = self.user_to_device(start);
                self.gs.path.line_to(d);
            }
            None => {
                let d = self.user_to_device(start);
                self.gs.path.move_to(d);
                self.gs.start = Some(start);
            }
        }
        let segs = ((sweep.abs() / 90.0).ceil() as usize).max(1);
        let step = sweep.to_radians() / segs as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let mut ang = a0.to_radians();
        for _ in 0..segs {
            let p0 = c + Vec2::from_angle(ang) * r;
            let p3 = c + Vec2::from_angle(ang + step) * r;
            let t0 = Vec2::new(-ang.sin(), ang.cos()) * r * k;
            let t1 = Vec2::new(-(ang + step).sin(), (ang + step).cos()) * r * k;
            self.gs.path.curve_to(
                self.user_to_device(p0 + t0),
                self.user_to_device(p3 - t1),
                self.user_to_device(p3),
            );
            ang += step;
        }
        self.gs.current = Some(c + Vec2::from_angle(ang) * r);
    }

    /// arct: tangent arc between the segments p0-p1 and p1-p2.
    fn arct(&mut self, p0: Point, p1: Point, p2: Point, r: f64) -> (Point, Point) {
        let d0 = (p0 - p1).normalize();
        let d2 = (p2 - p1).normalize();
        let cos_t = d0.dot(d2).clamp(-1.0, 1.0);
        let theta = cos_t.acos();
        if theta.abs() < 1e-9 || (std::f64::consts::PI - theta).abs() < 1e-9 || r <= 0.0 {
            let d = self.user_to_device(p1);
            self.gs.path.line_to(d);
            self.gs.current = Some(p1);
            return (p1, p1);
        }
        let dist = r / (theta / 2.0).tan();
        let t1 = p1 + d0 * dist;
        let t2 = p1 + d2 * dist;
        let bis = (d0 + d2).normalize();
        let center = p1 + bis * (r / (theta / 2.0).sin());
        let a0 = (t1 - center).atan2().to_degrees();
        let a1 = (t2 - center).atan2().to_degrees();
        let cross = d0.cross(d2);
        let d = self.user_to_device(t1);
        self.gs.path.line_to(d);
        self.gs.current = Some(t1);
        self.arc(center, r, a0, a1, cross > 0.0);
        self.gs.current = Some(t2);
        (t1, t2)
    }

    fn indexed_or_gray(&self, v: f64) -> Color {
        if let Some(pal) = &self.indexed_palette {
            let i = (v.max(0.0) as usize) * 3;
            if i + 2 < pal.len() {
                return Color::rgb8(pal[i], pal[i + 1], pal[i + 2]);
            }
        }
        if self.separation {
            return Color::Gray {
                v: (1.0 - v).clamp(0.0, 1.0) as f32,
            };
        }
        Color::Gray {
            v: v.clamp(0.0, 1.0) as f32,
        }
    }

    fn apply_pattern(&mut self, d: &Dict) {
        let m = d
            .borrow()
            .map
            .get("__matrix")
            .and_then(matrix_of)
            .unwrap_or(Affine::IDENTITY);
        let ptype = d
            .borrow()
            .map
            .get("PatternType")
            .and_then(|o| o.num())
            .unwrap_or(1.0) as i64;
        if ptype == 2 {
            let sh = d.borrow().map.get("Shading").cloned();
            if let Some(Obj::Dict(sh)) = sh {
                if let Some(f) = self.shading_fountain(&sh, self.base * m) {
                    self.gs.pattern = Some(Fill::Fountain(f));
                }
            }
            return;
        }
        // Tiling pattern: run PaintProc into a tile.
        let bbox = d
            .borrow()
            .map
            .get("BBox")
            .map(|o| nums_of(o))
            .unwrap_or_default();
        if bbox.len() != 4 {
            return;
        }
        let xstep = d
            .borrow()
            .map
            .get("XStep")
            .and_then(|o| o.num())
            .unwrap_or(bbox[2] - bbox[0]);
        let ystep = d
            .borrow()
            .map
            .get("YStep")
            .and_then(|o| o.num())
            .unwrap_or(bbox[3] - bbox[1]);
        let Some(proc_) = d.borrow().map.get("PaintProc").cloned() else {
            return;
        };
        let saved_shapes = std::mem::take(&mut self.shapes);
        let saved_gs = self.gs.clone();
        let saved_gstack = std::mem::take(&mut self.gstack);
        let scale = {
            let c = (self.base * m).as_coeffs();
            (c[0] * c[3] - c[1] * c[2]).abs().sqrt()
        };
        self.gs = GState::default();
        self.gs.ctm = Affine::scale(scale) * Affine::translate((-bbox[0], -bbox[1]));
        self.push(Obj::Dict(d.clone()));
        let _ = self.call(&proc_);
        let tile_shapes = std::mem::replace(&mut self.shapes, saved_shapes);
        self.gs = saved_gs;
        self.gstack = saved_gstack;
        if tile_shapes.is_empty() {
            return;
        }
        self.gs.pattern = Some(Fill::Pattern(tracedraw_core::Pattern::Vector {
            shapes: tile_shapes,
            tile: Size::new(
                (xstep.abs() * scale).max(tracedraw_core::Pattern::MIN_TILE_MM),
                (ystep.abs() * scale).max(tracedraw_core::Pattern::MIN_TILE_MM),
            ),
        }));
    }

    fn shfill(&mut self, sh: &Dict) {
        let Some(f) = self.shading_fountain(sh, self.gs.ctm) else {
            return;
        };
        let area = match &self.gs.clip {
            Some(c) => c.clone(),
            None => {
                let b = self
                    .bbox
                    .map(|b| Affine::scale(PT_MM).transform_rect_bbox(b))
                    .unwrap_or(Rect::new(-1000.0, -1000.0, 2000.0, 2000.0));
                b.to_path(0.01)
            }
        };
        let mut shape = self.new_shape(ShapeKind::Path {
            path: area,
            closed: true,
        });
        shape.fill = Fill::Fountain(f);
        shape.stroke = None;
        self.shapes.push(shape);
    }

    fn shading_fountain(&mut self, sh: &Dict, m: Affine) -> Option<Fountain> {
        let b = sh.borrow();
        let stype = b
            .map
            .get("ShadingType")
            .and_then(|o| o.num())
            .unwrap_or(2.0) as i64;
        let ncomp = b
            .map
            .get("ColorSpace")
            .map(color_space_components)
            .unwrap_or(3);
        let coords = b.map.get("Coords").map(nums_of).unwrap_or_default();
        let func = b.map.get("Function").cloned();
        drop(b);
        let mut stops = Vec::new();
        if let Some(func) = func {
            for i in 0..=8 {
                let t = i as f64 / 8.0;
                let comps = self.eval_function(&func, t, 0);
                if let Some(c) = color_from(&comps, ncomp) {
                    stops.push(Stop { pos: t, color: c });
                }
            }
        }
        stops.dedup_by(|a, b| a.color == b.color && (a.pos - b.pos).abs() < 1e-9);
        if stops.len() < 2 {
            let c = stops
                .first()
                .map(|s| s.color)
                .unwrap_or(Color::Gray { v: 0.5 });
            stops = vec![Stop { pos: 0.0, color: c }, Stop { pos: 1.0, color: c }];
        }
        let (kind, angle) = match (stype, coords.len()) {
            (2, 4) => {
                let a = m * Point::new(coords[0], coords[1]);
                let bpt = m * Point::new(coords[2], coords[3]);
                let d = bpt - a;
                (FountainKind::Linear, d.y.atan2(d.x).to_degrees())
            }
            (3, 6) => (FountainKind::Radial, 0.0),
            _ => (FountainKind::Linear, 0.0),
        };
        Some(Fountain {
            kind,
            stops,
            angle,
            offset: Point::ZERO,
            edge_pad: 0.0,
        })
    }

    /// Evaluate a PostScript function dictionary (types 0, 2, 3) or a
    /// procedure at t in 0..1.
    fn eval_function(&mut self, f: &Obj, t: f64, depth: usize) -> Vec<f64> {
        if depth > 4 {
            return Vec::new();
        }
        match f {
            Obj::Array(items, true) => {
                // A procedure taking t and leaving the components.
                let before = self.stack.len();
                self.push(Obj::Real(t));
                let _ = self.call(f);
                let out: Vec<f64> = self
                    .stack
                    .drain(before.min(self.stack.len())..)
                    .filter_map(|o| o.num())
                    .collect();
                let _ = items;
                out
            }
            Obj::Array(items, false) => {
                let parts = items.borrow().clone();
                parts
                    .iter()
                    .map(|p| {
                        self.eval_function(p, t, depth + 1)
                            .first()
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            }
            Obj::Dict(d) => {
                let b = d.borrow();
                let ftype = b
                    .map
                    .get("FunctionType")
                    .and_then(|o| o.num())
                    .unwrap_or(2.0) as i64;
                let domain = b
                    .map
                    .get("Domain")
                    .map(nums_of)
                    .unwrap_or_else(|| vec![0.0, 1.0]);
                let (d0, d1) = (
                    domain.first().copied().unwrap_or(0.0),
                    domain.get(1).copied().unwrap_or(1.0),
                );
                let x = d0 + (d1 - d0) * t;
                match ftype {
                    2 => {
                        let c0 = b.map.get("C0").map(nums_of).unwrap_or_else(|| vec![0.0]);
                        let c1 = b.map.get("C1").map(nums_of).unwrap_or_else(|| vec![1.0]);
                        let n = b.map.get("N").and_then(|o| o.num()).unwrap_or(1.0);
                        let tt = if n == 1.0 { t } else { t.powf(n) };
                        c0.iter()
                            .zip(c1.iter())
                            .map(|(a, b)| a + (b - a) * tt)
                            .collect()
                    }
                    3 => {
                        let funcs = b
                            .map
                            .get("Functions")
                            .map(|o| match o {
                                Obj::Array(a, _) => a.borrow().clone(),
                                _ => Vec::new(),
                            })
                            .unwrap_or_default();
                        let bounds = b.map.get("Bounds").map(nums_of).unwrap_or_default();
                        let encode = b.map.get("Encode").map(nums_of).unwrap_or_default();
                        drop(b);
                        let mut idx = 0;
                        while idx < bounds.len() && x >= bounds[idx] {
                            idx += 1;
                        }
                        let lo = if idx == 0 { d0 } else { bounds[idx - 1] };
                        let hi = if idx >= bounds.len() { d1 } else { bounds[idx] };
                        let local = if (hi - lo).abs() < 1e-12 {
                            0.0
                        } else {
                            (x - lo) / (hi - lo)
                        };
                        let (e0, e1) = (
                            encode.get(idx * 2).copied().unwrap_or(0.0),
                            encode.get(idx * 2 + 1).copied().unwrap_or(1.0),
                        );
                        let sub_t = (e0 + (e1 - e0) * local).clamp(0.0, 1.0);
                        match funcs.get(idx) {
                            Some(sf) => self.eval_function(sf, sub_t, depth + 1),
                            None => Vec::new(),
                        }
                    }
                    0 => {
                        let data = match b.map.get("DataSource") {
                            Some(Obj::Str(s)) => s.borrow().clone(),
                            Some(Obj::File(f)) => f.borrow().buf.clone(),
                            _ => Vec::new(),
                        };
                        let size = b.map.get("Size").map(nums_of).unwrap_or_default();
                        let bps = b
                            .map
                            .get("BitsPerSample")
                            .and_then(|o| o.num())
                            .unwrap_or(8.0) as u32;
                        let range = b.map.get("Range").map(nums_of).unwrap_or_default();
                        let nout = (range.len() / 2).max(1);
                        let n0 = size.first().copied().unwrap_or(2.0).max(1.0) as usize;
                        let i = ((t * (n0 as f64 - 1.0)).round() as usize).min(n0 - 1);
                        let max = ((1u64 << bps.min(32)) - 1) as f64;
                        let mut br = BitReader::new(&data);
                        br.skip(i * nout * bps as usize);
                        (0..nout)
                            .map(|k| {
                                let v = br.read(bps) as f64 / max;
                                let (r0, r1) = (
                                    range.get(k * 2).copied().unwrap_or(0.0),
                                    range.get(k * 2 + 1).copied().unwrap_or(1.0),
                                );
                                r0 + (r1 - r0) * v
                            })
                            .collect()
                    }
                    _ => vec![t],
                }
            }
            _ => Vec::new(),
        }
    }

    // ----- files -----

    /// Decode a lazy filter on the main file now (first read).
    fn materialize(&mut self, f: &Rc<RefCell<FileObj>>) {
        let pending = f.borrow().pending_filter.clone();
        let Some(fname) = pending else { return };
        let raw = self.file_raw_for_filter(f, &fname);
        let decoded = match &*fname {
            "ASCIIHexDecode" => decode_asciihex(&raw),
            "ASCII85Decode" => decode_ascii85(&raw),
            "RunLengthDecode" => decode_runlength(&raw),
            "FlateDecode" => inflate(&raw),
            "LZWDecode" => {
                self.warn("LZWDecode filter is not supported; data skipped");
                Vec::new()
            }
            _ => raw,
        };
        let mut fb = f.borrow_mut();
        fb.buf = decoded;
        fb.pos = 0;
        fb.main = false;
        fb.pending_filter = None;
    }

    fn file_read(&mut self, f: &Rc<RefCell<FileObj>>, n: usize) -> Vec<u8> {
        self.materialize(f);
        let main = f.borrow().main;
        if main {
            // Skip the single whitespace after the operator, then read raw.
            if matches!(
                self.scanner.peek(),
                Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'\t')
            ) {
                self.scanner.pos += 1;
            }
            return self.scanner.take_raw(n).to_vec();
        }
        let mut fb = f.borrow_mut();
        let start = fb.pos.min(fb.buf.len());
        let end = (start + n).min(fb.buf.len());
        fb.pos = end;
        fb.buf[start..end].to_vec()
    }

    fn file_read_hex(&mut self, f: &Rc<RefCell<FileObj>>, n: usize) -> Vec<u8> {
        self.materialize(f);
        let main = f.borrow().main;
        let mut out = Vec::with_capacity(n);
        let mut hi: Option<u8> = None;
        if main {
            while out.len() < n {
                let Some(c) = self.scanner.peek() else { break };
                self.scanner.pos += 1;
                let v = match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    b'>' => break,
                    _ => continue,
                };
                match hi.take() {
                    Some(h) => out.push(h * 16 + v),
                    None => hi = Some(v),
                }
            }
            return out;
        }
        let mut fb = f.borrow_mut();
        while out.len() < n && fb.pos < fb.buf.len() {
            let c = fb.buf[fb.pos];
            fb.pos += 1;
            let v = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                b'>' => break,
                _ => continue,
            };
            match hi.take() {
                Some(h) => out.push(h * 16 + v),
                None => hi = Some(v),
            }
        }
        out
    }

    /// Raw bytes for a filter on `currentfile`: the encoded data up to its
    /// end marker (`~>` for ASCII85, `>` for hex, or the rest of the stream
    /// for binary filters, bounded by the next `%%` DSC line).
    fn file_raw_for_filter(&mut self, f: &Rc<RefCell<FileObj>>, fname: &str) -> Vec<u8> {
        if !f.borrow().main {
            let fb = f.borrow();
            return fb.buf[fb.pos.min(fb.buf.len())..].to_vec();
        }
        if matches!(
            self.scanner.peek(),
            Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'\t')
        ) {
            self.scanner.pos += 1;
        }
        let src = self.scanner.src;
        let start = self.scanner.pos.min(src.len());
        let end = match fname {
            "ASCII85Decode" => src[start..]
                .windows(2)
                .position(|w| w == b"~>")
                .map(|i| start + i + 2)
                .unwrap_or(src.len()),
            "ASCIIHexDecode" => src[start..]
                .iter()
                .position(|c| *c == b'>')
                .map(|i| start + i + 1)
                .unwrap_or(src.len()),
            _ => {
                // Binary: up to the next DSC comment line.
                src[start..]
                    .windows(3)
                    .position(|w| w == b"\n%%")
                    .map(|i| start + i + 1)
                    .unwrap_or(src.len())
            }
        };
        self.scanner.pos = end;
        src[start..end].to_vec()
    }

    fn file_token(&mut self, f: &Rc<RefCell<FileObj>>) -> Option<Obj> {
        self.materialize(f);
        if f.borrow().main {
            return match self.scanner.next() {
                Token::Eof | Token::ProcEnd => None,
                Token::ProcStart => Some(self.read_proc()),
                Token::Obj(o) => Some(o),
            };
        }
        let text = {
            let fb = f.borrow();
            fb.buf[fb.pos.min(fb.buf.len())..].to_vec()
        };
        let mut sc = Scanner { src: &text, pos: 0 };
        let t = sc.next();
        f.borrow_mut().pos += sc.pos;
        match t {
            Token::Eof | Token::ProcEnd => None,
            Token::ProcStart => Some(mk_array(Vec::new(), true)),
            Token::Obj(o) => Some(o),
        }
    }

    fn skip_eexec(&mut self) {
        let src = self.scanner.src;
        let start = self.scanner.pos.min(src.len());
        // The clear-text trailer is 512 zeros followed by cleartomark.
        let end = src[start..]
            .windows(11)
            .position(|w| w == b"cleartomark")
            .map(|i| start + i + 11)
            .unwrap_or(src.len());
        self.scanner.pos = end;
    }

    // ----- images -----

    fn image_op(&mut self, op: &str) {
        // Level 2 dictionary form.
        if let Some(Obj::Dict(d)) = self.stack.last().cloned() {
            self.pop();
            let b = d.borrow();
            let w = b.map.get("Width").and_then(|o| o.num()).unwrap_or(0.0) as u32;
            let h = b.map.get("Height").and_then(|o| o.num()).unwrap_or(0.0) as u32;
            let bpc = b
                .map
                .get("BitsPerComponent")
                .and_then(|o| o.num())
                .unwrap_or(8.0) as u32;
            let m = b
                .map
                .get("ImageMatrix")
                .and_then(matrix_of)
                .unwrap_or(Affine::IDENTITY);
            let decode = b.map.get("Decode").map(nums_of).unwrap_or_default();
            let src = b.map.get("DataSource").cloned();
            let multi = b
                .map
                .get("MultipleDataSources")
                .map(|o| matches!(o, Obj::Bool(true)))
                .unwrap_or(false);
            drop(b);
            let ncomp = if op == "imagemask" {
                1
            } else {
                self.color_space
            };
            let is_mask = op == "imagemask";
            let Some(src) = src else { return };
            let bytes_needed = ((w as usize * ncomp * bpc as usize).div_ceil(8)) * h as usize;
            let data = if multi {
                // One source per component, read row by row in turn.
                if let Obj::Array(srcs, _) = &src {
                    let srcs = srcs.borrow().clone();
                    let planes = self.read_planes(&srcs, w as usize, h as usize, bpc);
                    interleave_planes(&planes, w as usize, h as usize, bpc)
                } else {
                    Vec::new()
                }
            } else {
                self.read_source(&src, bytes_needed)
            };
            self.place_image(w, h, bpc, ncomp, is_mask, &decode, &data, m);
            return;
        }
        // Level 1 forms: width height bpc matrix proc image
        //                width height bool matrix proc imagemask
        //                width height bpc matrix proc0..n multi ncomp colorimage
        let (ncomp, multi) = if op == "colorimage" {
            let n = self.pop_int().unwrap_or(3).clamp(1, 4) as usize;
            let multi = matches!(self.pop(), Some(Obj::Bool(true)));
            (n, multi)
        } else {
            (1, false)
        };
        let mut procs = Vec::new();
        let nprocs = if multi { ncomp } else { 1 };
        for _ in 0..nprocs {
            if let Some(p) = self.pop() {
                procs.push(p);
            }
        }
        procs.reverse();
        let Some(m) = self.pop_matrix() else { return };
        let third = self.pop();
        let h = self.pop_num().unwrap_or(0.0) as u32;
        let w = self.pop_num().unwrap_or(0.0) as u32;
        let (bpc, is_mask, decode) = match (op, third) {
            ("imagemask", Some(Obj::Bool(polarity))) => (
                1,
                true,
                if polarity {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                },
            ),
            (_, Some(o)) => (o.num().unwrap_or(8.0) as u32, false, Vec::new()),
            _ => (8, false, Vec::new()),
        };
        let per_plane =
            ((w as usize * if multi { 1 } else { ncomp } * bpc as usize).div_ceil(8)) * h as usize;
        let data = if multi {
            let planes = self.read_planes(&procs, w as usize, h as usize, bpc);
            interleave_planes(&planes, w as usize, h as usize, bpc)
        } else {
            match procs.first() {
                Some(p) => self.read_source(p, per_plane),
                None => Vec::new(),
            }
        };
        self.place_image(w, h, bpc, ncomp, is_mask, &decode, &data, m);
    }

    /// Multiple data sources deliver one row each in turn (red row, green
    /// row, blue row, next red row ...), so they are read round-robin.
    fn read_planes(&mut self, srcs: &[Obj], w: usize, h: usize, bpc: u32) -> Vec<Vec<u8>> {
        let row_bytes = (w * bpc as usize).div_ceil(8);
        let mut planes: Vec<Vec<u8>> = vec![Vec::with_capacity(row_bytes * h); srcs.len()];
        let mut pending: Vec<Vec<u8>> = vec![Vec::new(); srcs.len()];
        for _ in 0..h {
            for (k, src) in srcs.iter().enumerate() {
                let mut guard = 0;
                while pending[k].len() < row_bytes && guard < 10_000 {
                    guard += 1;
                    let chunk = self.read_source_once(src, row_bytes - pending[k].len());
                    if chunk.is_empty() {
                        break;
                    }
                    pending[k].extend_from_slice(&chunk);
                }
                if pending[k].len() < row_bytes {
                    return planes;
                }
                let rest = pending[k].split_off(row_bytes);
                planes[k].extend_from_slice(&pending[k]);
                pending[k] = rest;
            }
        }
        planes
    }

    /// One call of a data source (or up to `n` bytes of a file/string).
    fn read_source_once(&mut self, src: &Obj, n: usize) -> Vec<u8> {
        match src {
            Obj::Array(_, true) | Obj::Op(_) | Obj::Name(_, true) => {
                let before = self.stack.len();
                let _ = self.call(src);
                if self.stack.len() <= before {
                    return Vec::new();
                }
                match self.pop() {
                    Some(Obj::Str(s)) => s.borrow().clone(),
                    _ => Vec::new(),
                }
            }
            other => self.read_source(other, n),
        }
    }

    /// Pull `n` bytes from a data source: a procedure returning strings, a
    /// file, or a string.
    fn read_source(&mut self, src: &Obj, n: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n.min(MAX_STRING));
        match src {
            Obj::Str(s) => out.extend_from_slice(&s.borrow()),
            Obj::File(f) => {
                self.materialize(f);
                let is_jpeg = self.jpeg_files.contains(&(Rc::as_ptr(f) as usize));
                if is_jpeg {
                    // Whole JPEG stream; decoded by place_image.
                    let fb = f.borrow();
                    let mut v = b"JPEG".to_vec();
                    v.extend_from_slice(&fb.buf[fb.pos.min(fb.buf.len())..]);
                    return v;
                }
                out = self.file_read(f, n);
            }
            Obj::Array(_, true) | Obj::Op(_) | Obj::Name(_, true) => {
                let mut guard = 0;
                while out.len() < n && guard < 1_000_000 {
                    guard += 1;
                    let before = self.stack.len();
                    let _ = self.call(src);
                    if self.stack.len() <= before {
                        break;
                    }
                    match self.pop() {
                        Some(Obj::Str(s)) => {
                            let b = s.borrow();
                            if b.is_empty() {
                                break;
                            }
                            out.extend_from_slice(&b);
                        }
                        _ => break,
                    }
                    if self.ops > MAX_OPS {
                        break;
                    }
                }
            }
            _ => {}
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn place_image(
        &mut self,
        w: u32,
        h: u32,
        bpc: u32,
        ncomp: usize,
        is_mask: bool,
        decode: &[f64],
        data: &[u8],
        m: Affine,
    ) {
        if w == 0 || h == 0 || w > 20_000 || h > 20_000 || (w as u64) * (h as u64) > 80_000_000 {
            return;
        }
        let mut rgba: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        if data.starts_with(b"JPEG") {
            match image::load_from_memory_with_format(&data[4..], image::ImageFormat::Jpeg) {
                Ok(img) => {
                    let img = img.to_rgba8();
                    let (jw, jh) = img.dimensions();
                    let png = crate::svg::encode_png(jw, jh, img.as_raw());
                    self.emit_image(png, jw, jh, m);
                }
                Err(e) => self.warn(format!("JPEG image not decoded: {e}")),
            }
            return;
        }
        let row_bytes = (w as usize * ncomp * bpc as usize).div_ceil(8);
        if data.len() < row_bytes * h as usize {
            self.warn("image data shorter than its size; image skipped");
            return;
        }
        let max = ((1u32 << bpc.min(16)) - 1) as f32;
        let inverted = decode.first().map(|v| *v == 1.0).unwrap_or(false);
        let [fr, fg, fb] = self.gs.color.to_rgb8();
        for y in 0..h as usize {
            let row = &data[y * row_bytes..(y + 1) * row_bytes];
            let mut bits = BitReader::new(row);
            for _ in 0..w {
                let mut comps = [0f32; 4];
                let mut raw0 = 0u32;
                for (ci, c) in comps.iter_mut().enumerate().take(ncomp.min(4)) {
                    let v = bits.read(bpc);
                    if ci == 0 {
                        raw0 = v;
                    }
                    *c = v as f32 / max;
                }
                if is_mask {
                    // Sample 0 paints (Decode [0 1]); 1 paints with [1 0].
                    let paint = (raw0 == 0) != inverted;
                    rgba.extend_from_slice(&[fr, fg, fb, if paint { 255 } else { 0 }]);
                    continue;
                }
                let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                let (r, g, b) = if let Some(pal) = &self.indexed_palette {
                    let i = (raw0 as usize) * 3;
                    (
                        *pal.get(i).unwrap_or(&0),
                        *pal.get(i + 1).unwrap_or(&0),
                        *pal.get(i + 2).unwrap_or(&0),
                    )
                } else {
                    match ncomp {
                        3 => (to8(comps[0]), to8(comps[1]), to8(comps[2])),
                        4 => {
                            let (c, mm, yy, k) = (comps[0], comps[1], comps[2], comps[3]);
                            (
                                to8((1.0 - c) * (1.0 - k)),
                                to8((1.0 - mm) * (1.0 - k)),
                                to8((1.0 - yy) * (1.0 - k)),
                            )
                        }
                        _ => {
                            let v = if inverted { 1.0 - comps[0] } else { comps[0] };
                            let v = if self.separation { 1.0 - v } else { v };
                            (to8(v), to8(v), to8(v))
                        }
                    }
                };
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
        let png = crate::svg::encode_png(w, h, &rgba);
        self.emit_image(png, w, h, m);
    }

    fn emit_image(&mut self, png: Vec<u8>, w: u32, h: u32, m: Affine) {
        // The image matrix maps user space to image space (w x h, y down);
        // its inverse places the image: unit square = inverse(m) applied to
        // [0 w] x [0 h] with a flip.
        let inv = m.inverse();
        let to_user = inv * Affine::new([w as f64, 0.0, 0.0, h as f64, 0.0, 0.0]);
        // Our bitmap: rect (0,0)-(1,1) with the image top at y = 1.
        let flip = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, 1.0]);
        let transform = self.gs.ctm * to_user * flip;
        let mut shape = self.new_shape(ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            width_px: w,
            height_px: h,
            png,
        });
        shape.transform = transform;
        shape.fill = Fill::None;
        shape.stroke = None;
        self.emit(shape);
    }

    // ----- text -----

    /// Advance for `n` characters in user space: 0.5 em each (no metrics).
    fn text_advance(&self, n: usize) -> (f64, f64) {
        let Some(f) = &self.gs.font else {
            return (0.0, 0.0);
        };
        let em = f.matrix.as_coeffs();
        let w = match &f.widths {
            Some(_) => 500.0 * n as f64,
            None => 500.0 * n as f64,
        };
        let v = Vec2::new(em[0] * w, em[1] * w);
        (v.x, v.y)
    }

    fn show_text(&mut self, bytes: &[u8], spacing: Option<(f64, f64, f64, f64, i64)>) {
        let Some(font) = self.gs.font.clone() else {
            return;
        };
        let Some(cur) = self.gs.current else {
            return;
        };
        if bytes.is_empty() {
            return;
        }
        let text: String = bytes.iter().map(|&b| latin1_char(b)).collect();
        // Device matrix of the text origin: ctm * translate(current) * font matrix (x1000 to glyph em).
        let em_matrix = font.matrix * Affine::scale(1000.0);
        let trm = self.gs.ctm * Affine::translate(cur.to_vec2()) * em_matrix;
        let c = trm.as_coeffs();
        let scale = (c[0] * c[3] - c[1] * c[2]).abs().sqrt();
        if scale <= 0.0 || !scale.is_finite() {
            return;
        }
        let size_pt = scale / PT_MM;
        let transform = trm * Affine::scale(1.0 / scale);
        let span = TextSpan {
            bold: font.bold,
            italic: font.italic,
            ..TextSpan::new(text.clone(), font.family.clone(), size_pt)
        };
        let mut shape = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: Point::ZERO,
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        });
        shape.transform = transform;
        shape.fill = Fill::Solid(self.gs.color);
        shape.stroke = None;
        self.emit(shape);
        // Advance the current point.
        let n = bytes.len();
        let (mut ax, mut ay) = self.text_advance(n);
        if let Some((cax, cay, wx, wy, ch)) = spacing {
            ax += cax * n as f64;
            ay += cay * n as f64;
            let spaces = bytes.iter().filter(|b| **b as i64 == ch).count() as f64;
            ax += wx * spaces;
            ay += wy * spaces;
        }
        self.gs.current = Some(cur + Vec2::new(ax, ay));
        let d = self.user_to_device(cur + Vec2::new(ax, ay));
        if self.gs.path.elements().is_empty() {
            self.gs.path.move_to(d);
        }
    }
}

fn m_is_num(stack: &[Obj]) -> bool {
    matches!(stack.last(), Some(Obj::Int(_)) | Some(Obj::Real(_)))
}

fn latin1_char(b: u8) -> char {
    match b {
        0x80..=0x9f => char::from_u32(match b {
            0x80 => 0x20ac,
            0x82 => 0x201a,
            0x83 => 0x192,
            0x84 => 0x201e,
            0x85 => 0x2026,
            0x86 => 0x2020,
            0x87 => 0x2021,
            0x88 => 0x2c6,
            0x89 => 0x2030,
            0x8a => 0x160,
            0x8b => 0x2039,
            0x8c => 0x152,
            0x8e => 0x17d,
            0x91 => 0x2018,
            0x92 => 0x2019,
            0x93 => 0x201c,
            0x94 => 0x201d,
            0x95 => 0x2022,
            0x96 => 0x2013,
            0x97 => 0x2014,
            0x98 => 0x2dc,
            0x99 => 0x2122,
            0x9a => 0x161,
            0x9b => 0x203a,
            0x9c => 0x153,
            0x9e => 0x17e,
            0x9f => 0x178,
            _ => 0xfffd,
        })
        .unwrap_or('\u{fffd}'),
        _ => b as char,
    }
}

fn glyph_char(name: &str) -> char {
    if let Some(hex) = name.strip_prefix("uni") {
        if let Ok(v) = u32::from_str_radix(&hex[..hex.len().min(4)], 16) {
            return char::from_u32(v).unwrap_or('?');
        }
    }
    if name.chars().count() == 1 {
        return name.chars().next().unwrap_or('?');
    }
    match name {
        "space" => ' ',
        "period" => '.',
        "comma" => ',',
        "hyphen" | "minus" => '-',
        "colon" => ':',
        "semicolon" => ';',
        "quotesingle" => '\'',
        "quotedbl" => '"',
        "ampersand" => '&',
        "zero" => '0',
        "one" => '1',
        "two" => '2',
        "three" => '3',
        "four" => '4',
        "five" => '5',
        "six" => '6',
        "seven" => '7',
        "eight" => '8',
        "nine" => '9',
        "eacute" => 'é',
        "egrave" => 'è',
        "agrave" => 'à',
        "aacute" => 'á',
        "ccedilla" => 'ç',
        "atilde" => 'ã',
        "otilde" => 'õ',
        "ntilde" => 'ñ',
        "uacute" => 'ú',
        "oacute" => 'ó',
        "iacute" => 'í',
        "acircumflex" => 'â',
        "ecircumflex" => 'ê',
        "ocircumflex" => 'ô',
        "udieresis" => 'ü',
        "odieresis" => 'ö',
        "adieresis" => 'ä',
        _ => '?',
    }
}

fn key_of(o: &Obj) -> Option<Rc<str>> {
    match o {
        Obj::Name(n, _) => Some(n.clone()),
        Obj::Str(s) => Some(Rc::from(String::from_utf8_lossy(&s.borrow()).as_ref())),
        Obj::Int(i) => Some(Rc::from(i.to_string().as_str())),
        Obj::Real(r) => Some(Rc::from(format!("{r}").as_str())),
        _ => None,
    }
}

fn to_text(o: &Obj) -> String {
    match o {
        Obj::Int(i) => i.to_string(),
        Obj::Real(r) => {
            let s = format!("{r:.6}");
            let s = s.trim_end_matches('0').trim_end_matches('.');
            if s.is_empty() {
                "0".into()
            } else {
                s.to_string()
            }
        }
        Obj::Bool(b) => b.to_string(),
        Obj::Name(n, _) => n.to_string(),
        Obj::Str(s) => String::from_utf8_lossy(&s.borrow()).into_owned(),
        Obj::Op(op) => op.to_string(),
        _ => "--nostringval--".into(),
    }
}

fn to_radix(v: i64, radix: u32) -> String {
    if v == 0 {
        return "0".into();
    }
    let mut n = v.unsigned_abs();
    let mut digits = Vec::new();
    while n > 0 {
        let d = (n % radix as u64) as u32;
        digits.push(
            std::char::from_digit(d, radix)
                .unwrap_or('0')
                .to_ascii_uppercase(),
        );
        n /= radix as u64;
    }
    if v < 0 {
        digits.push('-');
    }
    digits.iter().rev().collect()
}

fn nums_of(o: &Obj) -> Vec<f64> {
    match o {
        Obj::Array(a, _) => a.borrow().iter().filter_map(|x| x.num()).collect(),
        Obj::Str(s) => s.borrow().iter().map(|b| *b as f64).collect(),
        other => other.num().into_iter().collect(),
    }
}

fn matrix_of(o: &Obj) -> Option<Affine> {
    let v = nums_of(o);
    if v.len() == 6 && v.iter().all(|x| x.is_finite()) {
        Some(Affine::new([v[0], v[1], v[2], v[3], v[4], v[5]]))
    } else {
        None
    }
}

fn affine_to_obj(a: Affine) -> Obj {
    mk_array(a.as_coeffs().iter().map(|v| Obj::Real(*v)).collect(), false)
}

/// Store `a` into the array object `m` and return it.
fn matrix_obj(m: &Arr, a: Affine) -> Obj {
    let c = a.as_coeffs();
    {
        let mut b = m.borrow_mut();
        if b.len() >= 6 {
            for i in 0..6 {
                b[i] = Obj::Real(c[i]);
            }
        } else {
            *b = c.iter().map(|v| Obj::Real(*v)).collect();
        }
    }
    Obj::Array(m.clone(), false)
}

fn color_space_components(cs: &Obj) -> usize {
    let name = match cs {
        Obj::Name(n, _) => n.to_string(),
        Obj::Array(a, _) => a
            .borrow()
            .first()
            .and_then(key_of)
            .map(|k| k.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    };
    match name.as_str() {
        "DeviceGray" | "CalGray" | "Indexed" | "Separation" => 1,
        "DeviceRGB" | "CalRGB" | "Lab" => 3,
        "DeviceCMYK" => 4,
        "DeviceN" => match cs {
            Obj::Array(a, _) => a
                .borrow()
                .get(1)
                .map(|names| match names {
                    Obj::Array(n, _) => n.borrow().len(),
                    _ => 1,
                })
                .unwrap_or(1),
            _ => 1,
        },
        "ICCBased" => match cs {
            Obj::Array(a, _) => a
                .borrow()
                .get(1)
                .and_then(|d| match d {
                    Obj::Dict(d) => d.borrow().map.get("N").and_then(|o| o.num()),
                    _ => None,
                })
                .unwrap_or(3.0) as usize,
            _ => 3,
        },
        "Pattern" => 0,
        _ => 1,
    }
}

fn color_from(v: &[f64], n: usize) -> Option<Color> {
    let c = |i: usize| v.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0) as f32;
    match (n, v.len()) {
        (1, 1) => Some(Color::Gray { v: c(0) }),
        (3, 3) => Some(Color::Rgb {
            r: c(0),
            g: c(1),
            b: c(2),
        }),
        (4, 4) => Some(Color::Cmyk {
            c: c(0),
            m: c(1),
            y: c(2),
            k: c(3),
        }),
        (_, 1) => Some(Color::Gray { v: 1.0 - c(0) }),
        (_, 3) => Some(Color::Rgb {
            r: c(0),
            g: c(1),
            b: c(2),
        }),
        (_, 4) => Some(Color::Cmyk {
            c: c(0),
            m: c(1),
            y: c(2),
            k: c(3),
        }),
        _ => None,
    }
}

fn interleave_planes(planes: &[Vec<u8>], w: usize, h: usize, bpc: u32) -> Vec<u8> {
    // Only 8-bit planes are interleaved exactly; others are expanded to 8 bits.
    let n = planes.len();
    let mut out = Vec::with_capacity(w * h * n);
    let row_bytes = (w * bpc as usize).div_ceil(8);
    let max = ((1u32 << bpc.min(16)) - 1) as f32;
    for y in 0..h {
        let mut readers: Vec<BitReader> = planes
            .iter()
            .map(|p| {
                BitReader::new(
                    p.get(y * row_bytes..((y + 1) * row_bytes).min(p.len()))
                        .unwrap_or(&[]),
                )
            })
            .collect();
        for _ in 0..w {
            for r in readers.iter_mut() {
                let v = r.read(bpc) as f32 / max;
                out.push((v * 255.0).round() as u8);
            }
        }
    }
    out
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0 }
    }
    fn skip(&mut self, bits: usize) {
        self.pos += bits;
    }
    fn read(&mut self, bits: u32) -> u32 {
        let mut v = 0u32;
        for _ in 0..bits.min(32) {
            let byte = self.data.get(self.pos / 8).copied().unwrap_or(0);
            let bit = (byte >> (7 - (self.pos % 8))) & 1;
            v = (v << 1) | bit as u32;
            self.pos += 1;
        }
        v
    }
}

fn font_info(d: &Dict) -> FontInfo {
    let b = d.borrow();
    let name = b
        .map
        .get("FontName")
        .and_then(key_of)
        .map(|k| k.to_string())
        .unwrap_or_else(|| "Helvetica".into());
    let matrix = b
        .map
        .get("FontMatrix")
        .and_then(matrix_of)
        .unwrap_or(Affine::scale(0.001));
    let (family, bold, italic) = split_name(&name);
    FontInfo {
        family,
        bold,
        italic,
        matrix,
        widths: None,
    }
}

fn split_name(base: &str) -> (String, bool, bool) {
    let name = match base.find('+') {
        Some(6) => &base[7..],
        _ => base,
    };
    let lower = name.to_ascii_lowercase();
    let bold = lower.contains("bold") || lower.contains("black") || lower.contains("heavy");
    let italic = lower.contains("italic") || lower.contains("oblique");
    let mut family = name.split([',', '-']).next().unwrap_or(name).to_string();
    for suffix in ["MT", "PSMT", "PS"] {
        if family.len() > suffix.len() + 2 && family.ends_with(suffix) {
            family.truncate(family.len() - suffix.len());
        }
    }
    let family = match family.as_str() {
        "Times" | "TimesNewRoman" | "TimesNewRomanPS" => "Times New Roman".to_string(),
        "Helvetica" | "ArialMT" => "Arial".to_string(),
        "Courier" | "CourierNew" => "Courier New".to_string(),
        other => other.to_string(),
    };
    (family, bold, italic)
}

fn standard_encoding_array() -> Obj {
    let mut v = Vec::with_capacity(256);
    for i in 0..256u32 {
        let name = match char::from_u32(i) {
            Some(c) if (0x20..0x7f).contains(&i) => match c {
                ' ' => "space".to_string(),
                other => other.to_string(),
            },
            _ => ".notdef".to_string(),
        };
        v.push(Obj::Name(Rc::from(name.as_str()), false));
    }
    mk_array(v, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ps: &str) -> Imported {
        let mut ids = IdSource::default();
        parse(ps.as_bytes(), &mut ids).expect("parse")
    }

    #[test]
    fn paths_fills_strokes_and_bounding_box() {
        let imp = run(
            "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 200 100\n1 0 0 setrgbcolor 4 setlinewidth newpath 10 10 moveto 60 10 lineto 60 40 lineto closepath gsave 0 0 1 setrgbcolor fill grestore stroke\n0.5 setgray 100 50 50 30 rectfill\nshowpage\n",
        );
        assert!(
            (imp.size.width - 200.0 * PT_MM).abs() < 1e-9,
            "{:?}",
            imp.size
        );
        assert_eq!(imp.shapes.len(), 3, "{:?}", imp.warnings);
        assert_eq!(
            imp.shapes[0].fill,
            Fill::Solid(Color::Rgb {
                r: 0.0,
                g: 0.0,
                b: 1.0
            })
        );
        let s = imp.shapes[1].stroke.as_ref().expect("stroke");
        assert_eq!(
            s.color,
            Color::Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0
            }
        );
        assert!((s.width - 4.0 * PT_MM).abs() < 1e-9);
        let b = imp.shapes[2].bounds();
        assert!(
            (b.x0 - 100.0 * PT_MM).abs() < 1e-6 && (b.y1 - 80.0 * PT_MM).abs() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn procedures_dictionaries_and_control_flow_run_a_prolog() {
        let ps = "%!PS\n/MyDict 10 dict def MyDict begin\n/sq { dup mul } bind def\n/box { /h exch def /w exch def /y exch def /x exch def x y moveto w 0 rlineto 0 h rlineto w neg 0 rlineto closepath } def\n/n 0 def 1 1 3 { /n exch n add def } for\n0 1 n 1 sub { 20 mul 10 exch 15 15 box fill } for\n[1 2 3] { pop } forall\n(abc) length 3 eq { 0 0 1 setrgbcolor } { 1 0 0 setrgbcolor } ifelse\n2 sq 4 eq not { 1 1 0 setrgbcolor } if\n100 100 10 10 box fill\nend\n";
        let imp = run(ps);
        // n = 6, so six boxes at y = 0, 20, ... 100 plus one blue box.
        assert_eq!(imp.shapes.len(), 7, "{:?}", imp.warnings);
        assert_eq!(
            imp.shapes[6].fill,
            Fill::Solid(Color::Rgb {
                r: 0.0,
                g: 0.0,
                b: 1.0
            })
        );
        assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    }

    #[test]
    fn arcs_translate_scale_and_clip() {
        let ps = "%!PS\n%%BoundingBox: 0 0 100 100\ngsave 50 50 translate 2 2 scale newpath 0 0 10 0 360 arc closepath fill grestore\ngsave newpath 0 0 20 20 rectclip 0 0 100 100 rectfill grestore\n";
        let imp = run(ps);
        assert_eq!(imp.shapes.len(), 2, "{:?}", imp.warnings);
        let b = imp.shapes[0].bounds();
        // Circle of radius 20 pt centred at (50,50).
        assert!((b.width() - 40.0 * PT_MM).abs() < 0.05, "{b:?}");
        assert!((b.center().x - 50.0 * PT_MM).abs() < 0.05);
        match &imp.shapes[1].kind {
            ShapeKind::ClipFrame { frame, .. } => {
                let fb = frame.bounds();
                assert!((fb.x1 - 20.0 * PT_MM).abs() < 1e-6, "{fb:?}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_images_and_eexec_skip() {
        let ps = "%!PS\n%%BoundingBox: 0 0 100 100\n/Helvetica-Bold findfont 12 scalefont setfont 10 20 moveto (Hello) show\n/Times-Roman 10 selectfont 10 40 moveto (World) show\ngsave 50 0 0 25 10 60 matrix concat 10 60 translate 50 25 scale\n2 1 8 [2 0 0 -1 0 1] {<ff000000ff00>} false 3 colorimage grestore\ncurrentfile eexec\n0000000000000000 cleartomark\n0 0 10 10 rectfill\n";
        let imp = run(ps);
        let texts: Vec<(String, f64, bool)> = imp
            .shapes
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    Some((spans[0].text.clone(), spans[0].size_pt, spans[0].bold))
                }
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2, "{:?} {:?}", texts, imp.warnings);
        assert_eq!(texts[0].0, "Hello");
        assert!((texts[0].1 - 12.0).abs() < 1e-6 && texts[0].2);
        assert_eq!(texts[1].0, "World");
        let img = imp
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
            .expect("image");
        match &img.kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                png,
                ..
            } => {
                assert_eq!((*width_px, *height_px), (2, 1));
                let pm = tiny_skia::Pixmap::decode_png(png).unwrap();
                assert_eq!(pm.pixels()[0].red(), 255);
                assert_eq!(pm.pixels()[1].green(), 255);
            }
            _ => unreachable!(),
        }
        // The rectangle after the eexec section was still painted.
        assert!(imp
            .shapes
            .iter()
            .any(|s| matches!(&s.kind, ShapeKind::Path { .. }) && s.bounds().width() > 3.0));
    }

    #[test]
    fn level2_image_dictionary_with_ascii85_and_dos_eps_header() {
        // 2x2 gray image, 8 bpc: 00 80 ff 40 as ASCII85.
        let data = {
            let raw = [0u8, 0x80, 0xff, 0x40];
            let v = u32::from_be_bytes(raw);
            let mut digits = Vec::new();
            let mut n = v;
            for _ in 0..5 {
                digits.push((n % 85) as u8 + b'!');
                n /= 85;
            }
            digits.reverse();
            String::from_utf8(digits).unwrap()
        };
        let ps = format!(
            "%!PS\n%%BoundingBox: 0 0 20 20\n/DeviceGray setcolorspace\n20 20 scale\n<< /ImageType 1 /Width 2 /Height 2 /BitsPerComponent 8 /Decode [0 1] /ImageMatrix [2 0 0 -2 0 2] /DataSource currentfile /ASCII85Decode filter >> image\n{data}~>\n0 0 1 1 rectfill\n"
        );
        let mut bytes = vec![0xc5, 0xd0, 0xd3, 0xc6];
        bytes.extend_from_slice(&30u32.to_le_bytes());
        bytes.extend_from_slice(&(ps.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&[0u8; 18]);
        bytes.extend_from_slice(ps.as_bytes());
        assert!(is_eps(&bytes));
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        let img = imp
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
            .unwrap_or_else(|| panic!("{:?}", imp.warnings));
        match &img.kind {
            ShapeKind::Bitmap { png, .. } => {
                let pm = tiny_skia::Pixmap::decode_png(png).unwrap();
                let px = pm.pixels();
                assert_eq!(px[0].red(), 0);
                assert_eq!(px[1].red(), 128);
                assert_eq!(px[2].red(), 255);
            }
            _ => unreachable!(),
        }
        // Image covers the 20 x 20 pt square.
        let b = img.bounds();
        assert!((b.width() - 20.0 * PT_MM).abs() < 1e-6, "{b:?}");
        // The rectangle after the image data was painted (the filter
        // consumed exactly up to ~>).
        assert_eq!(imp.shapes.len(), 2, "{:?}", imp.warnings);
    }

    #[test]
    fn shading_pattern_and_axial_shfill() {
        let ps = "%!PS\n%%BoundingBox: 0 0 100 100\n<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >> >> matrix makepattern setpattern 0 0 100 50 rectfill\n";
        let imp = run(ps);
        assert_eq!(imp.shapes.len(), 1, "{:?}", imp.warnings);
        match &imp.shapes[0].fill {
            Fill::Fountain(f) => {
                assert_eq!(f.kind, FountainKind::Linear);
                assert_eq!(
                    f.stops.first().map(|s| s.color),
                    Some(Color::Rgb {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0
                    })
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn garbage_and_runaway_programs_are_bounded() {
        let mut ids = IdSource::default();
        assert!(parse(b"hello world", &mut ids).is_err());
        let imp = parse(b"%!PS\n{ } loop\n", &mut ids).expect("parse");
        assert!(imp.warnings.iter().any(|w| w.contains("too long")));
        let imp = parse(b"%!PS\n/r { r } def r\n", &mut ids).expect("parse");
        assert!(imp.warnings.iter().any(|w| w.contains("deeply")));
        let imp = parse(
            b"%!PS\n1 2 3 add add add add pop pop ) ( ] [ } { < > >> <<",
            &mut ids,
        )
        .expect("parse");
        assert!(imp.shapes.is_empty());
    }
}
