//! Opening, closing, switching, and navigating tabs, and what follows the
//! tabs around: the recent list, the explorer highlight, and the watcher.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app::action::AppAction;
use crate::app::tests::Harness;
use crate::app::MarkdownApp;

const TWO_SECTIONS: &str = "# Title\n\nIntro.\n\n## Details\n\nMore.\n";

/// Three documents with `b.md` and `c.md` opened in tabs of their own.
fn three_tabs() -> Harness {
    let mut harness = Harness::with_three_docs();
    harness.app.open_in_new_tab(harness.path("b.md"));
    harness.app.open_in_new_tab(harness.path("c.md"));
    harness
}

fn tab_names(app: &MarkdownApp) -> Vec<String> {
    app.tabs.iter().map(|tab| tab.title().to_owned()).collect()
}

fn recent_paths(app: &MarkdownApp) -> Vec<PathBuf> {
    let recents = app.recent_files.iter();
    recents.map(|entry| entry.path.clone()).collect()
}

/// Make `picker` answer every dialog with `answer`, and report the folder
/// the last dialog started in.
fn answer_with(
    answer: Result<Option<PathBuf>, String>,
) -> (super::PathPicker, Rc<RefCell<Option<PathBuf>>>) {
    let started_in: Rc<RefCell<Option<PathBuf>>> = Rc::default();
    let seen = Rc::clone(&started_in);
    let picker: super::PathPicker = Box::new(move |start| {
        *seen.borrow_mut() = start.map(PathBuf::from);
        answer.clone()
    });
    (picker, started_in)
}

#[test]
fn a_new_tab_becomes_active_recent_and_highlighted() {
    let mut harness = Harness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));

    harness.app.open_in_new_tab(b.clone());

    assert_eq!(tab_names(&harness.app), ["a.md", "b.md"]);
    assert_eq!(harness.active_path(), b);
    assert_eq!(recent_paths(&harness.app), [b.clone(), a]);
    assert!(harness.app.open_tab_paths.contains(&b));
}

#[test]
fn opening_an_open_document_focuses_its_tab_instead_of_duplicating_it() {
    let mut harness = three_tabs();
    let a = harness.path("a.md");

    harness.app.open_in_new_tab(a.clone());

    assert_eq!(harness.app.tabs.len(), 3);
    assert_eq!(harness.active_path(), a);
}

#[test]
fn a_fragment_lands_on_its_heading_in_a_new_tab_and_in_an_open_one() {
    let mut harness = Harness::with_docs(&[("a.md", "# A"), ("b.md", TWO_SECTIONS)]);
    let b = harness.path("b.md");

    harness
        .app
        .open_in_new_tab_with_fragment(b.clone(), Some("details"));
    assert_eq!(harness.active_path(), b);
    assert!(harness.app.tabs[1].take_corrections().header_key.is_some());

    harness.app.focus_tab(0);
    harness
        .app
        .open_in_new_tab_with_fragment(b.clone(), Some("details"));
    assert_eq!(harness.active_path(), b);
    assert_eq!(harness.app.tabs.len(), 2);
    assert!(
        harness.app.tabs[1].take_corrections().header_key.is_some(),
        "focusing the open tab still follows the anchor"
    );

    harness.app.open_in_new_tab(b);
    assert!(harness.app.tabs[1].take_corrections().header_key.is_none());
}

#[test]
fn a_document_that_cannot_be_read_reports_the_failure_and_opens_nothing() {
    let mut harness = Harness::with_three_docs();
    let missing = harness.path("missing.md");

    harness.app.open_in_new_tab(missing.clone());

    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.app.active_tab, 0);
    let message = harness.app.error_message.clone().unwrap();
    assert!(message.contains(&missing.display().to_string()));
    assert!(!recent_paths(&harness.app).contains(&missing));
}

#[test]
fn navigating_from_the_welcome_page_opens_a_tab() {
    let mut harness = Harness::with_three_docs();
    let b = harness.path("b.md");
    harness.app.close_active_tab();
    assert!(harness.app.tabs.is_empty());

    harness.app.navigate_active_tab_to(&b);

    assert_eq!(tab_names(&harness.app), ["b.md"]);
}

#[test]
fn navigating_replaces_the_active_document_and_updates_what_mirrors_it() {
    let mut harness = Harness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));

    harness.app.navigate_active_tab_to(&b);

    assert_eq!(tab_names(&harness.app), ["b.md"]);
    assert_eq!(recent_paths(&harness.app), [b.clone(), a.clone()]);
    assert!(harness.app.open_tab_paths.contains(&b));
    assert!(!harness.app.open_tab_paths.contains(&a));

    harness.app.navigate_active_tab_to(&b);
    assert_eq!(
        harness.app.tabs[0].history_back,
        [a],
        "navigating to the document already shown is not a history step"
    );
}

#[test]
fn navigating_to_a_file_that_cannot_be_read_stays_put_and_says_why() {
    let mut harness = Harness::with_three_docs();
    let (a, missing) = (harness.path("a.md"), harness.path("missing.md"));

    harness.app.navigate_active_tab_to(&missing);

    assert_eq!(harness.active_path(), a);
    assert!(harness.app.tabs[0].history_back.is_empty());
    let message = harness.app.error_message.clone().unwrap();
    assert!(message.starts_with("Unable to open "), "{message}");
    assert!(message.contains(&missing.display().to_string()));
}

#[test]
fn navigating_to_a_folder_stays_put_and_says_nothing() {
    let mut harness = Harness::with_three_docs();
    let a = harness.path("a.md");

    harness.app.navigate_active_tab_to(&harness.dir.clone());

    assert_eq!(harness.active_path(), a);
    assert_eq!(harness.app.error_message, None);
}

#[test]
fn a_history_step_onto_a_deleted_document_stays_put_and_says_why() {
    let mut harness = Harness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    harness.app.navigate_active_tab_to(&b);
    fs::remove_file(&a).unwrap();

    harness.apply(AppAction::NavigateBack);

    assert_eq!(harness.active_path(), b);
    assert!(harness.app.error_message.is_some());
}

#[test]
fn history_actions_without_a_tab_do_nothing() {
    let mut harness = Harness::with_three_docs();
    harness.app.close_active_tab();

    harness.apply(AppAction::NavigateBack);
    harness.apply(AppAction::NavigateForward);

    assert!(harness.app.tabs.is_empty());
    assert!(harness.app.error_message.is_none());
}

#[test]
fn closing_a_tab_keeps_the_same_document_active_where_it_can() {
    let mut harness = three_tabs();
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));

    harness.app.focus_tab(1);
    harness.app.close_tab(0);
    assert_eq!(harness.active_path(), b, "closing a tab to the left");

    harness.app.close_tab(1);
    assert_eq!(harness.active_path(), b, "closing a tab to the right");

    harness.app.open_in_new_tab(c.clone());
    harness.app.close_tab(1);
    assert_eq!(harness.active_path(), b, "closing the active last tab");
    assert!(!harness.app.open_tab_paths.contains(&c));

    harness.app.close_tab(7);
    assert_eq!(tab_names(&harness.app), ["b.md"], "there is no eighth tab");

    harness.app.close_tab(0);
    assert!(harness.app.tabs.is_empty());
    assert_eq!(harness.app.active_tab, 0);
}

#[test]
fn switching_tabs_with_one_tab_or_none_changes_nothing() {
    let mut harness = Harness::with_three_docs();

    harness.app.next_tab();
    harness.app.prev_tab();
    harness.app.focus_tab(4);

    assert_eq!(harness.app.active_tab, 0);
}

#[test]
fn previous_tab_steps_left_before_it_wraps() {
    let mut harness = three_tabs();
    assert_eq!(harness.app.active_tab, 2);

    harness.app.prev_tab();
    assert_eq!(harness.app.active_tab, 1);
    harness.app.next_tab();
    assert_eq!(harness.app.active_tab, 2);
}

#[test]
fn the_session_keeps_only_tabs_whose_files_still_exist() {
    let harness = three_tabs();
    let (a, b, c) = (
        harness.path("a.md"),
        harness.path("b.md"),
        harness.path("c.md"),
    );
    fs::remove_file(&b).unwrap();

    assert_eq!(harness.app.get_open_tab_paths(), [a, c]);
}

#[test]
fn the_watcher_follows_tabs_as_they_open_navigate_and_close() {
    let mut harness = Harness::with_three_docs();
    let (a, b, c) = (
        harness.path("a.md"),
        harness.path("b.md"),
        harness.path("c.md"),
    );
    harness.apply(AppAction::ToggleWatch);
    assert!(harness.app.watched_files().contains(&a));

    harness.app.open_in_new_tab(b.clone());
    assert!(harness.app.watched_files().contains(&b));

    harness.app.navigate_active_tab_to(&c);
    assert!(harness.app.watched_files().contains(&c));
    assert!(!harness.app.watched_files().contains(&b));

    harness.app.close_active_tab();
    assert!(!harness.app.watched_files().contains(&c));
    assert!(harness.app.watched_files().contains(&a));
}

#[test]
fn the_file_dialog_starts_beside_the_active_document_and_opens_the_pick() {
    let mut harness = Harness::with_three_docs();
    let b = harness.path("b.md");
    let (picker, started_in) = answer_with(Ok(Some(b.clone())));
    harness.app.file_picker = picker;

    harness.apply(AppAction::OpenFile);

    assert_eq!(harness.active_path(), b);
    assert_eq!(*started_in.borrow(), Some(harness.dir.clone()));
}

#[test]
fn without_a_tab_the_file_dialog_starts_in_the_explorer_root() {
    let mut harness = Harness::with_three_docs();
    harness.app.close_active_tab();
    let (picker, started_in) = answer_with(Ok(None));
    harness.app.file_picker = picker;

    harness.apply(AppAction::OpenFile);

    assert!(
        harness.app.tabs.is_empty(),
        "a cancelled dialog opens nothing"
    );
    assert!(harness.app.error_message.is_none());
    assert_eq!(
        started_in.borrow().as_deref(),
        harness.app.file_explorer.root()
    );
    assert!(started_in.borrow().is_some());
}

#[test]
fn a_dialog_that_fails_to_open_is_reported() {
    let mut harness = Harness::with_three_docs();
    let root = harness.app.file_explorer.root().map(PathBuf::from);
    harness.app.file_picker = answer_with(Err("no portal".to_owned())).0;
    harness.app.folder_picker = answer_with(Err("no display".to_owned())).0;

    harness.apply(AppAction::OpenFile);
    let file_error = harness.app.error_message.take().unwrap();
    assert!(file_error.contains("no portal"));

    harness.apply(AppAction::OpenFolder);
    let folder_error = harness.app.error_message.take().unwrap();
    assert!(folder_error.contains("no display"));
    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.app.file_explorer.root(), root.as_deref());
}

#[test]
fn the_folder_dialog_starts_in_the_explorer_root_and_repoints_it() {
    let mut harness = Harness::with_three_docs();
    let folder = harness.path("notes");
    fs::create_dir(&folder).unwrap();
    harness.app.show_explorer = false;
    harness.apply(AppAction::ToggleWatch);
    let (picker, started_in) = answer_with(Ok(Some(folder.clone())));
    harness.app.folder_picker = picker;

    harness.apply(AppAction::OpenFolder);

    assert_eq!(*started_in.borrow(), Some(harness.dir.clone()));
    assert_eq!(harness.app.file_explorer.root(), Some(folder.as_path()));
    assert!(harness.app.show_explorer);
    assert!(
        harness.app.watched_explorer_dirs().contains(&folder),
        "live reload follows the explorer to its new root"
    );
}

#[test]
fn a_cancelled_folder_dialog_leaves_the_explorer_alone() {
    let mut harness = Harness::with_three_docs();
    harness.app.show_explorer = false;
    harness.app.folder_picker = answer_with(Ok(None)).0;

    harness.apply(AppAction::OpenFolder);

    assert_eq!(
        harness.app.file_explorer.root(),
        Some(harness.dir.as_path())
    );
    assert!(!harness.app.show_explorer);
    assert!(harness.app.error_message.is_none());
}
