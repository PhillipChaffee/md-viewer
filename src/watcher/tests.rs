use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use notify_debouncer_mini::DebounceEventResult;

use super::*;

/// How long a test waits for the watcher before it fails.
const EVENT_TIMEOUT: Duration = Duration::from_secs(10);

static NEXT_SCRATCH_DIR: AtomicUsize = AtomicUsize::new(0);

/// A directory of real files for the watcher backends, removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> Self {
        // Format log arguments as a run with `RUST_LOG` set does, so one
        // that panics fails a test.
        log::set_max_level(log::LevelFilter::Trace);
        let dir = std::env::temp_dir().join(format!(
            "md-viewer-watcher-test-{}-{}",
            std::process::id(),
            NEXT_SCRATCH_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir.canonicalize().unwrap())
    }

    fn file(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, "before").unwrap();
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A plan whose `polled` files go to the poll backend, as GVFS documents do.
fn plan_with_polled(local: &[&Path], polled: &[&Path]) -> WatchPlan {
    let owned = |files: &[&Path]| files.iter().map(|file| file.to_path_buf()).collect();
    WatchPlan {
        local_files: owned(local),
        gvfs_files: owned(polled),
        explorer_dirs: Vec::new(),
    }
}

/// Every settled change the watcher reports up to and including the first
/// one to `last`. Fails when `last` is not reported in time.
fn changes_until(events: &Receiver<DebounceEventResult>, last: &Path) -> Vec<PathBuf> {
    let deadline = Instant::now() + EVENT_TIMEOUT;
    let mut changed = Vec::new();
    while !changed.iter().any(|path| path == last) {
        let wait = deadline.saturating_duration_since(Instant::now());
        let batch = events.recv_timeout(wait).unwrap_or_else(|_| {
            panic!(
                "no change to {} was reported within {EVENT_TIMEOUT:?}",
                last.display()
            )
        });
        changed.extend(settled_paths(batch.unwrap()));
    }
    changed
}

#[test]
fn starting_a_plan_registers_its_documents_and_explorer_directories() {
    let dir = ScratchDir::new();
    let doc = dir.file("a.md");
    let plan = WatchPlan::new(vec![doc.clone()], vec![dir.0.clone()]);
    let (handler, _events) = mpsc::channel();

    let (mut watcher, accepted) = FileWatcher::start(&plan, handler).unwrap();

    assert_eq!(
        accepted,
        Watched {
            files: HashSet::from([doc]),
            explorer_dirs: HashSet::from([dir.0.clone()]),
        }
    );
    assert!(watcher.inotify_watcher().is_some());
    assert!(watcher.poll_watcher().is_none());
}

#[test]
fn a_started_watcher_reports_a_document_written_afterwards() {
    let dir = ScratchDir::new();
    let doc = dir.file("a.md");
    let (handler, events) = mpsc::channel();
    let _watcher = FileWatcher::start(&plan_with_polled(&[&doc], &[]), handler).unwrap();

    fs::write(&doc, "after").unwrap();

    assert_eq!(changes_until(&events, &doc), [doc]);
}

#[test]
fn a_watched_directory_reports_a_file_created_in_it() {
    let dir = ScratchDir::new();
    let plan = WatchPlan::new(Vec::new(), vec![dir.0.clone()]);
    let (handler, events) = mpsc::channel();
    let _watcher = FileWatcher::start(&plan, handler).unwrap();

    let created = dir.file("new.md");

    assert!(changes_until(&events, &created).contains(&created));
}

#[test]
fn a_path_that_cannot_be_watched_is_left_out_of_what_was_registered() {
    let dir = ScratchDir::new();
    let doc = dir.file("a.md");
    let gone = dir.0.join("deleted.md");
    let (handler, _events) = mpsc::channel();

    let (_watcher, accepted) =
        FileWatcher::start(&plan_with_polled(&[&gone, &doc], &[]), handler).unwrap();

    assert_eq!(accepted.files, HashSet::from([doc]));
}

#[test]
fn polled_documents_start_the_poll_backend_alone() {
    let dir = ScratchDir::new();
    let doc = dir.file("remote.md");
    let (handler, _events) = mpsc::channel();

    let (mut watcher, accepted) =
        FileWatcher::start(&plan_with_polled(&[], &[&doc]), handler).unwrap();

    assert_eq!(accepted.files, HashSet::from([doc.clone()]));
    assert!(watcher.inotify_watcher().is_none());
    // The poll backend holds the document: only a watched path can be unwatched.
    assert!(watcher.poll_watcher().unwrap().unwatch(&doc).is_ok());
}

#[test]
fn local_and_polled_documents_start_both_backends() {
    let dir = ScratchDir::new();
    let (local, remote) = (dir.file("local.md"), dir.file("remote.md"));
    let (handler, _events) = mpsc::channel();

    let (mut watcher, accepted) =
        FileWatcher::start(&plan_with_polled(&[&local], &[&remote]), handler).unwrap();

    assert_eq!(
        accepted.files,
        HashSet::from([local.clone(), remote.clone()])
    );
    assert!(watcher.inotify_watcher().unwrap().unwatch(&local).is_ok());
    assert!(watcher.poll_watcher().unwrap().unwatch(&remote).is_ok());
}

#[test]
fn a_plan_with_nothing_to_watch_starts_no_watcher() {
    let plan = WatchPlan::new(Vec::new(), Vec::new());
    let (handler, _events) = mpsc::channel();

    assert!(plan.is_empty());
    assert!(FileWatcher::start(&plan, handler).is_none());
}

#[test]
fn a_backend_that_cannot_be_created_is_reported_as_not_started() {
    let failed = Err(notify::Error::generic("no inotify instances left"));
    assert!(started::<RecommendedWatcher>(failed, "inotify").is_none());

    let (handler, _events) = mpsc::channel::<DebounceEventResult>();
    assert!(started(new_debouncer(DEBOUNCE, handler), "inotify").is_some());
}

#[test]
fn applying_a_diff_watches_added_documents_and_drops_removed_ones() {
    let dir = ScratchDir::new();
    let (closed, opened) = (dir.file("closed.md"), dir.file("opened.md"));
    let (handler, events) = mpsc::channel();
    let (mut watcher, _) = FileWatcher::start(&plan_with_polled(&[&closed], &[]), handler).unwrap();

    let diff = WatchDiff {
        add: vec![opened.clone()],
        remove: vec![closed.clone()],
    };
    watcher.apply(&diff, "file");
    // Written first, so a change to it would be reported no later than the
    // change to the document that is still watched.
    fs::write(&closed, "after").unwrap();
    fs::write(&opened, "after").unwrap();

    assert_eq!(changes_until(&events, &opened), [opened]);
}

#[test]
fn a_diff_skips_documents_whose_backend_is_not_running() {
    let dir = ScratchDir::new();
    let (remote, local) = (dir.file("remote.md"), dir.file("local.md"));
    let (handler, _events) = mpsc::channel();
    let (mut watcher, _) = FileWatcher::start(&plan_with_polled(&[], &[&remote]), handler).unwrap();

    let diff = WatchDiff {
        add: vec![local.clone()],
        remove: Vec::new(),
    };
    watcher.apply(&diff, "file");

    // A local document needs inotify; the poll backend must not take it.
    assert!(watcher.poll_watcher().unwrap().unwatch(&local).is_err());
}

#[test]
fn a_gvfs_document_added_by_a_diff_goes_to_the_poll_backend() {
    let dir = ScratchDir::new();
    let (local, remote) = (dir.file("local.md"), dir.file("remote.md"));
    let (handler, events) = mpsc::channel();
    let (mut watcher, _) =
        FileWatcher::start(&plan_with_polled(&[&local], &[&remote]), handler).unwrap();

    let diff = WatchDiff {
        add: paths([GVFS_DOC]),
        remove: Vec::new(),
    };
    watcher.apply(&diff, "file");

    // Only the poll backend reports a path it cannot read through the event
    // handler; inotify refuses such a path without an event.
    let reported = events.recv_timeout(EVENT_TIMEOUT).unwrap();
    assert!(reported.is_err(), "expected a poll error, got {reported:?}");
}

#[test]
fn watcher_retry_sequence_stops_at_configured_limit() {
    assert_eq!(next_watcher_retry(0), Some(1));
    assert_eq!(next_watcher_retry(1), Some(2));
    assert_eq!(next_watcher_retry(2), Some(3));
    assert_eq!(next_watcher_retry(3), None);
    assert_eq!(next_watcher_retry(u32::MAX), None);
}

const GVFS_DOC: &str = "/run/user/1000/gvfs/sftp:host=example/notes/remote.md";

fn paths<const N: usize>(items: [&str; N]) -> Vec<PathBuf> {
    items.into_iter().map(PathBuf::from).collect()
}

fn path_set<const N: usize>(items: [&str; N]) -> HashSet<PathBuf> {
    items.into_iter().map(PathBuf::from).collect()
}

#[test]
fn gvfs_documents_are_polled_and_local_ones_use_inotify() {
    assert_eq!(Backend::for_path(Path::new(GVFS_DOC)), Backend::Poll);
    assert_eq!(
        Backend::for_path(Path::new("/home/me/notes/local.md")),
        Backend::Inotify
    );
    // A runtime directory that is not a GVFS mount is an ordinary local path.
    assert_eq!(
        Backend::for_path(Path::new("/run/user/1000/doc/local.md")),
        Backend::Inotify
    );
}

#[test]
fn watch_plan_splits_open_files_by_backend() {
    let plan = WatchPlan::new(
        paths(["/home/me/a.md", GVFS_DOC, "/home/me/b.md"]),
        paths(["/home/me"]),
    );

    assert_eq!(plan.local_files, paths(["/home/me/a.md", "/home/me/b.md"]));
    assert_eq!(plan.gvfs_files, paths([GVFS_DOC]));
    assert_eq!(plan.explorer_dirs, paths(["/home/me"]));
    assert!(plan.needs_inotify());
    assert!(plan.needs_poll());
}

#[test]
fn watch_plan_starts_only_the_backends_it_has_paths_for() {
    let remote_only = WatchPlan::new(paths([GVFS_DOC]), Vec::new());
    assert!(!remote_only.needs_inotify());
    assert!(remote_only.needs_poll());

    let explorer_only = WatchPlan::new(Vec::new(), paths(["/home/me"]));
    assert!(explorer_only.needs_inotify());
    assert!(!explorer_only.needs_poll());

    let nothing = WatchPlan::new(Vec::new(), Vec::new());
    assert!(!nothing.needs_inotify());
    assert!(!nothing.needs_poll());
}

#[test]
fn explorer_watches_cover_the_root_and_its_expanded_directories() {
    let expanded = path_set(["/home/me/docs", "/home/me/docs/api"]);

    let dirs = explorer_watch_dirs(Some(Path::new("/home/me")), &expanded, |_| true);

    assert_eq!(
        dirs,
        path_set(["/home/me", "/home/me/docs", "/home/me/docs/api"])
    );
}

#[test]
fn explorer_watches_skip_vanished_and_remote_directories() {
    let expanded = path_set([
        "/home/me/docs",
        "/home/me/deleted",
        "/run/user/1000/gvfs/sftp:host=example/notes",
    ]);

    let dirs = explorer_watch_dirs(Some(Path::new("/home/me")), &expanded, |dir| {
        dir != Path::new("/home/me/deleted")
    });

    assert_eq!(dirs, path_set(["/home/me", "/home/me/docs"]));
}

#[test]
fn explorer_watches_are_empty_without_a_local_root() {
    let expanded = path_set(["/home/me/docs"]);
    let gvfs_root = Path::new("/run/user/1000/gvfs/sftp:host=example/notes");

    assert!(explorer_watch_dirs(None, &expanded, |_| true).is_empty());
    assert!(explorer_watch_dirs(Some(gvfs_root), &expanded, |_| true).is_empty());
}

#[test]
fn watch_diff_adds_new_paths_and_removes_stale_ones() {
    let desired = path_set(["/d/keep.md", "/d/new-b.md", "/d/new-a.md"]);
    let watched = path_set(["/d/keep.md", "/d/closed.md"]);

    let diff = watch_diff(&desired, &watched);

    assert_eq!(diff.add, paths(["/d/new-a.md", "/d/new-b.md"]));
    assert_eq!(diff.remove, paths(["/d/closed.md"]));
}

#[test]
fn watch_diff_is_empty_when_nothing_changed() {
    let watched = path_set(["/d/a.md", "/d/b.md"]);

    assert_eq!(watch_diff(&watched, &watched), WatchDiff::default());
}

#[test]
fn restart_is_needed_only_for_a_backend_that_is_not_running() {
    let local = path_set(["/home/me/a.md"]);
    let remote = path_set([GVFS_DOC]);
    let mixed = path_set(["/home/me/a.md", GVFS_DOC]);

    // (open files, has inotify, has poll) -> restart
    assert!(!needs_backend_restart(&local, true, false));
    assert!(needs_backend_restart(&local, false, true));
    assert!(!needs_backend_restart(&remote, false, true));
    assert!(needs_backend_restart(&remote, true, false));
    assert!(needs_backend_restart(&mixed, true, false));
    assert!(needs_backend_restart(&mixed, false, true));
    assert!(!needs_backend_restart(&mixed, true, true));
    assert!(!needs_backend_restart(&HashSet::new(), false, false));
}

#[test]
fn only_settled_change_events_are_reported() {
    let event = |path: &str, kind| DebouncedEvent {
        path: PathBuf::from(path),
        kind,
    };
    let events = vec![
        event("/d/done.md", DebouncedEventKind::Any),
        event("/d/still-writing.md", DebouncedEventKind::AnyContinuous),
        event("/d/also-done.md", DebouncedEventKind::Any),
    ];

    let settled: Vec<PathBuf> = settled_paths(events).collect();

    assert_eq!(settled, paths(["/d/done.md", "/d/also-done.md"]));
}

#[test]
fn ancestors_within_root_run_from_the_parent_up_to_the_root() {
    let ancestors: Vec<&Path> = ancestors_within(
        Path::new("/home/me/docs/api/index.md"),
        Path::new("/home/me"),
    )
    .collect();

    assert_eq!(
        ancestors,
        [
            Path::new("/home/me/docs/api"),
            Path::new("/home/me/docs"),
            Path::new("/home/me"),
        ]
    );
}

#[test]
fn a_change_outside_the_root_has_no_ancestors_within_it() {
    let root = Path::new("/home/me");

    assert_eq!(
        ancestors_within(Path::new("/tmp/other/file.md"), root).count(),
        0
    );
    // A sibling whose name merely starts with the root's name is outside it.
    assert_eq!(
        ancestors_within(Path::new("/home/meadow/file.md"), root).count(),
        0
    );
    assert_eq!(ancestors_within(root, root).count(), 0);
}

#[test]
fn a_change_under_the_root_refreshes_its_parent_listing() {
    let root = Path::new("/home/me");

    assert_eq!(
        listing_to_refresh(Path::new("/home/me/docs/new.md"), root),
        Some(Path::new("/home/me/docs"))
    );
    assert_eq!(
        listing_to_refresh(Path::new("/home/me/new.md"), root),
        Some(root)
    );
    assert_eq!(listing_to_refresh(Path::new("/tmp/new.md"), root), None);
}
