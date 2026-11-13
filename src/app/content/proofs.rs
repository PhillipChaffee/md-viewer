//! Kani proofs for the scroll corrections of the document pane.
//! `PROOFS.md` lists each harness with its bounds.
//!
//! Each harness is loop-free float arithmetic over every finite input.
//! Infinite offsets and heights are left out: their differences are NaN,
//! which the verifier reports as an error wherever it arises.

use super::{
    header_correction, search_correction, wheel_scroll_target, worth_scrolling,
    HEADER_SCROLL_MARGIN, SEARCH_VISIBLE_MARGIN,
};

/// A wheel step during a selection never panics, and an offset it stores
/// is strictly inside the document, half a pixel clear of either end, and
/// more than half a pixel from where the view was.
#[kani::proof]
fn wheel_scroll_never_stores_an_end_of_the_document() {
    let (current, wheel, max_scroll): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(current.is_finite() && wheel.is_finite());
    // The one caller passes `(content - viewport).max(0.0)`, and `f32::max`
    // with 0.0 is never negative and never NaN.
    kani::assume(max_scroll >= 0.0);

    let stored = wheel_scroll_target(current, wheel, max_scroll);

    if let Some(offset) = stored {
        assert!(offset >= 0.5 && offset <= max_scroll - 0.5);
        assert!((offset - current).abs() > 0.5);
    }

    kani::cover!(stored.is_some());
    kani::cover!(stored.is_none() && wheel != 0.0);
    kani::cover!(max_scroll.is_infinite());
}

/// A correction toward a search match or a clicked heading never scrolls
/// above the top of the document, and none is asked for when the view is
/// already within two pixels of where it would go.
#[kani::proof]
fn corrections_never_scroll_above_the_top() {
    let (target_y, current, viewport): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(target_y.is_finite() && current.is_finite() && viewport.is_finite());

    let for_search = search_correction(target_y, current, viewport);
    let for_header = header_correction(target_y, current);

    if let Some(offset) = for_search {
        assert!(offset >= 0.0);
        assert!((offset - current).abs() > 2.0);
        assert!(viewport > 0.0);
    }
    if let Some(offset) = for_header {
        assert!(offset >= 0.0);
        assert!((offset - current).abs() > 2.0);
        assert!(offset == (target_y - HEADER_SCROLL_MARGIN).max(0.0));
    }
    if let Some(offset) = worth_scrolling(target_y, current) {
        assert!(offset == target_y);
    }

    kani::cover!(for_search.is_some());
    kani::cover!(for_search == Some(0.0));
    kani::cover!(for_header == Some(0.0));
    kani::cover!(for_header.is_none());
}

/// A search match that is on screen, clear of the margins, is left where
/// it is.
#[kani::proof]
fn a_visible_match_is_left_alone() {
    let (match_y, current, viewport): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(current.is_finite() && viewport.is_finite());
    kani::assume(match_y >= current + SEARCH_VISIBLE_MARGIN);
    kani::assume(match_y <= current + viewport - SEARCH_VISIBLE_MARGIN);

    assert!(search_correction(match_y, current, viewport).is_none());

    kani::cover!(viewport > 100.0);
}
