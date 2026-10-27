# Patched dependencies

Copies of published crates with a small change TraceDraw needs before the
upstream release that carries it can be used. They are not workspace
members (`exclude` in the root `Cargo.toml`) and are wired in with
`[patch.crates-io]`. Each is the published crate as downloaded from
crates.io, licence and authors unchanged (see `ATTRIBUTION.md`), with the
change marked `TraceDraw:` in a comment.

## eframe 0.36.2

- Change: `native/glow_integration.rs` creates the OpenGL context with
  three attempts in turn, glutin's default (OpenGL 3.3 core), OpenGL ES,
  then a compatibility profile of any version (`context_attributes_to_try`,
  `create_first_context` and their tests), backported unchanged from
  eframe 0.37.0. 0.36.2 stopped after OpenGL ES, which WGL cannot make on
  older Intel drivers, so a card with OpenGL 2.1 only (Intel HD Graphics
  Ironlake, #616) could not open the window with `--renderer=gl`.
- `create_first_context` sits in `native/first_context.rs` (0.37 has it in
  `glow_integration.rs`) with its tests, so that TraceDraw's tests include
  and run it (`gpu::eframe_context_fallback::tests`); the copy's own tests
  run in no gate. The licence texts, which the published crate leaves
  out, are egui's `LICENSE-MIT` and `LICENSE-APACHE` at tag 0.36.2.
- Change: a painter for the fixed-function OpenGL pipeline
  (`native/fixed_function_painter.rs`, its arithmetic and tests in
  `native/fixed_function_geometry.rs`, the choice in
  `native/gl_painter.rs`), TraceDraw's own code. The OpenGL renderer draws
  with it instead of `egui_glow`'s painter when the context is desktop
  OpenGL below 3.0, or when `TRACEDRAW_GL_PAINTER=fixed` asks for it
  (`shaders` asks for `egui_glow`'s): client-side vertex arrays and
  `GL_MODULATE` texturing, no shaders, buffer objects or vertex array
  objects. The OpenGL 2.1 driver of the Intel HD Graphics of 2010
  (8.15.10.2900, #616) compiles `egui_glow`'s GLSL 1.20 shaders and then
  draws everything after the first mesh of a frame as black boxes and
  stray triangles; the fixed-function pipeline is what such drivers were
  made for, and it computes what egui's shader does (vertex colour times
  texel, premultiplied alpha). `TRACEDRAW_GL_CONTEXT=compat` asks for the
  compatibility profile before OpenGL 3.3 core and OpenGL ES, for tests
  under Mesa forced to OpenGL 2.1 (which would answer the ES attempt);
  `fixed` alone, on a core profile, draws nothing (every fixed-function
  call is invalid there, the log shows `OpenGL error` lines), which is
  how TraceDraw's read-back check of the renderer is exercised.
  TraceDraw's tests include and run the arithmetic
  (`gpu::eframe_fixed_function_geometry::tests`).
- Drop the copy and the `[patch.crates-io]` entry when the workspace moves
  to egui/eframe 0.37 or later: what holds that back is the egui 0.37
  API migration across the app. The fixed-function painter has no
  upstream counterpart: carry it over, or drop it once no supported card
  needs it.
