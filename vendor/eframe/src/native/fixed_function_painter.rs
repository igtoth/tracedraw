// TraceDraw: a painter for egui on the fixed-function OpenGL pipeline,
// used when the driver offers desktop OpenGL below 3.0 (see
// vendor/README.md and TraceDraw's docs/behavior/about-and-startup.md).

//! A painter for egui on the fixed-function OpenGL pipeline: OpenGL 2.0 or
//! later driven the OpenGL 1.x way, with client-side vertex arrays
//! (`glVertexPointer`, `glColorPointer`, `glTexCoordPointer`,
//! `glDrawElements` on the mesh's own memory), no shaders, no buffer
//! objects and no vertex array objects.
//!
//! `egui_glow` draws with GLSL shaders, generic vertex attributes and two
//! buffer objects re-specified at every draw call. The OpenGL 2.1 drivers
//! of older cards (the Intel HD Graphics of 2010 PCs under Windows, driver
//! 8.15.10.2900, is the known case) accept all of that and then draw
//! everything after the first mesh of a frame as black boxes and stray
//! triangles. The fixed-function pipeline is what those drivers were made
//! and tested for, and it computes exactly what egui's shader does: the
//! vertex colour times the texel (`GL_MODULATE`), blended with
//! premultiplied alpha.

#![expect(unsafe_code)]

use core::ffi::{CStr, c_void};
use core::mem::offset_of;
use std::collections::HashMap;
use std::sync::Arc;

use egui::epaint::{Primitive, Vertex};
use glow::HasContext as _;

use super::fixed_function_geometry::{fits_u16, scissor_box, texture_side_limit};

// OpenGL 1.x names that `glow` leaves out.
const VERTEX_ARRAY: u32 = 0x8074;
const COLOR_ARRAY: u32 = 0x8076;
const TEXTURE_COORD_ARRAY: u32 = 0x8078;
const MODELVIEW: u32 = 0x1700;
const PROJECTION: u32 = 0x1701;
const TEXTURE_ENV: u32 = 0x2300;
const TEXTURE_ENV_MODE: u32 = 0x2200;
const MODULATE: u32 = 0x2100;
const LIGHTING: u32 = 0x0B50;
const ALPHA_TEST: u32 = 0x0BC0;
const FOG: u32 = 0x0B60;
const COLOR_MATERIAL: u32 = 0x0B57;

type EnableClientState = unsafe extern "system" fn(u32);
type Pointer = unsafe extern "system" fn(i32, u32, i32, *const c_void);
type DrawElements = unsafe extern "system" fn(u32, i32, u32, *const c_void);
type MatrixMode = unsafe extern "system" fn(u32);
type LoadIdentity = unsafe extern "system" fn();
type Ortho = unsafe extern "system" fn(f64, f64, f64, f64, f64, f64);
type TexEnvI = unsafe extern "system" fn(u32, u32, i32);

/// The OpenGL 1.x entry points the painter needs beyond `glow`.
struct Functions {
    enable_client_state: EnableClientState,
    disable_client_state: EnableClientState,
    vertex_pointer: Pointer,
    tex_coord_pointer: Pointer,
    color_pointer: Pointer,
    draw_elements: DrawElements,
    matrix_mode: MatrixMode,
    load_identity: LoadIdentity,
    ortho: Ortho,
    tex_env_i: TexEnvI,
}

/// The entry point `name` from the driver as a function of type `$ty`.
macro_rules! entry_point {
    ($get:expr, $name:literal, $ty:ty) => {{
        let address = $get($name);
        if address.is_null() {
            return Err(format!(
                "the driver has no {}",
                $name.to_str().unwrap_or("OpenGL function")
            ));
        }
        // SAFETY: the driver gave this address for the function of that
        // name, whose signature is the one of the OpenGL specification.
        unsafe { core::mem::transmute::<*const c_void, $ty>(address) }
    }};
}

impl Functions {
    fn load(get: &mut dyn FnMut(&CStr) -> *const c_void) -> Result<Self, String> {
        Ok(Self {
            enable_client_state: entry_point!(get, c"glEnableClientState", EnableClientState),
            disable_client_state: entry_point!(get, c"glDisableClientState", EnableClientState),
            vertex_pointer: entry_point!(get, c"glVertexPointer", Pointer),
            tex_coord_pointer: entry_point!(get, c"glTexCoordPointer", Pointer),
            color_pointer: entry_point!(get, c"glColorPointer", Pointer),
            draw_elements: entry_point!(get, c"glDrawElements", DrawElements),
            matrix_mode: entry_point!(get, c"glMatrixMode", MatrixMode),
            load_identity: entry_point!(get, c"glLoadIdentity", LoadIdentity),
            ortho: entry_point!(get, c"glOrtho", Ortho),
            tex_env_i: entry_point!(get, c"glTexEnvi", TexEnvI),
        })
    }
}

/// How many OpenGL errors are logged before the log goes quiet.
const ERRORS_LOGGED: u32 = 12;

/// An egui painter on the fixed-function pipeline.
pub struct FixedFunctionPainter {
    gl: Arc<glow::Context>,
    f: Functions,
    max_texture_side: usize,
    textures: HashMap<egui::TextureId, glow::Texture>,
    next_native_tex_id: u64,
    /// The indices of the mesh being drawn as 16-bit values.
    indices16: Vec<u16>,
    frames: u64,
    errors_logged: u32,
    callback_warned: bool,
    destroyed: bool,
}

impl FixedFunctionPainter {
    /// The painter for the current context, whose functions `get` finds.
    ///
    /// # Errors
    /// When the context is below OpenGL 2.0 or lacks an entry point.
    pub fn new(
        gl: Arc<glow::Context>,
        get: &mut dyn FnMut(&CStr) -> *const c_void,
    ) -> Result<Self, String> {
        let version = gl.version();
        if version.is_embedded || version.major < 2 {
            return Err(format!(
                "the fixed-function painter needs desktop OpenGL 2.0 or later, not {}.{}",
                version.major, version.minor
            ));
        }
        let f = Functions::load(get)?;
        // SAFETY: plain queries on the current context.
        let (reported, vendor, renderer) = unsafe {
            (
                gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE),
                gl.get_parameter_string(glow::VENDOR),
                gl.get_parameter_string(glow::RENDERER),
            )
        };
        let max_texture_side = texture_side_limit(reported);
        let extensions = gl.supported_extensions().len();
        log::info!(
            "OpenGL {}.{} ({vendor}, {renderer}): drawing with the fixed-function pipeline, textures up to {max_texture_side} px (the driver says {reported}), {extensions} extensions",
            version.major,
            version.minor
        );
        let mut painter = Self {
            gl,
            f,
            max_texture_side,
            textures: HashMap::default(),
            next_native_tex_id: 1 << 32,
            indices16: Vec::new(),
            frames: 0,
            errors_logged: 0,
            callback_warned: false,
            destroyed: false,
        };
        painter.report_errors("the painter's start");
        Ok(painter)
    }

    /// The shared glow context.
    pub fn gl(&self) -> &Arc<glow::Context> {
        &self.gl
    }

    pub fn max_texture_side(&self) -> usize {
        self.max_texture_side
    }

    /// Log the errors the driver has noted since the last call, after
    /// `what`, up to [`ERRORS_LOGGED`] in all.
    fn report_errors(&mut self, what: &str) {
        loop {
            // SAFETY: a plain query on the current context.
            let code = unsafe { self.gl.get_error() };
            if code == glow::NO_ERROR {
                return;
            }
            if self.errors_logged < ERRORS_LOGGED {
                self.errors_logged += 1;
                log::warn!("OpenGL error {code:#06x} after {what}");
                if self.errors_logged == ERRORS_LOGGED {
                    log::warn!("further OpenGL errors are not logged");
                }
            }
        }
    }

    pub fn clear(&self, screen_size_in_pixels: [u32; 2], clear_color: [f32; 4]) {
        egui_glow::painter::clear(&self.gl, screen_size_in_pixels, clear_color);
    }

    /// Set the state every mesh is drawn with: the fixed-function pipeline
    /// with texturing that modulates the vertex colour, premultiplied
    /// alpha blending, scissoring, and points mapped to the window.
    fn prepare_painting(&self, [width_px, height_px]: [u32; 2], pixels_per_point: f32) {
        let gl = &self.gl;
        let width_points = f64::from(width_px as f32 / pixels_per_point);
        let height_points = f64::from(height_px as f32 / pixels_per_point);
        // SAFETY: state changes on the current context, all OpenGL 2.0.
        unsafe {
            gl.enable(glow::SCISSOR_TEST);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.disable(LIGHTING);
            gl.disable(COLOR_MATERIAL);
            gl.disable(ALPHA_TEST);
            gl.disable(FOG);
            gl.color_mask(true, true, true, true);
            gl.enable(glow::BLEND);
            gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
            // egui outputs colours with premultiplied alpha.
            gl.blend_func_separate(
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE_MINUS_DST_ALPHA,
                glow::ONE,
            );
            gl.active_texture(glow::TEXTURE0);
            gl.enable(glow::TEXTURE_2D);
            (self.f.tex_env_i)(TEXTURE_ENV, TEXTURE_ENV_MODE, MODULATE as i32);
            gl.viewport(0, 0, width_px as i32, height_px as i32);
            (self.f.matrix_mode)(PROJECTION);
            (self.f.load_identity)();
            (self.f.ortho)(0.0, width_points, height_points, 0.0, -1.0, 1.0);
            (self.f.matrix_mode)(MODELVIEW);
            (self.f.load_identity)();
            (self.f.enable_client_state)(VERTEX_ARRAY);
            (self.f.enable_client_state)(COLOR_ARRAY);
            (self.f.enable_client_state)(TEXTURE_COORD_ARRAY);
        }
    }

    /// Upload the textures that changed, paint, free the textures that
    /// are done with.
    pub fn paint_and_update_textures(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &mut egui::TexturesDelta,
    ) {
        #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
        for (id, image_deltas) in textures_delta.set.drain() {
            for image_delta in image_deltas {
                self.set_texture(id, &image_delta);
            }
        }
        self.paint_primitives(screen_size_px, pixels_per_point, clipped_primitives);
        #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
        for id in textures_delta.free.drain() {
            self.free_texture(id);
        }
    }

    fn paint_primitives(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
    ) {
        self.assert_not_destroyed();
        self.prepare_painting(screen_size_px, pixels_per_point);
        let mut meshes = 0_usize;
        let mut vertices = 0_usize;
        for egui::ClippedPrimitive {
            clip_rect,
            primitive,
        } in clipped_primitives
        {
            let [x, y, width, height] = scissor_box(
                screen_size_px,
                pixels_per_point,
                [
                    clip_rect.min.x,
                    clip_rect.min.y,
                    clip_rect.max.x,
                    clip_rect.max.y,
                ],
            );
            // SAFETY: a state change on the current context.
            unsafe { self.gl.scissor(x, y, width, height) };
            match primitive {
                Primitive::Mesh(mesh) => {
                    meshes += 1;
                    vertices += mesh.vertices.len();
                    self.paint_mesh(mesh);
                }
                Primitive::Callback(_) => {
                    if !self.callback_warned {
                        self.callback_warned = true;
                        log::warn!(
                            "the fixed-function painter cannot run paint callbacks; one was skipped"
                        );
                    }
                }
            }
        }
        // SAFETY: state changes on the current context.
        unsafe {
            (self.f.disable_client_state)(VERTEX_ARRAY);
            (self.f.disable_client_state)(COLOR_ARRAY);
            (self.f.disable_client_state)(TEXTURE_COORD_ARRAY);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.disable(glow::SCISSOR_TEST);
        }
        self.frames += 1;
        if self.frames == 1 {
            log::info!(
                "the first frame drawn with the fixed-function pipeline: {meshes} meshes, {vertices} vertices"
            );
        }
        if self.frames <= 2 || self.frames.is_multiple_of(600) {
            self.report_errors("painting");
        }
    }

    /// Draw one mesh straight from its memory.
    fn paint_mesh(&mut self, mesh: &egui::Mesh) {
        debug_assert!(mesh.is_valid(), "Mesh is not valid");
        let Some(texture) = self.textures.get(&mesh.texture_id).copied() else {
            log::warn!("Failed to find texture {:?}", mesh.texture_id);
            return;
        };
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return;
        }
        // The driver reads the vertices from the mesh's own memory, so an
        // index past them is checked here, not left to it.
        let u16_indices = fits_u16(mesh.vertices.len());
        let highest = if u16_indices {
            self.indices16.clear();
            self.indices16.reserve(mesh.indices.len());
            let mut highest = 0;
            for &i in &mesh.indices {
                highest = highest.max(i);
                // `fits_u16`: every index of a valid mesh fits.
                self.indices16.push(i as u16);
            }
            highest
        } else {
            mesh.indices.iter().copied().max().unwrap_or(0)
        };
        if usize::try_from(highest)
            .ok()
            .is_none_or(|highest| highest >= mesh.vertices.len())
        {
            log::warn!(
                "a mesh names vertex {highest} of {}; not drawn",
                mesh.vertices.len()
            );
            return;
        }
        let stride = core::mem::size_of::<Vertex>() as i32;
        let base: *const u8 = mesh.vertices.as_ptr().cast();
        // SAFETY: the pointers stay inside `mesh.vertices` (a `Vertex` is
        // `repr(C)`: two `f32` pairs and four bytes) and the indices inside
        // `mesh.indices` or `self.indices16`; every index names a vertex,
        // checked above, and the arrays are read before `glDrawElements`
        // returns, as the client-side arrays of OpenGL 1.1 are.
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            (self.f.vertex_pointer)(
                2,
                glow::FLOAT,
                stride,
                base.add(offset_of!(Vertex, pos)).cast(),
            );
            (self.f.tex_coord_pointer)(
                2,
                glow::FLOAT,
                stride,
                base.add(offset_of!(Vertex, uv)).cast(),
            );
            (self.f.color_pointer)(
                4,
                glow::UNSIGNED_BYTE,
                stride,
                base.add(offset_of!(Vertex, color)).cast(),
            );
            if u16_indices {
                (self.f.draw_elements)(
                    glow::TRIANGLES,
                    self.indices16.len() as i32,
                    glow::UNSIGNED_SHORT,
                    self.indices16.as_ptr().cast(),
                );
            } else {
                (self.f.draw_elements)(
                    glow::TRIANGLES,
                    mesh.indices.len() as i32,
                    glow::UNSIGNED_INT,
                    mesh.indices.as_ptr().cast(),
                );
            }
        }
    }

    // ------------------------------------------------------------------------

    pub fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        self.assert_not_destroyed();
        let texture = match self.textures.get(&tex_id) {
            Some(texture) => *texture,
            None => {
                // SAFETY: a plain call on the current context.
                let created = unsafe { self.gl.create_texture() };
                match created {
                    Ok(texture) => {
                        self.textures.insert(tex_id, texture);
                        texture
                    }
                    Err(err) => {
                        log::warn!("could not create a texture: {err}");
                        return;
                    }
                }
            }
        };
        // SAFETY: a state change on the current context.
        unsafe { self.gl.bind_texture(glow::TEXTURE_2D, Some(texture)) };
        match &delta.image {
            egui::ImageData::Color(image) => {
                if image.width() * image.height() != image.pixels.len() {
                    log::warn!("a texture's size and its pixel count disagree; not uploaded");
                    return;
                }
                self.upload_texture(delta.pos, image.size, delta.options, image.as_raw());
            }
        }
    }

    fn upload_texture(
        &mut self,
        pos: Option<[usize; 2]>,
        [w, h]: [usize; 2],
        options: egui::TextureOptions,
        data: &[u8],
    ) {
        if w > self.max_texture_side || h > self.max_texture_side {
            log::warn!(
                "a texture of {w} x {h} px is larger than the {} px this driver allows; not uploaded",
                self.max_texture_side
            );
            return;
        }
        if data.len() != w * h * 4 {
            log::warn!("a texture's data does not match its size; not uploaded");
            return;
        }
        let filter = |f: egui::TextureFilter| match f {
            egui::TextureFilter::Linear => glow::LINEAR,
            egui::TextureFilter::Nearest => glow::NEAREST,
        };
        let wrap = match options.wrap_mode {
            egui::TextureWrapMode::ClampToEdge => glow::CLAMP_TO_EDGE,
            egui::TextureWrapMode::Repeat => glow::REPEAT,
            egui::TextureWrapMode::MirroredRepeat => glow::MIRRORED_REPEAT,
        };
        let gl = &self.gl;
        // SAFETY: the texture is bound and `data` holds `w * h` RGBA
        // pixels, checked above; the `RGBA8` internal format (sized, so
        // that no driver picks 16-bit storage) and `UNSIGNED_BYTE` data
        // are OpenGL 1.1.
        unsafe {
            // Mipmaps are not made (`glGenerateMipmap` is OpenGL 3.0):
            // minification filters without them.
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                filter(options.magnification) as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                filter(options.minification) as i32,
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap as i32);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            if let Some([x, y]) = pos {
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    x as i32,
                    y as i32,
                    w as i32,
                    h as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(data)),
                );
            } else {
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    w as i32,
                    h as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(data)),
                );
            }
        }
        if self.frames <= 2 {
            self.report_errors("a texture upload");
        }
    }

    pub fn free_texture(&mut self, tex_id: egui::TextureId) {
        if let Some(old_tex) = self.textures.remove(&tex_id) {
            // SAFETY: the texture was made on this context and is not bound.
            unsafe { self.gl.delete_texture(old_tex) };
        }
    }

    pub fn register_native_texture(&mut self, native: glow::Texture) -> egui::TextureId {
        self.assert_not_destroyed();
        let id = egui::TextureId::User(self.next_native_tex_id);
        self.next_native_tex_id += 1;
        self.textures.insert(id, native);
        id
    }

    pub fn read_screen_rgba(&self, [w, h]: [u32; 2]) -> egui::ColorImage {
        let mut pixels = vec![0_u8; (w * h * 4) as usize];
        // SAFETY: `pixels` holds `w * h` RGBA bytes.
        unsafe {
            self.gl.read_pixels(
                0,
                0,
                w as _,
                h as _,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        let mut flipped = Vec::with_capacity((w * h * 4) as usize);
        for row in pixels.chunks_exact((w * 4) as usize).rev() {
            flipped.extend_from_slice(row);
        }
        egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &flipped)
    }

    /// Delete the OpenGL objects; to be called before the painter is dropped.
    pub fn destroy(&mut self) {
        if !self.destroyed {
            #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
            for tex in self.textures.values() {
                // SAFETY: the textures were made on this context.
                unsafe { self.gl.delete_texture(*tex) };
            }
            self.textures.clear();
            self.destroyed = true;
        }
    }

    fn assert_not_destroyed(&self) {
        assert!(
            !self.destroyed,
            "the fixed-function painter has already been destroyed!"
        );
    }
}

impl Drop for FixedFunctionPainter {
    fn drop(&mut self) {
        if !self.destroyed {
            log::warn!(
                "You forgot to call destroy() on the fixed-function painter. Resources will leak!"
            );
        }
    }
}
