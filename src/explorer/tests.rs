use super::*;

use std::time::{Duration, SystemTime};

use crate::test_support::{wait_until, ScratchDir};

/// `set_root` and `refresh` scan on a worker thread since #84: wait for the
/// result before interacting with the tree.
fn wait_for_scan(explorer: &mut FileExplorer) {
    let landed = wait_until(Duration::from_secs(20), || {
        explorer.poll_pending_scan();
        explorer.pending_scan.is_none()
    });
    assert!(landed, "the background scan never reported back");
}

/// An explorer showing `root`, with the first scan already applied.
fn explorer_at(root: &Path) -> FileExplorer {
    let mut explorer = FileExplorer::default();
    explorer.set_root(root.to_path_buf());
    wait_for_scan(&mut explorer);
    explorer
}

fn names(nodes: &[FileTreeNode]) -> Vec<&str> {
    nodes.iter().map(FileTreeNode::name).collect()
}

/// Names of the loaded children of `directory`, in display order.
fn child_names<'tree>(explorer: &'tree FileExplorer, directory: &Path) -> Vec<&'tree str> {
    names(
        explorer
            .get_children(directory)
            .expect("directory is loaded"),
    )
}

/// Whether the loaded children of `directory` include an entry called `name`.
fn lists(explorer: &FileExplorer, directory: &Path, name: &str) -> bool {
    explorer
        .get_children(directory)
        .is_some_and(|children| children.iter().any(|node| node.name() == name))
}

fn file_node(name: &str, modified_secs: Option<u64>) -> FileTreeNode {
    FileTreeNode::File {
        path: PathBuf::from(name),
        name: name.to_owned(),
        modified: modified_secs.map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
    }
}

fn sorted(mut nodes: Vec<FileTreeNode>, order: SortOrder) -> Vec<String> {
    FileExplorer::sort_nodes(&mut nodes, order);
    nodes.iter().map(|node| node.name().to_owned()).collect()
}

fn set_modified(path: &Path, secs: u64) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
        .unwrap();
}

#[test]
fn root_scan_lists_folders_first_then_documents_and_skips_everything_else() {
    let dir = ScratchDir::new();
    dir.dir("zeta");
    dir.dir(".git");
    for name in ["b.md", "a.markdown", "notes.txt", "image.png", ".hidden.md"] {
        dir.file(name, "text");
    }

    let explorer = explorer_at(dir.path());

    assert_eq!(
        names(&explorer.tree),
        ["zeta", "a.markdown", "b.md", "notes.txt"]
    );
}

#[test]
fn root_that_cannot_be_read_shows_an_empty_tree() {
    let dir = ScratchDir::new();
    let explorer = explorer_at(&dir.path().join("gone"));
    assert!(explorer.tree.is_empty());
}

#[test]
fn empty_root_path_means_the_working_directory() {
    let mut explorer = FileExplorer::default();
    explorer.set_root(PathBuf::new());
    assert_eq!(explorer.root, Some(std::env::current_dir().unwrap()));
}

#[test]
fn sort_orders_have_distinct_menu_labels() {
    let labels = [
        SortOrder::NameAsc,
        SortOrder::NameDesc,
        SortOrder::DateAsc,
        SortOrder::DateDesc,
    ]
    .map(SortOrder::label);
    assert_eq!(
        labels,
        ["Name A-Z", "Name Z-A", "Oldest First", "Newest First"]
    );
}

#[test]
fn changing_the_sort_order_reorders_every_loaded_level() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    for name in ["x.md", "y.md", "docs/a.md", "docs/b.md"] {
        dir.file(name, "text");
    }
    let mut explorer = explorer_at(dir.path());
    explorer.toggle_expanded(&docs);

    explorer.set_sort_order(SortOrder::NameDesc);

    assert_eq!(names(&explorer.tree), ["docs", "y.md", "x.md"]);
    assert_eq!(child_names(&explorer, &docs), ["b.md", "a.md"]);

    explorer.set_sort_order(SortOrder::NameDesc);
    assert_eq!(names(&explorer.tree), ["docs", "y.md", "x.md"]);
    assert!(explorer.sort_order == SortOrder::NameDesc);
}

#[test]
fn date_orders_follow_the_modification_times_on_disk() {
    let dir = ScratchDir::new();
    set_modified(&dir.file("a-new.md", "text"), 2_000_000);
    set_modified(&dir.file("b-old.md", "text"), 1_000_000);
    set_modified(&dir.file("c-mid.md", "text"), 1_500_000);
    let mut explorer = explorer_at(dir.path());

    explorer.set_sort_order(SortOrder::DateAsc);
    assert_eq!(names(&explorer.tree), ["b-old.md", "c-mid.md", "a-new.md"]);

    explorer.set_sort_order(SortOrder::DateDesc);
    assert_eq!(names(&explorer.tree), ["a-new.md", "c-mid.md", "b-old.md"]);
}

#[test]
fn entries_without_a_modification_time_go_last_in_both_date_orders() {
    for order in [SortOrder::DateAsc, SortOrder::DateDesc] {
        // Both input orders, so the comparator sees the dated entry on
        // either side.
        let dated_first = vec![
            file_node("dated.md", Some(5)),
            file_node("undated.md", None),
        ];
        let undated_first = vec![
            file_node("undated.md", None),
            file_node("dated.md", Some(5)),
        ];
        assert_eq!(sorted(dated_first, order), ["dated.md", "undated.md"]);
        assert_eq!(sorted(undated_first, order), ["dated.md", "undated.md"]);

        let undated = vec![file_node("b.md", None), file_node("a.md", None)];
        assert_eq!(sorted(undated, order), ["a.md", "b.md"]);
    }
}

#[test]
fn folders_are_ordered_among_themselves_like_files() {
    let folder = |name: &str, modified_secs: u64| FileTreeNode::Directory {
        path: PathBuf::from(name),
        name: name.to_owned(),
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(modified_secs)),
        children: None,
    };
    let folders = || vec![folder("a-new", 9), folder("b-old", 1)];

    assert_eq!(sorted(folders(), SortOrder::NameDesc), ["b-old", "a-new"]);
    assert_eq!(sorted(folders(), SortOrder::DateAsc), ["b-old", "a-new"]);
    assert_eq!(sorted(folders(), SortOrder::DateDesc), ["a-new", "b-old"]);
}

#[test]
fn folders_stay_on_top_in_every_sort_order() {
    let folder = || FileTreeNode::Directory {
        path: PathBuf::from("m"),
        name: "m".to_owned(),
        modified: None,
        children: None,
    };
    for order in [
        SortOrder::NameAsc,
        SortOrder::NameDesc,
        SortOrder::DateAsc,
        SortOrder::DateDesc,
    ] {
        let folder_last = vec![
            file_node("a.md", Some(1)),
            file_node("z.md", Some(9)),
            folder(),
        ];
        assert_eq!(sorted(folder_last, order)[0], "m");
        let folder_first = vec![folder(), file_node("a.md", Some(1))];
        assert_eq!(sorted(folder_first, order), ["m", "a.md"]);
    }
}

#[test]
fn asynchronous_explorer_result_restores_expanded_children() {
    let dir = ScratchDir::new();
    let expanded = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");

    let (tx, rx) = mpsc::channel();
    tx.send(ExplorerScanResult {
        root: dir.path().to_path_buf(),
        sort_order: SortOrder::NameAsc,
        tree: FileExplorer::scan_directory_shallow(dir.path(), SortOrder::NameAsc),
    })
    .unwrap();
    let mut explorer = FileExplorer {
        root: Some(dir.path().to_path_buf()),
        expanded_dirs: HashSet::from([expanded.clone()]),
        pending_scan: Some(rx),
        ..Default::default()
    };

    assert!(explorer.poll_pending_scan());
    assert!(lists(&explorer, &expanded, "guide.md"));
}

/// An explorer on `root` holding the sending end of its pending scan.
fn explorer_awaiting_scan(root: &Path) -> (FileExplorer, mpsc::Sender<ExplorerScanResult>) {
    let (tx, rx) = mpsc::channel();
    let explorer = FileExplorer {
        root: Some(root.to_path_buf()),
        pending_scan: Some(rx),
        ..Default::default()
    };
    (explorer, tx)
}

#[test]
fn scan_result_for_a_previous_root_is_dropped() {
    let (mut explorer, tx) = explorer_awaiting_scan(Path::new("/current"));
    tx.send(ExplorerScanResult {
        root: PathBuf::from("/previous"),
        sort_order: SortOrder::NameAsc,
        tree: vec![file_node("stale.md", None)],
    })
    .unwrap();

    assert!(!explorer.poll_pending_scan());
    assert!(explorer.tree.is_empty());
    assert!(explorer.pending_scan.is_none());
}

#[test]
fn scan_result_is_resorted_when_the_order_changed_while_it_ran() {
    let (mut explorer, tx) = explorer_awaiting_scan(Path::new("/current"));
    explorer.sort_order = SortOrder::NameDesc;
    tx.send(ExplorerScanResult {
        root: PathBuf::from("/current"),
        sort_order: SortOrder::NameAsc,
        tree: vec![file_node("a.md", None), file_node("b.md", None)],
    })
    .unwrap();

    assert!(explorer.poll_pending_scan());
    assert_eq!(names(&explorer.tree), ["b.md", "a.md"]);
}

#[test]
fn polling_reports_nothing_until_a_scan_lands() {
    assert!(!FileExplorer::default().poll_pending_scan());

    let (mut explorer, tx) = explorer_awaiting_scan(Path::new("/current"));
    assert!(!explorer.poll_pending_scan());
    assert!(explorer.pending_scan.is_some(), "still waiting");

    // A worker that died without reporting must not be waited on forever.
    drop(tx);
    assert!(!explorer.poll_pending_scan());
    assert!(explorer.pending_scan.is_none());
}

#[test]
fn local_root_scan_runs_asynchronously_and_restores_expansion() {
    let dir = ScratchDir::new();
    let expanded = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");

    let mut explorer = FileExplorer {
        expanded_dirs: HashSet::from([expanded.clone()]),
        ..Default::default()
    };
    explorer.set_root(dir.path().to_path_buf());

    wait_for_scan(&mut explorer);

    assert!(explorer.pending_scan.is_none());
    assert!(lists(&explorer, &expanded, "guide.md"));
}

/// The expanded set has no order of its own; a folder can only be found
/// once the folders above it are loaded.
#[test]
fn root_scan_restores_expanded_folders_at_every_depth() {
    let dir = ScratchDir::new();
    let mut expanded = Vec::new();
    let mut folder = dir.path().to_path_buf();
    for depth in 0..6 {
        folder = folder.join(format!("level-{depth}"));
        expanded.push(folder.clone());
    }
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("deepest.md"), "# Deepest").unwrap();

    let mut explorer = FileExplorer {
        expanded_dirs: expanded.iter().cloned().collect(),
        ..Default::default()
    };
    explorer.set_root(dir.path().to_path_buf());
    wait_for_scan(&mut explorer);

    assert!(lists(&explorer, &folder, "deepest.md"));
}

/// How expanded folders were loaded before one walk did it: shallowest
/// first, each found by its own search from the top of the tree.
fn load_by_search(tree: &mut [FileTreeNode], expanded: &HashSet<PathBuf>, order: SortOrder) {
    let mut paths: Vec<&PathBuf> = expanded.iter().collect();
    paths.sort_by_key(|path| path.components().count());
    for path in paths {
        if let Some(children) = FileExplorer::children_slot_mut(tree, path) {
            children.get_or_insert_with(|| FileExplorer::scan_directory_shallow(path, order));
        }
    }
}

/// Every node in display order, with whether it is a loaded folder.
fn shape(nodes: &[FileTreeNode]) -> Vec<(PathBuf, bool)> {
    let mut rows = Vec::new();
    for node in nodes {
        match node {
            FileTreeNode::File { path, .. } => rows.push((path.clone(), false)),
            FileTreeNode::Directory { path, children, .. } => {
                rows.push((path.clone(), children.is_some()));
                rows.extend(shape(children.as_deref().unwrap_or_default()));
            }
        }
    }
    rows
}

#[test]
fn one_walk_loads_the_same_folders_as_a_search_for_each_expanded_path() {
    let dir = ScratchDir::new();
    let nested = dir.dir("open/nested");
    let below_closed = dir.dir("open/nested/closed/deeper");
    let loaded_not_expanded = dir.dir("loaded");
    let below_loaded = dir.dir("loaded/inner");
    let below_unloaded = dir.dir("unloaded/inner");
    for name in [
        "open/b.md",
        "open/a.md",
        "open/nested/c.md",
        "loaded/inner/d.md",
    ] {
        dir.file(name, "text");
    }
    let expanded = HashSet::from([
        dir.path().join("open"),
        nested.clone(),
        below_closed.clone(),
        below_loaded.clone(),
        below_unloaded.clone(),
        PathBuf::from("/md-viewer/not/in/the/tree"),
    ]);
    let order = SortOrder::NameDesc;
    let mut searched = FileExplorer::scan_directory_shallow(dir.path(), order);
    *FileExplorer::children_slot_mut(&mut searched, &loaded_not_expanded).unwrap() = Some(
        FileExplorer::scan_directory_shallow(&loaded_not_expanded, order),
    );
    let mut walked = searched.clone();

    load_by_search(&mut searched, &expanded, order);
    FileExplorer::load_expanded(&mut walked, &expanded, order);

    assert_eq!(shape(&walked), shape(&searched));
    let loaded = |path: &Path| shape(&walked).contains(&(path.to_path_buf(), true));
    assert!(loaded(&nested) && loaded(&below_loaded));
    assert!(!loaded(&below_closed) && !loaded(&below_unloaded));
}

#[test]
fn root_is_scanned_in_place_when_no_thread_can_be_started() {
    let dir = ScratchDir::new();
    let expanded = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");
    let mut explorer = FileExplorer {
        expanded_dirs: HashSet::from([expanded.clone()]),
        ..Default::default()
    };

    explorer.scan_root(dir.path().to_path_buf(), "explorer-scan", |_, _| {
        Err(io::Error::other("out of threads"))
    });

    assert!(explorer.pending_scan.is_none());
    assert_eq!(names(&explorer.tree), ["docs"]);
    assert!(lists(&explorer, &expanded, "guide.md"));
}

#[test]
fn scan_in_place_is_not_overwritten_by_an_older_scan_still_running() {
    let dir = ScratchDir::new();
    dir.file("fresh.md", "# Fresh");
    let (mut explorer, older_scan) = explorer_awaiting_scan(dir.path());

    explorer.scan_root(dir.path().to_path_buf(), "explorer-refresh", |_, _| {
        Err(io::Error::other("out of threads"))
    });
    // The older worker may already be gone by the time it reports.
    let _ = older_scan.send(ExplorerScanResult {
        root: dir.path().to_path_buf(),
        sort_order: SortOrder::NameAsc,
        tree: vec![file_node("stale.md", None)],
    });

    assert!(!explorer.poll_pending_scan());
    assert_eq!(names(&explorer.tree), ["fresh.md"]);
}

#[test]
fn refresh_picks_up_files_added_to_the_root() {
    let dir = ScratchDir::new();
    dir.file("old.md", "# Old");
    let mut explorer = explorer_at(dir.path());

    dir.file("new.md", "# New");
    explorer.refresh();
    wait_for_scan(&mut explorer);

    assert_eq!(names(&explorer.tree), ["new.md", "old.md"]);
}

#[test]
fn explorer_without_a_root_has_nothing_to_refresh() {
    let mut explorer = FileExplorer::default();
    explorer.refresh();
    explorer.refresh_directory(Path::new("/anywhere"));
    assert!(explorer.pending_scan.is_none());
    assert!(explorer.tree.is_empty());
}

#[test]
fn refreshing_the_root_directory_rescans_it_and_keeps_expanded_folders_loaded() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");
    let mut explorer = explorer_at(dir.path());
    explorer.toggle_expanded(&docs);

    dir.file("new.md", "# New");
    explorer.refresh_directory(dir.path());

    assert_eq!(names(&explorer.tree), ["docs", "new.md"]);
    assert!(lists(&explorer, &docs, "guide.md"));
}

#[test]
fn targeted_explorer_refresh_preserves_unrelated_subtrees() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    let notes = dir.dir("notes");
    dir.file("docs/old.md", "# Old");
    dir.file("notes/keep.md", "# Keep");

    let mut explorer = explorer_at(dir.path());
    explorer.toggle_expanded(&docs);
    explorer.toggle_expanded(&notes);
    dir.file("docs/new.md", "# New");

    explorer.refresh_directory(&docs);

    assert!(lists(&explorer, &docs, "new.md"));
    assert!(lists(&explorer, &notes, "keep.md"));
    assert!(explorer.is_expanded(&docs));
    assert!(explorer.is_expanded(&notes));
}

#[test]
fn nested_directory_loads_and_refreshes_below_an_expanded_parent() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    let deep = dir.dir("docs/deep");
    dir.file("docs/deep/old.md", "# Old");

    let mut explorer = explorer_at(dir.path());
    explorer.toggle_expanded(&docs);
    explorer.toggle_expanded(&deep);
    assert!(lists(&explorer, &deep, "old.md"));

    dir.file("docs/deep/new.md", "# New");
    explorer.refresh_directory(&deep);

    assert!(lists(&explorer, &deep, "new.md"));
    assert!(lists(&explorer, &docs, "deep"));
}

#[test]
fn refreshing_a_directory_below_an_unloaded_parent_changes_nothing() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    let deep = dir.dir("docs/deep");
    dir.file("docs/deep/guide.md", "# Guide");

    let mut explorer = explorer_at(dir.path());

    explorer.refresh_directory(&deep);

    assert!(explorer.get_children(&docs).is_none());
    assert!(explorer.get_children(&deep).is_none());
}

#[test]
fn toggling_a_folder_loads_it_once_and_then_only_flips_its_expansion() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");
    let mut explorer = explorer_at(dir.path());
    assert!(!explorer.is_expanded(&docs));
    assert!(explorer.get_children(&docs).is_none(), "loaded lazily");

    explorer.toggle_expanded(&docs);
    assert!(explorer.is_expanded(&docs));
    assert_eq!(child_names(&explorer, &docs), ["guide.md"]);

    // Collapsing keeps what was loaded; nothing on disk is read again.
    dir.file("docs/later.md", "# Later");
    explorer.toggle_expanded(&docs);
    assert!(!explorer.is_expanded(&docs));
    explorer.toggle_expanded(&docs);
    assert_eq!(child_names(&explorer, &docs), ["guide.md"]);
}

/// A folder stays expanded while a folder above it is closed. A rescan then
/// leaves the closed folder unread, so the one below it is not in the tree.
#[test]
fn expanding_a_folder_loads_the_expanded_folders_remembered_below_it() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    let deep = dir.dir("docs/deep");
    dir.file("docs/deep/guide.md", "# Guide");
    let mut explorer = explorer_at(dir.path());
    explorer.toggle_expanded(&docs);
    explorer.toggle_expanded(&deep);
    explorer.toggle_expanded(&docs);
    explorer.refresh();
    wait_for_scan(&mut explorer);
    assert!(explorer.get_children(&docs).is_none(), "closed, so unread");

    explorer.toggle_expanded(&docs);

    assert!(explorer.is_expanded(&deep));
    assert_eq!(child_names(&explorer, &deep), ["guide.md"]);
}

#[test]
fn loading_a_folder_reads_it_once_and_only_if_the_tree_lists_it() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    let deep = dir.dir("docs/deep");
    dir.file("docs/guide.md", "# Guide");
    let mut explorer = explorer_at(dir.path());

    explorer.load_children(&deep);
    assert!(explorer.get_children(&docs).is_none(), "not listed yet");

    explorer.load_children(&docs);
    dir.file("docs/later.md", "# Later");
    explorer.load_children(&docs);
    assert_eq!(child_names(&explorer, &docs), ["deep", "guide.md"]);
    assert!(!explorer.is_expanded(&docs));
}

#[test]
fn expanding_a_folder_that_is_not_in_the_tree_loads_nothing() {
    let dir = ScratchDir::new();
    dir.file("a.md", "# A");
    let mut explorer = explorer_at(dir.path());
    let elsewhere = PathBuf::from("/md-viewer/not/in/the/tree");

    explorer.toggle_expanded(&elsewhere);

    assert!(explorer.is_expanded(&elsewhere));
    assert!(explorer.get_children(&elsewhere).is_none());
    assert_eq!(names(&explorer.tree), ["a.md"]);
}

#[test]
fn expand_all_opens_every_folder_and_collapse_all_closes_them() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    let deep = dir.dir("docs/deep");
    let notes = dir.dir("notes");
    dir.file("docs/deep/guide.md", "# Guide");
    let mut explorer = explorer_at(dir.path());

    explorer.expand_all();

    assert_eq!(
        explorer.expanded_dirs,
        HashSet::from([docs, deep.clone(), notes])
    );
    assert_eq!(child_names(&explorer, &deep), ["guide.md"]);

    explorer.collapse_all();
    assert!(explorer.expanded_dirs.is_empty());
}

#[test]
fn expand_all_stops_at_the_depth_limit_with_every_expanded_folder_loaded() {
    let dir = ScratchDir::new();
    let mut levels = vec![dir.path().to_path_buf()];
    for depth in 0..=FileExplorer::MAX_EXPAND_DEPTH + 1 {
        let parent = levels.last().unwrap();
        levels.push(parent.join(format!("level-{depth}")));
    }
    fs::create_dir_all(levels.last().unwrap()).unwrap();
    let mut explorer = explorer_at(dir.path());

    explorer.expand_all();

    // `levels[n]` sits n levels below the root. The limit reads ten of them
    // and expands the folder the tenth lists, which is then read as well.
    let last_expanded = &levels[FileExplorer::MAX_EXPAND_DEPTH + 1];
    let first_closed = &levels[FileExplorer::MAX_EXPAND_DEPTH + 2];
    assert!(explorer.is_expanded(last_expanded));
    assert!(lists(&explorer, last_expanded, "level-11"));
    assert!(!explorer.is_expanded(first_closed));
    assert!(explorer.get_children(first_closed).is_none());
}
