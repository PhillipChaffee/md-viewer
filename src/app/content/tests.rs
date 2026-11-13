use eframe::egui::accesskit::Role;
use eframe::egui::{Event, Key, Modifiers, MouseWheelUnit};
use egui_kittest::kittest::Queryable as _;

use self::label_harness::LabelHarness;
use super::*;
use crate::test_support::assert_close;

pub(crate) mod label_harness;

/// One document that links to `b.md` and to a file that does not exist.
fn linked_docs() -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[
        ("a.md", "Go to [the other one](b.md) or [nowhere](gone.md)."),
        ("b.md", "The other document."),
    ]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

/// A document far taller than the window, with `needle` near its end. One
/// paragraph is a single source line that wraps over many, so a position
/// estimated from line counts is well off.
fn long_doc() -> LabelHarness {
    let filler = "A paragraph of text.\n\n".repeat(60);
    let wrapped = "word ".repeat(600);
    let content = format!("{wrapped}\n\n{filler}needle\n\n{filler}");
    let mut harness = LabelHarness::with_docs(&[("a.md", &content)]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

fn open_paths(harness: &LabelHarness) -> Vec<PathBuf> {
    let tabs = &harness.state().tabs;
    tabs.iter().map(|tab| tab.path.clone()).collect()
}

/// A point in the middle of the document pane.
const OVER_DOCUMENT: egui::Pos2 = egui::pos2(600.0, 500.0);
/// A point in the menu bar, above the document pane.
const OVER_MENU_BAR: egui::Pos2 = egui::pos2(600.0, 10.0);

/// Turns the mouse wheel by `delta_y` points with the pointer at `pointer`.
fn wheel_at(harness: &mut LabelHarness, pointer: egui::Pos2, delta_y: f32) {
    harness.event(Event::PointerMoved(pointer));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: egui::vec2(0.0, delta_y),
        modifiers: Modifiers::NONE,
    });
    harness.frames(4);
}

#[test]
fn clicking_a_link_shows_its_document_in_the_same_tab() {
    let mut harness = linked_docs();

    harness.click("the other one");

    assert_eq!(open_paths(&harness), [harness.path("b.md")]);
    assert_eq!(harness.state().error_message, None);
}

#[test]
fn ctrl_clicking_a_link_opens_its_document_in_a_new_tab() {
    let mut harness = linked_docs();

    harness
        .get_by_label("the other one")
        .click_modifiers(Modifiers::CTRL);
    harness.frames(2);

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("b.md")]
    );
}

#[test]
fn a_link_to_a_missing_document_is_reported_and_changes_nothing() {
    let mut harness = linked_docs();

    harness.click("nowhere");

    assert_eq!(open_paths(&harness), [harness.path("a.md")]);
    let message = harness.state().error_message.clone().unwrap();
    assert!(message.contains("gone.md"), "{message}");
}

#[test]
fn edit_mode_shows_the_source_and_takes_typing() {
    let mut harness = LabelHarness::with_docs(&[("a.md", "# Title")]);
    harness.click("Edit");

    let editor = harness.get_by_role(Role::MultilineTextInput);
    assert_eq!(editor.value().as_deref(), Some("# Title"));
    editor.click();
    harness.frames(2);
    harness.get_by_role(Role::MultilineTextInput).type_text("!");
    harness.frames(2);

    assert_eq!(harness.state().tabs[0].content, "# Title!");
}

#[test]
fn switching_to_edit_mode_keeps_the_place_in_a_long_document() {
    let mut harness = long_doc();
    harness.key_press(Key::PageDown);
    harness.key_press(Key::PageDown);
    harness.frames(2);
    let tab = &harness.state().tabs[0];
    let place = tab.scroll_offset / tab.last_content_height;
    assert!(place > 0.05, "the preview did not scroll: {place}");

    harness.click("Edit");
    harness.frames(2);

    let tab = &harness.state().tabs[0];
    let place_in_editor = tab.scroll_offset / tab.last_content_height;
    assert!(
        (place_in_editor - place).abs() < 0.02,
        "preview was at {place}, the editor opened at {place_in_editor}"
    );
}

#[test]
fn jumping_to_a_search_match_brings_it_into_view() {
    let mut harness = long_doc();
    harness.key_press_modifiers(Modifiers::CTRL, Key::F);
    harness.frames(1);

    harness.state_mut().search.query = "needle".to_owned();
    harness.frames(4);

    let tab = &harness.state().tabs[0];
    let match_y = tab.cache.active_search_y().unwrap();
    let visible = tab.scroll_offset..tab.scroll_offset + tab.last_viewport_height;
    assert!(
        visible.contains(&match_y),
        "the match at {match_y} is outside {visible:?}"
    );
}

#[test]
fn the_wheel_scrolls_the_document_under_the_pointer() {
    let mut harness = long_doc();

    wheel_at(&mut harness, OVER_DOCUMENT, -120.0);
    let after_scrolling_down = harness.state().tabs[0].scroll_offset;
    assert!(after_scrolling_down >= 120.0, "{after_scrolling_down}");

    wheel_at(&mut harness, OVER_DOCUMENT, 60.0);
    assert!(harness.state().tabs[0].scroll_offset < after_scrolling_down);
}

#[test]
fn the_wheel_leaves_the_document_alone_while_the_pointer_is_elsewhere() {
    let mut harness = long_doc();

    wheel_at(&mut harness, OVER_MENU_BAR, -120.0);

    assert_close(harness.state().tabs[0].scroll_offset, 0.0);
}

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
