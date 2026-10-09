//! Colour palettes: the built-in ones (generated, no licensed libraries),
//! palette files (.tdpal JSON, Adobe .ase, GIMP .gpl, Adobe .aco) and the
//! document palette.

use tracedraw_core::Color;

#[derive(Debug, Clone)]
pub struct Palette {
    pub name: String,
    /// i18n key when built in; empty for user palettes.
    pub key: &'static str,
    pub colors: Vec<(String, Color)>,
    pub builtin: bool,
    pub path: Option<std::path::PathBuf>,
}

impl Palette {
    pub fn name_key(&self) -> &str {
        if self.key.is_empty() {
            &self.name
        } else {
            self.key
        }
    }

    pub fn display_name(&self) -> String {
        if self.key.is_empty() {
            self.name.clone()
        } else {
            crate::i18n::tr(self.key)
        }
    }
}

/// The default CMYK palette of the reference workspace, top to bottom.
pub fn default_cmyk() -> Vec<(String, Color)> {
    let mut v: Vec<(String, Color)> = Vec::new();
    let mut add = |n: &str, c: f32, m: f32, y: f32, k: f32| {
        v.push((n.to_string(), Color::cmyk_pct(c, m, y, k)));
    };
    add("Black", 0.0, 0.0, 0.0, 100.0);
    add("90% Black", 0.0, 0.0, 0.0, 90.0);
    add("80% Black", 0.0, 0.0, 0.0, 80.0);
    add("70% Black", 0.0, 0.0, 0.0, 70.0);
    add("60% Black", 0.0, 0.0, 0.0, 60.0);
    add("50% Black", 0.0, 0.0, 0.0, 50.0);
    add("40% Black", 0.0, 0.0, 0.0, 40.0);
    add("30% Black", 0.0, 0.0, 0.0, 30.0);
    add("20% Black", 0.0, 0.0, 0.0, 20.0);
    add("10% Black", 0.0, 0.0, 0.0, 10.0);
    add("White", 0.0, 0.0, 0.0, 0.0);
    add("Blue", 100.0, 100.0, 0.0, 0.0);
    add("Cyan", 100.0, 0.0, 0.0, 0.0);
    add("Green", 100.0, 0.0, 100.0, 0.0);
    add("Yellow", 0.0, 0.0, 100.0, 0.0);
    add("Red", 0.0, 100.0, 100.0, 0.0);
    add("Magenta", 0.0, 100.0, 0.0, 0.0);
    add("Purple", 40.0, 100.0, 0.0, 0.0);
    add("Orange", 0.0, 60.0, 100.0, 0.0);
    add("Pink", 0.0, 40.0, 0.0, 0.0);
    add("Baby Blue", 40.0, 0.0, 0.0, 0.0);
    add("Pale Yellow", 0.0, 0.0, 40.0, 0.0);
    add("Lime", 40.0, 0.0, 100.0, 0.0);
    add("Teal", 100.0, 0.0, 40.0, 0.0);
    add("Brown", 0.0, 60.0, 100.0, 40.0);
    add("Navy", 100.0, 100.0, 0.0, 40.0);
    add("Forest Green", 100.0, 0.0, 100.0, 40.0);
    add("Maroon", 0.0, 100.0, 100.0, 40.0);
    add("Gold", 0.0, 20.0, 100.0, 10.0);
    add("Olive", 0.0, 0.0, 100.0, 50.0);
    add("Sky Blue", 60.0, 20.0, 0.0, 0.0);
    add("Lavender", 20.0, 40.0, 0.0, 0.0);
    add("Peach", 0.0, 30.0, 40.0, 0.0);
    add("Mint", 40.0, 0.0, 40.0, 0.0);
    add("Rose", 0.0, 60.0, 20.0, 0.0);
    add("Violet", 60.0, 80.0, 0.0, 0.0);
    add("Turquoise", 80.0, 0.0, 20.0, 0.0);
    add("Tan", 0.0, 20.0, 40.0, 10.0);
    add("Rust", 0.0, 80.0, 100.0, 30.0);
    add("Plum", 40.0, 100.0, 20.0, 20.0);
    v
}

/// Process colours in 10% steps of C, M, Y and K combinations (a subset).
pub fn process_palette() -> Vec<(String, Color)> {
    let mut v = Vec::new();
    for &c in &[0.0f32, 20.0, 40.0, 60.0, 80.0, 100.0] {
        for &m in &[0.0f32, 20.0, 40.0, 60.0, 80.0, 100.0] {
            for &y in &[0.0f32, 40.0, 100.0] {
                v.push((
                    format!("C{c:.0} M{m:.0} Y{y:.0} K0"),
                    Color::cmyk_pct(c, m, y, 0.0),
                ));
            }
        }
    }
    v
}

pub fn rgb_palette() -> Vec<(String, Color)> {
    let mut v = Vec::new();
    let steps = [0u8, 51, 102, 153, 204, 255];
    for &r in &steps {
        for &g in &steps {
            for &b in &steps {
                v.push((format!("R{r} G{g} B{b}"), Color::rgb8(r, g, b)));
            }
        }
    }
    v
}

pub fn gray_palette() -> Vec<(String, Color)> {
    (0..=20)
        .map(|i| {
            let p = i as f32 * 5.0;
            (format!("{p:.0}% Black"), Color::Gray { v: 1.0 - p / 100.0 })
        })
        .collect()
}

pub fn builtin_palettes() -> Vec<Palette> {
    vec![
        Palette {
            name: "Default CMYK".into(),
            key: "palette.default_cmyk",
            colors: default_cmyk(),
            builtin: true,
            path: None,
        },
        Palette {
            name: "Default RGB".into(),
            key: "palette.default_rgb",
            colors: rgb_palette(),
            builtin: true,
            path: None,
        },
        Palette {
            name: "Process".into(),
            key: "palette.process",
            colors: process_palette(),
            builtin: true,
            path: None,
        },
        Palette {
            name: "Grayscale".into(),
            key: "palette.grayscale",
            colors: gray_palette(),
            builtin: true,
            path: None,
        },
    ]
}

// ----- palette files ---------------------------------------------------------

/// Load a palette from .tdpal (JSON), .gpl, .ase or .aco.
pub fn load_palette(path: &std::path::Path) -> Result<Palette, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Palette".into());
    let colors = match ext.as_str() {
        "gpl" => parse_gpl(&std::fs::read_to_string(path).map_err(|e| e.to_string())?),
        "ase" => parse_ase(&std::fs::read(path).map_err(|e| e.to_string())?),
        "aco" => parse_aco(&std::fs::read(path).map_err(|e| e.to_string())?),
        "tdpal" | "json" => {
            parse_tdpal(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?
        }
        other => return Err(format!("unsupported palette format: {other}")),
    };
    Ok(Palette {
        name,
        key: "",
        colors,
        builtin: false,
        path: Some(path.to_path_buf()),
    })
}

pub fn save_palette(p: &Palette, path: &std::path::Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let data = match ext.as_str() {
        "gpl" => {
            let mut s = format!("GIMP Palette\nName: {}\nColumns: 8\n#\n", p.name);
            for (n, c) in &p.colors {
                let [r, g, b] = c.to_rgb8();
                s.push_str(&format!("{r:3} {g:3} {b:3}\t{n}\n"));
            }
            s.into_bytes()
        }
        _ => {
            let entries: Vec<serde_json::Value> = p
                .colors
                .iter()
                .map(|(n, c)| serde_json::json!({"name": n, "color": c}))
                .collect();
            serde_json::to_vec_pretty(&serde_json::json!({"name": p.name, "colors": entries}))
                .map_err(|e| e.to_string())?
        }
    };
    std::fs::write(path, data).map_err(|e| e.to_string())
}

fn parse_tdpal(src: &str) -> Result<Vec<(String, Color)>, String> {
    let v: serde_json::Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("colors").and_then(|c| c.as_array()) {
        for e in arr {
            let name = e
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(c) = e.get("color") {
                if let Ok(c) = serde_json::from_value::<Color>(c.clone()) {
                    out.push((name, c));
                }
            }
        }
    }
    Ok(out)
}

/// GIMP palette: `r g b [name]` per line after the header.
pub fn parse_gpl(src: &str) -> Vec<(String, Color)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.contains(':')
            || line.starts_with("GIMP")
        {
            continue;
        }
        let mut it = line.split_whitespace();
        let (Some(r), Some(g), Some(b)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        let (Ok(r), Ok(g), Ok(b)) = (r.parse::<u8>(), g.parse::<u8>(), b.parse::<u8>()) else {
            continue;
        };
        let name: String = it.collect::<Vec<_>>().join(" ");
        out.push((name, Color::rgb8(r, g, b)));
    }
    out
}

fn be_u16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(i)?, *d.get(i + 1)?]))
}
fn be_u32(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *d.get(i)?,
        *d.get(i + 1)?,
        *d.get(i + 2)?,
        *d.get(i + 3)?,
    ]))
}
fn be_f32(d: &[u8], i: usize) -> Option<f32> {
    Some(f32::from_bits(be_u32(d, i)?))
}

/// Adobe Swatch Exchange: blocks of colour entries with UTF-16 names.
pub fn parse_ase(d: &[u8]) -> Vec<(String, Color)> {
    let mut out = Vec::new();
    if d.len() < 12 || &d[0..4] != b"ASEF" {
        return out;
    }
    let count = be_u32(d, 8).unwrap_or(0) as usize;
    let mut pos = 12;
    for _ in 0..count {
        let Some(kind) = be_u16(d, pos) else { break };
        let Some(len) = be_u32(d, pos + 2) else { break };
        let body = pos + 6;
        if kind == 0x0001 {
            // Colour entry: name length (u16, chars incl. null), UTF-16BE name, model (4), values, type (u16).
            let Some(nlen) = be_u16(d, body) else { break };
            let name_bytes = (nlen as usize) * 2;
            let name_start = body + 2;
            let mut units = Vec::new();
            let mut i = name_start;
            while i + 1 < name_start + name_bytes {
                if let Some(u) = be_u16(d, i) {
                    if u != 0 {
                        units.push(u);
                    }
                }
                i += 2;
            }
            let name = String::from_utf16_lossy(&units);
            let mpos = name_start + name_bytes;
            let model = d.get(mpos..mpos + 4).unwrap_or(b"RGB ");
            let v = |k: usize| be_f32(d, mpos + 4 + 4 * k).unwrap_or(0.0);
            let color = match model {
                b"RGB " => Color::Rgb {
                    r: v(0),
                    g: v(1),
                    b: v(2),
                },
                b"CMYK" => Color::Cmyk {
                    c: v(0),
                    m: v(1),
                    y: v(2),
                    k: v(3),
                },
                b"LAB " => Color::Lab {
                    l: v(0) * 100.0,
                    a: v(1),
                    b: v(2),
                },
                b"Gray" => Color::Gray { v: v(0) },
                _ => Color::BLACK,
            };
            out.push((name, color));
        }
        pos = body + len as usize;
        if pos > d.len() {
            break;
        }
    }
    out
}

/// Adobe Color (.aco) version 1 and 2.
pub fn parse_aco(d: &[u8]) -> Vec<(String, Color)> {
    let mut out = Vec::new();
    let Some(version) = be_u16(d, 0) else {
        return out;
    };
    let count = be_u16(d, 2).unwrap_or(0) as usize;
    let mut pos = 4;
    for i in 0..count {
        let Some(space) = be_u16(d, pos) else { break };
        let w = |k: usize| be_u16(d, pos + 2 + 2 * k).unwrap_or(0) as f32;
        let color = match space {
            0 => Color::Rgb {
                r: w(0) / 65535.0,
                g: w(1) / 65535.0,
                b: w(2) / 65535.0,
            },
            1 => {
                let [r, g, b] = tracedraw_core::color::hsb_to_rgb(
                    w(0) / 65535.0 * 360.0,
                    w(1) / 65535.0,
                    w(2) / 65535.0,
                );
                Color::Rgb { r, g, b }
            }
            2 => Color::Cmyk {
                c: 1.0 - w(0) / 65535.0,
                m: 1.0 - w(1) / 65535.0,
                y: 1.0 - w(2) / 65535.0,
                k: 1.0 - w(3) / 65535.0,
            },
            8 => Color::Gray {
                v: 1.0 - w(0) / 10000.0,
            },
            _ => Color::BLACK,
        };
        pos += 10;
        let mut name = format!("Color {}", i + 1);
        if version == 2 {
            // u16 zero? then u32 length (chars incl. null) and UTF-16BE.
            let _ = be_u16(d, pos);
            let len = be_u32(d, pos + 2).unwrap_or(0) as usize;
            let start = pos + 6;
            let mut units = Vec::new();
            for k in 0..len.saturating_sub(1) {
                if let Some(u) = be_u16(d, start + 2 * k) {
                    units.push(u);
                }
            }
            name = String::from_utf16_lossy(&units);
            pos = start + len * 2;
        }
        out.push((name, color));
    }
    out
}

// ----- harmonies ---------------------------------------------------------------

/// Colour harmony rules (Color Styles docker > Harmony Editor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harmony {
    Analogous,
    Complementary,
    SplitComplementary,
    Triadic,
    Tetradic,
    Monochromatic,
}

impl Harmony {
    pub const ALL: [Harmony; 6] = [
        Harmony::Analogous,
        Harmony::Complementary,
        Harmony::SplitComplementary,
        Harmony::Triadic,
        Harmony::Tetradic,
        Harmony::Monochromatic,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Harmony::Analogous => "harmony.analogous",
            Harmony::Complementary => "harmony.complementary",
            Harmony::SplitComplementary => "harmony.split_complementary",
            Harmony::Triadic => "harmony.triadic",
            Harmony::Tetradic => "harmony.tetradic",
            Harmony::Monochromatic => "harmony.monochromatic",
        }
    }
    /// Colours derived from a base colour by rotating the hue.
    pub fn colors(self, base: Color) -> Vec<Color> {
        let (h, s, b) = base.to_hsb();
        let rot = |d: f64| Color::from_hsb((h + d).rem_euclid(360.0), s, b);
        match self {
            Harmony::Analogous => vec![rot(-30.0), base, rot(30.0)],
            Harmony::Complementary => vec![base, rot(180.0)],
            Harmony::SplitComplementary => vec![base, rot(150.0), rot(210.0)],
            Harmony::Triadic => vec![base, rot(120.0), rot(240.0)],
            Harmony::Tetradic => vec![base, rot(90.0), rot(180.0), rot(270.0)],
            Harmony::Monochromatic => vec![
                Color::from_hsb(h, s, (b * 0.4).max(0.0)),
                Color::from_hsb(h, s, (b * 0.7).max(0.0)),
                base,
                Color::from_hsb(h, (s * 0.6).max(0.0), b),
                Color::from_hsb(h, (s * 0.3).max(0.0), b),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpl_parses_lines() {
        let v = parse_gpl("GIMP Palette\nName: x\n#\n255 0 0\tRed\n0 0 255 Blue\n");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].0, "Red");
        assert_eq!(v[1].1.to_rgb8(), [0, 0, 255]);
    }

    #[test]
    fn ase_round_trip_of_one_rgb_colour() {
        // Hand-built ASE with one RGB entry named "A".
        let mut d = Vec::new();
        d.extend_from_slice(b"ASEF");
        d.extend_from_slice(&1u16.to_be_bytes());
        d.extend_from_slice(&0u16.to_be_bytes());
        d.extend_from_slice(&1u32.to_be_bytes());
        let mut body = Vec::new();
        body.extend_from_slice(&2u16.to_be_bytes()); // name length incl. null
        body.extend_from_slice(&(b'A' as u16).to_be_bytes());
        body.extend_from_slice(&0u16.to_be_bytes());
        body.extend_from_slice(b"RGB ");
        for v in [1.0f32, 0.5, 0.0] {
            body.extend_from_slice(&v.to_be_bytes());
        }
        body.extend_from_slice(&0u16.to_be_bytes());
        d.extend_from_slice(&1u16.to_be_bytes());
        d.extend_from_slice(&(body.len() as u32).to_be_bytes());
        d.extend_from_slice(&body);
        let v = parse_ase(&d);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].0, "A");
        assert_eq!(v[0].1.to_rgb8(), [255, 128, 0]);
    }

    #[test]
    fn harmonies_have_expected_counts() {
        let base = Color::rgb8(200, 30, 30);
        assert_eq!(Harmony::Triadic.colors(base).len(), 3);
        assert_eq!(Harmony::Tetradic.colors(base).len(), 4);
        let comp = Harmony::Complementary.colors(base)[1];
        let (h, _, _) = comp.to_hsb();
        assert!((h - 180.0).abs() < 2.0, "{h}");
    }
}
