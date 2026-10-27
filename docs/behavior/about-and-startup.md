# About box and start-up

## About TraceDraw

Help > About TraceDraw opens the About box from the Welcome Screen as well
as from a drawing. It is a chrome dialog (white title bar with the close
button) 480 points wide:

- A banner across the top: a diagonal gradient from `#244475` (top left)
  through `#1A3158` (top right) to `#0E192D` (bottom), faint light blue
  (`#5CAEFF`) curves on the right with one selected node (a square node,
  two handles along the tangent, round handle ends), the 76 px
  application icon, the name in the bold face at 29 px ("Trace" white,
  "Draw" `#7FC3FF`), the tagline and the version in a rounded chip,
  then a green ALPHA badge whose tooltip says the program is at an
  early stage: most tools work, some features are missing or
  incomplete, keep copies of important files, report problems on
  GitHub.
- The description at 13.5 px with 20 px lines, a light separator, then
  the facts as two columns with dim labels: Author, License, Source code
  (a link that opens the repository), Developed with.
- The copyright line in small dim text and a Close button (Enter or Esc).

## Splash screen

While TraceDraw starts, a borderless 640 x 380 point window centred on
the screen shows the same artwork as the About banner at a larger size
(two selected nodes on the main curve), the 96 px icon, the name at
46 px with the tagline, and a darker strip along the bottom with the
status line, a thin progress bar and the footer (copyright on the left,
the version on the right).

- The bundled interface font is installed at once; the system font list
  (and the system fonts for Chinese, Japanese, Arabic, Hindi and Bengali)
  is read on a worker thread. Texts in those languages appear once their
  fonts are in; the name and version always show.
- Status: "Loading fonts..." while the thread works, then "Opening
  <file>..." for a file named on the command line, else "Preparing the
  workspace...".
- The bar eases towards 80 % over 1.6 s while the fonts load and fills
  once they are in. The splash stays up at least 1.6 s.
- Then the editor is built and the window becomes the main window:
  decorations, resizable, the maximize button, a minimum of 800 x 500
  (never more than four fifths of the monitor, so it fits a 1366 x 768
  or 1280 x 720 screen with a taskbar at any interface size), the size
  kept from the last session (1600 x 950 the first time, never larger
  than 95 % of the monitor) centred on the monitor, then maximized in
  the next frames when it was left maximized or when that size does
  not fit the monitor (a small screen; see Window state below).
- The splash window is made resizable too, though it has no frame to
  drag: a window made not resizable stays at its size for good under
  some window managers (Xfwm4), and the main window could then neither
  be resized nor maximized
  (`splash::tests::small_screens_get_a_window_that_fits_and_starts_maximized`,
  `splash::tests::the_main_window_offers_the_maximize_button`).
- Options > General > Show the splash screen at start-up turns it off.
  `--no-splash` or `TRACEDRAW_NO_SPLASH=1` skip it for one run and open a
  plain 1600 x 950 window (automation and the visual checks use this).
- The browser build has no splash: the page shows its own loader.

## One window for every start (#646)

A file opened from the file manager, a shortcut or the command line while
TraceDraw runs opens in a tab of the running window (or shows its tab
when it is open already, see `documents.md`), which comes to the front;
the new start ends at once, before its log would replace the running
one's (`instance.rs`). Several files selected together all go there (the
Linux launcher passes them to one start, `Exec=tracedraw %F`). A start
with no file, one with `--new-instance`, and the `--mcp`, `--mcp-window`
and `--reset-settings` starts open windows of their own.

- The running window holds a lock on `instance.lock` in the settings
  folder for as long as it runs; the system lets go of it when the
  process ends, a crash included, so nothing stale is left in the way.
- It listens on a port of this computer (127.0.0.1) named, with a random
  token, in `instance.json` (readable by this user only on Linux and
  macOS). A start sends one line `{"token":…,"files":[…]}` with absolute
  paths and the window answers `{"ok":true}`; a wrong token or garbage
  reaches nothing. A start waits up to three seconds for a window that is
  just starting, then opens its own.
- The window takes the files between frames from a channel (one
  `try_recv` a frame when none came), waiting while a dialog or a question
  is up or the drawings are being closed. Wayland does not let a window
  raise itself: it asks for attention instead.
- A start with the next renderer after a failed one (see below) takes the
  place over. Portable copies and `TRACEDRAW_CONFIG_DIR` each have their
  own window. The browser build has none of this.
- Checks: `instance::tests::a_second_start_hands_its_files_to_the_first`,
  `a_crash_leaves_nothing_in_the_way`, `strangers_are_refused`,
  `starts_at_the_same_time_make_one_window`,
  `a_handed_over_file_opens_in_a_tab_once_no_question_is_up`,
  `the_linux_launcher_starts_once_for_several_files`; checked by hand on
  X11 with two starts (the second ended in 12 ms, its file in a new tab of
  the first window).

## The renderer (#445, #439)

- The desktop window is drawn with wgpu (Direct3D 12, Vulkan or Metal).
  Options > General > Use hardware acceleration (on) picks the graphics
  card named beside it, or with Fastest available a separate card before
  a built-in one, a virtual one, then others; never a software adapter
  (#616). Off, the software adapter (WARP on Windows, a CPU Vulkan driver
  elsewhere). Options shows what draws the window in this run; the
  choice takes effect at the next start.
- A start that cannot open the window that way runs TraceDraw again with
  the next renderer, the other arguments kept; the first process waits
  for it. With hardware acceleration on the order is the graphics card,
  then `--renderer=gl` (OpenGL, as earlier versions drew), then
  `--renderer=software`: the software adapter draws every frame on the
  CPU and always opens, so it is the last resort. With it off: the
  software adapter, then OpenGL. A computer whose card no wgpu backend
  accepts (an old driver, no Direct3D 12) thus draws with OpenGL instead
  of on the CPU (#616). The renderer that opened the window when the
  first choice did not is kept in the settings (`working_renderer`) and
  used straight away next time (and OpenGL from there falls back to the
  software adapter); changing the choice in Options clears it
  (`gpu::tests::a_renderer_that_fails_hands_over_to_the_next`,
  `gpu::tests::hardware_mode_never_draws_on_the_software_adapter`).
  So a virtual machine without a usable graphics card opens.
- The OpenGL renderer asks the driver for OpenGL 3.3 core, then OpenGL
  ES, then a compatibility context of any version. A card whose driver
  offers OpenGL 2.1 only (Intel HD Graphics of the Ironlake generation,
  which has no Direct3D 12 and which wgpu's OpenGL backend refuses below
  3.3) gets the last one and draws on the card through `--renderer=gl`,
  with the fixed-function painter of the next item
  (`TRACEDRAW_GL_PAINTER=shaders` keeps egui's GLSL 1.20 shaders),
  instead of on the CPU (#616). eframe 0.36
  stopped after OpenGL ES; the likely reason the reporter's
  `--renderer=gl` did not start is that Windows drivers of that age
  cannot make an OpenGL ES context (WGL needs an extension they do not
  offer), which the log of a build with this fallback will confirm.
  TraceDraw carries eframe 0.36.2 with the fallback of eframe 0.37
  (`vendor/README.md`,
  `gpu::tests::opengl_falls_back_to_a_compatibility_context`,
  `gpu::eframe_context_fallback::tests`). Checked
  under Mesa forced to OpenGL 2.1 and GLSL 1.20: the window, the page,
  gradients and text draw as on the other renderers.
- On desktop OpenGL below 3.0 the OpenGL renderer draws with the
  fixed-function pipeline of OpenGL 1.x (client-side vertex arrays,
  texturing that multiplies the vertex colour, premultiplied alpha
  blending; no shaders, buffer objects or vertex array objects), the
  painter TraceDraw carries in its eframe copy (`vendor/README.md`,
  `TRACEDRAW_GL_PAINTER=fixed` or `=shaders` overrides the choice). The
  2012 driver of the Intel HD Graphics of 2010 PCs (8.15.10.2900, OpenGL
  2.1, #616) opened the window with egui's shaders and drew everything
  after the first mesh of each frame as black boxes and stray triangles,
  so the window came out nearly black; the fixed-function pipeline is the
  path such drivers were made for. The log says `drawing with the
  fixed-function pipeline`, the driver's texture limit and extension
  count, the first frame's mesh count, and any OpenGL error the driver
  reports (up to twelve). Textures are kept to 8192 px a side at most,
  as on wgpu; mipmaps are not made (OpenGL 3.0), so a shrunk picture
  filters without them. Checked under Mesa forced to OpenGL 2.1
  (`TRACEDRAW_GL_CONTEXT=compat`, which asks for the compatibility
  profile first, as Mesa would otherwise answer the OpenGL ES attempt):
  the window matches the one egui's shaders draw
  (`gpu::eframe_fixed_function_geometry::tests` cover the scissor,
  index and texture-limit arithmetic).
- The OpenGL renderer is also checked once it draws (#616,
  `Shell::check_renderer`): at the main window's frame
  `gpu::CHECKED_FRAME` a read-back is asked for, and the frame after it
  is read back from the renderer with a probe drawn last in it, a light
  square `gpu::PROBE_SIDE` points wide near the bottom right corner, on
  the topmost layer (the reporter's driver drew only the first meshes of
  a frame right). The frame says the renderer is blind
  (`gpu::renderer_blind`) when the probe is missing and the frame is
  nearly black: below its top quarter fewer than one pixel in
  `gpu::BLIND_ONE_IN` (50) has a channel at `gpu::BLIND_BRIGHTNESS` (40 of
  255) or more; the reporter's recording has one in 200 to 500, a drawn
  window more than one in 2 in the light and dark themes and one in 30
  in the high-contrast theme with the welcome screen, which the probe
  settles. A read-back of a window another window is in front of does
  not count (a desktop without composition leaves covered pixels
  undrawn) and is logged as `came back nearly black from the renderer
  while another window is in front; checking again`; one that counts is
  logged as `came back nearly black from the renderer: it draws nothing
  that can be seen (1 of 2)`, and `gpu::RECHECK_FRAMES` (30) frames
  later another is asked for. After `gpu::BLIND_READS` (2) such
  read-backs the window closes whatever the documents hold, and the
  start goes on with the next renderer, which is remembered as usual
  (`working_renderer`, Options > General shows and changes it), so the
  next start takes it at once; a start again that fails is reported the
  way a window that would not open is. When no renderer is left to try
  (hardware acceleration off: the chain ends with OpenGL) the log says
  `no other renderer is left to try; the window stays`. A window drawn
  is logged as `read back drawn`
  (`gpu::tests::a_nearly_black_frame_is_a_blind_renderer`,
  `gpu::tests::the_probe_decides_with_the_frame`).
- A panic while a renderer starts (eframe's OpenGL set-up panics when
  the driver offers no matching pixel format) is caught and logged, and
  the next renderer is tried; it is not written to `crash.log` or shown.
  When no renderer is left, the reason is logged and shown in a message
  box naming the log (a panic of the last one is shown as a crash, with
  `crash.log`), and TraceDraw ends with an error.
- A renderer can also end the program without an error to hand on: a
  graphics driver that crashes the process, or hangs it until it is
  closed, inside wgpu's set-up. So before the window is opened the start
  writes the renderer it tries to `renderer-attempt` in the settings
  folder, and removes the file as soon as the window is open. A start
  that finds the file logs "the last start did not open the window with
  ..." and begins with the renderer after that one (after the last one,
  the first choice again); `--renderer=` from a hand-over still wins.
  The renderer that then opens the window is remembered as above, so a
  computer whose card crashes in wgpu opens with OpenGL from the second
  start on, without `--renderer=gl` typed (#570,
  `gpu::tests::a_start_that_never_opened_its_window_is_followed_by_the_next_renderer`).
- A start whose window has not opened 20 seconds after it began
  (`gpu::WINDOW_WAIT`: the driver hung it inside wgpu's set-up, so it
  never comes back to hand over) starts TraceDraw again with the next
  renderer itself, the other arguments kept, and ends. Before that it
  answers no to later starts, so a file opened meanwhile goes to a window
  of its own instead of to the window that never shows. With the last
  renderer it only logs and keeps waiting, which may just be slow. So the
  first start opens too, not only the one after it (#570,
  `gpu::tests::a_start_hung_without_its_window_goes_on_with_the_next_renderer`,
  `instance::tests::a_start_without_its_window_declines_files`).
- The log names every graphics adapter wgpu offers (backend, type,
  vendor and device ids, driver) with why it was taken or left, the
  renderer each start tries and the one that opened the window; an
  OpenGL start adds the driver's vendor, renderer and versions.
- The log goes to `tracedraw.log` in the settings folder as well as to
  the console (written afresh each start; a start handed over to the
  next renderer adds to it). A panic is written to `crash.log` there and
  shown in a message box naming that file, once; the Windows build has
  no console, so a start that failed showed nothing (#570,
  `startup_log::tests::the_log_and_the_crash_report_go_in_the_settings_folder`).
- At start the log names the processor and its threads (and on Windows
  the version and build, from `RtlGetVersion`), and wgpu's own lines on
  which backends started and which adapters it left out
  (`wgpu_core::instance` at debug; the default filter is
  `info,wgpu_core::instance=debug`, `RUST_LOG` replaces it). The first
  frame logs how long the start took, the window's size in pixels, the
  pixels per point and the monitor's size. With `RUST_LOG=debug` (or
  `info,tracedraw=debug`) a line every 10 s while frames happen gives
  their count, how many had input, the mean and longest time to build
  one, the mean and longest gap between them, and the five most frequent
  reasons a frame was asked for; an idle window logs nothing, and with
  debug off nothing is measured (#616,
  `frame_stats::tests::frames_are_summed_up_every_ten_seconds`,
  `startup_log::tests::the_log_names_the_computer`).
- The Windows builds link the C runtime statically (`.cargo/config.toml`,
  `+crt-static`): they do not need `vcruntime140.dll` (#570).
- The browser build draws with WebGL.
- A software adapter drawing the window (wgpu's CPU adapter, WARP on
  Windows, or an OpenGL renderer named GDI Generic, llvmpipe, softpipe,
  lavapipe or SwiftShader) rasterizes every frame on the CPU, and every
  pointer move is a frame, so the window then keeps lighter (#616):
  frames begin at least 1/30 s apart (the pointer moves in between are
  merged into the next frame), menus, dialogs and dockers cast no
  shadows, and the drawn pointers (cross hair, I-beam, hands, magnifiers
  and the others) give way to the system's nearest, as a drawn one
  trails the real pointer by a slow frame. The status bar says it once,
  naming the adapter, and the log says it at start. With Use hardware
  acceleration off in Options that is the reason, so the note says so
  ("Drawn on the CPU (...): hardware acceleration is off. Click to turn
  it on in Options > General.") and a click on it opens Options at the
  General page; with it on, the note says the card's own driver makes it
  faster, and is not a link. Nothing is turned on by itself: some cards
  draw wrongly with acceleration
  (`pick_drag_tests::the_software_renderer_note_says_hardware_acceleration_is_off`).
  A graphics card is
  never paced (`gpu::tests::frames_are_paced_to_the_cap`,
  `gpu::tests::software_renderers_are_recognised`,
  `ui::cursors::tests::a_software_adapter_shows_the_system_pointers`).

## Back to the defaults

Pressing F8 while the splash screen shows asks, once the editor is up,
whether to reset every preference to how TraceDraw comes (Options >
Global > Reset all settings... asks the same). Reset keeps the recent
drawings, the last folder and the window's size; Cancel changes
nothing. `tracedraw --reset-settings` resets without asking (scripts,
support). Drawings are never touched.

- `splash::tests::f8_while_starting_asks_to_reset_the_settings`
- `settings::tests::a_reset_keeps_history_and_nothing_else`
- `app::tests::resetting_the_settings_brings_back_the_defaults`

## Window state

The main window's state is kept in the preferences on exit: maximized or
not, and its inner size while it was neither maximized nor minimized.
The size is kept in system points (pixels over the system's scaling of
the monitor), not in interface points, which shrink as the interface
size (Options, or the system's text size) grows: the splash opens the
window before the interface size applies, and a size kept in interface
points came back smaller at every start (#787,
`splash::tests::the_window_comes_back_at_its_size_with_a_larger_interface`).

The splash turns into the main window by asking for its decorations,
its kept size and its position only. A window that should start
maximized (it was left maximized, or its kept size does not fit this
monitor) is maximized from the main window's first frame on, once the
window system has applied that size: on Windows a size applied after
the maximize left the window at its normal size in the corner of the
screen with the maximized button state (#787, seen with the interface
in Chinese and a new drawing at start, whose slow first frames change
the timing).

For the first 30 frames of the main window and at least its first 3
seconds (`splash::SETTLE_FRAMES`, `splash::SETTLE_SECONDS`) its state is
not kept: the window system may still report the splash's state then.
Meanwhile, every quarter of a second, a window that should start
maximized is checked against the monitor, not only by the state it
reports: one that reports it is not maximized is asked to maximize, and
one that reports it is maximized but has not filled at least 96 % of
the monitor's width or height (a taskbar sits on one side; a window at
its normal size starts at 95 % at most) for a quarter of a second is
restored and then maximized again (a second maximize does not change
such a window). It stops once the window is maximized and fills the
screen, and after 6 requests; each request is logged (#787,
`splash::tests::a_maximized_window_left_at_its_normal_size_is_maximized_again`,
`splash::tests::the_window_settles_for_its_frames_and_its_time`,
`splash::tests::a_dropped_maximize_is_asked_for_again_and_a_large_window_fills_the_screen`).

## Checks

- `ui::about::tests::the_about_box_draws_on_the_welcome_screen`
- `ui::brand::tests::the_embedded_icon_decodes`
- `ui::brand::tests::cubic_helpers_follow_the_curve`
- `ui::brand::tests::the_main_curve_spans_the_right_part`
- `splash::tests::progress_waits_for_the_fonts_and_the_minimum_time`
- `splash::tests::the_status_names_the_file_being_opened`
- `splash::tests::the_main_window_keeps_a_sensible_size`
- `splash::tests::the_main_window_is_centred`
- `splash::tests::a_splash_frame_draws_and_then_starts_the_editor`

## Recent drawings on the Welcome Screen

Get Started lists the drawings opened or saved last (ten at most), each
with a thumbnail of its first page, its name and its folder; a click
opens it, a drawing whose file is gone is greyed out, and Clear list
forgets them.

- The thumbnail is made when a drawing is opened or saved: the first
  page rendered with a white background, 160 pixels on its long side,
  kept as PNG in the preferences folder under `thumbnails/`, named by a
  hash of the drawing's path (FNV-1a, the same from one run to the next).
  Saving again makes it again.
- Clear list removes the thumbnails too. The browser version records no
  recent drawings, so it makes no thumbnails.
- `thumbnails::tests::opened_and_saved_drawings_get_a_thumbnail`

