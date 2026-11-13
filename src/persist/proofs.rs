//! Kani proofs for the restore of a saved sidebar width. `PROOFS.md`
//! lists each harness with its bounds.

use super::restored_sidebar_width;
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
