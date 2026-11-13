//! File-watcher backends, retry policy, and the rules for what to watch.
//!
//! The rules are pure functions over paths, so they are tested without a
//! filesystem watcher; [`FileWatcher`] is the only part that talks to `notify`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{PollWatcher, RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{
    new_debouncer, new_debouncer_opt, DebounceEventHandler, DebouncedEvent, DebouncedEventKind,
    Debouncer,
};

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

pub(crate) const MAX_WATCHER_RETRIES: u32 = 3;
pub(crate) const FLASH_DURATION_MS: u64 = 600;
/// Quiet time after the last write before a change is reported.
const DEBOUNCE: Duration = Duration::from_millis(200);
/// How often GVFS documents are checked. Each check is a remote round trip.
const GVFS_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub(crate) fn next_watcher_retry(current: u32) -> Option<u32> {
    (current < MAX_WATCHER_RETRIES).then(|| current + 1)
}

/// Check if a path is on a GVFS FUSE mount (e.g., SFTP via Thunar/Nautilus).
pub(crate) fn is_gvfs_path(path: &Path) -> bool {
    path.starts_with("/run/user/") && path.components().any(|c| c.as_os_str() == "gvfs")
}

/// The watcher backend a path needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Backend {
    /// Kernel change notifications, for local filesystems.
    Inotify,
    /// Periodic polling, for GVFS mounts that deliver no notifications.
    Poll,
}

impl Backend {
    pub(crate) fn for_path(path: &Path) -> Self {
        if is_gvfs_path(path) {
            Self::Poll
        } else {
            Self::Inotify
        }
    }
}

/// Everything a fresh watcher start registers, split by backend.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct WatchPlan {
    /// Open documents on local filesystems.
    pub(crate) local_files: Vec<PathBuf>,
    /// Open documents on GVFS mounts.
    pub(crate) gvfs_files: Vec<PathBuf>,
    /// Explorer directories, watched non-recursively through inotify.
    pub(crate) explorer_dirs: Vec<PathBuf>,
}

impl WatchPlan {
    pub(crate) fn new(open_files: Vec<PathBuf>, explorer_dirs: Vec<PathBuf>) -> Self {
        let (gvfs_files, local_files) = open_files.into_iter().partition(|p| is_gvfs_path(p));
        Self {
            local_files,
            gvfs_files,
            explorer_dirs,
        }
    }

    pub(crate) const fn needs_inotify(&self) -> bool {
        !self.local_files.is_empty() || !self.explorer_dirs.is_empty()
    }

    pub(crate) const fn needs_poll(&self) -> bool {
        !self.gvfs_files.is_empty()
    }

    /// Whether there is nothing to register, so no backend has to run.
    pub(crate) const fn is_empty(&self) -> bool {
        !self.needs_inotify() && !self.needs_poll()
    }
}

/// The explorer directories to watch: a local root plus each expanded local
/// directory that `is_dir` still finds.
///
/// The set mirrors the lazy explorer tree. Watching the root recursively walks
/// its whole subtree on the main thread at startup (a ~6s hang on a home
/// directory of ~455k directories), and only the directories the user can see
/// need change events. A GVFS root stays unwatched: polling a remote tree
/// costs an SFTP round trip per entry, and the explorer has a manual refresh.
pub(crate) fn explorer_watch_dirs(
    root: Option<&Path>,
    expanded: &HashSet<PathBuf>,
    is_dir: impl Fn(&Path) -> bool,
) -> HashSet<PathBuf> {
    let Some(root) = root.filter(|root| !is_gvfs_path(root)) else {
        return HashSet::new();
    };
    expanded
        .iter()
        .filter(|dir| !is_gvfs_path(dir) && is_dir(dir))
        .cloned()
        .chain(std::iter::once(root.to_path_buf()))
        .collect()
}

/// The registrations that move a watcher from one path set to another.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct WatchDiff {
    /// Paths to start watching, sorted.
    pub(crate) add: Vec<PathBuf>,
    /// Paths to stop watching, sorted.
    pub(crate) remove: Vec<PathBuf>,
}

pub(crate) fn watch_diff(desired: &HashSet<PathBuf>, watched: &HashSet<PathBuf>) -> WatchDiff {
    let sorted_difference = |from: &HashSet<PathBuf>, minus: &HashSet<PathBuf>| {
        let mut paths: Vec<PathBuf> = from.difference(minus).cloned().collect();
        paths.sort();
        paths
    };
    WatchDiff {
        add: sorted_difference(desired, watched),
        remove: sorted_difference(watched, desired),
    }
}

/// Whether `open_files` need a backend the running watcher does not have, for
/// example when the first GVFS document opens beside local ones.
pub(crate) fn needs_backend_restart(
    open_files: &HashSet<PathBuf>,
    has_inotify: bool,
    has_poll: bool,
) -> bool {
    let needs = |backend| {
        open_files
            .iter()
            .any(|path| Backend::for_path(path) == backend)
    };
    (needs(Backend::Poll) && !has_poll) || (needs(Backend::Inotify) && !has_inotify)
}

/// The paths whose burst of changes has ended. Events still inside a burst
/// (`AnyContinuous`) are dropped; the debouncer reports the path again once
/// the writes stop.
pub(crate) fn settled_paths(events: Vec<DebouncedEvent>) -> impl Iterator<Item = PathBuf> {
    events
        .into_iter()
        .filter(|event| event.kind == DebouncedEventKind::Any)
        .map(|event| event.path)
}

/// The directories between `changed` and the explorer `root`, nearest first
/// and ending with `root` itself. Empty when `changed` lies outside `root`.
pub(crate) fn ancestors_within<'a>(
    changed: &'a Path,
    root: &'a Path,
) -> impl Iterator<Item = &'a Path> {
    changed
        .ancestors()
        .skip(1)
        .take_while(move |dir| dir.starts_with(root))
}

/// The explorer directory whose listing a change to `changed` can alter, if
/// `changed` lies under `root`.
pub(crate) fn listing_to_refresh<'a>(changed: &'a Path, root: &Path) -> Option<&'a Path> {
    changed
        .starts_with(root)
        .then(|| changed.parent())
        .flatten()
}

/// Registers `path` non-recursively. A refusal is logged, naming the path as
/// a `what`, and reported as `false`.
fn watch_path(watcher: &mut dyn notify::Watcher, path: &Path, what: &str) -> bool {
    match watcher.watch(path, RecursiveMode::NonRecursive) {
        Ok(()) => true,
        Err(error) => {
            log::error!("Failed to watch {what} {}: {error}", path.display());
            false
        }
    }
}

/// Registers each of `paths`, yielding the ones the watcher accepted.
fn watch_all<'a>(
    watcher: &'a mut dyn notify::Watcher,
    paths: &'a [PathBuf],
    what: &'a str,
) -> impl Iterator<Item = PathBuf> + 'a {
    paths
        .iter()
        .filter(move |path| watch_path(watcher, path, what))
        .cloned()
}

/// The backend that was `created`, or `None` after logging why the backend
/// named `what` could not start.
fn started<W: notify::Watcher>(
    created: notify::Result<Debouncer<W>>,
    what: &str,
) -> Option<Debouncer<W>> {
    created
        .inspect_err(|error| log::error!("Failed to create {what} watcher: {error}"))
        .ok()
}

/// Starts the inotify backend and records in `accepted` which of the plan's
/// local paths it accepted.
fn start_inotify(
    plan: &WatchPlan,
    on_events: impl DebounceEventHandler,
    accepted: &mut Watched,
) -> Option<Debouncer<RecommendedWatcher>> {
    let mut debouncer = started(new_debouncer(DEBOUNCE, on_events), "inotify")?;
    let watcher = debouncer.watcher();
    accepted
        .files
        .extend(watch_all(watcher, &plan.local_files, "file"));
    accepted
        .explorer_dirs
        .extend(watch_all(watcher, &plan.explorer_dirs, "explorer dir"));
    Some(debouncer)
}

/// Starts the polling backend and records in `accepted` which of the plan's
/// GVFS documents it accepted.
fn start_poll(
    plan: &WatchPlan,
    on_events: impl DebounceEventHandler,
    accepted: &mut Watched,
) -> Option<Debouncer<PollWatcher>> {
    let config = notify_debouncer_mini::Config::default()
        .with_timeout(DEBOUNCE)
        .with_notify_config(notify::Config::default().with_poll_interval(GVFS_POLL_INTERVAL));
    let created = new_debouncer_opt::<_, PollWatcher>(config, on_events);
    let mut debouncer = started(created, "poll")?;
    let watcher = debouncer.watcher();
    accepted
        .files
        .extend(watch_all(watcher, &plan.gvfs_files, "polled file"));
    Some(debouncer)
}

/// The paths a watcher start registered with its backends.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Watched {
    /// Open documents, local and GVFS.
    pub(crate) files: HashSet<PathBuf>,
    /// Explorer directories.
    pub(crate) explorer_dirs: HashSet<PathBuf>,
}

/// The running watcher backends: inotify for local paths, polling for GVFS
/// documents. At least one runs.
pub(crate) struct FileWatcher {
    inotify: Option<Debouncer<RecommendedWatcher>>,
    poll: Option<Debouncer<PollWatcher>>,
}

impl FileWatcher {
    /// Starts the backends `plan` has paths for and registers those paths
    /// non-recursively. `on_events` runs on a watcher thread with each
    /// debounced batch of changes, or with the error that stopped a backend.
    ///
    /// Returns the watcher with the paths its backends accepted, or `None`
    /// when no backend could start.
    pub(crate) fn start(
        plan: &WatchPlan,
        on_events: impl DebounceEventHandler + Clone,
    ) -> Option<(Self, Watched)> {
        let mut accepted = Watched::default();
        let inotify = plan
            .needs_inotify()
            .then(|| start_inotify(plan, on_events.clone(), &mut accepted))
            .flatten();
        let poll = plan
            .needs_poll()
            .then(|| start_poll(plan, on_events, &mut accepted))
            .flatten();
        (inotify.is_some() || poll.is_some()).then_some((Self { inotify, poll }, accepted))
    }

    pub(crate) fn inotify_watcher(&mut self) -> Option<&mut dyn notify::Watcher> {
        let debouncer = self.inotify.as_mut()?;
        Some(debouncer.watcher())
    }

    pub(crate) fn poll_watcher(&mut self) -> Option<&mut dyn notify::Watcher> {
        let debouncer = self.poll.as_mut()?;
        Some(debouncer.watcher())
    }

    fn backend(&mut self, backend: Backend) -> Option<&mut dyn notify::Watcher> {
        match backend {
            Backend::Inotify => self.inotify_watcher(),
            Backend::Poll => self.poll_watcher(),
        }
    }

    /// Applies `diff` through the backend each path needs. A path whose
    /// backend is not running is skipped; `what` names the paths in the log.
    pub(crate) fn apply(&mut self, diff: &WatchDiff, what: &str) {
        for path in &diff.add {
            if let Some(watcher) = self.backend(Backend::for_path(path)) {
                watch_path(watcher, path, what);
            }
        }
        for path in &diff.remove {
            if let Some(watcher) = self.backend(Backend::for_path(path)) {
                // The path is often gone already, which notify reports as an
                // error; there is nothing to undo either way.
                let _ = watcher.unwatch(path);
            }
        }
    }
}
