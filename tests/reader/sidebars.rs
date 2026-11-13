//! The files sidebar on the left and the contents sidebar on the right.

use super::docs::{kitchen_sink, long_document, KITCHEN_SINK};
use super::support::Reader;

/// A folder with two documents at the top and one in a subfolder, opened
/// on `readme.md`.
fn library() -> Reader {
    Reader::with_docs(&[
        ("readme.md", "# Read me\n\nStart here."),
        ("changes.md", "# Changes\n\nWhat is new."),
        ("guides/setup.md", "# Setup\n\nHow to begin."),
    ])
}

#[test]
fn the_contents_sidebar_shows_beside_a_document_with_headings() {
    let reader = kitchen_sink();

    reader.assert_sees("CONTENTS");
    assert_eq!(
        reader.count("Lists"),
        2,
        "the heading and its outline entry"
    );
}

#[test]
fn a_document_without_headings_has_no_contents_sidebar() {
    let reader = Reader::with_docs(&[("flat.md", "Only a paragraph.")]);

    reader.assert_sees("Only a paragraph.");
    reader.assert_hides("CONTENTS");
}

#[test]
fn ctrl_shift_o_and_the_top_bar_button_hide_and_show_the_contents() {
    let mut reader = kitchen_sink();

    reader.press("Ctrl+Shift+O");
    reader.assert_hides("CONTENTS");
    reader.press("Ctrl+Shift+O");
    reader.assert_sees("CONTENTS");

    reader.click("Contents sidebar");
    reader.assert_hides("CONTENTS");
    reader.click("Contents sidebar");
    reader.assert_sees("CONTENTS");
}

#[test]
fn the_menu_shows_and_switches_the_contents_sidebar() {
    let mut reader = kitchen_sink();
    reader.click("More");
    assert!(reader.is_on("Show Outline"));

    reader.click("Show Outline");

    reader.assert_hides("CONTENTS");
    reader.click("More");
    assert!(!reader.is_on("Show Outline"));
}

#[test]
fn hiding_the_contents_gives_the_room_to_the_document() {
    let mut reader = kitchen_sink();
    let beside = reader.column().left;

    reader.press("Ctrl+Shift+O");

    assert!(
        reader.column().left > beside + 50.0,
        "the column did not move"
    );
}

#[test]
fn clicking_a_contents_entry_scrolls_to_its_heading() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(80))]);
    assert!(!reader.in_view("Paragraph 12 mentions the lighthouse once."));

    reader.click("Section 12");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Paragraph 12 mentions the lighthouse once.")
    });

    let heading = reader.text_area("Section 12");
    assert!(
        heading.top < 200.0,
        "the heading is near the top: {heading:?}"
    );
    assert!(!reader.in_view("Paragraph 1 mentions the lighthouse once."));
}

#[test]
fn a_contents_entry_folds_and_unfolds_what_is_under_it() {
    let mut reader = kitchen_sink();

    reader.click("Collapse Kitchen Sink");
    assert_eq!(
        reader.count("Lists"),
        1,
        "only the heading in the page is left\n{}",
        reader.describe()
    );
    assert_eq!(reader.count("Table"), 1);

    reader.click("Expand Kitchen Sink");
    assert_eq!(reader.count("Lists"), 2);
}

#[test]
fn the_contents_make_way_in_a_narrow_window() {
    let mut reader = kitchen_sink();
    reader.assert_sees("CONTENTS");

    reader.resize_window(700.0, 800.0);
    reader.assert_hides("CONTENTS");

    reader.resize_window(1200.0, 800.0);
    reader.assert_sees("CONTENTS");
}

#[test]
fn a_narrow_window_starts_without_the_contents() {
    let reader = Reader::launch()
        .doc("sink.md", KITCHEN_SINK)
        .args(&["sink.md"])
        .window(700.0, 800.0)
        .open();

    reader.assert_sees("Kitchen Sink");
    reader.assert_hides("CONTENTS");
}

#[test]
fn a_reader_who_asked_for_the_contents_keeps_them_in_a_narrow_window() {
    let mut reader = kitchen_sink();
    reader.resize_window(700.0, 800.0);
    reader.assert_hides("CONTENTS");

    reader.press("Ctrl+Shift+O");
    reader.assert_sees("CONTENTS");

    reader.resize_window(690.0, 800.0);
    reader.assert_sees("CONTENTS");
}

#[test]
fn the_files_sidebar_is_hidden_until_asked_for() {
    let mut reader = library();
    reader.assert_hides("FILES");

    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });

    reader.assert_sees("FILES");
    assert_eq!(reader.count("readme.md"), 1);
    assert_eq!(reader.count("guides"), 1);

    reader.click("Files sidebar");
    reader.assert_hides("FILES");
}

#[test]
fn the_menu_shows_and_switches_the_files_sidebar() {
    let mut reader = library();
    reader.click("More");
    assert!(!reader.is_on("Show Explorer"));

    reader.click("Show Explorer");

    reader.assert_sees("FILES");
    reader.click("More");
    assert!(reader.is_on("Show Explorer"));
}

#[test]
fn clicking_a_file_opens_it_in_place() {
    let mut reader = library();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });

    reader.click("changes.md");

    reader.assert_sees("What is new.");
    reader.assert_hides("Start here.");
    assert_eq!(reader.title(), "changes.md - Markdown Viewer");
    assert!(reader.is_enabled("Go back"));
}

#[test]
fn ctrl_clicking_a_file_opens_it_in_a_new_tab() {
    let mut reader = library();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });

    reader.ctrl_click("changes.md");

    reader.assert_sees("What is new.");
    assert_eq!(reader.count("changes.md"), 2, "one in the files, one tab");
    assert_eq!(reader.count("readme.md"), 2);
}

#[test]
fn the_tabs_still_switch_documents_with_the_files_open_beside_them() {
    let mut reader = library();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });
    reader.ctrl_click("changes.md");
    reader.assert_sees("What is new.");

    // In reading order the tab strip comes before the files.
    reader.click_nth("readme.md", 0);
    reader.assert_sees("Start here.");
    reader.assert_hides("What is new.");

    reader.click_nth("changes.md", 0);
    reader.assert_sees("What is new.");
    assert_eq!(reader.count("changes.md"), 2, "one tab, one file entry");
}

#[test]
fn clicking_a_folder_lists_what_is_in_it() {
    let mut reader = library();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("guides") == 1
    });
    assert_eq!(reader.count("setup.md"), 0);

    reader.click("guides");
    reader.wait_for("the subfolder to be listed", |reader| {
        reader.count("setup.md") == 1
    });
    reader.click("setup.md");

    reader.assert_sees("How to begin.");
}

#[test]
fn the_files_sidebar_names_the_folder_it_lists() {
    let mut reader = library();
    let folder = reader
        .folder()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    reader.press("Ctrl+Shift+E");

    reader.assert_sees(&format!("FILES {folder}"));
}

#[test]
fn a_file_made_while_the_sidebar_is_open_appears_in_it() {
    let mut reader = library();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });

    reader.write_file("fresh.md", "# Fresh");
    reader.wait_for("the new file to be listed", |reader| {
        reader.count("fresh.md") == 1
    });

    reader.delete_file("fresh.md");
    reader.wait_for("the deleted file to leave the list", |reader| {
        reader.count("fresh.md") == 0
    });
}
