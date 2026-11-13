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
        .map_or(0, |d| d.as_secs())
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

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const TWO_DAYS: u64 = 2 * DAY;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;

/// Format an epoch-seconds timestamp as a short relative time ("3m ago").
pub(crate) fn format_relative_time(epoch_secs: u64, now: u64) -> String {
    let age = now.saturating_sub(epoch_secs);
    match age {
        ..MINUTE => "just now".to_owned(),
        MINUTE..HOUR => format!("{}m ago", age / MINUTE),
        HOUR..DAY => format!("{}h ago", age / HOUR),
        DAY..TWO_DAYS => "yesterday".to_owned(),
        TWO_DAYS..WEEK => format!("{}d ago", age / DAY),
        WEEK..MONTH => format!("{}w ago", age / WEEK),
        MONTH.. => format!("{}mo ago", age / MONTH),
    }
}
