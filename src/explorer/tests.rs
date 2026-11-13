use super::*;

use std::time::{Duration, Instant};

use crate::recent::now_epoch_secs;

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
    assert!(explorer
        .get_children(&expanded)
        .is_some_and(|children| children.iter().any(|node| node.name() == "guide.md")));

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

    let deadline = Instant::now() + Duration::from_secs(2);
    while explorer.pending_scan.is_some() && Instant::now() < deadline {
        explorer.poll_pending_scan();
        std::thread::sleep(Duration::from_millis(1));
    }

    assert!(explorer.pending_scan.is_none());
    assert!(explorer
        .get_children(&expanded)
        .is_some_and(|children| children.iter().any(|node| node.name() == "guide.md")));

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
    // set_root scans asynchronously since #84: drain the pending scan
    // before interacting with the tree.
    let deadline = Instant::now() + Duration::from_secs(2);
    while explorer.pending_scan.is_some() && Instant::now() < deadline {
        explorer.poll_pending_scan();
        std::thread::sleep(Duration::from_millis(1));
    }
    explorer.toggle_expanded(&docs);
    explorer.toggle_expanded(&notes);
    fs::write(docs.join("new.md"), "# New").unwrap();

    explorer.refresh_directory(&docs);

    assert!(explorer
        .get_children(&docs)
        .is_some_and(|children| children.iter().any(|node| node.name() == "new.md")));
    assert!(explorer
        .get_children(&notes)
        .is_some_and(|children| children.iter().any(|node| node.name() == "keep.md")));
    assert!(explorer.is_expanded(&docs));
    assert!(explorer.is_expanded(&notes));

    fs::remove_dir_all(root).unwrap();
}
