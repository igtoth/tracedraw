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
- Change: a window made maximized is not given its size and position a
  second time once it exists (`native/window_start.rs`, used by
  `glow_integration.rs` and by `epi_integration::create_window`, which
  `wgpu_integration.rs` calls in place of `egui_winit::create_window`).
  egui-winit applies them again after creating a window, as it then knows
  the monitor's scaling; the window already has that size as the size it
  restores to, and on Windows a size or position asked for after the
  maximize clears it, so the hidden window was restored, shown at its
  normal size for a moment and maximized again: TraceDraw started without
  the splash screen opened at its normal size and then grew to the
  screen (#787). TraceDraw's tests include and run it
  (`splash::eframe_window_start::tests`). Check whether egui 0.37 still
  needs it before dropping the copy.
- Drop the copy and the `[patch.crates-io]` entry when the workspace moves
  to egui/eframe 0.37 or later: what holds that back is the egui 0.37
  API migration across the app. The fixed-function painter has no
  upstream counterpart: carry it over, or drop it once no supported card
  needs it.

## winit 0.30.13

- Change: a graphics tablet's pen reports its pressure on X11 and macOS
  (#850). winit 0.30 carries pen pressure on Windows only, as a
  `WindowEvent::Touch` with a `force`, which egui-winit hands to egui as
  `Event::Touch { force }` and TraceDraw reads (`apps/tracedraw/src/pen.rs`,
  `docs/behavior/pen-pressure.md`). The copy does the same on the two
  other desktops:
  - X11 (`platform_impl/linux/x11/mod.rs`, `event_processor.rs`,
    `atoms.rs`): a slave device's valuator labelled `Abs Pressure` is
    remembered with its range when the device is listed
    (`Device::pressure_axis`); a motion event carrying that valuator
    sends, besides the `AxisMotion` it always sent, a
    `WindowEvent::Touch` of phase `Moved` at the pointer's place with the
    reading as a share of the axis (`Force::Normalized`), or no force at
    the axis's minimum (the pen lifted).
  - macOS (`platform_impl/macos/view.rs`): a mouse move or drag whose
    `NSEvent` subtype is a tablet point sends the same `Touch` with the
    event's `pressure` (already 0 to 1). `subtype` and `pressure` are
    read only when the event's type is a mouse down, up, move or drag
    (left, right, other) or a tablet point: the scroll wheel and the
    magnify, smart magnify and rotate gestures go through the same
    `mouse_motion`, and AppKit raises an exception for `subtype` on them.
  - `src/pen_pressure.rs`: the shared arithmetic, with its tests; it
    depends on nothing in the crate, and TraceDraw's tests include and
    run it (`winit_pen_pressure::tests`).
  - Only `Moved` touches are sent, never `Started` or `Ended`: egui-winit
    turns those into presses and releases of the primary button, which
    the pen's real button events already send. The pointer move
    egui-winit also makes of a `Moved` touch repeats the one just sent.
    egui itself would still take any `Event::Touch` for a touch screen
    for the rest of the session (it makes a touch state for the device,
    and `has_touch_screen` turns on: lists scroll by dragging, windows
    move from anywhere, labels lose drag selection), so TraceDraw's raw
    input hook reads the force and then takes out every `Moved` touch
    whose id never had a `Started` one (`pen::filter_raw_input`); a real
    touch screen's touches start, move and end, and pass as they came.
    Wayland is unchanged: winit does not bind the tablet protocol
    (`zwp_tablet_v2`), so a pen reaches the window there only through
    XWayland (`tracedraw --x11`, which picks winit's X11 backend).
- The published crate's `examples/` and `tests/` stay as downloaded (the
  manifest lists them); they build in no gate.
- Drop the copy and the `[patch.crates-io]` entry when a winit release
  carries pen pressure on X11 and macOS (rust-windowing/winit#99), and
  check that egui-winit still maps `Touch` forces the same way.
