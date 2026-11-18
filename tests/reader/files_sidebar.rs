//! The files sidebar, in depth: what it lists and in what order, its
//! menus, and how it follows the folder on disk.

use std::time::Duration;

use super::support::{Launch, Reader};

/// Longer than a change on disk has ever taken to show, for asserting
/// that one does not.
const NO_CHANGE: Duration = Duration::from_millis(1200);

/// A folder of documents, other files and nested folders. `readme.md` is
/// the newest file and `guide.markdown` the oldest.
fn shelf() -> Launch {
    Reader::launch()
        .doc("readme.md", "# Read me\n\nStart here.")
        .doc("changes.md", "# Changes\n\nWhat is new.")
        .doc("notes.txt", "# Plain notes\n\nKept as text.")
        .doc("guide.markdown", "# Guide\n\nThe long form.")
        .doc(".secret.md", "# Secret")
        .doc("photo.png", "not really a picture")
        .doc("data.json", "{}")
        .doc(".cache/hidden.md", "# Hidden")
        .doc("guides/setup.md", "# Setup\n\nHow to begin.")
        .doc("guides/advanced.md", "# Advanced\n\nGoing further.")
        .doc("guides/deep/inner.md", "# Inner\n\nAt the bottom.")
        .doc("archive/old.md", "# Old\n\nKept for the record.")
        .folder("empty")
        .aged("guide.markdown", 4000)
        .aged("changes.md", 3000)
        .aged("notes.txt", 2000)
        .aged("readme.md", 1000)
}

/// The shelf, open on `readme.md` with the files sidebar showing.
fn shelf_with_files() -> Reader {
    let mut reader = shelf().args(&["readme.md"]).open();
    show_files(&mut reader);
    reader
}

/// Show the files sidebar and wait for it to list the folder.
fn show_files(reader: &mut Reader) {
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.sees("FILES") && listing(reader).len() > 1
    });
}

/// What the files sidebar lists, top to bottom: everything a screen reader
/// reads between its header and the contents sidebar's.
fn listing(reader: &Reader) -> Vec<String> {
    let names = reader.names().into_iter();
    names
        .skip_while(|name| !name.starts_with("FILES"))
        .skip(1)
        .take_while(|name| name != "CONTENTS")
        .collect()
}

/// The files among what the sidebar lists, without the folders.
fn listed_files(reader: &Reader) -> Vec<String> {
    let mut listed = listing(reader);
    listed.retain(|name| name.contains('.'));
    listed
}

/// Pick `item` from the sidebar's options menu.
fn pick_option(reader: &mut Reader, item: &str) {
    reader.click("Explorer options");
    reader.click(item);
}

const TOP_LEVEL: [&str; 7] = [
    "archive",
    "empty",
    "guides",
    "changes.md",
    "guide.markdown",
    "notes.txt",
    "readme.md",
];

#[test]
fn folders_come_first_and_then_documents_by_name() {
    let reader = shelf_with_files();

    assert_eq!(listing(&reader), TOP_LEVEL);
}

#[test]
fn hidden_files_and_files_that_are_not_documents_are_left_out() {
    let reader = shelf_with_files();

    for absent in [".secret.md", "photo.png", "data.json", ".cache"] {
        assert_eq!(reader.count(absent), 0, "{absent} is listed");
    }
}

#[test]
fn a_folder_opens_under_its_name_and_closes_again() {
    let mut reader = shelf_with_files();

    reader.click("guides");
    assert_eq!(
        listing(&reader),
        [
            "archive",
            "empty",
            "guides",
            "deep",
            "advanced.md",
            "setup.md",
            "changes.md",
            "guide.markdown",
            "notes.txt",
            "readme.md"
        ]
    );

    reader.click("guides");
    assert_eq!(listing(&reader), TOP_LEVEL);
}

#[test]
fn a_document_two_folders_down_can_be_reached_and_opened() {
    let mut reader = shelf_with_files();

    reader.click("guides");
    reader.click("deep");
    reader.click("inner.md");

    reader.assert_sees("At the bottom.");
    assert_eq!(reader.title(), "inner.md - Markdown Viewer");
    assert!(reader.is_enabled("Go back"), "it opened in place");
}

#[test]
fn opening_a_folder_leaves_the_document_alone() {
    let mut reader = shelf_with_files();

    reader.click("archive");

    reader.assert_sees("Start here.");
    assert_eq!(reader.title(), "readme.md - Markdown Viewer");
    assert!(!reader.is_enabled("Go back"));
}

#[test]
fn an_empty_folder_opens_to_nothing() {
    let mut reader = shelf_with_files();

    reader.click("empty");

    assert_eq!(listing(&reader), TOP_LEVEL);
}

#[test]
fn a_folder_stays_open_while_another_is_opened() {
    let mut reader = shelf_with_files();

    reader.click("guides");
    reader.click("archive");

    assert_eq!(reader.count("old.md"), 1);
    assert_eq!(reader.count("setup.md"), 1);
}

#[test]
fn expand_all_opens_every_folder_and_collapse_all_closes_them() {
    let mut reader = shelf_with_files();

    pick_option(&mut reader, "Expand All Folders");
    assert_eq!(
        listing(&reader),
        [
            "archive",
            "old.md",
            "empty",
            "guides",
            "deep",
            "inner.md",
            "advanced.md",
            "setup.md",
            "changes.md",
            "guide.markdown",
            "notes.txt",
            "readme.md"
        ]
    );

    pick_option(&mut reader, "Collapse All Folders");
    assert_eq!(listing(&reader), TOP_LEVEL);
}

#[test]
fn the_options_menu_offers_four_orders_with_name_a_to_z_chosen() {
    let mut reader = shelf_with_files();

    reader.click("Explorer options");

    assert!(reader.is_on("Name A-Z"));
    for other in ["Name Z-A", "Oldest First", "Newest First"] {
        assert!(!reader.is_on(other), "{other}");
    }
    assert_eq!(reader.count("Refresh"), 1);
}

#[test]
fn each_order_sorts_the_documents_its_own_way() {
    let cases = [
        (
            "Name Z-A",
            ["readme.md", "notes.txt", "guide.markdown", "changes.md"],
        ),
        (
            "Oldest First",
            ["guide.markdown", "changes.md", "notes.txt", "readme.md"],
        ),
        (
            "Newest First",
            ["readme.md", "notes.txt", "changes.md", "guide.markdown"],
        ),
        (
            "Name A-Z",
            ["changes.md", "guide.markdown", "notes.txt", "readme.md"],
        ),
    ];
    let mut reader = shelf_with_files();

    for (order, files) in cases {
        pick_option(&mut reader, order);

        assert_eq!(listed_files(&reader), files, "{order}");
        reader.click("Explorer options");
        assert!(reader.is_on(order), "{order} is the chosen one");
        reader.press("Escape");
    }
}

#[test]
fn folders_stay_above_documents_in_every_order() {
    let mut reader = shelf_with_files();

    for order in ["Name Z-A", "Oldest First", "Newest First"] {
        pick_option(&mut reader, order);

        let listed = listing(&reader);
        let first_file = listed.iter().position(|name| name.contains('.'));
        assert_eq!(first_file, Some(3), "{order}: {listed:?}");
    }
}

#[test]
fn the_order_applies_inside_folders_too() {
    let mut reader = shelf_with_files();
    reader.click("guides");

    pick_option(&mut reader, "Name Z-A");

    let listed = listing(&reader);
    let setup = listed.iter().position(|name| name == "setup.md");
    let advanced = listed.iter().position(|name| name == "advanced.md");
    assert!(setup < advanced, "{listed:?}");
}

#[test]
fn the_document_in_front_stands_out_in_the_list() {
    let mut reader = shelf_with_files();
    let quiet = reader.ink_of_each("changes.md");
    assert_ne!(reader.ink_of_each("readme.md"), quiet);
    assert_eq!(reader.ink_of_each("notes.txt"), quiet);

    reader.click("changes.md");

    assert_eq!(reader.ink_of_each("readme.md"), quiet);
    assert_ne!(reader.ink_of_each("changes.md"), quiet);
}

#[test]
fn the_mark_follows_the_tab_in_front() {
    let mut reader = shelf_with_files();
    reader.ctrl_click("changes.md");
    let quiet = reader.ink_of_each("notes.txt");
    let row_ink = |reader: &Reader, name: &str| reader.ink_of_each(name).last().copied();
    assert_ne!(row_ink(&reader, "changes.md"), quiet.last().copied());

    reader.press("Ctrl+Tab");

    assert_eq!(row_ink(&reader, "changes.md"), quiet.last().copied());
    assert_ne!(row_ink(&reader, "readme.md"), quiet.last().copied());
}

#[test]
fn a_documents_menu_copies_its_text_its_path_or_its_address() {
    let mut reader = shelf_with_files();
    let path = reader.path("changes.md").display().to_string();
    let cases = [
        ("Copy Contents", "# Changes\n\nWhat is new.".to_owned()),
        ("Copy Path", path.clone()),
        ("Copy File URI", format!("file://{path}")),
    ];

    for (item, copied) in cases {
        reader.right_click("changes.md");
        reader.click(item);

        assert_eq!(reader.clipboard(), Some(copied.as_str()), "{item}");
        assert_eq!(reader.count(item), 0, "the menu closed");
    }
    reader.assert_sees("Start here.");
}

#[test]
fn a_documents_menu_is_there_while_the_document_has_a_tab() {
    let mut reader = shelf_with_files();
    reader.ctrl_click("changes.md");

    // In reading order the tab comes before the row in the files.
    reader.right_click_nth("changes.md", 1);
    reader.click("Copy Path");

    let path = reader.path("changes.md").display().to_string();
    assert_eq!(reader.clipboard(), Some(path.as_str()));
    assert_eq!(reader.count("changes.md"), 2, "the tab is still open");
}

#[test]
fn a_folder_has_no_copy_menu() {
    let mut reader = shelf_with_files();

    reader.right_click("guides");

    assert_eq!(reader.count("Copy Path"), 0);
}

#[test]
fn middle_clicking_an_open_document_in_the_list_closes_its_tab() {
    let mut reader = shelf_with_files();
    reader.ctrl_click("changes.md");
    assert_eq!(reader.count("changes.md"), 2, "a tab and a row");

    reader.middle_click_nth("changes.md", 1);

    assert_eq!(reader.count("changes.md"), 1, "the row is left");
    reader.assert_sees("Start here.");
    assert_eq!(reader.title(), "readme.md - Markdown Viewer");
}

#[test]
fn middle_clicking_a_document_that_is_not_open_does_nothing() {
    let mut reader = shelf_with_files();

    reader.middle_click("changes.md");

    reader.assert_sees("Start here.");
    assert_eq!(reader.count("changes.md"), 1);
    assert_eq!(reader.title(), "readme.md - Markdown Viewer");
}

#[test]
fn a_text_file_and_a_dot_markdown_file_open_from_the_list() {
    let mut reader = shelf_with_files();

    reader.click("notes.txt");
    reader.assert_sees("Kept as text.");
    assert_eq!(reader.title(), "notes.txt - Markdown Viewer");

    reader.click("guide.markdown");
    reader.assert_sees("The long form.");
    assert_eq!(reader.title(), "guide.markdown - Markdown Viewer");
}

#[test]
fn the_list_is_of_the_folder_the_opened_document_is_in() {
    let mut reader = shelf().args(&["guides/setup.md"]).open();

    show_files(&mut reader);

    reader.assert_sees("FILES guides");
    assert_eq!(listing(&reader), ["deep", "advanced.md", "setup.md"]);
}

#[test]
fn a_document_made_in_an_open_subfolder_appears_there() {
    let mut reader = shelf_with_files();
    reader.click("guides");

    reader.write_file("guides/fresh.md", "# Fresh");
    reader.wait_for("the new document to be listed", |reader| {
        reader.count("fresh.md") == 1
    });

    let listed = listing(&reader);
    let at = |name: &str| listed.iter().position(|listed| listed == name);
    assert!(at("advanced.md") < at("fresh.md") && at("fresh.md") < at("setup.md"));
}

#[test]
fn a_new_folder_appears_and_a_removed_one_leaves() {
    let mut reader = shelf_with_files();

    reader.make_folder("drafts");
    reader.wait_for("the new folder to be listed", |reader| {
        reader.count("drafts") == 1
    });

    reader.delete_folder("archive");
    reader.wait_for("the removed folder to leave", |reader| {
        reader.count("archive") == 0
    });
    assert_eq!(listing(&reader)[..3], ["drafts", "empty", "guides"]);
}

#[test]
fn a_renamed_document_is_listed_under_its_new_name() {
    let mut reader = shelf_with_files();

    reader.rename_file("changes.md", "history.md");
    reader.wait_for("the new name to be listed", |reader| {
        reader.count("history.md") == 1 && reader.count("changes.md") == 0
    });

    assert_eq!(
        listed_files(&reader),
        ["guide.markdown", "history.md", "notes.txt", "readme.md"]
    );
}

#[test]
fn a_new_file_that_is_not_a_document_does_not_appear() {
    let mut reader = shelf_with_files();

    reader.write_file("sketch.png", "not a picture either");
    reader.write_file("zz-last.md", "# Last");
    reader.wait_for("the new document to be listed", |reader| {
        reader.count("zz-last.md") == 1
    });

    assert_eq!(reader.count("sketch.png"), 0);
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_read_says_so_and_leaves_the_open_document() {
    let mut reader = shelf()
        .link("lost.md", "gone.md")
        .args(&["readme.md"])
        .open();
    show_files(&mut reader);

    reader.click("lost.md");

    reader.assert_sees("Unable to open");
    reader.assert_sees("lost.md");
    reader.assert_sees("Start here.");
    assert_eq!(reader.title(), "readme.md - Markdown Viewer");
}

#[test]
fn without_live_reload_the_list_waits_for_refresh() {
    let mut reader = shelf().args(&["readme.md", "--no-watch"]).open();
    show_files(&mut reader);

    reader.write_file("fresh.md", "# Fresh");
    reader.idle_for(NO_CHANGE);
    assert_eq!(reader.count("fresh.md"), 0);

    pick_option(&mut reader, "Refresh");
    reader.wait_for("the new document to be listed", |reader| {
        reader.count("fresh.md") == 1
    });
}

#[test]
fn hiding_and_showing_the_sidebar_keeps_folders_open() {
    let mut reader = shelf_with_files();
    reader.click("guides");

    reader.press("Ctrl+Shift+E");
    reader.assert_hides("FILES");
    reader.press("Ctrl+Shift+E");

    assert_eq!(reader.count("setup.md"), 1);
}

#[test]
fn the_sidebar_gives_way_to_the_document_when_hidden() {
    let mut reader = shelf_with_files();
    let beside = reader.column().left;

    reader.click("Files sidebar");

    assert!(reader.column().left < beside - 50.0, "the column moved");
}
