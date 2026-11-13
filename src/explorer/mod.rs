//! File explorer tree model: lazy directory scanning, sorting, and expansion state.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Action from file explorer interaction
#[derive(Default)]
pub(crate) struct ExplorerAction {
    /// File selected with the primary mouse button. The caller decides whether
    /// it replaces the active document or opens a tab from the Ctrl modifier.
    pub(crate) file_to_open: Option<PathBuf>,
    /// File to close (middle-click on open file)
    pub(crate) file_to_close: Option<PathBuf>,
    /// Directory to toggle expansion (deferred to avoid clone)
    pub(crate) dir_to_toggle: Option<PathBuf>,
}

/// Sort order for file explorer
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum SortOrder {
    #[default]
    NameAsc,
    NameDesc,
    DateAsc,
    DateDesc,
}

impl SortOrder {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            SortOrder::NameAsc => "Name A-Z",
            SortOrder::NameDesc => "Name Z-A",
            SortOrder::DateAsc => "Oldest First",
            SortOrder::DateDesc => "Newest First",
        }
    }
}

/// A node in the file explorer tree
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
        /// None = not yet loaded, Some = loaded (may be empty)
        children: Option<Vec<FileTreeNode>>,
    },
}

impl FileTreeNode {
    fn name(&self) -> &str {
        match self {
            FileTreeNode::File { name, .. } => name,
            FileTreeNode::Directory { name, .. } => name,
        }
    }

    fn modified(&self) -> Option<std::time::SystemTime> {
        match self {
            FileTreeNode::File { modified, .. } => *modified,
            FileTreeNode::Directory { modified, .. } => *modified,
        }
    }

    fn is_directory(&self) -> bool {
        matches!(self, FileTreeNode::Directory { .. })
    }
}

pub(crate) struct ExplorerScanResult {
    root: PathBuf,
    sort_order: SortOrder,
    tree: Vec<FileTreeNode>,
}

/// File explorer state
#[derive(Default)]
pub(crate) struct FileExplorer {
    pub(crate) root: Option<PathBuf>,
    pub(crate) tree: Vec<FileTreeNode>,
    pub(crate) expanded_dirs: HashSet<PathBuf>,
    pub(crate) sort_order: SortOrder,
    /// Receiver for asynchronous root-directory scan results.
    pub(crate) pending_scan: Option<Receiver<ExplorerScanResult>>,
}

impl FileExplorer {
    /// Scan a directory shallowly - only one level, subdirectories marked as unloaded
    fn scan_directory_shallow(path: &PathBuf, sort_order: SortOrder) -> Vec<FileTreeNode> {
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
            } else if Self::is_markdown_file(&entry_path) {
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

    /// Sort nodes according to the given sort order (directories always on top)
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
                SortOrder::DateAsc => {
                    // Oldest first: None (unknown) sorts last
                    match (a.modified(), b.modified()) {
                        (Some(a_time), Some(b_time)) => a_time.cmp(&b_time),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name().cmp(b.name()),
                    }
                }
                SortOrder::DateDesc => {
                    // Newest first: None (unknown) sorts last
                    match (a.modified(), b.modified()) {
                        (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name().cmp(b.name()),
                    }
                }
            }
        });
    }

    fn is_markdown_file(path: &Path) -> bool {
        path.extension()
            .map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                ext == "md" || ext == "markdown" || ext == "txt"
            })
            .unwrap_or(false)
    }

    /// Set root directory and start a shallow background scan.
    pub(crate) fn set_root(&mut self, path: PathBuf) {
        // Convert empty path to current directory
        let path = if path.as_os_str().is_empty() {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            path
        };
        self.root = Some(path.clone());
        if !self.start_root_scan(path.clone(), "explorer-scan") {
            self.tree = Self::scan_directory_shallow(&path, self.sort_order);
            Self::restore_expanded_children(&mut self.tree, &self.expanded_dirs, self.sort_order);
        }
    }

    fn start_root_scan(&mut self, path: PathBuf, thread_name: &str) -> bool {
        let sort_order = self.sort_order;
        let expanded_dirs = self.expanded_dirs.clone();
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name(thread_name.to_owned())
            .spawn(move || {
                let mut tree = Self::scan_directory_shallow(&path, sort_order);
                Self::restore_expanded_children(&mut tree, &expanded_dirs, sort_order);
                let _ = tx.send(ExplorerScanResult {
                    root: path,
                    sort_order,
                    tree,
                });
            });
        if spawned.is_ok() {
            self.pending_scan = Some(rx);
            true
        } else {
            false
        }
    }

    /// Check if a background scan completed and apply results
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
        Self::restore_expanded_children(&mut result.tree, &self.expanded_dirs, self.sort_order);
        self.tree = result.tree;
        true
    }

    /// Refresh the file tree in the background.
    pub(crate) fn refresh(&mut self) {
        if let Some(root) = &self.root.clone() {
            if self.start_root_scan(root.clone(), "explorer-refresh") {
                return;
            }
            self.tree = Self::scan_directory_shallow(root, self.sort_order);
            Self::restore_expanded_children(&mut self.tree, &self.expanded_dirs, self.sort_order);
        }
    }

    fn restore_expanded_children(
        tree: &mut [FileTreeNode],
        expanded_dirs: &HashSet<PathBuf>,
        sort_order: SortOrder,
    ) {
        // Parents must be loaded before their expanded descendants can be
        // found in the lazy tree.
        let mut expanded: Vec<&PathBuf> = expanded_dirs.iter().collect();
        expanded.sort_by_key(|path| path.components().count());
        for path in expanded {
            Self::load_children_in_tree(tree, path, sort_order);
        }
    }

    /// Rescan only one changed directory while preserving the rest of the tree.
    pub(crate) fn refresh_directory(&mut self, directory: &Path) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        if directory == root {
            self.tree = Self::scan_directory_shallow(&root.clone(), self.sort_order);
        } else {
            let mut replacement = Some(Self::scan_directory_shallow(
                &directory.to_path_buf(),
                self.sort_order,
            ));
            if !Self::replace_directory_children(&mut self.tree, directory, &mut replacement) {
                return;
            }
        }

        // Restore expanded descendants from shallowest to deepest so their
        // parents exist before lazy children are loaded.
        let mut expanded: Vec<PathBuf> = self.expanded_dirs.iter().cloned().collect();
        expanded.sort_by_key(|path| path.components().count());
        for path in expanded {
            self.load_children(&path);
        }
    }

    fn replace_directory_children(
        nodes: &mut [FileTreeNode],
        directory: &Path,
        replacement: &mut Option<Vec<FileTreeNode>>,
    ) -> bool {
        for node in nodes {
            if let FileTreeNode::Directory { path, children, .. } = node {
                if path == directory {
                    *children = replacement.take();
                    return true;
                }
                if let Some(children) = children {
                    if Self::replace_directory_children(children, directory, replacement) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Load children for a specific directory (lazy loading)
    pub(crate) fn load_children(&mut self, dir_path: &PathBuf) {
        Self::load_children_in_tree(&mut self.tree, dir_path, self.sort_order);
    }

    /// Recursively find and load children for a directory in the tree
    fn load_children_in_tree(
        nodes: &mut [FileTreeNode],
        target_path: &PathBuf,
        sort_order: SortOrder,
    ) -> bool {
        for node in nodes.iter_mut() {
            if let FileTreeNode::Directory { path, children, .. } = node {
                if path == target_path {
                    // Found the target directory - load its children if not loaded
                    if children.is_none() {
                        *children = Some(Self::scan_directory_shallow(path, sort_order));
                    }
                    return true;
                }
                // Recurse into loaded children
                if let Some(ref mut child_nodes) = children {
                    if Self::load_children_in_tree(child_nodes, target_path, sort_order) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Toggle directory expansion (loads children if not yet loaded)
    pub(crate) fn toggle_expanded(&mut self, path: &PathBuf) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            // Load children before expanding if not yet loaded
            self.load_children(path);
            self.expanded_dirs.insert(path.clone());
        }
    }

    /// Check if a directory is expanded
    pub(crate) fn is_expanded(&self, path: &PathBuf) -> bool {
        self.expanded_dirs.contains(path)
    }

    /// Set sort order and re-sort the tree in place
    pub(crate) fn set_sort_order(&mut self, order: SortOrder) {
        if self.sort_order != order {
            self.sort_order = order;
            Self::resort_tree_recursive(&mut self.tree, order);
        }
    }

    /// Recursively re-sort all nodes in the tree
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

    /// Get children for a directory by path (looks up in original tree, not clone)
    pub(crate) fn get_children(&self, target_path: &PathBuf) -> Option<&Vec<FileTreeNode>> {
        Self::find_children_in_tree(&self.tree, target_path)
    }

    /// Recursively find children for a directory in the tree
    fn find_children_in_tree<'a>(
        nodes: &'a [FileTreeNode],
        target_path: &PathBuf,
    ) -> Option<&'a Vec<FileTreeNode>> {
        for node in nodes {
            if let FileTreeNode::Directory { path, children, .. } = node {
                if path == target_path {
                    return children.as_ref();
                }
                // Recurse into loaded children
                if let Some(child_nodes) = children {
                    if let Some(found) = Self::find_children_in_tree(child_nodes, target_path) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    /// Maximum depth for expand_all to prevent excessive recursion
    const MAX_EXPAND_DEPTH: usize = 10;

    /// Expand all directories in the tree (loads all children recursively up to MAX_EXPAND_DEPTH)
    pub(crate) fn expand_all(&mut self) {
        // First, recursively load all directories (with depth limit)
        Self::load_all_children(&mut self.tree, self.sort_order, 0);
        // Then collect all directory paths
        self.expanded_dirs = Self::collect_all_dirs(&self.tree);
    }

    /// Recursively load all unloaded directories (up to MAX_EXPAND_DEPTH)
    fn load_all_children(nodes: &mut [FileTreeNode], sort_order: SortOrder, depth: usize) {
        if depth >= Self::MAX_EXPAND_DEPTH {
            return;
        }
        for node in nodes.iter_mut() {
            if let FileTreeNode::Directory { path, children, .. } = node {
                // Load children if not yet loaded
                if children.is_none() {
                    *children = Some(Self::scan_directory_shallow(path, sort_order));
                }
                // Recurse into children
                if let Some(ref mut child_nodes) = children {
                    Self::load_all_children(child_nodes, sort_order, depth + 1);
                }
            }
        }
    }

    /// Collapse all directories in the tree
    pub(crate) fn collapse_all(&mut self) {
        self.expanded_dirs.clear();
    }

    /// Collect all directory paths from a tree recursively (only loaded directories)
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
