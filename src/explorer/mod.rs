//! File explorer tree model: lazy directory scanning, sorting, and expansion state.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use serde::{Deserialize, Serialize};

use crate::links::has_markdown_extension;

#[cfg(test)]
mod tests;

/// Sort order for file explorer.
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum SortOrder {
    #[default]
    NameAsc,
    NameDesc,
    DateAsc,
    DateDesc,
}

impl SortOrder {
    /// Every order, as the explorer's menu lists them.
    pub(crate) const ALL: [Self; 4] =
        [Self::NameAsc, Self::NameDesc, Self::DateAsc, Self::DateDesc];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::NameAsc => "Name A-Z",
            Self::NameDesc => "Name Z-A",
            Self::DateAsc => "Oldest First",
            Self::DateDesc => "Newest First",
        }
    }
}

/// A node in the file explorer tree.
#[derive(Clone)]
pub(crate) enum FileTreeNode {
    File {
        path: PathBuf,
        name: String,
        modified: Option<std::time::SystemTime>,
    },
    Directory {
        path: PathBuf,
        name: String,
        modified: Option<std::time::SystemTime>,
        /// None = not yet loaded, Some = loaded (may be empty).
        children: Option<Vec<Self>>,
    },
}

impl FileTreeNode {
    fn name(&self) -> &str {
        match self {
            Self::File { name, .. } | Self::Directory { name, .. } => name,
        }
    }

    const fn modified(&self) -> Option<std::time::SystemTime> {
        match self {
            Self::File { modified, .. } | Self::Directory { modified, .. } => *modified,
        }
    }

    const fn is_directory(&self) -> bool {
        matches!(self, Self::Directory { .. })
    }
}

pub(crate) struct ExplorerScanResult {
    root: PathBuf,
    sort_order: SortOrder,
    tree: Vec<FileTreeNode>,
}

/// A scan of the explorer root, with the settings it was requested under so
/// it can run on another thread while the explorer keeps changing.
#[derive(Clone)]
struct RootScan {
    root: PathBuf,
    sort_order: SortOrder,
    expanded_dirs: HashSet<PathBuf>,
}

impl RootScan {
    fn run(self) -> ExplorerScanResult {
        let mut tree = FileExplorer::scan_directory_shallow(&self.root, self.sort_order);
        FileExplorer::load_expanded(&mut tree, &self.expanded_dirs, self.sort_order);
        ExplorerScanResult {
            root: self.root,
            sort_order: self.sort_order,
            tree,
        }
    }
}

/// A job for a worker thread.
type Work = Box<dyn FnOnce() + Send>;

/// Starts a job on a thread with the given name, or says why it could not.
type SpawnThread = fn(&str, Work) -> io::Result<()>;

/// Run `work` on a new thread called `name`.
fn spawn_named_thread(name: &str, work: Work) -> io::Result<()> {
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(work)
        .map(drop)
}

/// File explorer state.
#[derive(Default)]
pub(crate) struct FileExplorer {
    root: Option<PathBuf>,
    tree: Vec<FileTreeNode>,
    expanded_dirs: HashSet<PathBuf>,
    sort_order: SortOrder,
    /// Receiver for asynchronous root-directory scan results.
    pending_scan: Option<Receiver<ExplorerScanResult>>,
}

impl FileExplorer {
    /// The explorer as a session left it: its order and its open folders
    /// are in place before `root` is scanned, so the scan reads them.
    pub(crate) fn restored(
        sort_order: SortOrder,
        expanded_dirs: HashSet<PathBuf>,
        root: Option<PathBuf>,
    ) -> Self {
        let mut explorer = Self {
            expanded_dirs,
            sort_order,
            ..Self::default()
        };
        if let Some(root) = root {
            explorer.set_root(root);
        }
        explorer
    }

    /// The folder the explorer lists.
    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The listing, as far as it has been read.
    pub(crate) fn tree(&self) -> &[FileTreeNode] {
        &self.tree
    }

    /// The folders left open, whether or not the listing reaches them.
    pub(crate) const fn expanded_dirs(&self) -> &HashSet<PathBuf> {
        &self.expanded_dirs
    }

    pub(crate) const fn sort_order(&self) -> SortOrder {
        self.sort_order
    }

    /// Whether a scan of the root is still to be collected.
    pub(crate) const fn is_scanning(&self) -> bool {
        self.pending_scan.is_some()
    }

    /// Scan a directory shallowly - only one level, subdirectories marked as unloaded.
    fn scan_directory_shallow(path: &Path, sort_order: SortOrder) -> Vec<FileTreeNode> {
        let Ok(entries) = fs::read_dir(path) else {
            return Vec::new();
        };

        let mut nodes: Vec<FileTreeNode> = Vec::new();

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }

            let modified = entry.metadata().ok().and_then(|m| m.modified().ok());

            if entry_path.is_dir() {
                // Show all directories - let users expand what they want
                // (Avoids O(n×m) scanning during initial directory scan)
                nodes.push(FileTreeNode::Directory {
                    path: entry_path,
                    name,
                    modified,
                    children: None, // Lazy - not loaded yet
                });
            } else if has_markdown_extension(&entry_path) {
                nodes.push(FileTreeNode::File {
                    path: entry_path,
                    name,
                    modified,
                });
            }
        }

        Self::sort_nodes(&mut nodes, sort_order);
        nodes
    }

    /// Sort nodes according to the given sort order (directories always on top).
    fn sort_nodes(nodes: &mut [FileTreeNode], sort_order: SortOrder) {
        nodes.sort_by(|a, b| {
            // Directories always come first
            match (a.is_directory(), b.is_directory()) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // Within the same type, sort by the selected criteria
            match sort_order {
                SortOrder::NameAsc => a.name().cmp(b.name()),
                SortOrder::NameDesc => b.name().cmp(a.name()),
                SortOrder::DateAsc => Self::by_date(a, b, false),
                SortOrder::DateDesc => Self::by_date(a, b, true),
            }
        });
    }

    /// Order two nodes by modification time. A node whose time is unknown
    /// goes last in both directions, and two such nodes go by name. Equal
    /// times compare equal, so the sort keeps the order they were in.
    fn by_date(a: &FileTreeNode, b: &FileTreeNode, newest_first: bool) -> std::cmp::Ordering {
        match (a.modified(), b.modified()) {
            (Some(a_time), Some(b_time)) if newest_first => b_time.cmp(&a_time),
            (Some(a_time), Some(b_time)) => a_time.cmp(&b_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name().cmp(b.name()),
        }
    }

    /// Set root directory and start a shallow background scan.
    pub(crate) fn set_root(&mut self, path: PathBuf) {
        // Convert empty path to current directory
        let path = if path.as_os_str().is_empty() {
            std::env::current_dir().unwrap_or(path)
        } else {
            path
        };
        self.root = Some(path.clone());
        self.scan_root(path, "explorer-scan", spawn_named_thread);
    }

    /// Scan `root` on a thread started through `spawn`, to be collected by
    /// `poll_pending_scan`. If no thread can be started, scan right here, so
    /// the tree still appears.
    fn scan_root(&mut self, root: PathBuf, thread_name: &str, spawn: SpawnThread) {
        let scan = RootScan {
            root,
            sort_order: self.sort_order,
            expanded_dirs: self.expanded_dirs.clone(),
        };
        let (tx, rx) = mpsc::channel();
        let background = scan.clone();
        let work: Work = Box::new(move || {
            let _ = tx.send(background.run());
        });
        if spawn(thread_name, work).is_ok() {
            self.pending_scan = Some(rx);
        } else {
            // An older scan still running would overwrite this newer tree.
            self.pending_scan = None;
            self.tree = scan.run().tree;
        }
    }

    /// Check if a background scan completed and apply results.
    pub(crate) fn poll_pending_scan(&mut self) -> bool {
        let received = self.pending_scan.as_ref().map(Receiver::try_recv);
        let mut result = match received {
            Some(Ok(result)) => result,
            Some(Err(TryRecvError::Empty)) | None => return false,
            Some(Err(TryRecvError::Disconnected)) => {
                self.pending_scan = None;
                return false;
            }
        };
        self.pending_scan = None;
        if self.root.as_deref() != Some(result.root.as_path()) {
            return false;
        }
        if result.sort_order != self.sort_order {
            Self::resort_tree_recursive(&mut result.tree, self.sort_order);
        }
        // Expansion can change while the worker is running. Reuse its snapshot
        // and load any directories expanded after it started.
        Self::load_expanded(&mut result.tree, &self.expanded_dirs, self.sort_order);
        self.tree = result.tree;
        true
    }

    /// Refresh the file tree in the background.
    pub(crate) fn refresh(&mut self) {
        if let Some(root) = self.root.clone() {
            self.scan_root(root, "explorer-refresh", spawn_named_thread);
        }
    }

    /// Load every expanded directory that can be reached through loaded
    /// ones. A directory is loaded before the walk goes into it, so an
    /// expanded directory below it is found in the same pass.
    fn load_expanded(
        nodes: &mut [FileTreeNode],
        expanded_dirs: &HashSet<PathBuf>,
        sort_order: SortOrder,
    ) {
        for node in nodes {
            let FileTreeNode::Directory { path, children, .. } = node else {
                continue;
            };
            if expanded_dirs.contains(path) {
                children.get_or_insert_with(|| Self::scan_directory_shallow(path, sort_order));
            }
            if let Some(loaded) = children {
                Self::load_expanded(loaded, expanded_dirs, sort_order);
            }
        }
    }

    /// Rescan only one changed directory while preserving the rest of the tree.
    pub(crate) fn refresh_directory(&mut self, directory: &Path) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        if directory == root {
            self.tree = Self::scan_directory_shallow(root, self.sort_order);
        } else {
            let Some(children) = Self::children_slot_mut(&mut self.tree, directory) else {
                return;
            };
            *children = Some(Self::scan_directory_shallow(directory, self.sort_order));
        }
        Self::load_expanded(&mut self.tree, &self.expanded_dirs, self.sort_order);
    }

    /// The children slot of the directory at `target_path`, searching only
    /// directories whose children are already loaded. `None` when the
    /// directory is not in the loaded tree; `Some(None)` when it is there
    /// but has not been scanned.
    fn children_slot_mut<'tree>(
        nodes: &'tree mut [FileTreeNode],
        target_path: &Path,
    ) -> Option<&'tree mut Option<Vec<FileTreeNode>>> {
        for node in nodes {
            let FileTreeNode::Directory { path, children, .. } = node else {
                continue;
            };
            if path == target_path {
                return Some(children);
            }
            let found = children
                .as_deref_mut()
                .and_then(|loaded| Self::children_slot_mut(loaded, target_path));
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// Toggle directory expansion. Expanding loads the directory and every
    /// directory below it that was left expanded.
    pub(crate) fn toggle_expanded(&mut self, path: &PathBuf) {
        if !self.expanded_dirs.remove(path) {
            self.expanded_dirs.insert(path.clone());
            Self::load_expanded(&mut self.tree, &self.expanded_dirs, self.sort_order);
        }
    }

    /// Check if a directory is expanded.
    pub(crate) fn is_expanded(&self, path: &PathBuf) -> bool {
        self.expanded_dirs.contains(path)
    }

    /// Set sort order and re-sort the tree in place.
    pub(crate) fn set_sort_order(&mut self, order: SortOrder) {
        if self.sort_order != order {
            self.sort_order = order;
            Self::resort_tree_recursive(&mut self.tree, order);
        }
    }

    /// Recursively re-sort all nodes in the tree.
    fn resort_tree_recursive(nodes: &mut [FileTreeNode], sort_order: SortOrder) {
        Self::sort_nodes(nodes, sort_order);
        for node in nodes.iter_mut() {
            if let FileTreeNode::Directory {
                children: Some(ref mut child_nodes),
                ..
            } = node
            {
                Self::resort_tree_recursive(child_nodes, sort_order);
            }
        }
    }

    /// Maximum depth for `expand_all` to prevent excessive recursion.
    const MAX_EXPAND_DEPTH: usize = 10;

    /// Expand all directories in the tree (loads all children recursively up to `MAX_EXPAND_DEPTH`).
    pub(crate) fn expand_all(&mut self) {
        // First, recursively load all directories (with depth limit)
        Self::load_all_children(&mut self.tree, self.sort_order, 0);
        // Then collect all directory paths
        self.expanded_dirs = Self::collect_all_dirs(&self.tree);
        // The directories listed at the depth limit are expanded but unread.
        Self::load_expanded(&mut self.tree, &self.expanded_dirs, self.sort_order);
    }

    /// Recursively load all unloaded directories (up to `MAX_EXPAND_DEPTH`).
    fn load_all_children(nodes: &mut [FileTreeNode], sort_order: SortOrder, depth: usize) {
        if depth >= Self::MAX_EXPAND_DEPTH {
            return;
        }
        for node in nodes.iter_mut() {
            if let FileTreeNode::Directory { path, children, .. } = node {
                let loaded =
                    children.get_or_insert_with(|| Self::scan_directory_shallow(path, sort_order));
                Self::load_all_children(loaded, sort_order, depth + 1);
            }
        }
    }

    /// Collapse all directories in the tree.
    pub(crate) fn collapse_all(&mut self) {
        self.expanded_dirs.clear();
    }

    /// Collect all directory paths from a tree recursively (only loaded directories).
    fn collect_all_dirs(nodes: &[FileTreeNode]) -> HashSet<PathBuf> {
        let mut dirs = HashSet::new();
        for node in nodes {
            if let FileTreeNode::Directory { path, children, .. } = node {
                dirs.insert(path.clone());
                // Only recurse into loaded children
                if let Some(child_nodes) = children {
                    dirs.extend(Self::collect_all_dirs(child_nodes));
                }
            }
        }
        dirs
    }
}
