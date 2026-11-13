//! The welcome page: a click on it goes in, the request it stands for comes
//! out. Nothing here depends on where the page puts its widgets.

use std::fs;
use std::path::PathBuf;

use eframe::egui;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::content::tests::label_harness::LabelHarness;

/// The moment the page's "opened ... ago" labels are relative to.
const NOW: u64 = 1_000_000;

type Page = egui_kittest::Harness<'static, Vec<WelcomeAction>>;

fn recent(path: &str, opened_secs_ago: u64) -> RecentEntry {
    RecentEntry {
        path: PathBuf::from(path),
        last_opened: NOW - opened_secs_ago,
    }
}

/// The page alone, over `rows`, recording every pick it reports.
fn page_with(rows: Vec<(RecentEntry, bool)>, has_more: bool, show_all: bool) -> Page {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(
            move |ui, picked: &mut Vec<WelcomeAction>| {
                let list = RecentList {
                    rows: &rows,
                    has_more,
                    show_all,
                    now: NOW,
                };
                picked.extend(welcome_page(ui, &list));
            },
            Vec::new(),
        )
}

fn click(page: &mut Page, label: &str) {
    page.get_by_label(label).click();
    page.run_steps(2);
}

#[test]
fn the_open_buttons_ask_for_a_file_and_a_folder() {
    let mut page = page_with(Vec::new(), false, false);

    click(&mut page, "Open File…");
    click(&mut page, "Open Folder…");

    assert_eq!(
        page.state(),
        &[WelcomeAction::OpenFile, WelcomeAction::OpenFolder]
    );
}

#[test]
fn clicking_a_recent_file_asks_to_open_it() {
    let rows = vec![
        (recent("/docs/a.md", 60), true),
        (recent("/docs/b.md", 120), true),
    ];
    let mut page = page_with(rows, false, false);

    click(&mut page, "b.md");

    assert_eq!(
        page.state(),
        &[WelcomeAction::OpenRecent(PathBuf::from("/docs/b.md"))]
    );
}

#[test]
fn a_recent_file_that_is_gone_cannot_be_picked() {
    let mut page = page_with(vec![(recent("/docs/gone.md", 60), false)], false, false);

    click(&mut page, "gone.md");

    assert_eq!(page.state(), &[]);
}

#[test]
fn a_recent_file_shows_its_folder_and_when_it_was_opened() {
    let page = page_with(vec![(recent("/docs/a.md", 180), true)], false, false);

    assert!(page.query_by_label_contains("/docs").is_some());
    assert!(page.query_by_label_contains("3m ago").is_some());
}

#[test]
fn a_long_recent_list_can_be_expanded_and_collapsed() {
    let rows = vec![(recent("/docs/a.md", 60), true)];
    let mut collapsed = page_with(rows.clone(), true, false);
    let mut expanded = page_with(rows, true, true);

    click(&mut collapsed, "Show more…");
    click(&mut expanded, "Show less");

    assert_eq!(collapsed.state(), &[WelcomeAction::ToggleShowAll]);
    assert_eq!(expanded.state(), &[WelcomeAction::ToggleShowAll]);
}

#[test]
fn a_short_recent_list_has_nothing_to_expand() {
    let page = page_with(vec![(recent("/docs/a.md", 60), true)], false, false);

    assert!(page.query_by_label("Show more…").is_none());
    assert!(page.query_by_label("Recent").is_some());
}

#[test]
fn without_recent_files_there_is_no_recent_list() {
    let page = page_with(Vec::new(), false, false);

    assert!(page.query_by_label("Recent").is_none());
}

/// The app with every document closed, so the welcome page shows, and
/// `names` as its recent files, most recent first.
fn welcome(names: &[&str]) -> LabelHarness {
    let docs: Vec<(&str, &str)> = names.iter().map(|name| (*name, "# Doc")).collect();
    let mut harness = LabelHarness::with_docs(&docs);
    let recent_files = names
        .iter()
        .map(|name| RecentEntry {
            path: harness.path(name),
            last_opened: now_epoch_secs(),
        })
        .collect();
    let app = harness.state_mut();
    app.recent_files = recent_files;
    // Hidden so that a file name labels its recent entry and nothing else.
    app.show_explorer = false;
    app.close_active_tab();
    harness.frames(2);
    harness
}

#[test]
fn clicking_a_recent_file_opens_it_in_a_tab() {
    let mut harness = welcome(&["a.md", "b.md"]);
    assert!(harness.state().tabs.is_empty());

    harness.click("b.md");

    let tabs = &harness.state().tabs;
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].path, harness.path("b.md"));
}

#[test]
fn a_deleted_recent_file_stays_on_the_welcome_page() {
    let mut harness = welcome(&["a.md", "b.md"]);
    fs::remove_file(harness.path("b.md")).unwrap();
    harness.frames(1);

    harness.click("b.md");

    assert!(harness.state().tabs.is_empty());
    assert_eq!(harness.state().error_message, None);
}

#[test]
fn the_recent_list_shows_six_files_until_it_is_expanded() {
    let names = ["1.md", "2.md", "3.md", "4.md", "5.md", "6.md", "7.md"];
    let mut harness = welcome(&names);
    assert!(harness.query_by_label("6.md").is_some());
    assert!(harness.query_by_label("7.md").is_none());

    harness.click("Show more…");
    assert!(harness.query_by_label("7.md").is_some());

    harness.click("Show less");
    assert!(harness.query_by_label("7.md").is_none());
}
