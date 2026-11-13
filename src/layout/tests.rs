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
fn capped_content_width_uses_optimal_width() {
    assert_eq!(content_default_width(false), Some(600));
}

#[test]
fn full_width_content_uses_available_width() {
    assert_eq!(content_default_width(true), None);
}

#[test]
fn capped_content_applies_the_same_limit_to_tables() {
    assert_eq!(content_width_limit(false, 900.0), 600);
    assert_eq!(content_width_limit(false, 480.0), 480);
}

#[test]
fn full_width_tables_use_the_available_content_width() {
    assert_eq!(content_width_limit(true, 900.0), 900);
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
