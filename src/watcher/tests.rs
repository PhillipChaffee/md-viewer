use super::*;

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
