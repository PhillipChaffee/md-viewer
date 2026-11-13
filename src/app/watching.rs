//! Live reload: watcher lifecycle, change polling, and tab reloads.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use notify::{PollWatcher, RecommendedWatcher};
use notify_debouncer_mini::{new_debouncer, new_debouncer_opt, DebounceEventResult, Debouncer};

use crate::app::MarkdownApp;
use crate::watcher::{
    ancestors_within, explorer_watch_dirs, listing_to_refresh, needs_backend_restart,
    next_watcher_retry, settled_paths, watch_diff, watch_path, FileWatcher, WatchPlan,
    MAX_WATCHER_RETRIES,
};

/// Quiet time after the last write before a change is reported.
const DEBOUNCE: Duration = Duration::from_millis(200);
/// How often GVFS documents are checked. Each check is a remote round trip.
const GVFS_POLL_INTERVAL: Duration = Duration::from_secs(2);

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

        // Both debouncers send to the same channel
        let (tx, debouncer_rx) = mpsc::channel();
        let inotify = plan
            .needs_inotify()
            .then(|| self.start_inotify(&plan, tx.clone()))
            .flatten();
        let poll = plan
            .needs_poll()
            .then(|| self.start_poll(&plan, tx))
            .flatten();

        let Some(watcher) = FileWatcher::from_backends(inotify, poll) else {
            log::error!("Failed to create any file watcher");
            self.error_message = Some("Failed to create file watcher".to_string());
            return;
        };
        log::info!(
            "Started watching {} local (inotify) + {} GVFS (poll) files, {} explorer dirs",
            plan.local_files.len(),
            plan.gvfs_files.len(),
            self.watched_explorer_dirs.len()
        );

        match self.spawn_bridge(debouncer_rx) {
            Ok(bridge_rx) => {
                self.watcher = Some(watcher);
                self.watcher_rx = Some(bridge_rx);
                self.watch_enabled = true;
            }
            Err(error) => {
                log::error!("Failed to spawn watcher bridge thread: {error}");
                self.error_message = Some(format!("Failed to start file watcher bridge: {error}"));
            }
        }
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

    /// Starts the inotify backend and records which of the plan's local paths
    /// it accepted.
    fn start_inotify(
        &mut self,
        plan: &WatchPlan,
        tx: Sender<DebounceEventResult>,
    ) -> Option<Debouncer<RecommendedWatcher>> {
        let mut debouncer = new_debouncer(DEBOUNCE, tx)
            .inspect_err(|error| log::error!("Failed to create inotify watcher: {error}"))
            .ok()?;
        let watcher = debouncer.watcher();
        for path in &plan.local_files {
            if watch_path(watcher, path, "file") {
                self.watched_paths.insert(path.clone());
            }
        }
        for dir in &plan.explorer_dirs {
            if watch_path(watcher, dir, "explorer dir") {
                self.watched_explorer_dirs.insert(dir.clone());
            }
        }
        Some(debouncer)
    }

    /// Starts the polling backend and records which of the plan's GVFS
    /// documents it accepted.
    fn start_poll(
        &mut self,
        plan: &WatchPlan,
        tx: Sender<DebounceEventResult>,
    ) -> Option<Debouncer<PollWatcher>> {
        let config = notify_debouncer_mini::Config::default()
            .with_timeout(DEBOUNCE)
            .with_notify_config(notify::Config::default().with_poll_interval(GVFS_POLL_INTERVAL));
        let mut debouncer = new_debouncer_opt::<_, PollWatcher>(config, tx)
            .inspect_err(|error| log::error!("Failed to create poll watcher: {error}"))
            .ok()?;
        let watcher = debouncer.watcher();
        for path in &plan.gvfs_files {
            if watch_path(watcher, path, "polled file") {
                self.watched_paths.insert(path.clone());
            }
        }
        Some(debouncer)
    }

    /// Forwards debounced events to the channel the UI thread drains and
    /// wakes egui for each batch, so an idle window does not have to poll.
    fn spawn_bridge(
        &self,
        debouncer_rx: Receiver<DebounceEventResult>,
    ) -> io::Result<Receiver<DebounceEventResult>> {
        let (bridge_tx, bridge_rx) = mpsc::channel();
        let ctx = self.egui_ctx.clone();
        std::thread::Builder::new()
            .name("watcher-bridge".into())
            .spawn(move || {
                while let Ok(event) = debouncer_rx.recv() {
                    let _ = bridge_tx.send(event);
                    ctx.request_repaint();
                }
            })?;
        Ok(bridge_rx)
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
