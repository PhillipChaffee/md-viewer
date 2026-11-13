use super::*;
use crate::test_support::assert_close;

#[test]
fn document_zoom_steps_are_multiplicative_and_clamped() {
    assert_close(zoomed(1.0, ZOOM_STEP), 1.25);
    assert!((zoomed(1.25, 1.0 / ZOOM_STEP) - 1.0).abs() < 1e-6);

    // Same bounds the additive path clamped to before.
    assert_close(zoomed(2.9, ZOOM_STEP), ZOOM_MAX);
    assert_close(zoomed(0.6, 1.0 / ZOOM_STEP), ZOOM_MIN);

    // Neutral and degenerate factors are no-ops (egui reports 1.0 on
    // frames without a zoom gesture; guards against divide-by-style bugs).
    assert_close(zoomed(1.7, 1.0), 1.7);
    assert_close(zoomed(1.7, 0.0), 1.7);
    assert_close(zoomed(1.7, -2.0), 1.7);
}

#[test]
fn zoom_bounds_match_the_persisted_value_clamp() {
    // PersistedState load clamps with the same constants.
    let persisted_level = 7.5_f32.clamp(ZOOM_MIN, ZOOM_MAX);
    assert_close(persisted_level, ZOOM_MAX);
}

#[test]
fn a_reading_width_stays_inside_its_range() {
    assert_close(ReadingWidth::new(500.0).get(), 500.0);
    assert_close(ReadingWidth::new(10.0).get(), ReadingWidth::MIN);
    assert_close(ReadingWidth::new(9_000.0).get(), ReadingWidth::MAX);
    assert_close(ReadingWidth::new(f32::INFINITY).get(), ReadingWidth::MAX);
    assert_close(
        ReadingWidth::new(f32::NEG_INFINITY).get(),
        ReadingWidth::MIN,
    );
    assert_eq!(ReadingWidth::new(f32::NAN), ReadingWidth::DEFAULT);
    assert_close(ReadingWidth::DEFAULT.get(), 680.0);
}

#[test]
fn the_column_keeps_a_margin_on_each_side_of_the_pane() {
    let margins = 2.0 * READING_COLUMN_MARGIN;
    // Room to spare: the column is as wide as asked.
    assert_close(ReadingWidth::DEFAULT.resolve(1_200.0), 680.0);
    // A narrow pane narrows the column, below the minimum if need be.
    assert_close(ReadingWidth::DEFAULT.resolve(500.0), 500.0 - margins);
    assert_close(ReadingWidth::DEFAULT.resolve(200.0), 200.0 - margins);
    assert_close(ReadingWidth::DEFAULT.resolve(10.0), 0.0);
    // Fit-window is the pane less its margins, on a whole pixel.
    assert_close(
        ReadingWidth::FIT_WINDOW.resolve(1_000.5),
        (1_000.5 - margins).floor(),
    );
}

#[test]
fn only_the_widest_width_fits_the_window() {
    assert!(ReadingWidth::FIT_WINDOW.fits_window());
    assert!(ReadingWidth::new(f32::INFINITY).fits_window());
    assert!(!ReadingWidth::new(3_999.0).fits_window());
    assert!(!ReadingWidth::DEFAULT.fits_window());
}

#[test]
fn a_width_that_fits_the_window_is_fixed_at_the_default() {
    assert_eq!(ReadingWidth::FIT_WINDOW.fixed(), ReadingWidth::DEFAULT);
    assert_eq!(ReadingWidth::new(512.0).fixed(), ReadingWidth::new(512.0));
    assert_eq!(ReadingWidth::DEFAULT.fixed(), ReadingWidth::DEFAULT);
}

#[test]
fn a_step_starts_from_the_column_that_is_shown() {
    let step = ReadingWidth::STEP;
    assert_close(ReadingWidth::DEFAULT.stepped(step, 1_200.0).get(), 720.0);
    assert_close(ReadingWidth::DEFAULT.stepped(-step, 1_200.0).get(), 640.0);
    // Out of fit-window, the first narrowing step is already visible.
    let shown = ReadingWidth::FIT_WINDOW.resolve(1_000.0);
    assert_close(
        ReadingWidth::FIT_WINDOW.stepped(-step, 1_000.0).get(),
        shown - step,
    );
    // Steps stop at the ends of the range.
    assert_close(
        ReadingWidth::new(ReadingWidth::MIN)
            .stepped(-step, 1_200.0)
            .get(),
        ReadingWidth::MIN,
    );
}

#[test]
fn dragging_an_edge_moves_both_edges_about_the_center() {
    // The right edge 300 px right of center: a 600 px column.
    assert_close(ReadingWidth::from_edge(900.0, 600.0).get(), 600.0);
    // The left edge the same distance the other way gives the same.
    assert_close(ReadingWidth::from_edge(300.0, 600.0).get(), 600.0);
    // 10 px further out on one side is 20 px wider.
    assert_close(ReadingWidth::from_edge(910.0, 600.0).get(), 620.0);
    // An edge dragged past the center stops at the narrowest column.
    assert_close(
        ReadingWidth::from_edge(600.0, 600.0).get(),
        ReadingWidth::MIN,
    );
}

#[test]
fn a_session_from_before_the_resizable_column_keeps_its_width_choice() {
    assert_close(ReadingWidth::restored(Some(512.0), None).get(), 512.0);
    // A saved width wins over the old flag, whatever the flag says.
    assert_close(ReadingWidth::restored(Some(512.0), Some(true)).get(), 512.0);
    assert_eq!(
        ReadingWidth::restored(None, Some(true)),
        ReadingWidth::FIT_WINDOW
    );
    assert_eq!(
        ReadingWidth::restored(None, Some(false)),
        ReadingWidth::DEFAULT
    );
    assert_eq!(ReadingWidth::restored(None, None), ReadingWidth::DEFAULT);
    assert_eq!(
        ReadingWidth::restored(Some(f32::NAN), None),
        ReadingWidth::DEFAULT
    );
}

#[test]
fn a_column_is_centered_on_a_whole_pixel() {
    assert_close(centered_column_left(100.0, 1_100.0, 600.0), 300.0);
    assert_close(centered_column_left(100.0, 1_101.0, 600.0), 300.0);
    // A column as wide as the pane, or wider, starts at the pane's edge.
    assert_close(centered_column_left(100.0, 700.0, 600.0), 100.0);
    assert_close(centered_column_left(100.0, 500.0, 600.0), 100.0);
}

#[test]
fn line_spacing_steps_grow_and_default_to_comfortable() {
    let heights = LineSpacing::ALL.map(LineSpacing::line_height);
    assert_close(heights[0], 1.45);
    assert_close(heights[1], 1.6);
    assert_close(heights[2], 1.8);
    assert_eq!(LineSpacing::default(), LineSpacing::Comfortable);
    assert_eq!(
        LineSpacing::ALL.map(LineSpacing::label),
        ["Compact", "Comfortable", "Airy"]
    );
}

#[test]
fn the_launch_window_holds_the_default_column_with_its_margins() {
    assert_close(
        ReadingWidth::DEFAULT.resolve(CONTENT_OPTIMAL_WIDTH),
        ReadingWidth::DEFAULT.get(),
    );
}

#[test]
fn whole_pixels_drops_the_fraction_and_floors_at_zero() {
    assert_eq!(whole_pixels(599.9), 599);
    assert_eq!(whole_pixels(0.0), 0);
    assert_eq!(whole_pixels(-12.5), 0);
    assert_eq!(whole_pixels(f32::NAN), 0);
}

#[test]
fn line_fraction_is_the_share_of_lines_above() {
    assert_close(line_fraction(0, 200), 0.0);
    assert_close(line_fraction(50, 200), 0.25);
    assert_close(line_fraction(200, 200), 1.0);
    assert_close(line_fraction(3, 0), 0.0);
}

#[test]
fn keyboard_scroll_target_moves_by_line_step() {
    assert_close(
        keyboard_scroll_target(100.0, 500.0, 2_000.0, KeyboardScrollAction::LineDown),
        148.0,
    );
    assert_close(
        keyboard_scroll_target(100.0, 500.0, 2_000.0, KeyboardScrollAction::LineUp),
        52.0,
    );
}

#[test]
fn keyboard_scroll_target_moves_by_page_step() {
    assert_close(
        keyboard_scroll_target(1_000.0, 500.0, 3_000.0, KeyboardScrollAction::PageDown),
        1_450.0,
    );
    assert_close(
        keyboard_scroll_target(1_000.0, 500.0, 3_000.0, KeyboardScrollAction::PageUp),
        550.0,
    );
}

#[test]
fn keyboard_scroll_target_clamps_to_document_bounds() {
    assert_close(
        keyboard_scroll_target(10.0, 500.0, 2_000.0, KeyboardScrollAction::PageUp),
        0.0,
    );
    assert_close(
        keyboard_scroll_target(1_490.0, 500.0, 2_000.0, KeyboardScrollAction::PageDown),
        1_500.0,
    );
}

#[test]
fn keyboard_scroll_target_handles_short_documents() {
    assert_close(
        keyboard_scroll_target(25.0, 500.0, 300.0, KeyboardScrollAction::LineDown),
        0.0,
    );
}
