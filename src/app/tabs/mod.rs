//! Opening, closing, switching, and navigating tabs.

use std::io;
use std::path::{Path, PathBuf};

use egui_commonmark_extended::animation;

use crate::app::MarkdownApp;
use crate::recent::{now_epoch_secs, push_recent, DocumentSummary};
use crate::tab::{name_tabs, Route, Tab};

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// The active tab's index once the tab at `closed` is gone and `remaining`
/// tabs are left: the same document stays active where it can, the last
/// tab takes over where the active one was the last, and an empty strip
/// rests at 0.
const fn active_after_close(active: usize, closed: usize, remaining: usize) -> usize {
    if remaining == 0 {
        0
    } else if active >= remaining {
        remaining - 1
    } else if active > closed {
        active - 1
    } else {
        active
    }
}

/// The index after `active` among `len` tabs, wrapping to the first.
const fn next_index(active: usize, len: usize) -> usize {
    (active + 1) % len
}

/// The index before `active` among `len` tabs, wrapping to the last.
const fn previous_index(active: usize, len: usize) -> usize {
    if active == 0 {
        len - 1
    } else {
        active - 1
    }
}

/// Asks the user for a path, with the dialog starting in the given folder.
/// `Ok(None)` means the user cancelled.
pub(crate) type PathPicker = Box<dyn Fn(Option<&Path>) -> Result<Option<PathBuf>, String>>;

impl MarkdownApp {
    /// The place of the tab in front, or `None` while the home page
    /// shows: with no tab open, or over the open ones.
    pub(crate) const fn front(&self) -> Option<usize> {
        if self.home_shows || self.active_tab >= self.tabs.len() {
            None
        } else {
            Some(self.active_tab)
        }
    }

    /// The tab in front, unless the home page shows.
    pub(crate) fn front_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.front()?)
    }

    /// The tab in front, unless the home page shows.
    pub(crate) fn front_tab_mut(&mut self) -> Option<&mut Tab> {
        let front = self.front()?;
        self.tabs.get_mut(front)
    }

    /// Show the home page over the open tabs, which stay open. With no
    /// tab open the home page shows already.
    pub(crate) const fn show_home(&mut self) {
        if !self.tabs.is_empty() && !self.home_shows {
            self.home_shows = true;
            self.window_title.mark_stale();
        }
    }

    /// Show the home page, or leave it for the tab that was in front.
    pub(crate) const fn toggle_home(&mut self) {
        if self.home_shows {
            self.activate_tab(self.active_tab);
        } else {
            self.show_home();
        }
    }

    pub(crate) fn open_file_dialog(&mut self) {
        // From the home page too, the dialog starts beside the document
        // that was in front.
        let initial_dir = self
            .tabs
            .get(self.active_tab)
            .and_then(|tab| tab.path.parent())
            .or_else(|| self.file_explorer.root());

        match (self.file_picker)(initial_dir) {
            Ok(Some(path)) => self.open_in_new_tab(path),
            Ok(None) => {}
            Err(error) => {
                log::error!("Unable to open file picker: {error}");
                self.error_message = Some(format!("Unable to open file picker: {error}"));
            }
        }
    }

    /// Open a native folder picker and point the file explorer at the chosen
    /// directory (issue #28). The chosen root is persisted via `save()`.
    pub(crate) fn open_folder_dialog(&mut self) {
        match (self.folder_picker)(self.file_explorer.root()) {
            Ok(Some(path)) => {
                self.file_explorer.set_root(path);
                // Make sure the explorer is visible so the result is seen.
                self.show_explorer = true;
                // Rebuild the watcher so the new root is watched recursively
                // (`update_watched_paths` only reconciles tab paths, not the root).
                if self.live_reload.is_enabled() {
                    self.start_watching();
                }
            }
            Ok(None) => {}
            Err(error) => {
                log::error!("Unable to open folder picker: {error}");
                self.error_message = Some(format!("Unable to open folder picker: {error}"));
            }
        }
    }

    pub(crate) fn open_in_new_tab(&mut self, path: PathBuf) {
        self.open_in_new_tab_with_fragment(path, None);
    }

    /// Open a document in its own tab, optionally landing on the heading a
    /// `#fragment` names. An already-open document is focused *and* scrolled
    /// to the fragment — the tab-switch must not silently drop the anchor.
    pub(crate) fn open_in_new_tab_with_fragment(&mut self, path: PathBuf, fragment: Option<&str>) {
        let path = path.canonicalize().unwrap_or(path);
        // Check if already open
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.activate_tab(idx);
            if let (Some(fragment), Some(tab)) = (fragment, self.tabs.get_mut(idx)) {
                tab.go_to_fragment(fragment);
            }
            return;
        }

        // Add new tab
        let mut tab = match Tab::new(path.clone()) {
            Ok(tab) => tab,
            Err(error) => {
                self.report_unopened(&path, &error);
                return;
            }
        };
        if let Some(fragment) = fragment {
            tab.go_to_fragment(fragment);
        }
        self.tabs.push(tab);
        self.record_recent(&path);
        self.activate_tab(self.tabs.len() - 1);
        self.tab_documents_changed();
    }

    /// Show the document at `path` in the tab in front, asking first if
    /// that would lose unsaved edits. From the home page, where no tab is
    /// in front, it gets a tab of its own, or the one it has.
    pub(crate) fn navigate_active_tab_to(&mut self, path: &Path) {
        if self.front().is_none() {
            self.open_in_new_tab(path.to_path_buf());
            return;
        }
        self.request_route(Route::File(path.to_path_buf()));
    }

    /// Say in the error bar that the file at `path` was asked for and
    /// could not be opened.
    fn report_unopened(&mut self, path: &Path, error: &io::Error) {
        self.error_message = Some(format!("Unable to open {}: {error}", path.display()));
    }

    /// Step the tab in front through its history, asking first if that
    /// would lose unsaved edits. From the home page over open tabs, back
    /// is the way to the tab that was in front.
    pub(crate) fn navigate_active_history(&mut self, go_back: bool) {
        if self.home_shows {
            if go_back {
                self.toggle_home();
            }
            return;
        }
        let step = if go_back { Route::Back } else { Route::Forward };
        self.request_route(step);
    }

    /// Follow a link the reader clicked in the active document: a plain
    /// click shows its target in place, asking first if that would lose
    /// unsaved edits, and `in_new_tab` gives the target a tab of its own.
    pub(crate) fn follow_link(&mut self, link: &str, in_new_tab: bool) {
        if !in_new_tab {
            self.request_route(Route::Link(link.to_owned()));
            return;
        }
        let target = self.front_tab().map(|tab| tab.link_target(link));
        match target {
            Some(Ok(Some((path, fragment)))) => {
                self.open_in_new_tab_with_fragment(path, fragment.as_deref());
            }
            Some(Ok(None)) | None => {}
            Some(Err(error)) => {
                self.error_message = Some(format!("Unable to open {link}: {error}"));
            }
        }
    }

    /// Take the tab at `index` where `route` leads now, with whatever it
    /// holds unsaved, and bring what follows the tabs along when the tab
    /// shows another document for it. What the reader asks for goes
    /// through `request_route`, which asks about unsaved edits first.
    ///
    /// What the tab reports of the step is not asked: a link to a heading
    /// of the same document reports success and moves no tab.
    pub(crate) fn take_route(&mut self, index: usize, route: &Route) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        let from = tab.path.clone();
        let outcome = tab.take(route);
        if tab.path != from {
            // The document the tab moved to is the most recent one.
            let shown = tab.path.clone();
            self.record_recent(&shown);
            self.tab_documents_changed();
        }
        // A file that cannot be read leaves the document in front, and
        // says so as one opened in a tab of its own does.
        match (outcome, route) {
            (Ok(_), _) => {}
            (Err(error), Route::Link(link)) => {
                self.error_message = Some(format!("Unable to open {link}: {error}"));
            }
            (Err(error), Route::File(path)) => self.report_unopened(path, &error),
            (Err(error), Route::Back | Route::Forward) => {
                self.error_message = Some(format!("Unable to navigate history: {error}"));
            }
        }
    }

    /// Bring everything that mirrors which document each tab holds back in
    /// line, after a tab opened, closed, or moved to another document: the
    /// names in the tab row, the window title, the find bar's matches, the
    /// explorer's open-file highlight, and the watched paths.
    ///
    /// The find bar holds its matches by a tab's place in the strip. A
    /// place can hold another document after any of these, the first place
    /// included when a document opens into an empty strip.
    fn tab_documents_changed(&mut self) {
        self.tab_names_changed();
        self.window_title.mark_stale();
        self.search.invalidate();
        self.refresh_open_tab_paths();
        self.update_watched_paths();
    }

    /// Work the names in the tab row out again, after the set of tabs
    /// changed or a tab's document did: on open, close, reload and edit.
    pub(crate) fn tab_names_changed(&mut self) {
        name_tabs(&mut self.tabs);
    }

    /// Record a file in the recent list (most-recent first, deduped, capped),
    /// with what the home list shows of it. The document is summarized here,
    /// once per load, from the tab that holds it.
    pub(crate) fn record_recent(&mut self, path: &Path) {
        let loaded = self.tabs.iter().find(|tab| tab.path == path);
        let summary =
            loaded.map(|tab| DocumentSummary::of(&tab.content, tab.document_title.as_deref()));
        push_recent(&mut self.recent_files, path, now_epoch_secs(), summary);
        self.home_list = None;
    }

    /// Close the tab at `idx` now, with whatever it holds unsaved. What
    /// the reader asks for goes through `request_close_tab`, which asks
    /// about unsaved edits first.
    pub(crate) fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }

        let closed = self.tabs.remove(idx);
        self.forget_pictures(&[closed]);
        // Adjust active tab index. Closing the last tab leaves `tabs` empty,
        // which renders the welcome page (active_tab kept in range / at 0).
        self.active_tab = active_after_close(self.active_tab, idx, self.tabs.len());
        // The home page stays over the tabs that are left, and needs no
        // flag once none is.
        self.home_shows &= !self.tabs.is_empty();
        self.tab_documents_changed();
    }

    /// Close every tab but the one at `keep`, which comes to the front,
    /// now and with whatever they hold unsaved. The reader's request goes
    /// through `request_close_other_tabs`.
    pub(crate) fn close_other_tabs(&mut self, keep: usize) {
        let kept = self.tabs.remove(keep);
        let closed = std::mem::replace(&mut self.tabs, vec![kept]);
        self.forget_pictures(&closed);
        self.activate_tab(0);
        self.tab_documents_changed();
    }

    /// Give back the memory egui keeps for the pictures the `closed` tabs
    /// showed: egui keeps a picture for as long as the app runs otherwise.
    /// A picture a tab still open has shown stays.
    fn forget_pictures(&self, closed: &[Tab]) {
        for uri in closed.iter().flat_map(Tab::pictures) {
            let still_shown = self.tabs.iter().flat_map(Tab::pictures);
            if !still_shown.into_iter().any(|shown| shown == uri) {
                animation::forget(&self.egui_ctx, uri);
            }
        }
    }

    /// Close the tab in front, asking first if it has unsaved edits.
    /// Close the tab in front. The home page is no tab: from it, nothing
    /// closes, least of all a document that is not on screen.
    pub(crate) fn close_active_tab(&mut self) {
        if let Some(front) = self.front() {
            self.request_close_tab(front);
        }
    }

    /// Go to the tab after the one in front. From the home page the way
    /// leads back to the tab that was in front first.
    pub(crate) const fn next_tab(&mut self) {
        if self.home_shows {
            self.toggle_home();
        } else if self.tabs.len() > 1 {
            self.activate_tab(next_index(self.active_tab, self.tabs.len()));
        }
    }

    /// Go to the tab before the one in front, or from the home page back
    /// to the tab that was in front.
    pub(crate) const fn prev_tab(&mut self) {
        if self.home_shows {
            self.toggle_home();
        } else if self.tabs.len() > 1 {
            self.activate_tab(previous_index(self.active_tab, self.tabs.len()));
        }
    }

    pub(crate) const fn focus_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.activate_tab(idx);
        }
    }

    /// Bring the tab at `index` to the front, in place of the home page
    /// when that shows. The window is named after the document in front,
    /// so the title follows.
    const fn activate_tab(&mut self, index: usize) {
        self.active_tab = index;
        self.home_shows = false;
        self.window_title.mark_stale();
    }

    pub(crate) fn get_open_tab_paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter(|t| t.path.exists())
            .map(|t| t.path.clone())
            .collect()
    }

    /// Rebuild the cached `open_tab_paths` set.
    pub(crate) fn refresh_open_tab_paths(&mut self) {
        self.open_tab_paths.clear();
        for tab in &self.tabs {
            self.open_tab_paths.insert(tab.path.clone());
        }
    }
}
