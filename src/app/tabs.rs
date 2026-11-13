//! Opening, closing, switching, and navigating tabs.

use std::path::{Path, PathBuf};

use crate::app::MarkdownApp;
use crate::dialogs::{pick_file, pick_folder};
use crate::links::apply_fragment_navigation;
use crate::recent::{now_epoch_secs, push_recent};
use crate::tab::Tab;

impl MarkdownApp {
    pub(crate) fn open_file_dialog(&mut self) {
        let initial_dir = self
            .tabs
            .get(self.active_tab)
            .and_then(|tab| tab.path.parent())
            .map(Path::to_path_buf)
            .or_else(|| self.file_explorer.root.clone());

        match pick_file(initial_dir.as_deref()) {
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
        match pick_folder(self.file_explorer.root.as_deref()) {
            Ok(Some(path)) => {
                self.file_explorer.set_root(path);
                // Make sure the explorer is visible so the result is seen.
                self.show_explorer = true;
                // Rebuild the watcher so the new root is watched recursively
                // (`update_watched_paths` only reconciles tab paths, not the root).
                if self.watch_enabled {
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
            self.active_tab = idx;
            self.title_dirty = true;
            if let (Some(fragment), Some(tab)) = (fragment, self.tabs.get_mut(idx)) {
                apply_fragment_navigation(tab, fragment);
            }
            return;
        }

        // Add new tab
        let mut tab = match Tab::new(path.clone()) {
            Ok(tab) => tab,
            Err(error) => {
                self.error_message = Some(format!("Unable to open {}: {error}", path.display()));
                return;
            }
        };
        if let Some(fragment) = fragment {
            apply_fragment_navigation(&mut tab, fragment);
        }
        self.record_recent(&path);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.title_dirty = true;
        self.refresh_open_tab_paths();

        // Update watcher if enabled
        if self.watch_enabled {
            self.update_watched_paths();
        }
    }

    pub(crate) fn navigate_active_tab_to(&mut self, path: &Path) {
        if self.tabs.is_empty() {
            self.open_in_new_tab(path.to_path_buf());
            return;
        }

        let changed = self
            .tabs
            .get_mut(self.active_tab)
            .is_some_and(|tab| tab.navigate_to_path(path).unwrap_or(false));
        if changed {
            self.active_tab_navigated();
        }
    }

    pub(crate) fn navigate_active_history(&mut self, go_back: bool) {
        let result = self.tabs.get_mut(self.active_tab).map(|tab| {
            if go_back {
                tab.navigate_back()
            } else {
                tab.navigate_forward()
            }
        });
        match result {
            Some(Ok(true)) => self.active_tab_navigated(),
            Some(Err(error)) => {
                self.error_message = Some(format!("Unable to navigate history: {error}"));
            }
            Some(Ok(false)) | None => {}
        }
    }

    /// Bring everything that mirrors tab paths back in line after the active
    /// tab moved to another document: the recent list, the window title, the
    /// explorer's open-file highlight, and the watched paths.
    pub(crate) fn active_tab_navigated(&mut self) {
        if let Some(path) = self.tabs.get(self.active_tab).map(|tab| tab.path.clone()) {
            self.record_recent(&path);
        }
        self.title_dirty = true;
        self.refresh_open_tab_paths();
        if self.watch_enabled {
            self.update_watched_paths();
        }
    }

    /// Record a file in the recent list (most-recent first, deduped, capped).
    pub(crate) fn record_recent(&mut self, path: &Path) {
        push_recent(&mut self.recent_files, path, now_epoch_secs());
    }

    pub(crate) fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }

        self.tabs.remove(idx);
        self.title_dirty = true;
        self.refresh_open_tab_paths();

        // Adjust active tab index. Closing the last tab leaves `tabs` empty,
        // which renders the welcome page (active_tab kept in range / at 0).
        if self.tabs.is_empty() {
            self.active_tab = 0;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        } else if self.active_tab > idx {
            self.active_tab -= 1;
        }

        // Update watcher
        if self.watch_enabled {
            self.update_watched_paths();
        }
    }

    pub(crate) fn close_active_tab(&mut self) {
        self.close_tab(self.active_tab);
    }

    pub(crate) const fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.title_dirty = true;
        }
    }

    pub(crate) const fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
            self.title_dirty = true;
        }
    }

    pub(crate) const fn focus_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active_tab = idx;
            self.title_dirty = true;
        }
    }

    pub(crate) fn get_open_tab_paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter(|t| t.path.exists())
            .map(|t| t.path.clone())
            .collect()
    }

    /// Rebuild the cached `open_tab_paths` set (call after tab open/close/navigate).
    pub(crate) fn refresh_open_tab_paths(&mut self) {
        self.open_tab_paths.clear();
        for tab in &self.tabs {
            self.open_tab_paths.insert(tab.path.clone());
        }
    }
}
