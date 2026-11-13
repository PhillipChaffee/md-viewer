use super::*;

#[test]
fn a_search_match_inside_the_viewport_needs_no_correction() {
    // The viewport shows 1000..1600; the match must sit 20px inside either edge.
    assert_eq!(search_correction(1300.0, 1000.0, 600.0), None);
    assert_eq!(search_correction(1020.0, 1000.0, 600.0), None);
    assert_eq!(search_correction(1580.0, 1000.0, 600.0), None);
}

#[test]
fn a_search_match_outside_the_viewport_lands_a_third_of_the_way_down() {
    // 35% of a 1000px viewport is 350px above the match.
    assert_eq!(search_correction(3000.0, 500.0, 1000.0), Some(2650.0));
    // Inside the viewport but within the 20px edge band still counts as hidden.
    assert_eq!(search_correction(1510.0, 1500.0, 1000.0), Some(1160.0));
    assert_eq!(search_correction(2495.0, 1500.0, 1000.0), Some(2145.0));
}

#[test]
fn a_search_correction_never_scrolls_above_the_document_top() {
    assert_eq!(search_correction(100.0, 800.0, 1000.0), Some(0.0));
}

#[test]
fn a_search_correction_needs_a_laid_out_viewport() {
    assert_eq!(search_correction(3000.0, 500.0, 0.0), None);
}

#[test]
fn a_search_correction_is_skipped_when_the_view_is_already_at_the_target() {
    // A 40px viewport has no comfortable band, so the match counts as hidden,
    // but its target (114 - 35% of 40) is where the view already is.
    assert_eq!(search_correction(114.0, 100.0, 40.0), None);
}

#[test]
fn a_clicked_header_is_placed_below_the_viewport_top() {
    assert_eq!(header_correction(500.0, 0.0), Some(450.0));
    assert_eq!(header_correction(500.0, 2000.0), Some(450.0));
    // A header near the top cannot scroll above the document.
    assert_eq!(header_correction(30.0, 400.0), Some(0.0));
}

#[test]
fn a_header_correction_is_skipped_within_two_pixels_of_the_target() {
    assert_eq!(header_correction(500.0, 449.0), None);
    assert_eq!(header_correction(500.0, 452.0), None);
    assert_eq!(header_correction(30.0, 0.0), None);
}

#[test]
fn a_wheel_step_moves_the_view_against_the_wheel_delta() {
    assert_eq!(wheel_scroll_target(100.0, -30.0, 1000.0), Some(130.0));
    assert_eq!(wheel_scroll_target(100.0, 30.0, 1000.0), Some(70.0));
}

#[test]
fn a_wheel_step_that_would_reach_either_end_is_left_to_the_scroll_area() {
    assert_eq!(wheel_scroll_target(20.0, 30.0, 1000.0), None);
    assert_eq!(wheel_scroll_target(990.0, -30.0, 1000.0), None);
    // A document that fits the viewport has nowhere to scroll.
    assert_eq!(wheel_scroll_target(0.0, -30.0, 0.0), None);
}

#[test]
fn a_wheel_step_smaller_than_half_a_pixel_is_ignored() {
    assert_eq!(wheel_scroll_target(100.0, 0.25, 1000.0), None);
}
