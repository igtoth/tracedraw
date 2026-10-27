// TraceDraw: what egui-winit applies again to a window it has just made
// (see vendor/README.md, #787). Kept apart so that TraceDraw's own tests
// compile and run it (`splash::eframe_window_start::tests`): this copy is
// outside the workspace and its own tests run in no gate.

/// The part of `builder` to apply to a window just created from it.
///
/// egui-winit applies the size and position a second time once the window
/// exists, as it then knows the monitor's scaling. A window created
/// maximized already has that size as the size it restores to, and on
/// Windows a size or position asked for after the maximize clears it: the
/// hidden window is restored to that size (shown while it is), then
/// maximized again when the builder's maximize is applied, so it opened
/// at its normal size first and grew to the screen after. A window to be
/// maximized gets neither again.
pub(crate) fn after_creation(builder: &egui::ViewportBuilder) -> egui::ViewportBuilder {
    let mut builder = builder.clone();
    if builder.maximized == Some(true) {
        builder.inner_size = None;
        builder.position = None;
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_maximized_window_is_not_given_its_size_again() {
        let builder = egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 900.0])
            .with_position([40.0, 30.0])
            .with_min_inner_size([800.0, 500.0])
            .with_maximized(true);
        let after = after_creation(&builder);
        assert_eq!(after.inner_size, None);
        assert_eq!(after.position, None);
        assert_eq!(after.maximized, Some(true));
        assert_eq!(after.min_inner_size, builder.min_inner_size);
    }

    #[test]
    fn a_window_at_its_normal_size_keeps_its_size_and_position() {
        for maximized in [None, Some(false)] {
            let mut builder = egui::ViewportBuilder::default()
                .with_inner_size([1500.0, 900.0])
                .with_position([40.0, 30.0]);
            builder.maximized = maximized;
            let after = after_creation(&builder);
            assert_eq!(after.inner_size, builder.inner_size);
            assert_eq!(after.position, builder.position);
            assert_eq!(after.maximized, maximized);
        }
    }
}
