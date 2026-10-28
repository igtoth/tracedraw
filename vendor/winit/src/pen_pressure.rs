//! TraceDraw: the pressure of a graphics tablet's pen as one number in
//! 0..1, shared by the X11 and macOS backends (see `vendor/README.md`).
//! The file has no dependency on the rest of the crate, so that
//! TraceDraw's tests can include and run it.

/// `value` read on a pressure axis that runs from `min` to `max`, as a
/// share of the axis, 0 to 1. `None` when the pen does not press (the
/// value is at `min` or below), when the axis has no range, or when any
/// number is not finite: no pressure at all is what a platform without
/// pressure reports, so a pen at rest never leaves a stale reading.
pub(crate) fn normalized(value: f64, min: f64, max: f64) -> Option<f64> {
    if !(value.is_finite() && min.is_finite() && max.is_finite()) || max <= min {
        return None;
    }
    let share = (value - min) / (max - min);
    (share > 0.0).then(|| share.min(1.0))
}

#[cfg(test)]
mod tests {
    use super::normalized;

    #[test]
    fn a_pressure_reading_becomes_a_share_of_its_axis() {
        assert_eq!(normalized(0.0, 0.0, 2047.0), None);
        assert_eq!(normalized(2047.0, 0.0, 2047.0), Some(1.0));
        let half = normalized(1023.5, 0.0, 2047.0).expect("a share");
        assert!((half - 0.5).abs() < 1e-12);
        // An axis that starts above zero, and a reading past its end.
        assert_eq!(normalized(10.0, 10.0, 20.0), None);
        assert_eq!(normalized(15.0, 10.0, 20.0), Some(0.5));
        assert_eq!(normalized(25.0, 10.0, 20.0), Some(1.0));
        assert_eq!(normalized(-5.0, 0.0, 20.0), None);
        // No range, or numbers that are not numbers.
        assert_eq!(normalized(1.0, 0.0, 0.0), None);
        assert_eq!(normalized(1.0, 5.0, 2.0), None);
        assert_eq!(normalized(f64::NAN, 0.0, 1.0), None);
        assert_eq!(normalized(0.5, 0.0, f64::INFINITY), None);
        // macOS reports 0..1 already.
        assert_eq!(normalized(0.25, 0.0, 1.0), Some(0.25));
    }
}
