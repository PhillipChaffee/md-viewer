//! Live reload: watcher lifecycle, change polling, and tab reloads.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::PollWatcher;
use notify_debouncer_mini::{new_debouncer, new_debouncer_opt, DebouncedEventKind};

use crate::app::MarkdownApp;
use crate::watcher::{is_gvfs_path, next_watcher_retry, FileWatcher, MAX_WATCHER_RETRIES};

impl MarkdownApp {
    /// Start watching because of a user action or configuration change.
    pub(crate) fn start_watching(&mut self) {
        self.watcher_retry_count = 0;
        self.start_watching_attempt();
    }

    /// Construct watchers without resetting the recovery-attempt counter.
    fn start_watching_attempt(&mut self) {
        self.stop_watching();

        let tab_paths = self.get_open_tab_paths();
        let explorer_root = self.file_explorer.root.clone();

        // Need something to watch
        if tab_paths.is_empty() && explorer_root.is_none() {
            return;
        }

        // Partition tab paths into local (inotify) and GVFS (poll)
        let (gvfs_paths, local_paths): (Vec<_>, Vec<_>) =
            tab_paths.iter().partition(|p| is_gvfs_path(p));
        let explorer_is_gvfs = explorer_root.as_ref().is_some_and(|r| is_gvfs_path(r));
        // Explorer root only watched via inotify (local) — GVFS explorer roots are
        // skipped to avoid costly recursive polling over SFTP
        let has_local = !local_paths.is_empty() || (explorer_root.is_some() && !explorer_is_gvfs);
        let has_gvfs = !gvfs_paths.is_empty();

        // Non-recursive: mirror the lazy explorer tree. Watching the whole root
        // subtree recursively walks every directory on the main thread at startup
        // (~6s hang on /home/ahmet, ~455k dirs). We only need change events for
        // the directories the user can actually see: the root plus each expanded
        // directory. GVFS explorer roots stay unwatched (costly over SFTP).
        let mut explorer_dirs: Vec<PathBuf> = Vec::new();
        if let Some(ref root) = explorer_root {
            if !explorer_is_gvfs {
                explorer_dirs.push(root.clone());
                explorer_dirs.extend(
                    self.file_explorer
                        .expanded_dirs
                        .iter()
                        .filter(|d| !is_gvfs_path(d) && d.is_dir())
                        .cloned(),
                );
            }
        }

        // Both debouncers send to the same channel
        let (tx, debouncer_rx) = mpsc::channel();

        // Create inotify debouncer for local paths
        let inotify_debouncer = if has_local {
            match new_debouncer(Duration::from_millis(200), tx.clone()) {
                Ok(mut debouncer) => {
                    for path in &local_paths {
                        if let Err(e) = debouncer
                            .watcher()
                            .watch(path, notify::RecursiveMode::NonRecursive)
                        {
                            log::error!("Failed to watch file {:?}: {}", path, e);
                        } else {
                            self.watched_paths.insert((*path).clone());
                        }
                    }
                    for dir in &explorer_dirs {
                        if let Err(e) = debouncer
                            .watcher()
                            .watch(dir, notify::RecursiveMode::NonRecursive)
                        {
                            log::error!("Failed to watch explorer dir {:?}: {}", dir, e);
                        } else {
                            self.watched_explorer_dirs.insert(dir.clone());
                        }
                    }
                    Some(debouncer)
                }
                Err(e) => {
                    log::error!("Failed to create inotify watcher: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create poll debouncer for GVFS/remote paths
        let poll_debouncer = if has_gvfs {
            let poll_config = notify_debouncer_mini::Config::default()
                .with_timeout(Duration::from_millis(200))
                .with_notify_config(
                    notify::Config::default().with_poll_interval(Duration::from_secs(2)),
                );
            match new_debouncer_opt::<_, PollWatcher>(poll_config, tx.clone()) {
                Ok(mut debouncer) => {
                    for path in &gvfs_paths {
                        if let Err(e) = debouncer
                            .watcher()
                            .watch(path, notify::RecursiveMode::NonRecursive)
                        {
                            log::error!("Failed to poll-watch file {:?}: {}", path, e);
                        } else {
                            self.watched_paths.insert((*path).clone());
                        }
                    }
                    // Skip recursive watching of GVFS explorer root — polling an
                    // entire remote directory tree every 2s causes lag from SFTP
                    // roundtrips (stat + read_dir for each entry). Users can refresh
                    // the explorer manually instead.
                    Some(debouncer)
                }
                Err(e) => {
                    log::error!("Failed to create poll watcher: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Build FileWatcher enum from whichever debouncers succeeded
        let file_watcher = match (inotify_debouncer, poll_debouncer) {
            (Some(i), Some(p)) => Some(FileWatcher::Dual {
                inotify: i,
                poll: p,
            }),
            (Some(d), None) => Some(FileWatcher::Inotify(d)),
            (None, Some(d)) => Some(FileWatcher::Poll(d)),
            (None, None) => None,
        };

        if let Some(fw) = file_watcher {
            let local_count = local_paths.len();
            let gvfs_count = gvfs_paths.len();
            log::info!(
                "Started watching {} local (inotify) + {} GVFS (poll) files, {} explorer dirs",
                local_count,
                gvfs_count,
                self.watched_explorer_dirs.len()
            );

            // Bridge thread: forward events and wake egui on demand
            let (bridge_tx, bridge_rx) = mpsc::channel();
            let ctx = self.egui_ctx.clone();
            let bridge = std::thread::Builder::new()
                .name("watcher-bridge".into())
                .spawn(move || {
                    while let Ok(event) = debouncer_rx.recv() {
                        let _ = bridge_tx.send(event);
                        ctx.request_repaint();
                    }
                });

            if let Err(error) = bridge {
                log::error!("Failed to spawn watcher bridge thread: {error}");
                self.error_message = Some(format!("Failed to start file watcher bridge: {error}"));
                return;
            }

            self.watcher = Some(fw);
            self.watcher_rx = Some(bridge_rx);
            self.watch_enabled = true;
        } else {
            log::error!("Failed to create any file watcher");
            self.error_message = Some("Failed to create file watcher".to_string());
        }
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
            // Check if we need a watcher type that doesn't currently exist
            let needs_poll = current_paths.iter().any(|p| is_gvfs_path(p));
            let needs_inotify = current_paths.iter().any(|p| !is_gvfs_path(p));
            let has_poll = fw.poll_watcher().is_some();
            let has_inotify = fw.inotify_watcher().is_some();

            if (needs_poll && !has_poll) || (needs_inotify && !has_inotify) {
                // Watcher configuration changed (e.g., first GVFS tab added), restart
                log::info!("Watcher type mismatch, restarting watchers");
                self.start_watching();
                return;
            }

            // Add new paths to the appropriate watcher
            for path in current_paths.difference(&self.watched_paths) {
                let watcher = if is_gvfs_path(path) {
                    fw.poll_watcher()
                } else {
                    fw.inotify_watcher()
                };
                if let Some(w) = watcher {
                    if let Err(e) = w.watch(path, notify::RecursiveMode::NonRecursive) {
                        log::error!("Failed to watch file {:?}: {}", path, e);
                    }
                }
            }

            // Remove old paths from the appropriate watcher
            for path in self.watched_paths.difference(&current_paths) {
                let watcher = if is_gvfs_path(path) {
                    fw.poll_watcher()
                } else {
                    fw.inotify_watcher()
                };
                if let Some(w) = watcher {
                    let _ = w.unwatch(path);
                }
            }
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

        // Desired set: root + each expanded dir (local, still existing).
        let mut desired: HashSet<PathBuf> = HashSet::new();
        if let Some(root) = &self.file_explorer.root {
            if !is_gvfs_path(root) {
                desired.insert(root.clone());
            }
        }
        for d in &self.file_explorer.expanded_dirs {
            if !is_gvfs_path(d) && d.is_dir() {
                desired.insert(d.clone());
            }
        }

        if let Some(fw) = &mut self.watcher {
            if let Some(w) = fw.inotify_watcher() {
                for path in desired.difference(&self.watched_explorer_dirs) {
                    if let Err(e) = w.watch(path, notify::RecursiveMode::NonRecursive) {
                        log::error!("Failed to watch explorer dir {:?}: {}", path, e);
                    }
                }
                for path in self.watched_explorer_dirs.difference(&desired) {
                    let _ = w.unwatch(path);
                }
            }
        }

        self.watched_explorer_dirs = desired;
    }

    pub(crate) fn check_file_changes(&mut self) -> Vec<PathBuf> {
        let Some(rx) = &self.watcher_rx else {
            // Attempt recovery if watching is enabled and there's something to watch
            // Check actual tabs and explorer root, not watched_paths (which may be empty after failure)
            let has_watchable = !self.tabs.is_empty() || self.file_explorer.root.is_some();
            if self.watch_enabled && has_watchable {
                if let Some(attempt) = next_watcher_retry(self.watcher_retry_count) {
                    self.watcher_retry_count = attempt;
                    log::info!("Attempting to recover file watcher (attempt {attempt})");
                    self.start_watching_attempt();
                    self.egui_ctx.request_repaint_after(Duration::from_secs(2));
                } else {
                    self.error_message = Some(format!(
                        "File watcher failed after {MAX_WATCHER_RETRIES} retries"
                    ));
                    self.watch_enabled = false;
                }
            }
            return Vec::new();
        };

        let mut changed_paths = Vec::new();

        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(events) => {
                    self.watcher_retry_count = 0;
                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            log::debug!("File change detected: {:?}", event.path);
                            changed_paths.push(event.path);
                        }
                    }
                }
                Err(e) => {
                    log::error!("File watcher error: {}", e);
                    self.watcher = None;
                    self.watcher_rx = None;

                    if let Some(attempt) = next_watcher_retry(self.watcher_retry_count) {
                        self.watcher_retry_count = attempt;
                        log::info!("Attempting watcher recovery (attempt {attempt})");
                        self.start_watching_attempt();
                        self.egui_ctx.request_repaint_after(Duration::from_secs(2));
                    } else {
                        self.error_message = Some(format!(
                            "File watcher failed after {} retries: {}",
                            MAX_WATCHER_RETRIES, e
                        ));
                        self.watch_enabled = false;
                    }
                    return Vec::new();
                }
            }
        }

        changed_paths
    }

    pub(crate) fn reload_changed_tabs(&mut self, changed_paths: Vec<PathBuf>) {
        let now = Instant::now();
        let mut refresh_directories = HashSet::new();
        // If the active tab gets reloaded while the find bar is open, its
        // `search_matches` will be cleared by `Tab::reload`. Force a rebuild
        // on the next frame by invalidating the cache-validity shadow state.
        let active_path = self.tabs.get(self.active_tab).map(|t| t.path.clone());

        for path in changed_paths {
            // Trigger flash effect for the changed file (use canonical path for consistent lookup)
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            self.flashing_paths.insert(canonical, now);

            // Also flash parent directories up to the explorer root
            if let Some(root) = &self.file_explorer.root {
                // Check if the changed path is within the explorer root
                if path.starts_with(root) {
                    if let Some(parent) = path.parent() {
                        refresh_directories.insert(parent.to_path_buf());
                    }
                }

                let mut current = path.parent();
                while let Some(parent) = current {
                    if parent.starts_with(root) || parent == root {
                        self.flashing_paths.insert(
                            parent
                                .canonicalize()
                                .unwrap_or_else(|_| parent.to_path_buf()),
                            now,
                        );
                    }
                    if parent == root {
                        break;
                    }
                    current = parent.parent();
                }
            }

            // Reload the tab content
            let mut active_was_reloaded = false;
            for tab in &mut self.tabs {
                if tab.path == path {
                    // Never clobber unsaved edit-mode edits with a disk
                    // reload (dirty-reload suppression, mirroring the
                    // dsh-better-sidebar editor). A reload of identical
                    // content (e.g. our own save echoing back through the
                    // watcher) still proceeds.
                    if tab.is_editing && tab.content != tab.saved_content {
                        log::info!("Skipping reload of dirty edit tab: {:?}", path);
                        continue;
                    }
                    log::info!("Reloading tab: {:?}", path);
                    if let Err(error) = tab.reload() {
                        self.error_message =
                            Some(format!("Unable to reload {}: {error}", path.display()));
                        continue;
                    }
                    if Some(&tab.path) == active_path.as_ref() {
                        active_was_reloaded = true;
                    }
                }
            }
            if active_was_reloaded && self.search.is_open {
                // Invalidate maybe_rebuild_search's "cached for" shadow
                self.search.last_tab = None;
            }
        }

        // Refresh only affected parents instead of rebuilding the entire tree.
        for directory in refresh_directories {
            log::debug!("Refreshing explorer directory: {:?}", directory);
            self.file_explorer.refresh_directory(&directory);
        }
    }
}
