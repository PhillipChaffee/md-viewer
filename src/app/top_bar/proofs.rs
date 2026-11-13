//! Kani proofs for where the top bar puts the document's title.
//! `PROOFS.md` lists each harness with its bounds.

use eframe::egui;

use super::{title_center, TITLE_GAP};

/// For finite geometry the title's center never panics the clamp, and the
/// title keeps its gap from the buttons at both ends whenever the free
/// span has room for it; where it has not, the title is centered in the
/// span.
#[kani::proof]
fn the_title_keeps_clear_of_the_buttons() {
    let (bar_center, min, max, width): (f32, f32, f32, f32) =
        (kani::any(), kani::any(), kani::any(), kani::any());
    kani::assume(bar_center.is_finite() && min.is_finite() && max.is_finite());
    kani::assume(width.is_finite());
    let free = egui::Rangef::new(min, max);

    let center = title_center(bar_center, free, width);

    let half = width / 2.0 + TITLE_GAP;
    let (leftmost, rightmost) = (min + half, max - half);
    if leftmost <= rightmost {
        assert!(center >= leftmost && center <= rightmost);
        if bar_center >= leftmost && bar_center <= rightmost {
            assert!(center == bar_center);
        }
    } else {
        assert!(center == free.center());
    }

    kani::cover!(leftmost <= rightmost && center != bar_center);
    kani::cover!(leftmost <= rightmost && center == bar_center);
    kani::cover!(leftmost > rightmost);
}
