//! The find bar: its counter and buttons, and where a match scrolls the
//! document to.

use eframe::egui::accesskit;
use egui_kittest::kittest::Queryable as _;

use crate::app::tests::Harness;
use crate::app::ui_harness::UiHarness;
use crate::test_support::assert_close;

/// Ten lines with `needle` on the fifth and the tenth.
const TWO_NEEDLES: &str = "one\ntwo\nthree\nfour\nneedle\nsix\nseven\neight\nnine\nneedle\n";

/// Open the find bar and type `query` into it.
fn find(harness: &mut UiHarness, query: &str) {
    harness.pick("File", "Find");
    harness
        .ui
        .get_by_role(accesskit::Role::TextInput)
        .type_text(query);
    harness.settle();
}

#[test]
fn the_counter_and_arrow_buttons_walk_the_matches_and_wrap() {
    let mut harness = UiHarness::with_docs(&[("a.md", "needle\n\nneedle\n\nneedle\n")]);
    find(&mut harness, "needle");
    assert!(harness.shows("1 / 3"));
    assert_eq!(harness.enabled("↓"), [true]);

    harness.click_exact("↓");
    assert!(harness.shows("2 / 3"));
    harness.click_exact("↑");
    harness.click_exact("↑");
    assert!(harness.shows("3 / 3"), "stepping back from the first wraps");
    harness.click_exact("↓");
    assert!(harness.shows("1 / 3"), "stepping on from the last wraps");
}

#[test]
fn a_query_without_matches_says_so_and_disables_the_arrows() {
    let mut harness = UiHarness::with_docs(&[("a.md", "needle\n")]);
    find(&mut harness, "haystack");

    assert!(harness.shows("0 matches"));
    assert_eq!(harness.enabled("↑"), [false]);
    assert_eq!(harness.enabled("↓"), [false]);
}

#[test]
fn searching_with_no_document_open_finds_nothing() {
    let mut harness = UiHarness::with_docs(&[("a.md", "needle\n")]);
    harness.pick("File", "Close Tab");

    find(&mut harness, "needle");

    assert!(harness.shows("0 matches"));
}

#[test]
fn the_close_button_closes_the_bar_and_drops_the_matches() {
    let mut harness = UiHarness::with_docs(&[("a.md", "needle\n")]);
    find(&mut harness, "needle");
    assert_eq!(harness.app().tabs[0].search_matches.len(), 1);

    harness.click_exact("✕");

    assert!(!harness.app().search.is_open);
    assert!(harness.app().tabs[0].search_matches.is_empty());
    assert!(!harness.shows("1 / 1"));
}

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

fn pending_scroll(harness: &Harness) -> Option<f32> {
    harness.app.tabs[0].pending_scroll_offset
}

#[test]
fn a_new_query_scrolls_its_first_match_a_third_of_the_way_down_the_pane() {
    let harness = searching_measured(1000.0, 400.0);

    // Line 5 of 10 sits at 500; 35% of the 400 pane stays above it.
    assert_close(pending_scroll(&harness).unwrap(), 360.0);
    assert!(harness.app.tabs[0].correct_active_search_pending);
}

#[test]
fn stepping_to_the_next_match_scrolls_to_it() {
    let mut harness = searching_measured(1000.0, 400.0);

    harness.app.jump_match(1);

    // Line 10 of 10 sits at 1000.
    assert_close(pending_scroll(&harness).unwrap(), 860.0);
}

#[test]
fn before_the_pane_is_measured_a_fixed_margin_stays_above_the_match() {
    let harness = searching_measured(1000.0, 0.0);

    assert_close(pending_scroll(&harness).unwrap(), 400.0);
}

#[test]
fn a_match_near_the_top_scrolls_to_the_top_not_above_it() {
    let harness = searching_measured(100.0, 400.0);

    // Line 5 of 10 sits at 50, less than the margin kept above a match.
    assert_close(pending_scroll(&harness).unwrap(), 0.0);
}

#[test]
fn a_document_not_yet_laid_out_is_left_where_it_is() {
    let harness = searching_measured(0.0, 400.0);

    assert_eq!(pending_scroll(&harness), None);
    assert_eq!(harness.app.tabs[0].search_matches.len(), 2);
}
