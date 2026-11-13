//! Recently opened files shown on the welcome page.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

// Welcome page recent-files: how many to keep, and how many to show before "Show more".
pub(crate) const RECENT_FILES_CAP: usize = 20;
pub(crate) const RECENT_SHOWN: usize = 6;

/// A recently opened file, for the welcome page's "Recent" list.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RecentEntry {
    pub(crate) path: PathBuf,
    /// Seconds since the Unix epoch when the file was last opened.
    pub(crate) last_opened: u64,
}

/// Current time in seconds since the Unix epoch (0 if the clock is before epoch).
pub(crate) fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Push a path to the front of the recent list, deduped and capped (most-recent first).
pub(crate) fn push_recent(list: &mut Vec<RecentEntry>, path: &Path, now: u64) {
    list.retain(|e| e.path.as_path() != path);
    list.insert(
        0,
        RecentEntry {
            path: path.to_path_buf(),
            last_opened: now,
        },
    );
    list.truncate(RECENT_FILES_CAP);
}

/// Format an epoch-seconds timestamp as a short relative time ("3m ago").
pub(crate) fn format_relative_time(epoch_secs: u64, now: u64) -> String {
    let diff = now.saturating_sub(epoch_secs);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 172_800 {
        "yesterday".to_string()
    } else if diff < 604_800 {
        format!("{}d ago", diff / 86_400)
    } else if diff < 2_592_000 {
        format!("{}w ago", diff / 604_800)
    } else {
        format!("{}mo ago", diff / 2_592_000)
    }
}
