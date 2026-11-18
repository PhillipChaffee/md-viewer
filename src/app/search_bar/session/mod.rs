//! The find session: opening and closing the bar, keeping the matches
//! those of the document in front, and stepping through them.

use crate::app::MarkdownApp;
use crate::search::{stepped_match_index, SearchState};

#[cfg(test)]
mod tests;

impl MarkdownApp {
    pub(super) fn active_tab_match_count(&self) -> usize {
        self.tabs
            .get(self.active_tab)
            .map_or(0, |tab| tab.search_matches.len())
    }

    /// Show the find bar, or refocus its input when it is already open.
    pub(crate) const fn open_search(&mut self) {
        self.search.is_open = true;
        self.search.focus_requested = true;
    }

    /// Move the active match index forward (dir > 0) or backward (dir < 0), wrapping
    /// around, and request a scroll to the match it lands on.
    pub(crate) fn jump_match(&mut self, dir: i32) {
        let n = self.active_tab_match_count();
        if n == 0 {
            return;
        }
        self.search.active_match_index =
            stepped_match_index(self.search.active_match_index, n, dir);
        self.scroll_to_active_match();
    }

    /// Request a scroll-to-line for the current `active_match_index`. No-op when
    /// there's no active tab, no matches, or content height hasn't been measured yet.
    /// Used both by `jump_match` (cycling) and by `maybe_rebuild_search` (so typing a
    /// fresh query or switching tabs lands the view on the first match instead of
    /// leaving the view at wherever the user previously scrolled).
    fn scroll_to_active_match(&mut self) {
        let index = self.search.active_match_index;
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.jump_to_match(index);
        }
    }

    /// Rebuild the active tab's search matches if the query or active tab has changed.
    /// Cheap (no-op) when nothing changed.
    pub(crate) fn maybe_rebuild_search(&mut self) {
        if !self.search.is_open {
            return;
        }
        let tab_idx = self.active_tab;
        if self.search.is_built_for(tab_idx) {
            return;
        }
        if let Some(tab) = self.tabs.get_mut(tab_idx) {
            tab.rebuild_search(&self.search.query);
        }
        self.search.mark_built_for(tab_idx);
        // Always start at the first match after any rebuild — query change, tab
        // change, or watcher reload. Match indices are not semantically comparable
        // across rebuilds; preserving a numeric index across docs is coincidental
        // (matches Firefox / Chrome / VS Code behavior).
        self.search.active_match_index = 0;
        // Scroll to the new first match so the user sees the active highlight
        // immediately rather than wherever they were previously scrolled. Without
        // this, the visible match (somewhere else on screen) gets the regular
        // highlight color, leaving the user with the impression that the active
        // highlight is "stuck on the previous result".
        self.scroll_to_active_match();
    }

    /// Close the find bar and clear highlight state on every tab.
    pub(crate) fn close_search(&mut self) {
        self.search = SearchState::default();
        for tab in &mut self.tabs {
            tab.search_matches.clear();
            tab.cache.clear_search_ranges();
        }
    }
}
