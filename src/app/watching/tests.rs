//! Live reload against real files: a change on disk goes in, tab and explorer
//! state come out.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{Key, Modifiers};

use crate::app::action::AppAction;
use crate::app::tests::Harness;
use crate::app::MarkdownApp;
use crate::explorer::{FileExplorer, FileTreeNode, SortOrder};
use crate::test_support::evaluate_log_arguments;
use crate::watcher::{Backend, FileWatcher, WatchPlan, Watched, MAX_WATCHER_RETRIES};

impl MarkdownApp {
    /// What the running watcher holds, for the tests of every module that
    /// changes it; `None` while no watcher runs.
    pub(in crate::app) fn watched(&self) -> Option<&Watched> {
        let running = self.live_reload.running.as_ref()?;
        Some(running.watcher.watched())
    }

    /// The documents the running watcher holds.
    pub(in crate::app) fn watched_files(&self) -> &HashSet<PathBuf> {
        &self.watched().unwrap().files
    }

    /// The explorer directories the running watcher holds.
    pub(in crate::app) fn watched_explorer_dirs(&self) -> &HashSet<PathBuf> {
        &self.watched().unwrap().explorer_dirs
    }
}

/// How long a test waits for the watcher before it fails.
const EVENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs frames until `done` holds. Fails, naming `what`, after ten seconds.
fn run_until(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    let deadline = Instant::now() + EVENT_TIMEOUT;
    loop {
        harness.idle();
        if done(harness) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The app on `docs`, with live reload off.
fn open(docs: &[(&str, &str)]) -> Harness {
    // A log argument that panics fails a test.
    evaluate_log_arguments();
    Harness::with_docs(docs)
}

/// The app on `docs` with live reload on and the explorer's first listing
/// in place, so a later refresh is not overwritten by it.
fn watching(docs: &[(&str, &str)]) -> Harness {
    let mut harness = open(docs);
    harness.apply(AppAction::ToggleWatch);
    run_until(&mut harness, "the explorer scan", |harness| {
        !harness.app.file_explorer.is_scanning()
    });
    harness
}

/// Writes `content` to `name` under the scratch directory, creating its
/// folders.
fn write_doc(harness: &Harness, name: &str, content: &str) -> PathBuf {
    let path = harness.path(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

/// Makes the app's watcher report an error on the next frame.
fn fail_watcher(harness: &mut Harness) {
    let (events, receiver) = mpsc::channel();
    events
        .send(Err(notify::Error::generic("disk on fire")))
        .unwrap();
    harness.app.live_reload.running.as_mut().unwrap().events = receiver;
}

/// An explorer with nothing open, pointed at `root`.
fn explorer_on(root: impl Into<PathBuf>) -> FileExplorer {
    FileExplorer::restored(SortOrder::default(), HashSet::new(), Some(root.into()))
}

fn explorer_root_names(harness: &Harness) -> Vec<&str> {
    fn name(node: &FileTreeNode) -> &str {
        match node {
            FileTreeNode::File { name, .. } | FileTreeNode::Directory { name, .. } => name,
        }
    }
    harness.app.file_explorer.tree().iter().map(name).collect()
}

#[test]
fn starting_live_reload_watches_the_open_document_and_the_explorer_root() {
    let harness = watching(&[("a.md", "# A")]);

    assert_eq!(
        *harness.app.watched_files(),
        HashSet::from([harness.path("a.md")])
    );
    assert_eq!(
        *harness.app.watched_explorer_dirs(),
        HashSet::from([harness.dir.clone()])
    );
    assert_eq!(harness.app.error_message, None);
}

#[test]
fn stopping_live_reload_forgets_what_was_watched() {
    let mut harness = watching(&[("a.md", "# A")]);

    harness.apply(AppAction::ToggleWatch);

    assert!(!harness.app.live_reload.is_live());
    assert!(harness.app.watched().is_none());
}

#[test]
fn a_document_changed_on_disk_is_reloaded_and_flashes_in_the_explorer() {
    let mut harness = watching(&[("a.md", "old")]);
    let doc = harness.path("a.md");

    fs::write(&doc, "new").unwrap();
    run_until(&mut harness, "a.md to reload", |harness| {
        harness.app.tabs[0].content == "new"
    });

    assert!(harness.app.flashing_paths.contains_key(&doc));
    assert!(harness.app.flashing_paths.contains_key(&harness.dir));
}

#[test]
fn a_change_on_disk_wakes_an_idle_window() {
    let mut harness = watching(&[("a.md", "old")]);
    let (wake, woken) = mpsc::channel();
    harness.ctx.set_request_repaint_callback(move |info| {
        if info.delay.is_zero() {
            let _ = wake.send(());
        }
    });
    // Idle means the last frame asked for no immediate repaint; egui only
    // reports a request that is more urgent than the pending one.
    run_until(&mut harness, "the window to go idle", |harness| {
        !harness.ctx.has_requested_repaint()
    });
    while woken.try_recv().is_ok() {}

    fs::write(harness.path("a.md"), "new").unwrap();
    // No frame runs while this waits, so only the watcher can end the wait.
    woken
        .recv_timeout(EVENT_TIMEOUT)
        .expect("the watcher never asked for a repaint");

    run_until(&mut harness, "a.md to reload", |harness| {
        harness.app.tabs[0].content == "new"
    });
}

#[test]
fn a_document_opened_while_watching_is_reloaded_too() {
    let mut harness = watching(&[("a.md", "# A")]);
    // Outside the explorer root's own listing, so only the document's own
    // watch can report the change.
    let doc = write_doc(&harness, "sub/b.md", "old");
    harness.app.open_in_new_tab(doc.clone());
    assert!(harness.app.watched_files().contains(&doc));

    fs::write(&doc, "new").unwrap();

    run_until(&mut harness, "sub/b.md to reload", |harness| {
        harness.app.tabs[1].content == "new"
    });
}

#[test]
fn closing_a_tab_stops_watching_its_document() {
    let mut harness = watching(&[("a.md", "# A"), ("b.md", "# B")]);
    harness.app.open_in_new_tab(harness.path("b.md"));

    harness.apply(AppAction::CloseTab);

    assert_eq!(
        *harness.app.watched_files(),
        HashSet::from([harness.path("a.md")])
    );
}

#[test]
fn a_file_created_in_the_explorer_root_appears_in_the_tree() {
    let mut harness = watching(&[("a.md", "# A")]);

    write_doc(&harness, "new.md", "# New");

    run_until(&mut harness, "new.md to be listed", |harness| {
        explorer_root_names(harness).contains(&"new.md")
    });
}

#[test]
fn an_expanded_folder_is_watched_until_it_is_collapsed() {
    let mut harness = watching(&[("a.md", "# A")]);
    write_doc(&harness, "sub/x.md", "# X");
    let sub = harness.path("sub");

    harness.app.file_explorer.toggle_expanded(&sub);
    harness.app.reconcile_explorer_watches();
    assert!(harness.app.watched_explorer_dirs().contains(&sub));
    let created = write_doc(&harness, "sub/new.md", "# New");
    run_until(&mut harness, "the change in sub/ to be seen", |harness| {
        harness.app.flashing_paths.contains_key(&created)
    });

    harness.app.file_explorer.toggle_expanded(&sub);
    harness.app.reconcile_explorer_watches();
    assert_eq!(
        *harness.app.watched_explorer_dirs(),
        HashSet::from([harness.dir.clone()])
    );
}

#[test]
fn folders_are_not_watched_while_live_reload_is_off() {
    let mut harness = open(&[("a.md", "# A")]);
    let sub = write_doc(&harness, "sub/x.md", "# X")
        .parent()
        .unwrap()
        .to_path_buf();

    harness.app.file_explorer.toggle_expanded(&sub);
    harness.app.reconcile_explorer_watches();

    assert!(harness.app.watched().is_none());
}

#[test]
fn live_reload_waits_while_there_is_nothing_to_watch() {
    let mut harness = open(&[("a.md", "# A")]);
    harness.apply(AppAction::CloseTab);
    harness.app.file_explorer = FileExplorer::default();

    harness.apply(AppAction::ToggleWatch);
    for _ in 0..=MAX_WATCHER_RETRIES {
        harness.idle();
    }
    assert!(harness.app.live_reload.enabled);
    assert!(!harness.app.live_reload.is_live());
    assert_eq!(harness.app.error_message, None);

    harness.app.open_in_new_tab(harness.path("a.md"));
    harness.idle();
    assert!(harness.app.live_reload.is_live());
}

#[test]
fn switching_live_reload_off_with_nothing_to_watch_keeps_it_off() {
    let mut harness = open(&[("a.md", "# A")]);
    harness.apply(AppAction::CloseTab);
    harness.app.file_explorer = FileExplorer::default();
    harness.apply(AppAction::ToggleWatch);
    assert!(harness.app.live_reload.is_enabled());
    assert!(!harness.app.live_reload.is_live());

    harness.apply(AppAction::ToggleWatch);
    assert!(!harness.app.live_reload.is_enabled());

    harness.app.open_in_new_tab(harness.path("a.md"));
    harness.idle();
    assert!(!harness.app.live_reload.is_live());
    assert_eq!(harness.app.error_message, None);
}

#[test]
fn a_remote_folder_without_open_documents_is_not_a_watcher_failure() {
    let mut harness = open(&[("a.md", "# A")]);
    harness.apply(AppAction::CloseTab);
    // GVFS folders are never watched, so this leaves nothing to register.
    harness.app.file_explorer = explorer_on("/run/user/1000/gvfs/sftp:host=example/notes");

    harness.apply(AppAction::ToggleWatch);
    for _ in 0..=MAX_WATCHER_RETRIES {
        harness.idle();
    }

    assert_eq!(harness.app.error_message, None);
    assert!(harness.app.live_reload.enabled);
}

#[test]
fn a_watcher_that_cannot_start_is_reported_in_the_error_bar() {
    let mut harness = open(&[("a.md", "# A")]);
    let (_events, receiver) = mpsc::channel();

    harness.app.adopt_watcher(None, receiver);

    let message = harness.app.error_message.clone().unwrap();
    assert!(message.contains("file watcher"), "{message}");
    assert!(!harness.app.live_reload.is_live());
}

#[test]
fn a_local_document_beside_remote_ones_starts_the_inotify_backend() {
    let mut harness = open(&[("a.md", "# A")]);
    // A session that so far watched remote documents: the poll backend alone.
    let remote_only = WatchPlan {
        local_files: Vec::new(),
        gvfs_files: vec![harness.path("a.md")],
        explorer_dirs: Vec::new(),
    };
    let (handler, receiver) = mpsc::channel();
    harness
        .app
        .adopt_watcher(FileWatcher::start(&remote_only, handler), receiver);
    assert!(!running_watcher(&mut harness).runs(Backend::Inotify));

    harness.app.update_watched_paths();

    assert!(running_watcher(&mut harness).runs(Backend::Inotify));
    assert_eq!(
        *harness.app.watched_files(),
        HashSet::from([harness.path("a.md")])
    );
}

fn running_watcher(harness: &mut Harness) -> &mut FileWatcher {
    &mut harness.app.live_reload.running.as_mut().unwrap().watcher
}

#[test]
fn a_watcher_error_restarts_the_watcher() {
    let mut harness = watching(&[("a.md", "old")]);

    fail_watcher(&mut harness);
    harness.idle();
    assert!(harness.app.live_reload.enabled);
    assert_eq!(harness.app.error_message, None);

    fs::write(harness.path("a.md"), "new").unwrap();
    run_until(
        &mut harness,
        "the restarted watcher to reload a.md",
        |harness| harness.app.tabs[0].content == "new",
    );
}

#[test]
fn live_reload_turns_off_after_three_failed_restarts() {
    let mut harness = watching(&[("a.md", "# A")]);
    for _ in 0..MAX_WATCHER_RETRIES {
        fail_watcher(&mut harness);
        harness.idle();
        assert!(harness.app.live_reload.enabled);
    }

    fail_watcher(&mut harness);
    harness.idle();

    assert!(!harness.app.live_reload.enabled);
    assert!(!harness.app.live_reload.is_live());
    let message = harness.app.error_message.clone().unwrap();
    assert!(message.contains("3 retries"), "{message}");
    assert!(message.contains("disk on fire"), "{message}");
}

#[test]
fn a_reported_change_restores_the_retry_budget() {
    let mut harness = watching(&[("a.md", "# A")]);
    for _ in 0..MAX_WATCHER_RETRIES {
        fail_watcher(&mut harness);
        harness.idle();
    }
    let (events, receiver) = mpsc::channel();
    events.send(Ok(Vec::new())).unwrap();
    harness.app.live_reload.running.as_mut().unwrap().events = receiver;
    harness.idle();

    fail_watcher(&mut harness);
    harness.idle();

    assert!(harness.app.live_reload.enabled);
    assert!(harness.app.live_reload.is_live());
}

#[test]
fn a_watcher_that_stopped_is_restarted_on_the_next_frame() {
    let mut harness = watching(&[("a.md", "# A")]);

    harness.app.stop_watching();
    harness.idle();

    assert!(harness.app.live_reload.is_live());
}

#[test]
fn a_watcher_that_keeps_stopping_turns_live_reload_off() {
    let mut harness = watching(&[("a.md", "# A")]);
    for _ in 0..MAX_WATCHER_RETRIES {
        harness.app.stop_watching();
        harness.idle();
    }

    harness.app.stop_watching();
    harness.idle();

    assert!(!harness.app.live_reload.enabled);
    let message = harness.app.error_message.clone().unwrap();
    assert!(message.ends_with("3 retries"), "{message}");
}

#[test]
fn unsaved_edits_are_not_overwritten_by_a_change_on_disk() {
    let mut harness = open(&[("a.md", "on disk")]);
    let doc = harness.path("a.md");
    harness.app.tabs[0].set_edit_mode(true);
    harness.app.tabs[0].content = "my edit".to_owned();

    fs::write(&doc, "changed elsewhere").unwrap();
    harness.app.reload_changed_tabs(vec![doc]);

    assert_eq!(harness.app.tabs[0].content, "my edit");
}

#[test]
fn unsaved_edits_shown_in_the_preview_are_not_overwritten_either() {
    let mut harness = open(&[("a.md", "on disk")]);
    let doc = harness.path("a.md");
    harness.app.tabs[0].set_edit_mode(true);
    harness.app.tabs[0].content = "my edit".to_owned();
    harness.app.tabs[0].set_edit_mode(false);

    fs::write(&doc, "changed elsewhere").unwrap();
    harness.app.reload_changed_tabs(vec![doc]);

    assert_eq!(harness.app.tabs[0].content, "my edit");
}

#[test]
fn an_edit_tab_without_unsaved_changes_follows_the_disk() {
    let mut harness = open(&[("a.md", "on disk")]);
    let doc = harness.path("a.md");
    harness.app.tabs[0].set_edit_mode(true);

    fs::write(&doc, "changed elsewhere").unwrap();
    harness.app.reload_changed_tabs(vec![doc]);

    assert_eq!(harness.app.tabs[0].content, "changed elsewhere");
}

#[test]
fn a_reload_brings_the_find_bar_matches_up_to_date() {
    let mut harness = open(&[("a.md", "needle")]);
    let doc = harness.path("a.md");
    harness.press(Modifiers::CTRL, Key::F);
    harness.app.search.query = "needle".to_owned();
    harness.idle();
    assert_eq!(harness.app.tabs[0].search_matches.len(), 1);

    fs::write(&doc, "needle and needle").unwrap();
    harness.app.reload_changed_tabs(vec![doc]);
    harness.idle();

    assert_eq!(harness.app.tabs[0].search_matches.len(), 2);
}

#[test]
fn a_document_that_can_no_longer_be_read_is_reported() {
    let mut harness = open(&[("a.md", "# A")]);
    let doc = harness.path("a.md");

    fs::remove_file(&doc).unwrap();
    harness.app.reload_changed_tabs(vec![doc]);

    let message = harness.app.error_message.clone().unwrap();
    assert!(message.contains("a.md"), "{message}");
    assert_eq!(harness.app.tabs[0].content, "# A");
}

#[test]
fn a_change_flashes_the_file_and_every_folder_up_to_the_explorer_root() {
    let mut harness = open(&[("a.md", "# A")]);
    let changed = write_doc(&harness, "sub/deep/x.md", "# X");

    harness.app.reload_changed_tabs(vec![changed.clone()]);

    let flashing: HashSet<PathBuf> = harness.app.flashing_paths.keys().cloned().collect();
    let expected = [
        changed,
        harness.path("sub/deep"),
        harness.path("sub"),
        harness.dir.clone(),
    ];
    assert_eq!(flashing, HashSet::from(expected));
}

#[test]
fn a_change_outside_the_explorer_root_flashes_only_the_file() {
    let mut harness = open(&[("a.md", "old")]);
    let doc = harness.path("a.md");
    let elsewhere = write_doc(&harness, "sub/x.md", "# X")
        .parent()
        .unwrap()
        .to_path_buf();
    harness.app.file_explorer = explorer_on(elsewhere);

    fs::write(&doc, "new").unwrap();
    harness.app.reload_changed_tabs(vec![doc.clone()]);

    assert_eq!(harness.app.tabs[0].content, "new");
    let flashing: Vec<&PathBuf> = harness.app.flashing_paths.keys().collect();
    assert_eq!(flashing, [&doc]);
}

#[test]
fn a_change_is_reloaded_without_an_explorer_root() {
    let mut harness = open(&[("a.md", "old")]);
    let doc = harness.path("a.md");
    harness.app.file_explorer = FileExplorer::default();

    fs::write(&doc, "new").unwrap();
    harness.app.reload_changed_tabs(vec![doc.clone()]);

    assert_eq!(harness.app.tabs[0].content, "new");
    assert!(harness.app.flashing_paths.contains_key(&doc));
}
