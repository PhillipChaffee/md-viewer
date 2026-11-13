//! The tab strip, clicked through by label: a click goes in, the open tabs
//! and the active document come out.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::accesskit::Toggled;
use eframe::egui::{self, Event, Key, PointerButton};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{STRIP_HEIGHT, UNDERLINE};
use crate::app::content::tests::label_harness::LabelHarness;
use crate::test_support::{assert_close, painted_shapes};

/// `a.md`, `b.md`, and `c.md`, each in its own tab with `c.md` active. The
/// explorer is hidden so that a file name labels its tab and nothing else.
fn three_tabs() -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[("a.md", "# A"), ("b.md", "# B"), ("c.md", "# C")]);
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));
    let app = harness.state_mut();
    app.show_explorer = false;
    app.open_in_new_tab(b);
    app.open_in_new_tab(c);
    harness.frames(2);
    harness
}

/// One document in one tab, with the explorer hidden.
fn one_tab(content: &str) -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[("a.md", content)]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

fn open_paths(harness: &LabelHarness) -> Vec<PathBuf> {
    let tabs = &harness.state().tabs;
    tabs.iter().map(|tab| tab.path.clone()).collect()
}

fn active_path(harness: &LabelHarness) -> PathBuf {
    let app = harness.state();
    app.tabs[app.active_tab].path.clone()
}

fn open_context_menu(harness: &mut LabelHarness, tab: &str) {
    harness.get_by_label(tab).click_secondary();
    harness.frames(2);
}

#[test]
fn clicking_a_tab_shows_its_document() {
    let mut harness = three_tabs();
    assert_eq!(active_path(&harness), harness.path("c.md"));

    harness.click("a.md");

    assert_eq!(active_path(&harness), harness.path("a.md"));
    assert_eq!(open_paths(&harness).len(), 3);
}

#[test]
fn middle_clicking_a_tab_closes_it() {
    let mut harness = three_tabs();

    harness
        .get_by_label("b.md")
        .click_button(PointerButton::Middle);
    harness.frames(2);

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("c.md")]
    );
}

#[test]
fn a_tab_under_the_pointer_shows_a_close_button_until_the_pointer_leaves() {
    let close_buttons = |harness: &LabelHarness| harness.query_all_by_label("Close tab").count();
    let mut harness = three_tabs();
    assert_eq!(close_buttons(&harness), 0);

    harness.get_by_label("a.md").hover();
    harness.frames(2);
    assert_eq!(close_buttons(&harness), 1);

    harness.event(Event::PointerMoved(egui::pos2(600.0, 500.0)));
    harness.frames(2);
    assert_eq!(close_buttons(&harness), 0);
}

#[test]
fn the_close_button_of_a_hovered_tab_closes_that_tab() {
    let mut harness = three_tabs();
    harness.get_by_label("a.md").hover();
    harness.frames(2);

    harness.click("Close tab");

    assert_eq!(
        open_paths(&harness),
        [harness.path("b.md"), harness.path("c.md")]
    );
    assert_eq!(active_path(&harness), harness.path("c.md"));
}

#[test]
fn closing_the_active_tab_shows_the_one_before_it() {
    let mut harness = three_tabs();
    harness.get_by_label("c.md").hover();
    harness.frames(2);

    harness.click("Close tab");

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("b.md")]
    );
    assert_eq!(active_path(&harness), harness.path("b.md"));
}

#[test]
fn the_tabs_keep_their_places_when_a_close_button_appears() {
    let lefts = |harness: &LabelHarness| -> Vec<f32> {
        ["a.md", "b.md", "c.md"]
            .map(|tab| harness.get_by_label(tab).rect().left())
            .into()
    };
    let mut harness = three_tabs();
    let at_rest = lefts(&harness);

    harness.get_by_label("a.md").hover();
    harness.frames(2);

    assert_eq!(lefts(&harness), at_rest);
    let (tab, close) = (
        harness.get_by_label("a.md").rect(),
        harness.get_by_label("Close tab").rect(),
    );
    assert!(tab.contains_rect(close), "{close:?} inside {tab:?}");
}

#[test]
fn the_context_menu_closes_the_tab_it_was_opened_on() {
    let mut harness = three_tabs();

    open_context_menu(&mut harness, "a.md");
    harness.click("Close");

    assert_eq!(
        open_paths(&harness),
        [harness.path("b.md"), harness.path("c.md")]
    );
}

#[test]
fn close_others_keeps_only_the_tab_it_was_opened_on() {
    let mut harness = three_tabs();

    open_context_menu(&mut harness, "b.md");
    harness.click("Close Others");

    let kept = harness.path("b.md");
    assert_eq!(open_paths(&harness), std::slice::from_ref(&kept));
    assert_eq!(active_path(&harness), kept);
    assert_eq!(harness.state().open_tab_paths, HashSet::from([kept]));
}

#[test]
fn close_others_stops_watching_the_closed_documents() {
    let mut harness = three_tabs();
    harness.key_press(Key::F5);
    harness.frames(2);
    assert_eq!(harness.state().watched_paths.len(), 3);

    open_context_menu(&mut harness, "b.md");
    harness.click("Close Others");

    assert_eq!(
        harness.state().watched_paths,
        HashSet::from([harness.path("b.md")])
    );
}

#[test]
fn a_lone_tab_is_not_offered_close_others() {
    let mut harness = one_tab("# A");
    harness.state_mut().tabs[0].set_edit_mode(true);
    harness.frames(2);

    open_context_menu(&mut harness, "a.md");

    assert!(harness.query_by_label("Close").is_some());
    assert!(harness.query_by_label("Close Others").is_none());
}

/// Whether the strip is on screen: its open-file button is.
fn shows_strip(harness: &LabelHarness) -> bool {
    harness.query_by_label("Open file in new tab").is_some()
}

#[test]
fn one_document_has_no_strip_and_a_second_brings_it_in() {
    let mut harness = one_tab("# A");
    assert!(!shows_strip(&harness));
    assert!(harness.query_by_label("a.md").is_none());

    let other = harness.dir.join("b.md");
    std::fs::write(&other, "# B").unwrap();
    harness.state_mut().open_in_new_tab(other);
    harness.frames(2);
    assert!(shows_strip(&harness));
    assert!(harness.query_by_label("a.md").is_some());

    harness.get_by_label("b.md").hover();
    harness.frames(2);
    harness.click("Close tab");
    assert!(!shows_strip(&harness));
}

#[test]
fn no_document_has_no_strip_either() {
    let mut harness = one_tab("# A");
    harness.state_mut().close_tab(0);
    harness.frames(2);

    assert!(harness.state().tabs.is_empty());
    assert!(!shows_strip(&harness));
}

#[test]
fn the_strip_is_the_height_of_one_row_with_every_tab_filling_it() {
    let harness = three_tabs();

    let open = harness.get_by_label("Open file in new tab").rect();
    for tab in ["a.md", "b.md", "c.md"] {
        let rect = harness.get_by_label(tab).rect();
        assert_close(rect.height(), STRIP_HEIGHT);
        assert_close(rect.top(), crate::app::chrome::BAR_HEIGHT);
        assert_close(rect.center().y, open.center().y);
    }
}

/// The text the last frame painted that reads `text`, in the strip.
#[track_caller]
fn strip_text(harness: &LabelHarness, text: &str) -> egui::epaint::TextShape {
    let strip = crate::app::chrome::BAR_HEIGHT..=crate::app::chrome::BAR_HEIGHT + STRIP_HEIGHT;
    let found = painted_shapes(harness.output())
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(shape)
                if shape.galley.text() == text && strip.contains(&shape.pos.y) =>
            {
                Some(shape)
            }
            _ => None,
        });
    found.expect("the strip did not paint the text")
}

#[test]
fn the_active_tab_is_dark_and_underlined_in_the_accent_and_the_rest_are_muted() {
    let harness = three_tabs();
    let palette = harness.state().theme.palette();

    assert_eq!(strip_text(&harness, "c.md").fallback_color, palette.text);
    assert_eq!(strip_text(&harness, "a.md").fallback_color, palette.muted);
    assert_eq!(strip_text(&harness, "b.md").fallback_color, palette.muted);

    let underlines: Vec<egui::Rect> = painted_shapes(harness.output())
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == palette.accent => Some(rect.rect),
            _ => None,
        })
        .collect();
    let [line] = underlines[..] else {
        panic!("one underline: {underlines:?}");
    };
    let title = strip_text(&harness, "c.md");
    assert_close(line.height(), UNDERLINE);
    assert_close(line.left(), title.pos.x);
    assert_close(line.width(), title.galley.size().x);
    assert_close(line.bottom(), crate::app::chrome::BAR_HEIGHT + STRIP_HEIGHT);
}

#[test]
fn a_muted_tab_darkens_under_the_pointer() {
    let mut harness = three_tabs();
    let palette = harness.state().theme.palette();

    harness.get_by_label("a.md").hover();
    harness.frames(2);

    assert_eq!(strip_text(&harness, "a.md").fallback_color, palette.text);
    assert_eq!(strip_text(&harness, "b.md").fallback_color, palette.muted);
}

#[test]
fn a_tab_tells_assistive_technology_whether_it_is_the_one_shown() {
    let harness = three_tabs();
    let shown = |tab| harness.get_by_label(tab).accesskit_node().toggled();

    assert_eq!(shown("c.md"), Some(Toggled::True));
    assert_eq!(shown("a.md"), Some(Toggled::False));
}

#[test]
fn the_plus_button_opens_what_the_picker_returns_in_a_new_tab() {
    let mut harness = three_tabs();
    let other = harness.dir.join("d.md");
    std::fs::write(&other, "# D").unwrap();
    let picked = other.clone();
    harness.state_mut().file_picker = Box::new(move |_| Ok(Some(picked.clone())));

    harness.click("Open file in new tab");

    assert_eq!(open_paths(&harness).len(), 4);
    assert_eq!(active_path(&harness), other);
}

#[test]
fn the_mode_toggle_switches_between_preview_and_edit() {
    let mut harness = three_tabs();
    let editing = |harness: &LabelHarness| harness.state().tabs[2].is_editing;
    let chosen = |harness: &LabelHarness, mode| {
        harness.get_by_label(mode).accesskit_node().toggled() == Some(Toggled::True)
    };
    assert!(chosen(&harness, "Preview"));

    harness.click("Edit");
    assert!(editing(&harness));
    assert!(chosen(&harness, "Edit"));
    assert!(!chosen(&harness, "Preview"));

    harness.click("Preview");
    assert!(!editing(&harness));
}

#[test]
fn a_lone_document_being_edited_brings_the_strip_in_for_the_way_back() {
    let mut harness = one_tab("# A");
    harness.state_mut().tabs[0].set_edit_mode(true);
    harness.frames(2);
    assert!(shows_strip(&harness));

    harness.click("Preview");

    assert!(!harness.state().tabs[0].is_editing);
    assert!(!shows_strip(&harness));
}

#[test]
fn unsaved_edits_are_marked_beside_the_mode_toggle() {
    let mut harness = one_tab("# A");
    harness.state_mut().tabs[0].set_edit_mode(true);
    harness.frames(2);
    assert!(harness.query_by_label("Unsaved changes").is_none());

    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.frames(1);

    let dot = harness.get_by_label("Unsaved changes").rect();
    let toggle = harness.get_by_label("Preview").rect();
    assert!(dot.right() <= toggle.left(), "{dot:?} before {toggle:?}");
    assert_close(dot.center().y, toggle.center().y);
    let warning = harness.state().theme.palette().warning;
    let dots = painted_shapes(harness.output())
        .into_iter()
        .filter(|shape| matches!(shape, egui::Shape::Circle(dot) if dot.fill == warning))
        .count();
    assert_eq!(dots, 1);
}

#[test]
fn unsaved_edits_keep_the_strip_of_a_lone_document_after_leaving_edit_mode() {
    let mut harness = one_tab("# A");
    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.frames(2);

    assert!(shows_strip(&harness));
    assert!(harness.query_by_label("Unsaved changes").is_some());
}

#[test]
fn a_save_reports_its_outcome_beside_the_mode_toggle() {
    let mut harness = one_tab("# A");
    assert!(harness.query_by_label("Saved").is_none());

    harness.state_mut().tabs[0].save_flash = Some((true, Instant::now()));
    harness.frames(1);
    assert!(harness.query_by_label("Saved").is_some());
    let palette = harness.state().theme.palette();
    assert_eq!(
        strip_text(&harness, "Saved").galley.job.sections[0]
            .format
            .color,
        palette.positive
    );

    harness.state_mut().tabs[0].save_flash = Some((false, Instant::now()));
    harness.frames(1);
    assert!(harness.query_by_label("Save failed").is_some());
    assert!(harness.query_by_label("Saved").is_none());
    assert_eq!(
        strip_text(&harness, "Save failed").galley.job.sections[0]
            .format
            .color,
        palette.warning
    );
}

#[test]
fn a_save_outcome_disappears_after_a_moment() {
    let mut harness = one_tab("# A");
    let expired = Instant::now().checked_sub(super::SAVE_FLASH).unwrap();

    harness.state_mut().tabs[0].save_flash = Some((true, expired));
    harness.frames(2);

    assert!(harness.query_by_label("Saved").is_none());
    assert_eq!(harness.state().tabs[0].save_flash, None);
    assert!(!shows_strip(&harness), "nothing is left to report");
}
