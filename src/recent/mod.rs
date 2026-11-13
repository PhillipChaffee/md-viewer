//! Recently opened files shown on the home page.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) use summary::DocumentSummary;

mod summary;
#[cfg(test)]
mod tests;

// Home page recent files: how many to keep, and how many to show before "Show more".
pub(crate) const RECENT_FILES_CAP: usize = 20;
pub(crate) const RECENT_SHOWN: usize = 6;

/// A recently opened file, for the home page's "Recently read" list.
///
/// The fields after `last_opened` are what the list shows of the document.
/// A session saved before the list showed them has none, and loads with
/// each of them `None`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecentEntry {
    pub(crate) path: PathBuf,
    /// Seconds since the Unix epoch when the file was last opened.
    pub(crate) last_opened: u64,
    /// The document's first level-one heading.
    #[serde(default)]
    pub(crate) title: Option<String>,
    /// The start of the document's first paragraph, as plain text.
    #[serde(default)]
    pub(crate) excerpt: Option<String>,
    #[serde(default)]
    pub(crate) word_count: Option<u32>,
}

/// Current time in seconds since the Unix epoch (0 if the clock is before epoch).
pub(crate) fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Push a path to the front of the recent list, deduped and capped (most-recent first).
///
/// `summary` is what the document says now. `None` means it could not be
/// read, and what the list already knew about it is kept.
pub(crate) fn push_recent(
    list: &mut Vec<RecentEntry>,
    path: &Path,
    now: u64,
    summary: Option<DocumentSummary>,
) {
    let known = list
        .iter()
        .position(|entry| entry.path.as_path() == path)
        .map(|index| list.remove(index));
    let summary = summary.unwrap_or_else(|| {
        known.map_or_else(DocumentSummary::default, |entry| DocumentSummary {
            title: entry.title,
            excerpt: entry.excerpt,
            word_count: entry.word_count,
        })
    });
    list.insert(
        0,
        RecentEntry {
            path: path.to_path_buf(),
            last_opened: now,
            title: summary.title,
            excerpt: summary.excerpt,
            word_count: summary.word_count,
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
