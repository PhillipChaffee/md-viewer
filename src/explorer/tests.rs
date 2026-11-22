use super::*;

use std::time::{Duration, SystemTime};

use crate::test_support::{wait_until, ScratchDir};

/// `add_root` and `refresh` scan on a worker thread since #84: wait for the
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
    explorer.add_root(root.to_path_buf());
    wait_for_scan(&mut explorer);
    explorer
}

/// What the first root lists, as far as it has been read.
fn listing(explorer: &FileExplorer) -> &[FileTreeNode] {
    match explorer.tree.first() {
        Some(FileTreeNode::Directory {
            children: Some(listed),
            ..
        }) => listed,
        _ => &[],
    }
}

/// A root's node, read as `listed`.
fn root_listing(path: &Path, listed: Vec<FileTreeNode>) -> FileTreeNode {
    FileTreeNode::Directory {
        path: path.to_path_buf(),
        name: folder_name(path).into_owned(),
        modified: None,
        children: Some(listed),
    }
}

fn names(nodes: &[FileTreeNode]) -> Vec<&str> {
    nodes.iter().map(FileTreeNode::name).collect()
}

/// What the tree has read of `directory`: `None` when it does not list the
/// directory, or lists it unread.
fn children_of<'tree>(
    nodes: &'tree [FileTreeNode],
    directory: &Path,
) -> Option<&'tree [FileTreeNode]> {
    nodes.iter().find_map(|node| match node {
        FileTreeNode::Directory { path, children, .. } if path == directory => children.as_deref(),
        FileTreeNode::Directory {
            children: Some(loaded),
            ..
        } => children_of(loaded, directory),
        _ => None,
    })
}

/// Names of the loaded children of `directory`, in display order.
fn child_names<'tree>(explorer: &'tree FileExplorer, directory: &Path) -> Vec<&'tree str> {
    names(children_of(&explorer.tree, directory).expect("directory is loaded"))
}

/// Whether the loaded children of `directory` include an entry called `name`.
fn lists(explorer: &FileExplorer, directory: &Path, name: &str) -> bool {
    children_of(&explorer.tree, directory)
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
        names(listing(&explorer)),
        ["zeta", "a.markdown", "b.md", "notes.txt"]
    );
}

#[test]
fn root_that_cannot_be_read_shows_an_empty_tree() {
    let dir = ScratchDir::new();
    let explorer = explorer_at(&dir.path().join("gone"));
    assert!(listing(&explorer).is_empty());
}

#[test]
fn empty_root_path_means_the_working_directory() {
    let mut explorer = FileExplorer::default();
    explorer.add_root(PathBuf::new());
    assert_eq!(explorer.roots(), [std::env::current_dir().unwrap()]);
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

    assert_eq!(names(listing(&explorer)), ["docs", "y.md", "x.md"]);
    assert_eq!(child_names(&explorer, &docs), ["b.md", "a.md"]);

    explorer.set_sort_order(SortOrder::NameDesc);
    assert_eq!(names(listing(&explorer)), ["docs", "y.md", "x.md"]);
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
    assert_eq!(
        names(listing(&explorer)),
        ["b-old.md", "c-mid.md", "a-new.md"]
    );

    explorer.set_sort_order(SortOrder::DateDesc);
    assert_eq!(
        names(listing(&explorer)),
        ["a-new.md", "c-mid.md", "b-old.md"]
    );
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
fn entries_modified_at_the_same_time_keep_the_order_they_were_listed_in() {
    for order in [SortOrder::DateAsc, SortOrder::DateDesc] {
        let same_time = vec![file_node("b.md", Some(5)), file_node("a.md", Some(5))];
        assert_eq!(sorted(same_time, order), ["b.md", "a.md"]);
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
    let listed = FileExplorer::scan_directory_shallow(dir.path(), SortOrder::NameAsc);
    tx.send(vec![root_listing(dir.path(), listed)]).unwrap();
    let mut explorer = FileExplorer {
        roots: vec![dir.path().to_path_buf()],
        expanded_dirs: HashSet::from([expanded.clone()]),
        pending_scan: Some(rx),
        ..Default::default()
    };

    assert!(explorer.poll_pending_scan());
    assert!(lists(&explorer, &expanded, "guide.md"));
}

/// An explorer on `root` holding the sending end of its pending scan.
fn explorer_awaiting_scan(root: &Path) -> (FileExplorer, mpsc::Sender<Vec<FileTreeNode>>) {
    let (tx, rx) = mpsc::channel();
    let explorer = FileExplorer {
        roots: vec![root.to_path_buf()],
        expanded_dirs: HashSet::from([root.to_path_buf()]),
        pending_scan: Some(rx),
        ..Default::default()
    };
    (explorer, tx)
}

#[test]
fn a_scan_started_for_a_new_root_takes_the_place_of_the_one_before() {
    let previous = ScratchDir::new();
    previous.file("kept.md", "# Kept");
    let current = ScratchDir::new();
    current.file("fresh.md", "# Fresh");
    let (mut explorer, older_scan) = explorer_awaiting_scan(previous.path());

    explorer.add_root(current.path().to_path_buf());
    let stale = root_listing(previous.path(), vec![file_node("stale.md", None)]);
    assert!(
        older_scan.send(vec![stale]).is_err(),
        "nothing waits for the older scan any more"
    );
    wait_for_scan(&mut explorer);

    assert_eq!(names(listing(&explorer)), ["kept.md"]);
    assert_eq!(child_names(&explorer, current.path()), ["fresh.md"]);
}

#[test]
fn scan_result_is_resorted_when_the_order_changed_while_it_ran() {
    let (mut explorer, tx) = explorer_awaiting_scan(Path::new("/current"));
    explorer.sort_order = SortOrder::NameDesc;
    let listed = vec![file_node("a.md", None), file_node("b.md", None)];
    tx.send(vec![root_listing(Path::new("/current"), listed)])
        .unwrap();

    assert!(explorer.poll_pending_scan());
    assert_eq!(names(listing(&explorer)), ["b.md", "a.md"]);
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
fn a_restored_explorer_lists_its_root_in_its_order_with_its_folders_open() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");
    dir.file("a.md", "# A");
    dir.file("b.md", "# B");

    let open = HashSet::from([dir.path().to_path_buf(), docs.clone()]);
    let mut explorer = FileExplorer::restored(
        SortOrder::NameDesc,
        open.clone(),
        vec![dir.path().to_path_buf()],
    );
    assert!(explorer.is_scanning());
    wait_for_scan(&mut explorer);

    assert!(!explorer.is_scanning());
    assert_eq!(explorer.roots(), [dir.path()]);
    assert!(explorer.sort_order() == SortOrder::NameDesc);
    assert_eq!(explorer.expanded_dirs(), &open);
    assert_eq!(names(listing(&explorer)), ["docs", "b.md", "a.md"]);
    assert!(lists(&explorer, &docs, "guide.md"));
}

#[test]
fn an_explorer_restored_without_a_root_scans_nothing() {
    let explorer = FileExplorer::restored(SortOrder::NameAsc, HashSet::new(), Vec::new());

    assert!(!explorer.is_scanning());
    assert!(explorer.roots().is_empty());
    assert!(explorer.tree().is_empty());
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
    explorer.add_root(dir.path().to_path_buf());

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
    explorer.add_root(dir.path().to_path_buf());
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
        roots: vec![dir.path().to_path_buf()],
        expanded_dirs: HashSet::from([dir.path().to_path_buf(), expanded.clone()]),
        ..Default::default()
    };

    explorer.scan_roots("explorer-scan", |_, _| {
        Err(io::Error::other("out of threads"))
    });

    assert!(explorer.pending_scan.is_none());
    assert_eq!(names(listing(&explorer)), ["docs"]);
    assert!(lists(&explorer, &expanded, "guide.md"));
}

#[test]
fn scan_in_place_is_not_overwritten_by_an_older_scan_still_running() {
    let dir = ScratchDir::new();
    dir.file("fresh.md", "# Fresh");
    let (mut explorer, older_scan) = explorer_awaiting_scan(dir.path());

    explorer.scan_roots("explorer-refresh", |_, _| {
        Err(io::Error::other("out of threads"))
    });
    // The older worker may already be gone by the time it reports.
    let stale = root_listing(dir.path(), vec![file_node("stale.md", None)]);
    let _ = older_scan.send(vec![stale]);

    assert!(!explorer.poll_pending_scan());
    assert_eq!(names(listing(&explorer)), ["fresh.md"]);
}

#[test]
fn refresh_picks_up_files_added_to_the_root() {
    let dir = ScratchDir::new();
    dir.file("old.md", "# Old");
    let mut explorer = explorer_at(dir.path());

    dir.file("new.md", "# New");
    explorer.refresh();
    wait_for_scan(&mut explorer);

    assert_eq!(names(listing(&explorer)), ["new.md", "old.md"]);
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

    assert_eq!(names(listing(&explorer)), ["docs", "new.md"]);
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

    assert!(children_of(&explorer.tree, &docs).is_none());
    assert!(children_of(&explorer.tree, &deep).is_none());
}

#[test]
fn toggling_a_folder_loads_it_once_and_then_only_flips_its_expansion() {
    let dir = ScratchDir::new();
    let docs = dir.dir("docs");
    dir.file("docs/guide.md", "# Guide");
    let mut explorer = explorer_at(dir.path());
    assert!(!explorer.is_expanded(&docs));
    assert!(
        children_of(&explorer.tree, &docs).is_none(),
        "loaded lazily"
    );

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
    assert!(
        children_of(&explorer.tree, &docs).is_none(),
        "closed, so unread"
    );

    explorer.toggle_expanded(&docs);

    assert!(explorer.is_expanded(&deep));
    assert_eq!(child_names(&explorer, &deep), ["guide.md"]);
}

#[test]
fn expanding_a_folder_that_is_not_in_the_tree_loads_nothing() {
    let dir = ScratchDir::new();
    dir.file("a.md", "# A");
    let mut explorer = explorer_at(dir.path());
    let elsewhere = PathBuf::from("/md-viewer/not/in/the/tree");

    explorer.toggle_expanded(&elsewhere);

    assert!(explorer.is_expanded(&elsewhere));
    assert!(children_of(&explorer.tree, &elsewhere).is_none());
    assert_eq!(names(listing(&explorer)), ["a.md"]);
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
        HashSet::from([dir.path().to_path_buf(), docs, deep.clone(), notes])
    );
    assert_eq!(child_names(&explorer, &deep), ["guide.md"]);

    explorer.collapse_all();
    assert_eq!(
        explorer.expanded_dirs,
        HashSet::from([dir.path().to_path_buf()])
    );
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
    assert!(children_of(&explorer.tree, first_closed).is_none());
}

fn root_path(node: &FileTreeNode) -> &Path {
    match node {
        FileTreeNode::File { path, .. } | FileTreeNode::Directory { path, .. } => path,
    }
}

#[test]
fn several_folders_are_listed_in_the_order_they_were_added_each_open() {
    let (zeta, alpha) = (ScratchDir::new(), ScratchDir::new());
    zeta.file("z.md", "# Z");
    alpha.file("a.md", "# A");
    let mut explorer = explorer_at(zeta.path());

    explorer.add_root(alpha.path().to_path_buf());
    wait_for_scan(&mut explorer);

    assert_eq!(explorer.roots(), [zeta.path(), alpha.path()]);
    let roots: Vec<&Path> = explorer.tree.iter().map(root_path).collect();
    assert_eq!(roots, [zeta.path(), alpha.path()]);
    assert!(explorer.is_expanded(&alpha.path().to_path_buf()));
    assert_eq!(child_names(&explorer, zeta.path()), ["z.md"]);
    assert_eq!(child_names(&explorer, alpha.path()), ["a.md"]);

    // Sorting orders what each folder lists, never the folders.
    explorer.set_sort_order(SortOrder::NameDesc);
    let roots: Vec<&Path> = explorer.tree.iter().map(root_path).collect();
    assert_eq!(roots, [zeta.path(), alpha.path()]);
}

#[test]
fn adding_a_folder_already_listed_only_opens_it() {
    let dir = ScratchDir::new();
    dir.file("a.md", "# A");
    let mut explorer = explorer_at(dir.path());
    let root = dir.path().to_path_buf();
    explorer.toggle_expanded(&root);
    assert!(!explorer.is_expanded(&root));

    explorer.add_root(root.clone());

    assert_eq!(explorer.roots(), [dir.path()]);
    assert_eq!(explorer.tree.len(), 1);
    assert!(explorer.is_expanded(&root));
    assert!(!explorer.is_scanning(), "nothing new to read");
}

#[test]
fn a_folder_closes_and_opens_like_any_folder_and_keeps_what_it_read() {
    let dir = ScratchDir::new();
    dir.file("a.md", "# A");
    let mut explorer = explorer_at(dir.path());
    let root = dir.path().to_path_buf();

    explorer.toggle_expanded(&root);
    assert!(!explorer.is_expanded(&root));
    assert_eq!(explorer.roots(), [dir.path()], "still listed");

    explorer.toggle_expanded(&root);
    assert!(explorer.is_expanded(&root));
    assert_eq!(names(listing(&explorer)), ["a.md"]);
}

#[test]
fn a_folder_left_closed_is_not_read_until_it_is_opened() {
    let dir = ScratchDir::new();
    dir.file("a.md", "# A");
    let root = dir.path().to_path_buf();
    let mut explorer =
        FileExplorer::restored(SortOrder::NameAsc, HashSet::new(), vec![root.clone()]);
    wait_for_scan(&mut explorer);
    assert!(
        children_of(&explorer.tree, &root).is_none(),
        "closed, so unread"
    );

    explorer.toggle_expanded(&root);

    assert_eq!(child_names(&explorer, &root), ["a.md"]);
}

#[test]
fn removing_a_folder_forgets_the_folders_open_in_it_that_no_other_holds() {
    let (first, second) = (ScratchDir::new(), ScratchDir::new());
    let first_docs = first.dir("docs");
    let shared = second.dir("shared");
    let inner = second.dir("shared/inner");
    let elsewhere = second.dir("elsewhere");
    let mut explorer = explorer_at(first.path());
    explorer.add_root(second.path().to_path_buf());
    explorer.add_root(shared.clone());
    wait_for_scan(&mut explorer);
    for folder in [&first_docs, &inner, &elsewhere] {
        explorer.toggle_expanded(folder);
    }

    explorer.remove_root(second.path());

    assert_eq!(explorer.roots(), [first.path(), shared.as_path()]);
    assert!(explorer
        .tree
        .iter()
        .all(|root| root_path(root) != second.path()));
    assert!(!explorer.is_expanded(&second.path().to_path_buf()));
    assert!(!explorer.is_expanded(&elsewhere));
    assert!(explorer.is_expanded(&inner), "still inside a listed folder");
    assert!(explorer.is_expanded(&first_docs));
}

#[test]
fn a_path_is_held_by_the_first_listed_folder_it_is_in() {
    let explorer = FileExplorer {
        roots: vec![PathBuf::from("/notes"), PathBuf::from("/notes/inner")],
        ..FileExplorer::default()
    };

    assert_eq!(
        explorer.root_holding(Path::new("/notes/inner/a.md")),
        Some(Path::new("/notes"))
    );
    assert_eq!(explorer.root_holding(Path::new("/elsewhere/a.md")), None);
}
