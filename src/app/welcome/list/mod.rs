//! The home page's list of recently read documents, as the page draws it.
//! It is built when the recent files change, never while painting.

use std::path::{Path, PathBuf};

use crate::recent::{format_relative_time, RecentEntry, RECENT_FILES_CAP, RECENT_SHOWN};
use crate::tab::minutes_for_words;

#[cfg(test)]
mod tests;

/// How long a list is shown before it is built again, in seconds: its
/// "opened ... ago" labels age, and a listed file can go away.
const REFRESH_SECS: u64 = 30;

/// One recently read document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HomeRow {
    pub(super) path: PathBuf,
    /// The document's title, or without one the file's name less its
    /// extension.
    pub(super) title: String,
    /// The folder the file is in, how long it takes to read, and when it
    /// was last opened.
    pub(super) meta: String,
    /// The opening of the document's first paragraph.
    pub(super) excerpt: Option<String>,
    /// The file is still there. One that is gone cannot be opened.
    pub(super) exists: bool,
}

impl HomeRow {
    fn new(entry: &RecentEntry, exists: bool, now: u64) -> Self {
        let folder = entry.path.parent().and_then(Path::file_name);
        let folder = folder.map(|name| name.to_string_lossy().into_owned());
        let length = entry.word_count.map(|words| {
            let words = usize::try_from(words).unwrap_or(usize::MAX);
            format!("{} min read", minutes_for_words(words))
        });
        let opened = if exists {
            format!("opened {}", format_relative_time(entry.last_opened, now))
        } else {
            "file not found".to_owned()
        };
        let meta: Vec<String> = [folder, length, Some(opened)]
            .into_iter()
            .flatten()
            .collect();
        Self {
            path: entry.path.clone(),
            title: entry
                .title
                .clone()
                .unwrap_or_else(|| file_stem(&entry.path)),
            meta: meta.join(" · "),
            excerpt: entry.excerpt.clone(),
            exists,
        }
    }
}

/// The name of the file at `path` without its extension.
fn file_stem(path: &Path) -> String {
    let name = path.file_stem().unwrap_or(path.as_os_str());
    name.to_string_lossy().into_owned()
}

/// The recently read documents the home page lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HomeList {
    pub(super) rows: Vec<HomeRow>,
    /// More documents are remembered than the short list shows.
    pub(super) has_more: bool,
    /// The whole list is shown, not only its start.
    pub(super) show_all: bool,
    /// Epoch seconds the list was built at.
    built_at: u64,
}

impl HomeList {
    /// The list in `slot`, built now from `recents` when the slot is empty
    /// (a change to the recent files empties it) or what it holds has aged.
    pub(super) fn current<'slot>(
        slot: &'slot mut Option<Self>,
        recents: &[RecentEntry],
        show_all: bool,
        now: u64,
        exists: impl Fn(&Path) -> bool,
    ) -> &'slot Self {
        if slot.as_ref().is_some_and(|list| list.is_stale(now)) {
            *slot = None;
        }
        slot.get_or_insert_with(|| Self::build(recents, show_all, now, exists))
    }

    /// The list of `recents`, most recent first, as of `now`. `exists`
    /// says whether a file is still there.
    fn build(
        recents: &[RecentEntry],
        show_all: bool,
        now: u64,
        exists: impl Fn(&Path) -> bool,
    ) -> Self {
        let shown = if show_all {
            RECENT_FILES_CAP
        } else {
            RECENT_SHOWN
        };
        let rows = recents.iter().take(shown);
        Self {
            rows: rows
                .map(|entry| HomeRow::new(entry, exists(&entry.path), now))
                .collect(),
            has_more: recents.len() > RECENT_SHOWN,
            show_all,
            built_at: now,
        }
    }

    /// Whether the list was built long enough before `now` to be built
    /// again.
    const fn is_stale(&self, now: u64) -> bool {
        now.saturating_sub(self.built_at) >= REFRESH_SECS
    }
}
