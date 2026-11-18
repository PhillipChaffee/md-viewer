//! The find session: which document the matches are of, and where a match
//! scrolls the document to.

use crate::app::action::AppAction;
use crate::app::tests::Harness;
use crate::test_support::assert_close;

/// Ten lines with `needle` on the fifth and the tenth.
const TWO_NEEDLES: &str = "one\ntwo\nthree\nfour\nneedle\nsix\nseven\neight\nnine\nneedle\n";

/// A document measured `content_height` tall in a `viewport_height` tall
/// pane, with the find bar open on `needle`.
fn searching_measured(content_height: f32, viewport_height: f32) -> Harness {
    let mut harness = Harness::with_docs(&[("a.md", TWO_NEEDLES)]);
    harness.app.tabs[0].last_content_height = content_height;
    harness.app.tabs[0].last_viewport_height = viewport_height;
    harness.app.open_search();
    harness.app.search.query = "needle".to_owned();
    harness.app.maybe_rebuild_search();
    harness
}

/// Where the document is to scroll to, as the pane will be told once.
fn pending_scroll(harness: &mut Harness) -> Option<f32> {
    harness.app.tabs[0].take_pending_scroll()
}

#[test]
fn a_new_query_scrolls_its_first_match_a_third_of_the_way_down_the_pane() {
    let mut harness = searching_measured(1000.0, 400.0);

    // Line 5 of 10 sits at 500; 35% of the 400 pane stays above it.
    assert_close(pending_scroll(&mut harness).unwrap(), 360.0);
    assert!(harness.app.tabs[0].take_corrections().search);
}

#[test]
fn stepping_to_the_next_match_scrolls_to_it() {
    let mut harness = searching_measured(1000.0, 400.0);

    harness.app.jump_match(1);

    // Line 10 of 10 sits at 1000.
    assert_close(pending_scroll(&mut harness).unwrap(), 860.0);
}

#[test]
fn before_the_pane_is_measured_a_fixed_margin_stays_above_the_match() {
    let mut harness = searching_measured(1000.0, 0.0);

    assert_close(pending_scroll(&mut harness).unwrap(), 400.0);
}

#[test]
fn a_match_near_the_top_scrolls_to_the_top_not_above_it() {
    let mut harness = searching_measured(100.0, 400.0);

    // Line 5 of 10 sits at 50, less than the margin kept above a match.
    assert_close(pending_scroll(&mut harness).unwrap(), 0.0);
}

#[test]
fn a_document_not_yet_laid_out_is_left_where_it_is() {
    let mut harness = searching_measured(0.0, 400.0);

    assert_eq!(pending_scroll(&mut harness), None);
    assert_eq!(harness.app.tabs[0].search_matches.len(), 2);
}

/// The matches the find bar holds for the tab in front, once it has caught
/// up with what the tab shows.
fn matches_in_front(harness: &mut Harness) -> usize {
    harness.app.maybe_rebuild_search();
    harness.app.tabs[harness.app.active_tab]
        .search_matches
        .len()
}

#[test]
fn the_matches_follow_a_tab_to_the_document_it_moves_to() {
    let mut harness = Harness::with_docs(&[("a.md", "needle\n"), ("b.md", "needle needle\n")]);
    harness.app.open_search();
    harness.app.search.query = "needle".to_owned();
    assert_eq!(matches_in_front(&mut harness), 1);
    harness.app.search.active_match_index = 1;

    harness.app.navigate_active_tab_to(&harness.path("b.md"));
    assert_eq!(matches_in_front(&mut harness), 2);
    assert_eq!(harness.app.search.active_match_index, 0);

    harness.apply(AppAction::NavigateBack);
    assert_eq!(matches_in_front(&mut harness), 1);
}

#[test]
fn closing_a_tab_counts_the_document_that_takes_its_place() {
    let mut harness = Harness::with_docs(&[
        ("a.md", "# A\n"),
        ("b.md", "# B\n"),
        ("c.md", "needle needle needle\n"),
    ]);
    harness.app.open_in_new_tab(harness.path("b.md"));
    harness.app.open_in_new_tab(harness.path("c.md"));
    harness.app.focus_tab(1);
    harness.app.open_search();
    harness.app.search.query = "needle".to_owned();
    assert_eq!(matches_in_front(&mut harness), 0);

    harness.app.close_active_tab();

    assert_eq!(harness.active_path(), harness.path("c.md"));
    assert_eq!(matches_in_front(&mut harness), 3);
}

#[test]
fn a_document_opened_under_a_waiting_query_is_searched() {
    let mut harness = Harness::with_docs(&[("a.md", "needle needle\n")]);
    harness.app.close_active_tab();
    harness.app.open_search();
    harness.app.search.query = "needle".to_owned();
    harness.app.maybe_rebuild_search();

    harness.app.open_in_new_tab(harness.path("a.md"));

    assert_eq!(matches_in_front(&mut harness), 2);
}
