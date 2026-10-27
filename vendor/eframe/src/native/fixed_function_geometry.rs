// TraceDraw: the arithmetic of the fixed-function painter
// (native/fixed_function_painter.rs, see vendor/README.md), kept apart
// from the OpenGL calls so that TraceDraw's own tests compile and run it
// (`gpu::eframe_fixed_function_geometry::tests`).

/// The scissor box in pixels for `clip_rect` (`[min_x, min_y, max_x,
/// max_y]` in points, y down) on a window of `screen_size_px`, as
/// `[x, y, width, height]` with y up from the bottom, the way OpenGL
/// counts: the same arithmetic as `egui_glow`'s painter.
pub(crate) fn scissor_box(
    [width_px, height_px]: [u32; 2],
    pixels_per_point: f32,
    [min_x, min_y, max_x, max_y]: [f32; 4],
) -> [i32; 4] {
    let width_px = i32::try_from(width_px).unwrap_or(i32::MAX);
    let height_px = i32::try_from(height_px).unwrap_or(i32::MAX);
    let px = |points: f32| (pixels_per_point * points).round() as i32;
    let clip_min_x = px(min_x).clamp(0, width_px);
    let clip_min_y = px(min_y).clamp(0, height_px);
    let clip_max_x = px(max_x).clamp(clip_min_x, width_px);
    let clip_max_y = px(max_y).clamp(clip_min_y, height_px);
    [
        clip_min_x,
        height_px - clip_max_y,
        clip_max_x - clip_min_x,
        clip_max_y - clip_min_y,
    ]
}

/// Whether a mesh of `vertices` vertices is drawn with 16-bit indices,
/// which every driver handles, instead of 32-bit ones.
pub(crate) fn fits_u16(vertices: usize) -> bool {
    vertices <= usize::from(u16::MAX) + 1
}

/// The largest texture side the painter lets egui use: what the driver
/// reports, never more than 8192 (what wgpu's default limits allow, so the
/// two renderers hold the same pictures) and at least 1024 when the
/// driver answers nothing sensible.
pub(crate) fn texture_side_limit(reported: i32) -> usize {
    match usize::try_from(reported) {
        Ok(side) if side >= 1024 => side.min(8192),
        _ => 1024,
    }
}

/// Whether a fixed-function painter is wanted for a context of this
/// OpenGL version: `forced` is what the environment asks for ("fixed" or
/// "shaders"), else any desktop OpenGL below 3.0, whose drivers were made
/// for that pipeline. OpenGL ES has no fixed-function pipeline.
pub(crate) fn fixed_function_wanted(forced: Option<&str>, major: u32, is_embedded: bool) -> bool {
    match forced {
        Some("fixed") => true,
        Some("shaders") => false,
        _ => !is_embedded && major < 3,
    }
}

#[cfg(test)]
mod tests {
    use super::{fits_u16, fixed_function_wanted, scissor_box, texture_side_limit};

    #[test]
    fn the_scissor_box_counts_from_the_bottom_and_stays_on_screen() {
        // The whole window.
        assert_eq!(
            scissor_box([640, 380], 1.0, [0.0, 0.0, 640.0, 380.0]),
            [0, 0, 640, 380]
        );
        // A rect near the top: its y is measured from the bottom.
        assert_eq!(
            scissor_box([640, 380], 1.0, [10.0, 20.0, 110.0, 70.0]),
            [10, 310, 100, 50]
        );
        // Points scale to pixels and round.
        assert_eq!(
            scissor_box([1280, 760], 2.0, [10.2, 20.0, 110.0, 70.4]),
            [20, 619, 200, 101]
        );
        // Off-screen parts are cut; an empty or inverted rect is empty.
        assert_eq!(
            scissor_box([640, 380], 1.0, [-50.0, -50.0, 1000.0, 1000.0]),
            [0, 0, 640, 380]
        );
        assert_eq!(
            scissor_box([640, 380], 1.0, [100.0, 100.0, 50.0, 50.0]),
            [100, 280, 0, 0]
        );
    }

    #[test]
    fn small_meshes_take_16_bit_indices() {
        assert!(fits_u16(0));
        assert!(fits_u16(65_536));
        assert!(!fits_u16(65_537));
    }

    #[test]
    fn the_texture_limit_is_what_the_driver_says_within_reason() {
        assert_eq!(texture_side_limit(8192), 8192);
        assert_eq!(texture_side_limit(2048), 2048);
        assert_eq!(texture_side_limit(16_384), 8192);
        assert_eq!(texture_side_limit(0), 1024);
        assert_eq!(texture_side_limit(-1), 1024);
        assert_eq!(texture_side_limit(512), 1024);
    }

    #[test]
    fn the_fixed_function_painter_is_for_desktop_opengl_below_3() {
        assert!(fixed_function_wanted(None, 2, false));
        assert!(fixed_function_wanted(None, 1, false));
        assert!(!fixed_function_wanted(None, 3, false));
        assert!(!fixed_function_wanted(None, 4, false));
        // OpenGL ES has no fixed-function pipeline.
        assert!(!fixed_function_wanted(None, 2, true));
        // The environment decides when it says so.
        assert!(fixed_function_wanted(Some("fixed"), 4, false));
        assert!(!fixed_function_wanted(Some("shaders"), 2, false));
        assert!(fixed_function_wanted(Some("nonsense"), 2, false));
    }
}
