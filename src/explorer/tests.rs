use super::*;

use std::time::{Duration, Instant};

use crate::recent::now_epoch_secs;

/// `set_root` scans on a worker thread since #84: wait for its result
/// before interacting with the tree.
fn wait_for_scan(explorer: &mut FileExplorer) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while explorer.pending_scan.is_some() && Instant::now() < deadline {
        explorer.poll_pending_scan();
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Whether the loaded children of `directory` include an entry called `name`.
fn lists(explorer: &FileExplorer, directory: &Path, name: &str) -> bool {
    explorer
        .get_children(directory)
        .is_some_and(|children| children.iter().any(|node| node.name() == name))
}

#[test]
fn asynchronous_explorer_result_restores_expanded_children() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-explorer-refresh-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    let expanded = root.join("docs");
    fs::create_dir_all(&expanded).unwrap();
    fs::write(expanded.join("guide.md"), "# Guide").unwrap();

    let (tx, rx) = mpsc::channel();
    tx.send(ExplorerScanResult {
        root: root.clone(),
        sort_order: SortOrder::NameAsc,
        tree: FileExplorer::scan_directory_shallow(&root, SortOrder::NameAsc),
    })
    .unwrap();
    let mut explorer = FileExplorer {
        root: Some(root.clone()),
        expanded_dirs: HashSet::from([expanded.clone()]),
        pending_scan: Some(rx),
        ..Default::default()
    };

    assert!(explorer.poll_pending_scan());
    assert!(lists(&explorer, &expanded, "guide.md"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn local_root_scan_runs_asynchronously_and_restores_expansion() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-local-explorer-scan-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    let expanded = root.join("docs");
    fs::create_dir_all(&expanded).unwrap();
    fs::write(expanded.join("guide.md"), "# Guide").unwrap();

    let mut explorer = FileExplorer {
        expanded_dirs: HashSet::from([expanded.clone()]),
        ..Default::default()
    };
    explorer.set_root(root.clone());

    wait_for_scan(&mut explorer);

    assert!(explorer.pending_scan.is_none());
    assert!(lists(&explorer, &expanded, "guide.md"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn targeted_explorer_refresh_preserves_unrelated_subtrees() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-targeted-refresh-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    let docs = root.join("docs");
    let notes = root.join("notes");
    fs::create_dir_all(&docs).unwrap();
    fs::create_dir_all(&notes).unwrap();
    fs::write(docs.join("old.md"), "# Old").unwrap();
    fs::write(notes.join("keep.md"), "# Keep").unwrap();

    let mut explorer = FileExplorer::default();
    explorer.set_root(root.clone());
    wait_for_scan(&mut explorer);
    explorer.toggle_expanded(&docs);
    explorer.toggle_expanded(&notes);
    fs::write(docs.join("new.md"), "# New").unwrap();

    explorer.refresh_directory(&docs);

    assert!(lists(&explorer, &docs, "new.md"));
    assert!(lists(&explorer, &notes, "keep.md"));
    assert!(explorer.is_expanded(&docs));
    assert!(explorer.is_expanded(&notes));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_directory_loads_and_refreshes_below_an_expanded_parent() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-nested-refresh-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    let docs = root.join("docs");
    let deep = docs.join("deep");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("old.md"), "# Old").unwrap();

    let mut explorer = FileExplorer::default();
    explorer.set_root(root.clone());
    wait_for_scan(&mut explorer);
    explorer.toggle_expanded(&docs);
    explorer.toggle_expanded(&deep);
    assert!(lists(&explorer, &deep, "old.md"));

    fs::write(deep.join("new.md"), "# New").unwrap();
    explorer.refresh_directory(&deep);

    assert!(lists(&explorer, &deep, "new.md"));
    assert!(lists(&explorer, &docs, "deep"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refreshing_a_directory_below_an_unloaded_parent_changes_nothing() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-unloaded-refresh-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    let docs = root.join("docs");
    let deep = docs.join("deep");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("guide.md"), "# Guide").unwrap();

    let mut explorer = FileExplorer::default();
    explorer.set_root(root.clone());
    wait_for_scan(&mut explorer);

    explorer.refresh_directory(&deep);

    assert!(explorer.get_children(&docs).is_none());
    assert!(explorer.get_children(&deep).is_none());

    fs::remove_dir_all(root).unwrap();
}
