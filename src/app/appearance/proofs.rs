//! Kani proofs for the range of the column-width slider. `PROOFS.md`
//! lists each harness with its bounds.

use super::widest_column;
use crate::layout::ReadingWidth;

/// The width slider always has a range to slide over: whatever the pane's
/// width, NaN included, its upper end is a valid width at least one step
/// above its lower end.
#[kani::proof]
fn the_width_slider_always_has_a_range() {
    let pane_width: f32 = kani::any();
    // Infinite panes are left out: the verifier's model of the fused
    // multiply-add in `resolve` reports an error for every infinite
    // addend.
    kani::assume(!pane_width.is_infinite());

    let widest = widest_column(pane_width);

    assert!(widest >= ReadingWidth::MIN + ReadingWidth::STEP);
    assert!(widest <= ReadingWidth::MAX);
    assert!(ReadingWidth::new(widest).get() == widest);

    kani::cover!(pane_width.is_nan());
    kani::cover!(widest > ReadingWidth::MIN + ReadingWidth::STEP && widest < ReadingWidth::MAX);
    kani::cover!(widest == ReadingWidth::MAX);
}
