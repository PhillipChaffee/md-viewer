//! Live reload: watcher lifecycle, change polling, and tab reloads.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use notify_debouncer_mini::DebounceEventResult;

use crate::app::MarkdownApp;
use crate::watcher::{
    ancestors_within, explorer_watch_dirs, next_watcher_retry, settled_paths, FileWatcher,
    WatchPlan, MAX_WATCHER_RETRIES,
};

#[cfg(test)]
mod tests;

/// Live reload: the reader's setting, and the watcher while one runs.
///
/// On with nothing running means either that there is nothing to watch, or
/// that the watcher failed and the next frame tries to restart it.
pub(super) struct LiveReload {
    enabled: bool,
    /// Restarts tried since the watcher last started on request or reported
    /// a change.
    retries: u32,
    running: Option<Running>,
}

/// A watcher (inotify for local paths, polling for GVFS) and the channel
/// its events arrive on.
struct Running {
    watcher: FileWatcher,
    events: Receiver<DebounceEventResult>,
}

impl LiveReload {
    pub(super) const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            retries: 0,
            running: None,
        }
    }

    pub(super) const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Whether a watcher runs: what the live indicator shows.
    pub(super) const fn is_live(&self) -> bool {
        self.running.is_some()
    }
}

impl MarkdownApp {
    /// Turns live reload off while a watcher runs, and on otherwise: also
    /// when it is on already with nothing to watch.
    pub(crate) fn toggle_watching(&mut self) {
        if self.live_reload.is_live() {
            self.stop_watching();
            self.live_reload.enabled = false;
        } else {
            self.live_reload.enabled = true;
            self.start_watching();
        }
    }

    /// Start watching because of a user action or configuration change.
    pub(crate) fn start_watching(&mut self) {
        self.live_reload.retries = 0;
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

    /// What a fresh start registers, or `None` when there is nothing to
    /// watch: no open document is on disk, and the explorer has no local
    /// root.
    fn watch_plan(&self) -> Option<WatchPlan> {
        let explorer_dirs = explorer_watch_dirs(
            self.file_explorer.root(),
            self.file_explorer.expanded_dirs(),
            Path::is_dir,
        );
        let plan = WatchPlan::new(
            self.get_open_tab_paths(),
            explorer_dirs.into_iter().collect(),
        );
        (!plan.is_empty()).then_some(plan)
    }

    /// Puts a freshly started watcher into use, with the channel its events
    /// arrive on, or reports that no backend could start.
    fn adopt_watcher(
        &mut self,
        started: Option<FileWatcher>,
        events: Receiver<DebounceEventResult>,
    ) {
        let Some(watcher) = started else {
            log::error!("Failed to create any file watcher");
            self.error_message = Some("Failed to create file watcher".to_string());
            return;
        };
        let accepted = watcher.watched();
        let (files, dirs) = (accepted.files.len(), accepted.explorer_dirs.len());
        log::info!("Started watching {files} files and {dirs} explorer dirs");
        self.live_reload.running = Some(Running { watcher, events });
        self.live_reload.enabled = true;
    }

    pub(crate) fn stop_watching(&mut self) {
        if self.live_reload.is_live() {
            log::info!("Stopped watching files");
        }
        self.live_reload.running = None;
    }

    pub(crate) fn update_watched_paths(&mut self) {
        if !self.live_reload.enabled {
            return;
        }

        let open_files: HashSet<PathBuf> = self.get_open_tab_paths().into_iter().collect();
        let Some(running) = &mut self.live_reload.running else {
            return;
        };
        if running.watcher.sync_files(open_files).is_err() {
            log::info!("Watcher type mismatch, restarting watchers");
            self.start_watching();
        }
    }

    /// Reconcile the non-recursive explorer-directory watches (root + expanded
    /// dirs) against the live watcher after the expanded set changes. Mirrors
    /// `update_watched_paths`'s incremental diff so expand/collapse doesn't tear
    /// down and rebuild the debouncer + bridge thread.
    pub(crate) fn reconcile_explorer_watches(&mut self) {
        if !self.live_reload.enabled {
            return;
        }

        let desired = explorer_watch_dirs(
            self.file_explorer.root(),
            self.file_explorer.expanded_dirs(),
            Path::is_dir,
        );
        if let Some(running) = &mut self.live_reload.running {
            running.watcher.sync_explorer_dirs(desired);
        }
    }

    pub(crate) fn check_file_changes(&mut self) -> Vec<PathBuf> {
        let Some(running) = &self.live_reload.running else {
            // No watcher runs. With live reload on, that is a failure to
            // recover from only when there is something to watch; the plan
            // is rebuilt from the tabs and the explorer, since what the
            // watcher held went with it.
            if self.live_reload.enabled && self.watch_plan().is_some() {
                self.recover_watcher(None);
            }
            return Vec::new();
        };

        let mut changed_paths = Vec::new();

        while let Ok(result) = running.events.try_recv() {
            match result {
                Ok(events) => {
                    self.live_reload.retries = 0;
                    changed_paths.extend(settled_paths(events).inspect(|path| {
                        log::debug!("File change detected: {}", path.display());
                    }));
                }
                Err(error) => {
                    log::error!("File watcher error: {error}");
                    self.live_reload.running = None;
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
        if let Some(attempt) = next_watcher_retry(self.live_reload.retries) {
            self.live_reload.retries = attempt;
            log::info!("Attempting to recover file watcher (attempt {attempt})");
            self.start_watching_attempt();
            self.egui_ctx.request_repaint_after(Duration::from_secs(2));
        } else {
            let detail = cause.map(|error| format!(": {error}")).unwrap_or_default();
            self.error_message = Some(format!(
                "File watcher failed after {MAX_WATCHER_RETRIES} retries{detail}"
            ));
            self.live_reload.enabled = false;
        }
    }

    pub(crate) fn reload_changed_tabs(&mut self, changed_paths: Vec<PathBuf>) {
        let now = Instant::now();
        let mut refresh_directories = HashSet::new();

        for path in changed_paths {
            self.flash_changed_path(&path, now);
            if let Some(root) = self.file_explorer.root() {
                // The change can alter one listing: that of the directory
                // holding the path, when it is inside the explorer.
                let listing = ancestors_within(&path, root).next();
                refresh_directories.extend(listing.map(Path::to_path_buf));
            }

            // If the active tab gets reloaded while the find bar is open, its
            // `search_matches` will be cleared by `Tab::reload`. Force a rebuild
            // on the next frame by invalidating the cache-validity shadow state.
            if self.reload_tabs_showing(&path) && self.search.is_open {
                self.search.invalidate();
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
        if let Some(root) = self.file_explorer.root() {
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
            // Never clobber unsaved edits with a disk reload, whether the
            // tab shows the editor or the preview of what was typed. A
            // reload of identical content (e.g. our own save echoing back
            // through the watcher) still proceeds.
            if tab.is_dirty() {
                log::info!("Skipping reload of unsaved tab: {}", path.display());
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
