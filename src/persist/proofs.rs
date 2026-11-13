//! Kani proofs for what a saved session's numbers come back as.
//! `PROOFS.md` lists each harness with its bounds.
//!
//! Each harness is loop-free and covers every value the settings file can
//! hold for the field.

use super::{restored_math_scale, restored_sidebar_width, restored_zoom};
use crate::layout::SIDEBAR_MIN_WIDTH;

/// Every saved sidebar width comes back as a finite width no narrower
/// than the minimum: a finite saved width is kept or widened to the
/// minimum, and a missing, infinite or NaN one gives the default.
#[kani::proof]
fn restored_sidebar_width_is_usable() {
    let saved: Option<f32> = kani::any();
    let default: f32 = kani::any();
    // The two callers pass the explorer's and the outline's default width.
    kani::assume(default.is_finite() && default >= SIDEBAR_MIN_WIDTH);

    let width = restored_sidebar_width(saved, default);

    assert!(width.is_finite() && width >= SIDEBAR_MIN_WIDTH);
    match saved {
        Some(saved) if saved.is_finite() => {
            assert!(width == saved.max(SIDEBAR_MIN_WIDTH));
        }
        _ => {
            assert!(width == default);
        }
    }

    kani::cover!(saved.is_some_and(f32::is_nan));
    kani::cover!(saved.is_some_and(f32::is_infinite));
    kani::cover!(saved.is_some_and(|saved| saved < SIDEBAR_MIN_WIDTH));
    kani::cover!(saved.is_some_and(|saved| saved > SIDEBAR_MIN_WIDTH && saved.is_finite()));
}

/// A saved zoom that is a number comes back inside the zoom range, a zoom
/// already in range comes back as saved, and no saved zoom is 100%.
///
/// A saved NaN is not covered: see the finding in `PROOFS.md`.
#[kani::proof]
fn restored_zoom_is_in_range() {
    let saved: Option<f32> = kani::any();
    kani::assume(!saved.is_some_and(f32::is_nan));

    let zoom = restored_zoom(saved);

    assert!((0.5..=3.0).contains(&zoom));
    match saved {
        Some(saved) if (0.5..=3.0).contains(&saved) => {
            assert!(zoom == saved);
        }
        Some(_) => {}
        None => {
            assert!(zoom == 1.0);
        }
    }

    kani::cover!(saved.is_some_and(f32::is_infinite));
    kani::cover!(saved.is_some_and(|saved| saved < 0.0));
}

/// A saved formula size that is a number comes back between 1 and 2.
///
/// A saved NaN is not covered: see the finding in `PROOFS.md`.
#[kani::proof]
fn restored_math_scale_is_in_range() {
    let saved: Option<f32> = kani::any();
    kani::assume(!saved.is_some_and(f32::is_nan));

    let scale = restored_math_scale(saved);

    assert!((1.0..=2.0).contains(&scale));
    match saved {
        Some(saved) if (1.0..=2.0).contains(&saved) => {
            assert!(scale == saved);
        }
        Some(_) => {}
        None => {
            assert!(scale == 1.0);
        }
    }

    kani::cover!(saved.is_some_and(|saved| saved > 2.0));
}

/// The gap the two harnesses above leave: a settings file that holds NaN
/// for the zoom or the formula size starts the app with NaN, because
/// `f32::clamp` passes NaN through. This harness pins that behavior so the
/// finding in `PROOFS.md` stays true until the code changes.
#[kani::proof]
fn restored_zoom_passes_nan_through() {
    assert!(restored_zoom(Some(f32::NAN)).is_nan());
    assert!(restored_math_scale(Some(f32::NAN)).is_nan());
}
