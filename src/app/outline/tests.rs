use eframe::egui::accesskit::Role;
use eframe::egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::content::tests::label_harness::LabelHarness;

/// A document long enough to scroll: `Guide` holds `Alpha` and `Beta`, and
/// `Alpha` holds `Alpha One` and `Alpha Two`. One paragraph is a single
/// source line that wraps over many, so line counts misjudge positions.
fn guide() -> String {
    let filler = "A paragraph of text.\n\n".repeat(40);
    let wrapped = "word ".repeat(600);
    format!(
        "# Guide\n\n## Alpha\n\n{wrapped}\n\n{filler}### Alpha One\n\n{filler}\
         ### Alpha Two\n\n{filler}## Beta\n\n{filler}{filler}"
    )
}

/// The app on `content`, with the explorer hidden.
fn outline_of(content: &str) -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[("guide.md", content)]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

/// The outline's entry for the heading `title`. The document pane shows
/// the same text, but as a label and not as a button.
fn entry<'a>(harness: &'a LabelHarness, title: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, title)
}

fn has_entry(harness: &LabelHarness, title: &str) -> bool {
    harness
        .query_by_role_and_label(Role::Button, title)
        .is_some()
}

fn primary_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// How far the document's scroll position is from showing the heading
/// `title` just below the top of the pane.
fn distance_from_heading(harness: &LabelHarness, title: &str) -> f32 {
    let tab = &harness.state().tabs[0];
    let headers = &tab.outline_headers;
    let header = headers.iter().find(|header| header.title == title);
    let key = header_position_key(header.unwrap().source_start);
    let heading_y = tab.cache.get_header_position(&key).unwrap();
    (tab.scroll_offset - (heading_y - HEADER_SCROLL_MARGIN)).abs()
}

fn outline_is_on_screen(harness: &LabelHarness) -> bool {
    egui::containers::panel::PanelState::load(&harness.ctx, egui::Id::new("outline")).is_some()
}

#[test]
fn the_outline_lists_every_heading_of_the_document() {
    let harness = outline_of(&guide());

    for title in ["Guide", "Alpha", "Alpha One", "Alpha Two", "Beta"] {
        assert!(has_entry(&harness, title), "{title} is missing");
    }
}

#[test]
fn clicking_a_heading_scrolls_the_document_to_it() {
    let mut harness = outline_of(&guide());
    assert!(distance_from_heading(&harness, "Beta") > 1000.0);

    entry(&harness, "Beta").click();
    harness.frames(4);

    let distance = distance_from_heading(&harness, "Beta");
    assert!(distance <= 2.0, "stopped {distance}px from the heading");
}

#[test]
fn a_heading_clicked_before_it_was_measured_still_lands_on_it() {
    let mut harness = outline_of(&guide());
    let at = entry(&harness, "Beta").rect().center();
    harness.event(Event::PointerMoved(at));
    harness.event(primary_button(at, true));
    harness.frames(1);
    // A reload forgets where every heading was painted.
    harness.state_mut().tabs[0].reload().unwrap();

    harness.event(primary_button(at, false));
    harness.frames(4);

    let distance = distance_from_heading(&harness, "Beta");
    assert!(distance <= 2.0, "stopped {distance}px from the heading");
}

#[test]
fn a_click_while_another_button_is_held_does_not_scroll() {
    let mut harness = outline_of(&guide());
    let at = entry(&harness, "Beta").rect().center();
    harness.event(Event::PointerMoved(at));
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });

    harness.event(primary_button(at, true));
    harness.event(primary_button(at, false));
    harness.frames(4);

    assert!(distance_from_heading(&harness, "Beta") > 1000.0);
}

#[test]
fn a_heading_folds_and_unfolds_its_subheadings() {
    let mut harness = outline_of(&guide());

    harness.click("Collapse Alpha");
    assert!(!has_entry(&harness, "Alpha One"));
    assert!(!has_entry(&harness, "Alpha Two"));
    assert!(has_entry(&harness, "Beta"));

    harness.click("Expand Alpha");
    assert!(has_entry(&harness, "Alpha One"));
}

#[test]
fn collapse_all_and_expand_all_fold_every_heading_with_subheadings() {
    let mut harness = outline_of(&guide());

    harness.click("Collapse All");
    assert!(has_entry(&harness, "Guide"));
    assert!(!has_entry(&harness, "Alpha"));
    assert!(!has_entry(&harness, "Beta"));

    harness.click("Expand All");
    assert!(has_entry(&harness, "Alpha Two"));
}

#[test]
fn an_outline_without_subheadings_has_no_fold_controls() {
    let harness = outline_of("## One\n\ntext\n\n## Two\n\ntext\n");

    assert!(has_entry(&harness, "One"));
    assert!(has_entry(&harness, "Two"));
    assert!(harness.query_by_label("Collapse All").is_none());
    assert!(harness.query_by_label_contains("Collapse").is_none());
}

#[test]
fn the_outline_is_shown_only_on_request_and_for_documents_with_headings() {
    let mut with_headings = outline_of(&guide());
    assert!(outline_is_on_screen(&with_headings));
    with_headings.state_mut().show_outline = false;
    with_headings.frames(1);
    assert!(!has_entry(&with_headings, "Beta"));

    let without_headings = outline_of("Plain text, no headings.\n");
    assert!(!outline_is_on_screen(&without_headings));
}

#[test]
fn a_measured_header_scrolls_to_just_above_its_position() {
    assert_eq!(
        header_jump(Some(400.0), 10, 200, 4000.0),
        HeaderJump {
            scroll_offset: 350.0,
            is_estimate: false,
        }
    );
}

#[test]
fn a_measured_header_near_the_top_scrolls_to_the_top() {
    assert_eq!(
        header_jump(Some(20.0), 1, 200, 4000.0),
        HeaderJump {
            scroll_offset: 0.0,
            is_estimate: false,
        }
    );
}

#[test]
fn an_unmeasured_header_is_estimated_from_its_line_number() {
    // Line 50 of 200 is a quarter of the way down a 4000px document.
    assert_eq!(
        header_jump(None, 50, 200, 4000.0),
        HeaderJump {
            scroll_offset: 950.0,
            is_estimate: true,
        }
    );
}

#[test]
fn before_layout_an_unmeasured_header_is_estimated_at_the_top() {
    let at_the_top = HeaderJump {
        scroll_offset: 0.0,
        is_estimate: true,
    };

    assert_eq!(header_jump(None, 50, 200, 0.0), at_the_top);
    assert_eq!(header_jump(None, 50, 0, 4000.0), at_the_top);
}

#[test]
fn a_measured_position_does_not_depend_on_layout_totals() {
    assert_eq!(
        header_jump(Some(400.0), 50, 0, 0.0),
        HeaderJump {
            scroll_offset: 350.0,
            is_estimate: false,
        }
    );
}
