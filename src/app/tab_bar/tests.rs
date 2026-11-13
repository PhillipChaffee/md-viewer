//! The tab strip, clicked through by label: a click goes in, the open tabs
//! and the active document come out.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{Key, PointerButton};
use egui_kittest::kittest::Queryable as _;

use crate::app::content::tests::label_harness::LabelHarness;

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
fn the_close_button_closes_its_tab() {
    let mut harness = three_tabs();

    harness.click("×");

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("b.md")]
    );
    assert_eq!(active_path(&harness), harness.path("b.md"));
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

    open_context_menu(&mut harness, "a.md");

    assert!(harness.query_by_label("Close").is_some());
    assert!(harness.query_by_label("Close Others").is_none());
}

#[test]
fn the_strip_says_so_when_no_document_is_open() {
    let mut harness = one_tab("# A");
    assert!(harness.query_by_label("No file open").is_none());

    harness.click("×");

    assert!(harness.state().tabs.is_empty());
    assert!(harness.query_by_label("No file open").is_some());
    assert!(harness.query_by_label("+").is_some());
}

#[test]
fn the_mode_toggle_switches_between_preview_and_edit() {
    let mut harness = one_tab("# A");

    harness.click("Edit");
    assert!(harness.state().tabs[0].is_editing);

    harness.click("Preview");
    assert!(!harness.state().tabs[0].is_editing);
}

#[test]
fn unsaved_edits_are_marked_beside_the_mode_toggle() {
    let mut harness = one_tab("# A");
    harness.click("Edit");
    assert!(harness.query_by_label("●").is_none());

    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.frames(1);

    assert!(harness.query_by_label("●").is_some());
}

#[test]
fn a_save_reports_its_outcome_beside_the_mode_toggle() {
    let mut harness = one_tab("# A");
    assert!(harness.query_by_label("Saved ✓").is_none());

    harness.state_mut().tabs[0].save_flash = Some((true, Instant::now()));
    harness.frames(1);
    assert!(harness.query_by_label("Saved ✓").is_some());

    harness.state_mut().tabs[0].save_flash = Some((false, Instant::now()));
    harness.frames(1);
    assert!(harness.query_by_label("Save failed").is_some());
    assert!(harness.query_by_label("Saved ✓").is_none());
}

#[test]
fn a_save_outcome_disappears_after_a_moment() {
    let mut harness = one_tab("# A");
    let expired = Instant::now().checked_sub(super::SAVE_FLASH).unwrap();

    harness.state_mut().tabs[0].save_flash = Some((true, expired));
    harness.frames(1);

    assert!(harness.query_by_label("Saved ✓").is_none());
    assert_eq!(harness.state().tabs[0].save_flash, None);
}
