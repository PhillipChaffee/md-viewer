//! Kani proofs for the width of a segmented control's segments.
//! `PROOFS.md` lists each harness with its bounds.

use super::segment_share;

/// A segment's width never divides by zero: for any finite width to
/// share and any count, none included, it is a finite width that is
/// never negative, and up to `MAX_SEGMENTS` segments together never take
/// more than the width they share.
#[kani::proof]
fn segments_share_the_width_they_are_given() {
    /// More segments than any control has; it keeps the count exact as an
    /// `f32`.
    const MAX_SEGMENTS: usize = 64;

    let available: f32 = kani::any();
    let count: usize = kani::any();
    // Infinite widths are left out: the verifier's model of the fused
    // multiply-add reports an error for every infinite addend.
    kani::assume(available.is_finite());

    let share = segment_share(available, count);

    assert!(share >= 0.0 && share.is_finite());
    if count >= 1 && count <= MAX_SEGMENTS && available >= 0.0 {
        assert!(share <= available);
    }

    kani::cover!(count == 0 && share > 0.0);
    kani::cover!(share > 0.0 && count == 3);
    kani::cover!(available < 0.0);
}
