mod app_icon;
mod epi_integration;
mod event_loop_context;
pub mod run;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

/// File storage which can be used by native backends.
#[cfg(feature = "persistence")]
pub mod file_storage;

pub(crate) mod winit_integration;

// TraceDraw: a maximized window is not given its size again once made
// (see vendor/README.md, #787).
mod window_start;

#[cfg(feature = "glow")]
mod glow_integration;

// TraceDraw: the fixed-function painter of the OpenGL renderer (see
// vendor/README.md).
#[cfg(feature = "glow")]
mod fixed_function_geometry;
#[cfg(feature = "glow")]
mod fixed_function_painter;
#[cfg(feature = "glow")]
mod gl_painter;

#[cfg(feature = "wgpu_no_default_features")]
mod wgpu_integration;
