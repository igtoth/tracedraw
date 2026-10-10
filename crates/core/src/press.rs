//! The built-in CMYK press model, used to show and convert CMYK colours
//! when no CMYK profile is loaded.
//!
//! A Yule-Nielsen modified Neugebauer model of process inks on coated
//! stock: each ink's nominal coverage goes through a dot gain curve, the
//! sixteen overprints of solid inks (the Neugebauer primaries) are mixed
//! with the Demichel weights of those coverages in a power-law space (one
//! exponent per screen channel), and the result is linear sRGB. A few
//! primaries lie slightly outside sRGB (solid cyan has no red left at all);
//! mixtures beyond the screen gamut are clipped.
//!
//! The constants were fitted to the colours the target design shows
//! with its default colour settings (a coated web offset CMYK space,
//! relative colorimetric intent with black point compensation): its
//! default palette comes out within about 2.5 Delta E (CIE76), and ink
//! combinations up to 320% within about 1 on average, 2.2 for 99% of them.
//!
//! The way back ([`srgb_to_cmyk`]) follows the same default settings:
//! pure black becomes black ink only, neutral greys use black ink only,
//! and other colours get a medium grey component replacement (black ink
//! grows with the darkness of the colour) with C, M and Y solved so the
//! colour shows as asked, or as close as the press gamut allows, within a
//! total ink limit of 300%.

/// Effective dot area of each ink (C, M, Y, K) at 0%, 10%, ... 100%.
const GAIN: [[f64; 11]; 4] = [
    [
        0.0, 0.1511, 0.2930, 0.4164, 0.5249, 0.6218, 0.7099, 0.7919, 0.8673, 0.9354, 1.0,
    ],
    [
        0.0, 0.1444, 0.2831, 0.4104, 0.5245, 0.6237, 0.7120, 0.7933, 0.8678, 0.9356, 1.0,
    ],
    [
        0.0, 0.1542, 0.2996, 0.4276, 0.5406, 0.6413, 0.7326, 0.8177, 0.8918, 0.9499, 1.0,
    ],
    [
        0.0, 0.1946, 0.3294, 0.4700, 0.5752, 0.6842, 0.7574, 0.8359, 0.8962, 0.9543, 1.0,
    ],
];

/// Yule-Nielsen exponent of each screen channel (R, G, B).
const EXPONENT: [f64; 3] = [1.1273, 1.0872, 1.0515];

/// The Neugebauer primaries: linear sRGB of each overprint of solid inks,
/// raised to `1 / EXPONENT` (sign kept). Index bits: C 8, M 4, Y 2, K 1.
const PRIMARIES: [[f64; 3]; 16] = [
    [1.0, 1.0, 1.0],                // paper
    [0.02486, 0.01844, 0.01674],    // K
    [1.05995, 0.88774, -0.00023],   // Y
    [0.01715, 0.01223, -0.00771],   // Y K
    [0.85894, -0.00188, 0.27920],   // M
    [0.02625, -0.01533, -0.00366],  // M K
    [0.86473, 0.01582, 0.02105],    // M Y
    [0.02508, -0.00754, -0.00757],  // M Y K
    [-0.27971, 0.45208, 0.87214],   // C
    [-0.02011, 0.00683, 0.01807],   // C K
    [-0.14479, 0.41366, 0.09180],   // C Y
    [-0.02207, 0.01015, -0.00351],  // C Y K
    [0.04143, 0.03941, 0.30907],    // C M
    [-0.00491, -0.01189, 0.00175],  // C M K
    [0.06150, 0.05224, 0.05554],    // C M Y
    [-0.00981, -0.00943, -0.00942], // C M Y K
];

/// Total ink limit of the way back (300%).
const INK_LIMIT: f64 = 3.0;
/// How strongly the way back keeps black ink at its replacement level.
const BLACK_WEIGHT: f64 = 0.003;

/// Screen sRGB (gamma encoded, 0..1) of a CMYK colour (each 0..1).
pub fn cmyk_to_srgb(cmyk: [f32; 4]) -> [f32; 3] {
    forward(cmyk.map(|v| clean(v as f64))).map(|v| v as f32)
}

/// CMYK (each 0..1) that shows as the given sRGB colour (gamma encoded,
/// 0..1), or the nearest colour the press can print.
pub fn srgb_to_cmyk(rgb: [f32; 3]) -> [f32; 4] {
    let t = rgb.map(|v| clean(v as f64));
    let hi = t[0].max(t[1]).max(t[2]);
    let lo = t[0].min(t[1]).min(t[2]);
    if hi - lo < 0.5 / 255.0 {
        let k = black_for((t[0] + t[1] + t[2]) / 3.0);
        return [0.0, 0.0, 0.0, k as f32];
    }
    solve(t).map(|v| v as f32)
}

fn clean(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

fn dot_gain(curve: &[f64; 11], v: f64) -> f64 {
    let x = v * 10.0;
    let i = (x.floor().max(0.0) as usize).min(9);
    let t = x - i as f64;
    let a = curve.get(i).copied().unwrap_or(0.0);
    let b = curve.get(i + 1).copied().unwrap_or(1.0);
    a + (b - a) * t
}

fn encode(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// The model before the screen clips it: values may leave 0..1 where the
/// mixture lies outside sRGB.
fn mix(x: [f64; 4]) -> [f64; 3] {
    let mut area = [0.0f64; 4];
    for ((a, v), curve) in area.iter_mut().zip(x).zip(&GAIN) {
        *a = dot_gain(curve, v);
    }
    let mut sum = [0.0f64; 3];
    for (i, primary) in PRIMARIES.iter().enumerate() {
        let mut w = 1.0;
        for (bit, a) in area.iter().enumerate() {
            w *= if (i >> (3 - bit)) & 1 == 1 {
                *a
            } else {
                1.0 - a
            };
        }
        for (s, p) in sum.iter_mut().zip(primary) {
            *s += w * p;
        }
    }
    let mut out = [0.0f64; 3];
    for ((o, s), e) in out.iter_mut().zip(sum).zip(EXPONENT) {
        *o = encode(s.signum() * s.abs().powf(e));
    }
    out
}

fn forward(x: [f64; 4]) -> [f64; 3] {
    mix(x).map(|v| v.clamp(0.0, 1.0))
}

/// Black ink alone that matches a grey level (0 black .. 1 white).
fn black_for(grey: f64) -> f64 {
    if grey <= 0.5 / 255.0 {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        let [r, g, b] = forward([0.0, 0.0, 0.0, mid]);
        if (r + g + b) / 3.0 > grey {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let k = (lo + hi) / 2.0;
    if k < 1e-6 {
        0.0
    } else {
        k
    }
}

/// Levenberg-Marquardt on C, M, Y and K: match the colour and keep K near
/// the grey component replacement level, within the ink limit. Channels
/// asked at 0 or 1 only need the model to reach that far (the screen clips
/// the rest), and inks held at 0% or 100% stay there while the step pushes
/// them out.
fn solve(t: [f64; 3]) -> [f64; 4] {
    const EDGE: f64 = 0.5 / 255.0;
    let dark = 1.0 - t[0].max(t[1]).max(t[2]);
    let k_target = 0.95 * ((dark - 0.25) / 0.75).max(0.0).powf(1.4);
    let mut x = [
        (1.0 - t[0] - k_target).clamp(0.0, 1.0),
        (1.0 - t[1] - k_target).clamp(0.0, 1.0),
        (1.0 - t[2] - k_target).clamp(0.0, 1.0),
        k_target,
    ];
    limit_ink(&mut x);
    let residual = |x: &[f64; 4]| -> [f64; 4] {
        let u = mix(*x);
        let mut r = [0.0f64; 4];
        for ((r, u), t) in r.iter_mut().zip(u).zip(t) {
            let d = u - t;
            *r = if t <= EDGE {
                d.max(0.0)
            } else if t >= 1.0 - EDGE {
                d.min(0.0)
            } else {
                d
            };
        }
        r[3] = BLACK_WEIGHT * (x[3] - k_target);
        r
    };
    let cost = |r: &[f64; 4]| r.iter().map(|v| v * v).sum::<f64>();
    let mut r = residual(&x);
    let mut c = cost(&r);
    let mut mu = 1e-3;
    for _ in 0..100 {
        if r.iter().take(3).all(|v| v.abs() < 2e-5) {
            break;
        }
        // Jacobian by finite differences, one column per ink.
        let mut jac = [[0.0f64; 4]; 4];
        for ink in 0..4 {
            let mut xp = x;
            let h = if xp[ink] > 1.0 - 1e-4 { -1e-4 } else { 1e-4 };
            xp[ink] += h;
            let rp = residual(&xp);
            for (row, (a, b)) in jac.iter_mut().zip(rp.iter().zip(&r)) {
                row[ink] = (a - b) / h;
            }
        }
        let mut a = [[0.0f64; 4]; 4];
        let mut g = [0.0f64; 4];
        for i in 0..4 {
            for j in 0..4 {
                a[i][j] = jac.iter().map(|row| row[i] * row[j]).sum();
            }
            a[i][i] += mu;
            g[i] = -jac.iter().zip(&r).map(|(row, v)| row[i] * v).sum::<f64>();
        }
        // Inks at a limit that the step would push further out stay put.
        for i in 0..4 {
            if (x[i] >= 1.0 && g[i] > 0.0) || (x[i] <= 0.0 && g[i] < 0.0) {
                for row in a.iter_mut() {
                    row[i] = 0.0;
                }
                a[i] = [0.0; 4];
                a[i][i] = 1.0;
                g[i] = 0.0;
            }
        }
        let Some(d) = solve4(a, g) else {
            break;
        };
        let mut xn = x;
        for (v, dv) in xn.iter_mut().zip(d) {
            *v = (*v + dv).clamp(0.0, 1.0);
        }
        limit_ink(&mut xn);
        let rn = residual(&xn);
        let cn = cost(&rn);
        if cn < c {
            let step = d.iter().fold(0.0f64, |m, v| m.max(v.abs()));
            x = xn;
            r = rn;
            c = cn;
            mu = (mu / 3.0).max(1e-9);
            if step < 1e-8 {
                break;
            }
        } else {
            mu *= 4.0;
            if mu > 1e4 {
                break;
            }
        }
    }
    x
}

/// Scale C, M and Y down so the total stays within the ink limit.
fn limit_ink(x: &mut [f64; 4]) {
    let cmy = x[0] + x[1] + x[2];
    let room = (INK_LIMIT - x[3]).max(0.0);
    if cmy > room && cmy > 0.0 {
        let f = room / cmy;
        for v in x.iter_mut().take(3) {
            *v *= f;
        }
    }
}

/// Solve a 4 x 4 linear system by Gaussian elimination with partial
/// pivoting; `None` when it is singular.
fn solve4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> Option<[f64; 4]> {
    for col in 0..4 {
        let pivot = (col..4).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-14 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let pivot_row = a[col];
        for row in col + 1..4 {
            let f = a[row][col] / pivot_row[col];
            for (v, p) in a[row].iter_mut().zip(pivot_row).skip(col) {
                *v -= f * p;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0f64; 4];
    for row in (0..4).rev() {
        let s: f64 = (row + 1..4).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb8(cmyk_pct: [f32; 4]) -> [i32; 3] {
        cmyk_to_srgb(cmyk_pct.map(|v| v / 100.0)).map(|v| (v * 255.0).round() as i32)
    }

    fn close(a: [i32; 3], b: [i32; 3], tol: i32) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
    }

    #[test]
    fn process_inks_show_as_in_the_reference_editor() {
        // Colours of the target design's default palette on screen.
        let cases = [
            ([0.0, 0.0, 0.0, 0.0], [255, 255, 255]),
            ([0.0, 0.0, 0.0, 100.0], [34, 31, 32]),
            ([0.0, 0.0, 0.0, 50.0], [146, 148, 151]),
            ([0.0, 0.0, 0.0, 10.0], [229, 230, 231]),
            ([100.0, 0.0, 0.0, 0.0], [0, 173, 239]),
            ([0.0, 100.0, 0.0, 0.0], [236, 0, 139]),
            ([0.0, 0.0, 100.0, 0.0], [255, 241, 0]),
            ([100.0, 100.0, 0.0, 0.0], [46, 48, 146]),
            ([100.0, 0.0, 100.0, 0.0], [0, 165, 80]),
            ([0.0, 100.0, 100.0, 0.0], [237, 27, 35]),
            ([20.0, 80.0, 0.0, 20.0], [165, 70, 134]),
            ([0.0, 60.0, 100.0, 0.0], [245, 129, 30]),
            ([60.0, 0.0, 40.0, 20.0], [77, 163, 145]),
            ([40.0, 0.0, 40.0, 0.0], [154, 210, 173]),
        ];
        for (cmyk, want) in cases {
            let got = rgb8(cmyk);
            assert!(close(got, want, 5), "{cmyk:?}: {got:?} vs {want:?}");
        }
    }

    #[test]
    fn more_ink_never_shows_lighter() {
        let steps = [0.0f32, 0.25, 0.5, 0.75, 1.0];
        for &c in &steps {
            for &m in &steps {
                for &y in &steps {
                    for &k in &steps {
                        let base = cmyk_to_srgb([c, m, y, k]);
                        // Relative luminance: a channel may brighten where
                        // the screen gamut clips (cyan over a little
                        // magenta gains some red) while the whole darkens.
                        let lum = |v: [f32; 3]| {
                            let l = |c: f32| {
                                if c <= 0.04045 {
                                    c / 12.92
                                } else {
                                    ((c + 0.055) / 1.055).powf(2.4)
                                }
                            };
                            0.2126 * l(v[0]) + 0.7152 * l(v[1]) + 0.0722 * l(v[2])
                        };
                        for ink in 0..4 {
                            let mut more = [c, m, y, k];
                            more[ink] = (more[ink] + 0.1).min(1.0);
                            // Under solid black the screen shows next to
                            // nothing either way: a level of slack.
                            assert!(
                                lum(cmyk_to_srgb(more)) <= lum(base) + 1e-3,
                                "{more:?} lighter than {:?}",
                                [c, m, y, k]
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn printable_colours_round_trip() {
        let steps = [0.0f32, 0.2, 0.45, 0.7, 0.9];
        for &c in &steps {
            for &m in &steps {
                for &y in &steps {
                    for &k in &[0.0f32, 0.3, 0.6] {
                        if c + m + y + k > 2.6 {
                            continue;
                        }
                        let shown = cmyk_to_srgb([c, m, y, k]);
                        let back = cmyk_to_srgb(srgb_to_cmyk(shown));
                        for (a, b) in shown.iter().zip(back) {
                            assert!(
                                (a - b).abs() * 255.0 <= 1.0,
                                "{:?}: {shown:?} came back as {back:?}",
                                [c, m, y, k]
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn black_and_greys_use_black_ink_only() {
        assert_eq!(srgb_to_cmyk([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(srgb_to_cmyk([1.0, 1.0, 1.0]), [0.0, 0.0, 0.0, 0.0]);
        let grey = srgb_to_cmyk([0.5, 0.5, 0.5]);
        assert_eq!(&grey[..3], &[0.0, 0.0, 0.0]);
        assert!(grey[3] > 0.5 && grey[3] < 0.7, "{grey:?}");
    }

    #[test]
    fn screen_colours_outside_the_press_gamut_come_out_close() {
        // Pure screen red has no exact print: the nearest is solid M and Y.
        let red = srgb_to_cmyk([1.0, 0.0, 0.0]);
        assert!(red[0] < 0.05 && red[1] > 0.95 && red[2] > 0.95, "{red:?}");
        let ink: f32 = srgb_to_cmyk([0.02, 0.0, 0.05]).iter().sum();
        assert!(ink <= 3.0 + 1e-3, "{ink}");
    }

    #[test]
    fn bad_numbers_do_not_break_it() {
        let v = cmyk_to_srgb([f32::NAN, 2.0, -1.0, f32::INFINITY]);
        assert!(v.iter().all(|c| c.is_finite()));
        let v = srgb_to_cmyk([f32::NAN, 2.0, -1.0]);
        assert!(v.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)));
    }
}
