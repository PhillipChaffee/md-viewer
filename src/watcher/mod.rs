//! File-watcher backends, retry policy, and the rules for what to watch.
//!
//! The rules are pure functions over paths, so they are tested without a
//! filesystem watcher; [`FileWatcher`] is the only part that talks to `notify`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use notify::{PollWatcher, RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind, Debouncer};

#[cfg(test)]
mod tests;

pub(crate) const MAX_WATCHER_RETRIES: u32 = 3;
pub(crate) const FLASH_DURATION_MS: u64 = 600;

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
pub(crate) fn watch_path(watcher: &mut dyn notify::Watcher, path: &Path, what: &str) -> bool {
    match watcher.watch(path, RecursiveMode::NonRecursive) {
        Ok(()) => true,
        Err(error) => {
            log::error!("Failed to watch {what} {}: {error}", path.display());
            false
        }
    }
}

/// Wrapper for file watchers that supports both inotify (local) and poll (GVFS/remote).
pub(crate) enum FileWatcher {
    Inotify(Debouncer<RecommendedWatcher>),
    Poll(Debouncer<PollWatcher>),
    Dual {
        inotify: Debouncer<RecommendedWatcher>,
        poll: Debouncer<PollWatcher>,
    },
}

impl FileWatcher {
    /// Wraps whichever backends started; `None` when neither did.
    pub(crate) fn from_backends(
        inotify: Option<Debouncer<RecommendedWatcher>>,
        poll: Option<Debouncer<PollWatcher>>,
    ) -> Option<Self> {
        match (inotify, poll) {
            (Some(inotify), Some(poll)) => Some(Self::Dual { inotify, poll }),
            (Some(inotify), None) => Some(Self::Inotify(inotify)),
            (None, Some(poll)) => Some(Self::Poll(poll)),
            (None, None) => None,
        }
    }

    pub(crate) fn inotify_watcher(&mut self) -> Option<&mut dyn notify::Watcher> {
        match self {
            Self::Inotify(d) | Self::Dual { inotify: d, .. } => Some(d.watcher()),
            Self::Poll(_) => None,
        }
    }

    pub(crate) fn poll_watcher(&mut self) -> Option<&mut dyn notify::Watcher> {
        match self {
            Self::Poll(d) | Self::Dual { poll: d, .. } => Some(d.watcher()),
            Self::Inotify(_) => None,
        }
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
