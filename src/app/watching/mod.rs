//! Live reload: watcher lifecycle, change polling, and tab reloads.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use notify_debouncer_mini::DebounceEventResult;

use crate::app::MarkdownApp;
use crate::watcher::{
    ancestors_within, explorer_watch_dirs, listing_to_refresh, needs_backend_restart,
    next_watcher_retry, settled_paths, watch_diff, FileWatcher, WatchPlan, Watched,
    MAX_WATCHER_RETRIES,
};

#[cfg(test)]
mod tests;

impl MarkdownApp {
    /// Start watching because of a user action or configuration change.
    pub(crate) fn start_watching(&mut self) {
        self.watcher_retry_count = 0;
        self.start_watching_attempt();
    }

    /// Construct watchers without resetting the recovery-attempt counter.
    fn start_watching_attempt(&mut self) {
        self.stop_watching();

        let Some(plan) = self.watch_plan() else {
            return;
        };

        let (tx, events) = mpsc::channel();
        let ctx = self.egui_ctx.clone();
        // Runs on a watcher thread: queue the batch for the UI thread, then
        // wake egui so an idle window does not have to poll.
        let on_events = move |batch: DebounceEventResult| {
            let _ = tx.send(batch);
            ctx.request_repaint();
        };
        self.adopt_watcher(FileWatcher::start(&plan, on_events), events);
    }

    /// What a fresh start registers, or `None` when no document is open and
    /// the explorer has no root.
    fn watch_plan(&self) -> Option<WatchPlan> {
        let open_files = self.get_open_tab_paths();
        let root = self.file_explorer.root.as_deref();
        if open_files.is_empty() && root.is_none() {
            return None;
        }
        let explorer_dirs =
            explorer_watch_dirs(root, &self.file_explorer.expanded_dirs, Path::is_dir);
        Some(WatchPlan::new(
            open_files,
            explorer_dirs.into_iter().collect(),
        ))
    }

    /// Puts a freshly started watcher into use, with the channel its events
    /// arrive on, or reports that no backend could start.
    fn adopt_watcher(
        &mut self,
        started: Option<(FileWatcher, Watched)>,
        events: Receiver<DebounceEventResult>,
    ) {
        let Some((watcher, accepted)) = started else {
            log::error!("Failed to create any file watcher");
            self.error_message = Some("Failed to create file watcher".to_string());
            return;
        };
        let (files, dirs) = (accepted.files.len(), accepted.explorer_dirs.len());
        log::info!("Started watching {files} files and {dirs} explorer dirs");
        self.watcher = Some(watcher);
        self.watcher_rx = Some(events);
        self.watched_paths = accepted.files;
        self.watched_explorer_dirs = accepted.explorer_dirs;
        self.watch_enabled = true;
    }

    pub(crate) fn stop_watching(&mut self) {
        if self.watcher.is_some() {
            log::info!("Stopped watching files");
        }
        self.watcher = None;
        self.watcher_rx = None;
        self.watched_paths.clear();
        self.watched_explorer_dirs.clear();
    }

    pub(crate) fn update_watched_paths(&mut self) {
        if !self.watch_enabled {
            return;
        }

        let current_paths: HashSet<PathBuf> = self.get_open_tab_paths().into_iter().collect();

        if let Some(fw) = &mut self.watcher {
            let has_inotify = fw.inotify_watcher().is_some();
            let has_poll = fw.poll_watcher().is_some();
            if needs_backend_restart(&current_paths, has_inotify, has_poll) {
                log::info!("Watcher type mismatch, restarting watchers");
                self.start_watching();
                return;
            }
            fw.apply(&watch_diff(&current_paths, &self.watched_paths), "file");
        }

        self.watched_paths = current_paths;
    }

    /// Reconcile the non-recursive explorer-directory watches (root + expanded
    /// dirs) against the live watcher after the expanded set changes. Mirrors
    /// `update_watched_paths`'s incremental diff so expand/collapse doesn't tear
    /// down and rebuild the debouncer + bridge thread.
    pub(crate) fn reconcile_explorer_watches(&mut self) {
        if !self.watch_enabled {
            return;
        }

        let desired = explorer_watch_dirs(
            self.file_explorer.root.as_deref(),
            &self.file_explorer.expanded_dirs,
            Path::is_dir,
        );
        if let Some(fw) = &mut self.watcher {
            fw.apply(
                &watch_diff(&desired, &self.watched_explorer_dirs),
                "explorer dir",
            );
        }
        self.watched_explorer_dirs = desired;
    }

    pub(crate) fn check_file_changes(&mut self) -> Vec<PathBuf> {
        let Some(rx) = &self.watcher_rx else {
            // Attempt recovery if watching is enabled and there's something to watch
            // Check actual tabs and explorer root, not watched_paths (which may be empty after failure)
            let has_watchable = !self.tabs.is_empty() || self.file_explorer.root.is_some();
            if self.watch_enabled && has_watchable {
                self.recover_watcher(None);
            }
            return Vec::new();
        };

        let mut changed_paths = Vec::new();

        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(events) => {
                    self.watcher_retry_count = 0;
                    changed_paths.extend(settled_paths(events).inspect(|path| {
                        log::debug!("File change detected: {}", path.display());
                    }));
                }
                Err(error) => {
                    log::error!("File watcher error: {error}");
                    self.watcher = None;
                    self.watcher_rx = None;
                    self.recover_watcher(Some(&error));
                    return Vec::new();
                }
            }
        }

        changed_paths
    }

    /// Restarts the watcher after a failure, or disables live reload once the
    /// retry budget is spent. `cause` is the error that stopped it, if one
    /// was reported.
    fn recover_watcher(&mut self, cause: Option<&notify::Error>) {
        if let Some(attempt) = next_watcher_retry(self.watcher_retry_count) {
            self.watcher_retry_count = attempt;
            log::info!("Attempting to recover file watcher (attempt {attempt})");
            self.start_watching_attempt();
            self.egui_ctx.request_repaint_after(Duration::from_secs(2));
        } else {
            let detail = cause.map(|error| format!(": {error}")).unwrap_or_default();
            self.error_message = Some(format!(
                "File watcher failed after {MAX_WATCHER_RETRIES} retries{detail}"
            ));
            self.watch_enabled = false;
        }
    }

    pub(crate) fn reload_changed_tabs(&mut self, changed_paths: Vec<PathBuf>) {
        let now = Instant::now();
        let mut refresh_directories = HashSet::new();

        for path in changed_paths {
            self.flash_changed_path(&path, now);
            if let Some(root) = &self.file_explorer.root {
                refresh_directories.extend(listing_to_refresh(&path, root).map(Path::to_path_buf));
            }

            // If the active tab gets reloaded while the find bar is open, its
            // `search_matches` will be cleared by `Tab::reload`. Force a rebuild
            // on the next frame by invalidating the cache-validity shadow state.
            if self.reload_tabs_showing(&path) && self.search.is_open {
                self.search.last_tab = None;
            }
        }

        // Refresh only affected parents instead of rebuilding the entire tree.
        for directory in refresh_directories {
            log::debug!("Refreshing explorer directory: {}", directory.display());
            self.file_explorer.refresh_directory(&directory);
        }
    }

    /// Starts the explorer highlight for a changed file and for each
    /// directory between it and the explorer root.
    fn flash_changed_path(&mut self, path: &Path, now: Instant) {
        // The explorer looks rows up by canonical path.
        let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.flashing_paths.insert(canonical(path), now);
        if let Some(root) = &self.file_explorer.root {
            for dir in ancestors_within(path, root) {
                self.flashing_paths.insert(canonical(dir), now);
            }
        }
    }

    /// Reloads every tab showing `path` from disk. Returns whether a reload
    /// replaced the document the active tab shows.
    fn reload_tabs_showing(&mut self, path: &Path) -> bool {
        let active_shows_path = self
            .tabs
            .get(self.active_tab)
            .is_some_and(|tab| tab.path == path);
        let mut reloaded = false;
        for tab in self.tabs.iter_mut().filter(|tab| tab.path == path) {
            // Never clobber unsaved edit-mode edits with a disk
            // reload (dirty-reload suppression, mirroring the
            // dsh-better-sidebar editor). A reload of identical
            // content (e.g. our own save echoing back through the
            // watcher) still proceeds.
            if tab.is_editing && tab.content != tab.saved_content {
                log::info!("Skipping reload of dirty edit tab: {}", path.display());
                continue;
            }
            log::info!("Reloading tab: {}", path.display());
            match tab.reload() {
                Ok(()) => reloaded = true,
                Err(error) => {
                    self.error_message =
                        Some(format!("Unable to reload {}: {error}", path.display()));
                }
            }
        }
        reloaded && active_shows_path
    }
}
