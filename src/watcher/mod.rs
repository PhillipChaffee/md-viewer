//! File-watcher backends and retry policy.

use std::path::Path;

use notify::{PollWatcher, RecommendedWatcher};
use notify_debouncer_mini::Debouncer;

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
    pub(crate) fn inotify_watcher(&mut self) -> Option<&mut dyn notify::Watcher> {
        match self {
            FileWatcher::Inotify(d) | FileWatcher::Dual { inotify: d, .. } => Some(d.watcher()),
            FileWatcher::Poll(_) => None,
        }
    }

    pub(crate) fn poll_watcher(&mut self) -> Option<&mut dyn notify::Watcher> {
        match self {
            FileWatcher::Poll(d) | FileWatcher::Dual { poll: d, .. } => Some(d.watcher()),
            FileWatcher::Inotify(_) => None,
        }
    }
}
