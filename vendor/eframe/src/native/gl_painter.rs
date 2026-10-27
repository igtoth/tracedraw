// TraceDraw: the painter of the OpenGL renderer is one of two (see
// vendor/README.md): egui_glow's, with shaders and buffer objects, or the
// fixed-function one on desktop OpenGL below 3.0, whose drivers draw the
// first wrongly (#616).

use core::ffi::{CStr, c_void};
use std::sync::Arc;

use glow::HasContext as _;

use super::fixed_function_geometry::fixed_function_wanted;
use super::fixed_function_painter::FixedFunctionPainter;

/// The environment variable that picks the painter by hand: `fixed` or
/// `shaders`.
pub const PAINTER_ENV: &str = "TRACEDRAW_GL_PAINTER";

/// What draws egui on the OpenGL context.
pub enum GlPainter {
    /// `egui_glow`: GLSL shaders, generic vertex attributes, buffer objects.
    Shaders(egui_glow::Painter),
    /// The fixed-function pipeline of OpenGL 1.x.
    FixedFunction(FixedFunctionPainter),
}

impl GlPainter {
    /// The painter for the context `gl`: the fixed-function one when the
    /// context is desktop OpenGL below 3.0 (or [`PAINTER_ENV`] asks for
    /// it), else `egui_glow`'s.
    ///
    /// # Errors
    /// When the painter cannot be set up on this context.
    pub fn new(
        gl: Arc<glow::Context>,
        get_proc_address: &mut dyn FnMut(&CStr) -> *const c_void,
        shader_version: Option<egui_glow::ShaderVersion>,
        dithering: bool,
    ) -> Result<Self, egui_glow::PainterError> {
        let forced = std::env::var(PAINTER_ENV).ok();
        let version = gl.version();
        if fixed_function_wanted(forced.as_deref(), version.major, version.is_embedded) {
            let painter = FixedFunctionPainter::new(gl, get_proc_address)
                .map_err(egui_glow::PainterError::from)?;
            Ok(Self::FixedFunction(painter))
        } else {
            Ok(Self::Shaders(egui_glow::Painter::new(
                gl,
                "",
                shader_version,
                dithering,
            )?))
        }
    }

    pub fn gl(&self) -> &Arc<glow::Context> {
        match self {
            Self::Shaders(p) => p.gl(),
            Self::FixedFunction(p) => p.gl(),
        }
    }

    pub fn max_texture_side(&self) -> usize {
        match self {
            Self::Shaders(p) => p.max_texture_side(),
            Self::FixedFunction(p) => p.max_texture_side(),
        }
    }

    pub fn clear(&self, screen_size_in_pixels: [u32; 2], clear_color: [f32; 4]) {
        match self {
            Self::Shaders(p) => p.clear(screen_size_in_pixels, clear_color),
            Self::FixedFunction(p) => p.clear(screen_size_in_pixels, clear_color),
        }
    }

    pub fn paint_and_update_textures(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &mut egui::TexturesDelta,
    ) {
        match self {
            Self::Shaders(p) => p.paint_and_update_textures(
                screen_size_px,
                pixels_per_point,
                clipped_primitives,
                textures_delta,
            ),
            Self::FixedFunction(p) => p.paint_and_update_textures(
                screen_size_px,
                pixels_per_point,
                clipped_primitives,
                textures_delta,
            ),
        }
    }

    pub fn read_screen_rgba(&self, screen_size_in_pixels: [u32; 2]) -> egui::ColorImage {
        match self {
            Self::Shaders(p) => p.read_screen_rgba(screen_size_in_pixels),
            Self::FixedFunction(p) => p.read_screen_rgba(screen_size_in_pixels),
        }
    }

    pub fn register_native_texture(&mut self, native: glow::Texture) -> egui::TextureId {
        match self {
            Self::Shaders(p) => p.register_native_texture(native),
            Self::FixedFunction(p) => p.register_native_texture(native),
        }
    }

    pub fn destroy(&mut self) {
        match self {
            Self::Shaders(p) => p.destroy(),
            Self::FixedFunction(p) => p.destroy(),
        }
    }
}
