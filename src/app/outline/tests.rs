use eframe::egui::accesskit::Role;
use eframe::egui::{Event, Modifiers, PointerButton, Pos2};
use egui_commonmark_extended::header_position_key;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::content::tests::label_harness::LabelHarness;
use crate::app::ui_harness::MemoryStorage;
use crate::persist::{PersistedState, APP_KEY};
use crate::tab::scroll::HEADER_SCROLL_MARGIN;
use crate::test_support::{assert_close, painted_shapes};

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
    let scroll_offset = harness.state().tabs[0].scroll_offset;
    (scroll_offset - (heading_position(harness, title) - HEADER_SCROLL_MARGIN)).abs()
}

fn outline_is_on_screen(harness: &LabelHarness) -> bool {
    egui::containers::panel::PanelState::load(&harness.ctx, egui::Id::new("outline")).is_some()
}

/// Whether the outline is drawn this frame. A panel that was shown once
/// stays in egui's memory.
fn outline_is_shown(harness: &LabelHarness) -> bool {
    harness.query_by_label("CONTENTS").is_some()
}

/// Pick `item` from the menu behind the outline's three dots.
fn pick(harness: &mut LabelHarness, item: &str) {
    harness.click("Outline options");
    harness.click(item);
}

/// Where the renderer painted the heading `title`, in px from the top of
/// the document.
fn heading_position(harness: &LabelHarness, title: &str) -> f32 {
    let tab = &harness.state().tabs[0];
    let headers = tab.outline.headers();
    let header = headers.iter().find(|header| header.title == title);
    let key = header_position_key(header.unwrap().source_start);
    tab.cache.get_header_position(&key).unwrap()
}

/// The color the outline's entry for `title` is written in, and where its
/// text starts.
fn ink_and_left(harness: &LabelHarness, title: &str) -> (egui::Color32, f32) {
    let row = entry(harness, title).rect();
    let shapes = painted_shapes(harness.output());
    let mut texts = shapes.iter().filter_map(|shape| match shape {
        egui::Shape::Text(text) if text.galley.text() == title && row.contains(text.pos) => {
            Some((text.galley.job.sections[0].format.color, text.pos.x))
        }
        _ => None,
    });
    texts.next_back().unwrap()
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

    pick(&mut harness, "Collapse All");
    assert!(has_entry(&harness, "Guide"));
    assert!(!has_entry(&harness, "Alpha"));
    assert!(!has_entry(&harness, "Beta"));
    assert!(
        harness.query_by_label("Expand All").is_none(),
        "a pick closes the menu"
    );

    pick(&mut harness, "Expand All");
    assert!(has_entry(&harness, "Alpha Two"));
}

#[test]
fn a_click_beside_the_fold_control_jumps_and_does_not_fold() {
    let mut harness = outline_of(&guide());

    entry(&harness, "Beta").click();
    harness.frames(4);
    entry(&harness, "Alpha").click();
    harness.frames(4);

    assert!(has_entry(&harness, "Alpha One"), "still unfolded");
    let distance = distance_from_heading(&harness, "Alpha");
    assert!(distance <= 2.0, "stopped {distance}px from the heading");
}

#[test]
fn the_outline_is_headed_contents_on_the_surface_color() {
    let harness = outline_of(&guide());
    let palette = harness.state().theme.palette();
    let panel = egui::containers::panel::PanelState::load(&harness.ctx, egui::Id::new("outline"));
    let panel = panel.unwrap().rect;

    let shapes = painted_shapes(harness.output());
    let fill = shapes.iter().find_map(|shape| match shape {
        egui::Shape::Rect(rect) if rect.rect == panel => Some((rect.fill, rect.stroke)),
        _ => None,
    });
    let edges: Vec<_> = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == palette.rule && (points[0].x - panel.left()).abs() < 1.0 =>
            {
                Some(points[1].x)
            }
            _ => None,
        })
        .collect();

    assert!(harness.query_by_label("CONTENTS").is_some());
    assert_eq!(fill, Some((palette.surface, egui::Stroke::NONE)));
    assert_eq!(
        edges,
        [panel.left() + 0.5],
        "one hairline, on the inner edge"
    );
}

#[test]
fn subheadings_are_indented_under_their_heading() {
    let harness = outline_of(&guide());
    let left = |title| ink_and_left(&harness, title).1;

    assert_close(left("Alpha One") - left("Alpha"), INDENT);
    assert_close(left("Beta"), left("Alpha"));
    assert_close(left("Guide"), left("Alpha"));
}

#[test]
fn the_heading_being_read_is_inked_and_the_others_are_muted() {
    let mut harness = outline_of(&guide());
    let palette = harness.state().theme.palette();
    let ink = |harness: &LabelHarness, title| ink_and_left(harness, title).0;
    assert_eq!(ink(&harness, "Guide"), palette.text);
    assert_eq!(ink(&harness, "Alpha"), palette.muted);
    assert_eq!(ink(&harness, "Beta"), palette.muted);

    entry(&harness, "Alpha Two").click();
    harness.frames(4);
    // Off the row, so that no hover inks it.
    harness.hover_at(egui::pos2(400.0, 400.0));
    harness.frames(2);

    assert_eq!(ink(&harness, "Alpha Two"), palette.text);
    assert_eq!(ink(&harness, "Guide"), palette.muted);
    assert_eq!(ink(&harness, "Alpha"), palette.muted);
    assert_eq!(ink(&harness, "Beta"), palette.muted);
}

#[test]
fn scrolling_the_document_moves_the_mark_to_the_heading_in_view() {
    let mut harness = outline_of(&guide());
    let palette = harness.state().theme.palette();
    let beta = heading_position(&harness, "Beta");

    harness.state_mut().tabs[0].pending_scroll_offset = Some(beta + 300.0);
    harness.frames(4);

    assert_eq!(ink_and_left(&harness, "Beta").0, palette.text);
    assert_eq!(ink_and_left(&harness, "Alpha Two").0, palette.muted);
}

#[test]
fn a_folded_heading_stands_for_the_subheading_being_read() {
    let mut harness = outline_of(&guide());
    let palette = harness.state().theme.palette();
    entry(&harness, "Alpha Two").click();
    harness.frames(4);

    harness.click("Collapse Alpha");
    harness.hover_at(egui::pos2(400.0, 400.0));
    harness.frames(2);

    assert!(!has_entry(&harness, "Alpha Two"));
    assert_eq!(ink_and_left(&harness, "Alpha").0, palette.text);
    assert_eq!(ink_and_left(&harness, "Beta").0, palette.muted);
}

#[test]
fn a_long_outline_scrolls_to_keep_the_heading_being_read_on_screen() {
    let sections: Vec<String> = (0..80)
        .map(|n| format!("## Section {n}\n\n{}", "A paragraph of text.\n\n".repeat(6)))
        .collect();
    let mut harness = outline_of(&sections.concat());
    assert!(has_entry(&harness, "Section 0"));
    assert!(!has_entry(&harness, "Section 70"), "below the fold");
    let target = heading_position(&harness, "Section 70");

    harness.state_mut().tabs[0].pending_scroll_offset = Some(target);
    harness.frames(8);

    assert!(has_entry(&harness, "Section 70"));
    assert!(!has_entry(&harness, "Section 0"));
    let palette = harness.state().theme.palette();
    assert_eq!(ink_and_left(&harness, "Section 70").0, palette.text);
}

#[test]
fn the_row_under_the_pointer_is_washed_and_no_other() {
    let mut harness = outline_of(&guide());
    let raised = harness.state().theme.palette().raised;
    let washed = |harness: &LabelHarness| {
        let shapes = painted_shapes(harness.output());
        let rects = shapes.iter().filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == raised => Some(rect.rect),
            _ => None,
        });
        rects.collect::<Vec<_>>()
    };
    assert!(washed(&harness).is_empty());

    entry(&harness, "Beta").hover();
    harness.frames(2);

    assert_eq!(washed(&harness), [entry(&harness, "Beta").rect()]);
}

#[test]
fn a_fresh_install_shows_the_outline_only_while_the_window_has_room() {
    let mut harness = outline_of(&guide());
    assert!(harness.state().show_outline, "1200 px hold both");

    harness.set_size(egui::vec2(900.0, 800.0));
    harness.frames(2);
    assert!(!harness.state().show_outline);
    assert!(!outline_is_shown(&harness));

    harness.set_size(egui::vec2(1200.0, 800.0));
    harness.frames(2);
    assert!(harness.state().show_outline);
    assert!(outline_is_shown(&harness));
}

#[test]
fn the_explorer_takes_room_the_outline_would_have() {
    let mut harness = outline_of(&guide());
    // 744 px for the text and 208 for the outline, and nothing to spare.
    harness.set_size(egui::vec2(960.0, 800.0));
    harness.frames(2);
    assert!(harness.state().show_outline);

    harness.state_mut().show_explorer = true;
    harness.frames(2);
    assert!(!harness.state().show_outline);

    harness.state_mut().show_explorer = false;
    harness.frames(2);
    assert!(harness.state().show_outline);
}

#[test]
fn the_readers_toggle_wins_over_the_room_and_is_what_a_session_saves() {
    let mut harness = outline_of(&guide());
    let saved = |harness: &mut LabelHarness| {
        let mut storage = MemoryStorage::default();
        eframe::App::save(harness.state_mut(), &mut storage);
        let state: PersistedState = eframe::get_value(&storage, APP_KEY).unwrap();
        state.show_outline
    };
    assert_eq!(saved(&mut harness), None, "nothing chosen, nothing saved");
    harness.set_size(egui::vec2(900.0, 800.0));
    harness.frames(2);
    assert!(!harness.state().show_outline);

    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, egui::Key::O);
    harness.frames(2);
    assert!(harness.state().show_outline, "shown in a narrow window");
    assert_eq!(saved(&mut harness), Some(true));

    harness.set_size(egui::vec2(1200.0, 800.0));
    harness.frames(2);
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, egui::Key::O);
    harness.frames(2);
    assert!(!harness.state().show_outline, "hidden in a wide one");
    assert_eq!(saved(&mut harness), Some(false));

    harness.set_size(egui::vec2(1600.0, 800.0));
    harness.frames(2);
    assert!(!harness.state().show_outline);
}

#[test]
fn an_outline_without_subheadings_has_no_fold_controls() {
    let harness = outline_of("## One\n\ntext\n\n## Two\n\ntext\n");

    assert!(has_entry(&harness, "One"));
    assert!(has_entry(&harness, "Two"));
    assert!(harness.query_by_label("Outline options").is_none());
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
