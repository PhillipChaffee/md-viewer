//! Kani proofs for the zoom readout. `PROOFS.md` lists each harness with
//! its bounds.

use super::zoom_percent;

/// Every zoom has a readout: the conversion to a whole percentage never
/// panics, NaN reads as 0, and inside the widest zoom range the app has
/// (the lightbox's 10% to 1000%) the readout is within half a percent of
/// the zoom.
#[kani::proof]
fn zoom_percent_is_total_and_close() {
    let zoom: f32 = kani::any();
    let percent = zoom_percent(zoom);

    if zoom.is_nan() {
        assert!(percent == 0);
    }
    if (0.1..=10.0).contains(&zoom) {
        assert!((10..=1000).contains(&percent));
        assert!((f64::from(percent) - f64::from(zoom) * 100.0).abs() <= 0.501);
    }

    kani::cover!(zoom.is_infinite());
    kani::cover!(percent == 125);
    kani::cover!(percent == i32::MAX);
}
