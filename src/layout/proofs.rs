//! Kani proofs for the reading column, keyboard scrolling and zoom
//! arithmetic. `PROOFS.md` lists each harness with its bounds.
//!
//! Every harness here is loop-free, so each one covers all bit patterns of
//! its inputs, NaN and the infinities included, unless it says otherwise.

use super::{
    centered_column_left, keyboard_scroll_target, line_fraction, whole_pixels, zoomed,
    KeyboardScrollAction, ReadingWidth, READING_COLUMN_MARGIN, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP,
};

/// Any width the type can hold: whatever `new` makes of any `f32`.
fn any_width() -> ReadingWidth {
    ReadingWidth::new(kani::any())
}

/// Any pane width but the two infinities, NaN included. The verifier's
/// model of the fused multiply-add in `resolve` reports an error for every
/// infinite addend, though the real operation has none there, so the
/// infinite panes are left to `tests.rs`.
fn any_pane() -> f32 {
    let available: f32 = kani::any();
    kani::assume(!available.is_infinite());
    available
}

fn any_scroll_action() -> KeyboardScrollAction {
    match kani::any::<u8>() % 4 {
        0 => KeyboardScrollAction::LineUp,
        1 => KeyboardScrollAction::LineDown,
        2 => KeyboardScrollAction::PageUp,
        _ => KeyboardScrollAction::PageDown,
    }
}

/// Every `f32` becomes a width in `[MIN, MAX]`; one already in range is
/// kept as it is, and NaN becomes the default.
#[kani::proof]
fn reading_width_new_clamps_every_float() {
    let px: f32 = kani::any();
    let width = ReadingWidth::new(px).get();

    assert!(width >= ReadingWidth::MIN && width <= ReadingWidth::MAX);
    if px >= ReadingWidth::MIN && px <= ReadingWidth::MAX {
        assert!(width == px);
    }
    if px.is_nan() {
        assert!(width == ReadingWidth::DEFAULT.get());
    }
    assert!(ReadingWidth::new(width).get() == width);

    kani::cover!(px.is_nan());
    kani::cover!(px < ReadingWidth::MIN);
    kani::cover!(px > ReadingWidth::MAX);
    kani::cover!(px > ReadingWidth::MIN && px < ReadingWidth::MAX);
}

/// A wider request never gives a narrower column.
#[kani::proof]
fn reading_width_new_is_monotonic() {
    let (narrow, wide): (f32, f32) = (kani::any(), kani::any());
    kani::assume(narrow <= wide);

    assert!(ReadingWidth::new(narrow).get() <= ReadingWidth::new(wide).get());

    kani::cover!(narrow < wide && ReadingWidth::new(narrow).get() < ReadingWidth::new(wide).get());
}

/// The column shown in a pane is a whole, finite, non-negative number of
/// pixels, never wider than the stored width, and it leaves the margin on
/// both sides of any pane wide enough to have margins. A pane narrower
/// than its two margins, or one whose width is NaN, shows no column.
#[kani::proof]
fn resolve_fits_the_pane() {
    let width = any_width();
    let available = any_pane();
    let shown = width.resolve(available);

    assert!(shown >= 0.0 && shown <= width.get());
    assert!(shown == shown.floor());
    if available >= 2.0 * READING_COLUMN_MARGIN {
        assert!(shown + 2.0 * READING_COLUMN_MARGIN <= available);
    } else {
        assert!(shown == 0.0);
    }

    kani::cover!(available.is_nan());
    kani::cover!(available < 0.0);
    kani::cover!(shown > 0.0 && shown < width.get());
    kani::cover!(shown == width.get());
}

/// A wider pane never shows a narrower column.
#[kani::proof]
fn resolve_is_monotonic_in_the_pane() {
    let width = any_width();
    let (small, large) = (any_pane(), any_pane());
    kani::assume(small <= large);

    assert!(width.resolve(small) <= width.resolve(large));

    kani::cover!(width.resolve(small) < width.resolve(large));
}

/// The column sits inside its pane with at least the margin on each side,
/// where `centered_column_left` puts it, in every pane from two margins
/// wide up to `PANE_BOUND` px.
#[kani::proof]
fn column_keeps_both_margins() {
    /// Above this the float spacing exceeds a pixel and "a whole pixel"
    /// stops meaning anything; no display is this wide.
    const PANE_BOUND: f32 = 8_388_608.0;

    let width = any_width();
    let pane: f32 = kani::any();
    kani::assume(pane >= 2.0 * READING_COLUMN_MARGIN && pane <= PANE_BOUND);

    let shown = width.resolve(pane);
    let inset = centered_column_left(0.0, pane, shown);

    assert!(inset >= READING_COLUMN_MARGIN);
    assert!(inset + shown + READING_COLUMN_MARGIN <= pane);

    kani::cover!(shown == width.get());
    kani::cover!(shown < width.get());
}

/// The column's left edge is never left of the pane's, and a column as
/// wide as the pane or wider starts at the pane's edge.
#[kani::proof]
fn centered_column_starts_inside_the_pane() {
    let (left, right, width): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(left.is_finite() && right.is_finite() && width.is_finite());

    let column_left = centered_column_left(left, right, width);

    assert!(column_left >= left);
    if right - left <= width {
        assert!(column_left == left);
    }

    kani::cover!(column_left > left);
    kani::cover!(column_left == left);
}

/// A step is a valid width, a step out never narrows the column shown and
/// a step in never widens it.
#[kani::proof]
fn stepped_moves_the_way_it_was_asked() {
    let width = any_width();
    let (delta, available): (f32, f32) = (kani::any(), any_pane());

    let shown = ReadingWidth::new(width.resolve(available)).get();
    let stepped = width.stepped(delta, available).get();

    assert!(stepped >= ReadingWidth::MIN && stepped <= ReadingWidth::MAX);
    if delta >= 0.0 {
        assert!(stepped >= shown);
    }
    if delta <= 0.0 {
        assert!(stepped <= shown);
    }

    kani::cover!(delta > 0.0 && stepped > shown);
    kani::cover!(delta < 0.0 && stepped < shown);
    kani::cover!(delta.is_nan());
}

/// One press of the widen shortcut and one of the narrow shortcut, in a
/// pane with room for the wider column, give back the column that was
/// shown.
#[kani::proof]
fn a_step_out_and_back_is_the_identity() {
    let width = any_width();
    let available = any_pane();
    let shown = width.resolve(available);
    // The column is at least the minimum, and the pane has room for one
    // step more.
    kani::assume(shown >= ReadingWidth::MIN);
    let wider = width.stepped(ReadingWidth::STEP, available);
    kani::assume(wider.resolve(available) == shown + ReadingWidth::STEP);

    assert!(wider.get() == shown + ReadingWidth::STEP);
    let back = wider.stepped(-ReadingWidth::STEP, available);
    assert!(back.get() == shown);
    assert!(back.resolve(available) == shown);

    kani::cover!(width.fits_window());
    kani::cover!(!width.fits_window());
}

/// Dragging an edge anywhere gives a valid width, and an edge on the
/// center line gives the narrowest column. The two coordinates are any
/// `f32` but the infinities, whose difference is NaN.
#[kani::proof]
fn from_edge_is_always_a_valid_width() {
    let (edge_x, center_x): (f32, f32) = (kani::any(), kani::any());
    kani::assume(!edge_x.is_infinite() && !center_x.is_infinite());
    let width = ReadingWidth::from_edge(edge_x, center_x).get();

    assert!(width >= ReadingWidth::MIN && width <= ReadingWidth::MAX);
    if edge_x == center_x && edge_x.is_finite() {
        assert!(width == ReadingWidth::MIN);
    }

    kani::cover!(width == ReadingWidth::MAX);
    kani::cover!(width > ReadingWidth::MIN && width < ReadingWidth::MAX);
    kani::cover!(edge_x.is_nan());
}

/// Every saved session comes back with a valid width: a saved width wins,
/// the old "fill the window" flag becomes fit-window, and anything else is
/// the default.
#[kani::proof]
fn restored_maps_every_saved_session_to_a_valid_width() {
    let saved: Option<f32> = kani::any();
    let legacy_full_width: Option<bool> = kani::any();
    let width = ReadingWidth::restored(saved, legacy_full_width);

    assert!(width.get() >= ReadingWidth::MIN && width.get() <= ReadingWidth::MAX);
    match (saved, legacy_full_width) {
        (Some(px), _) => {
            assert!(width == ReadingWidth::new(px));
        }
        (None, Some(true)) => {
            assert!(width.fits_window());
        }
        (None, Some(false) | None) => {
            assert!(width == ReadingWidth::DEFAULT);
        }
    }

    kani::cover!(saved.is_some_and(f32::is_nan));
    kani::cover!(saved.is_none() && legacy_full_width == Some(true));
    kani::cover!(saved.is_some() && legacy_full_width == Some(true));
}

/// The width the chrome measures by is never fit-window, and is the
/// reader's own width whenever that is not fit-window.
#[kani::proof]
fn fixed_is_never_fit_window() {
    let width = any_width();
    let fixed = width.fixed();

    assert!(!fixed.fits_window());
    assert!(width.fits_window() == (width.get() == ReadingWidth::MAX));
    if width.fits_window() {
        assert!(fixed == ReadingWidth::DEFAULT);
    } else {
        assert!(fixed == width);
    }

    kani::cover!(width.fits_window());
    kani::cover!(!width.fits_window());
}

/// From any finite offset and heights, a keyboard scroll step never
/// panics and lands inside the document: between the top and the furthest
/// the view can scroll. The offset may start outside the document.
#[kani::proof]
fn keyboard_scroll_stays_inside_the_document() {
    let (current, viewport, content): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(current.is_finite() && viewport.is_finite() && content.is_finite());
    let action = any_scroll_action();

    let target = keyboard_scroll_target(current, viewport, content, action);

    let max_scroll = (content - viewport).max(0.0);
    assert!(target >= 0.0 && target <= max_scroll);

    kani::cover!(current < 0.0);
    kani::cover!(current > max_scroll);
    kani::cover!(target > 0.0 && target < max_scroll);
}

/// From a position inside the document, a step down never moves the view
/// up and a step up never moves it down.
#[kani::proof]
fn keyboard_scroll_moves_the_way_the_key_points() {
    let (current, viewport, content): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(viewport.is_finite() && viewport >= 0.0 && content.is_finite());
    kani::assume(current >= 0.0 && current <= (content - viewport).max(0.0));
    let action = any_scroll_action();

    let target = keyboard_scroll_target(current, viewport, content, action);

    match action {
        KeyboardScrollAction::LineDown | KeyboardScrollAction::PageDown => {
            assert!(target >= current);
        }
        KeyboardScrollAction::LineUp | KeyboardScrollAction::PageUp => {
            assert!(target <= current);
        }
    }

    kani::cover!(target > current);
    kani::cover!(target < current);
}

/// Every `f32` has a whole-pixel count: zero for NaN and anything
/// negative, and otherwise the width with its fraction dropped.
#[kani::proof]
fn whole_pixels_is_total() {
    /// 2^63: below it every whole `f32` fits a `usize` on any 64-bit
    /// target.
    const EXACT_BELOW: f32 = 9_223_372_036_854_775_808.0;

    let width: f32 = kani::any();
    let pixels = whole_pixels(width);

    if width.is_nan() || width <= 0.0 {
        assert!(pixels == 0);
    } else if width < EXACT_BELOW {
        // The count came from an `f32`, so it converts back exactly.
        assert!(pixels as f32 == width.floor());
    }

    kani::cover!(width.is_nan());
    kani::cover!(width.is_infinite() && width > 0.0);
    kani::cover!(pixels > 0 && width < EXACT_BELOW);
}

/// The fraction of a document above a line never divides by zero and is
/// never NaN or negative, for every pair of `usize`; an empty document has
/// nothing above any line.
#[kani::proof]
fn line_fraction_never_divides_by_zero() {
    let (line, lines): (usize, usize) = (kani::any(), kani::any());
    let fraction = line_fraction(line, lines);

    assert!(fraction >= 0.0);
    if lines == 0 {
        assert!(fraction == 0.0);
    }

    kani::cover!(lines == 0 && line > 0);
    kani::cover!(line > lines && lines > 0);
}

/// For a line inside a document of up to `MAX_LINES` lines, the fraction
/// above it is between 0 and 1.
#[kani::proof]
fn line_fraction_is_a_fraction() {
    /// 2^24: every line count up to here is exact as an `f32`, which is
    /// what `line_fraction` itself relies on.
    const MAX_LINES: usize = 16_777_216;

    let (line, lines): (usize, usize) = (kani::any(), kani::any());
    kani::assume(line <= lines && lines <= MAX_LINES);

    let fraction = line_fraction(line, lines);

    assert!((0.0..=1.0).contains(&fraction));

    kani::cover!(fraction > 0.0 && fraction < 1.0);
    kani::cover!(fraction == 1.0);
    kani::cover!(lines == MAX_LINES);
}

/// Zooming from any zoom that is a number lands inside the zoom range,
/// whatever the factor; a factor that is not a positive finite number
/// changes nothing but the clamp.
#[kani::proof]
fn zoomed_stays_in_range() {
    let (current, factor): (f32, f32) = (kani::any(), kani::any());
    kani::assume(!current.is_nan());

    let zoom = zoomed(current, factor);

    assert!(zoom >= ZOOM_MIN && zoom <= ZOOM_MAX);
    if !factor.is_finite() || factor <= 0.0 {
        assert!(zoom == current.clamp(ZOOM_MIN, ZOOM_MAX));
    }

    kani::cover!(factor.is_nan());
    kani::cover!(factor > 1.0 && zoom > current);
    kani::cover!(zoom == ZOOM_MAX && current < ZOOM_MAX);
}

/// A step in never zooms out and a step out never zooms in.
#[kani::proof]
fn zoomed_moves_the_way_it_was_asked() {
    let current: f32 = kani::any();
    kani::assume(current >= ZOOM_MIN && current <= ZOOM_MAX);

    assert!(zoomed(current, ZOOM_STEP) >= current);
    assert!(zoomed(current, 1.0 / ZOOM_STEP) <= current);

    kani::cover!(zoomed(current, ZOOM_STEP) > current);
    kani::cover!(zoomed(current, 1.0 / ZOOM_STEP) < current);
}

/// A zoom step in and a zoom step out, where neither runs into the end of
/// the range, come back to the zoom they started from to within a
/// millionth.
#[kani::proof]
fn a_zoom_step_in_and_out_comes_back() {
    const TOLERANCE: f32 = 1e-6;

    let current: f32 = kani::any();
    kani::assume(current >= ZOOM_MIN && current * ZOOM_STEP <= ZOOM_MAX);

    let back = zoomed(zoomed(current, ZOOM_STEP), 1.0 / ZOOM_STEP);

    assert!((back - current).abs() <= TOLERANCE);

    kani::cover!(back != current);
    kani::cover!(back == current);
}
