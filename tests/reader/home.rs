//! The home page: what shows with no document open.

use super::support::Reader;

/// Read `count` documents titled `Story 1` and up, one after the other,
/// and close them all.
fn after_reading(count: usize) -> Reader {
    let mut launch = Reader::launch();
    for number in 1..=count {
        let text = format!("# Story {number}\n\nOnce upon a time, number {number}.");
        launch = launch.doc(&format!("story-{number}.md"), &text);
    }
    let mut reader = launch.open();
    for number in 1..=count {
        reader.drop_file(&format!("story-{number}.md"));
    }
    for _ in 0..count {
        reader.press("Ctrl+W");
    }
    reader
}

/// The stories the home page lists, top to bottom.
fn listed(reader: &Reader) -> Vec<String> {
    let names = reader.names().into_iter();
    names.filter(|name| name.starts_with("Story ")).collect()
}

#[test]
fn a_first_run_explains_itself() {
    let reader = Reader::first_run();

    reader.assert_sees("md-viewer");
    reader.assert_sees("Nothing read yet.");
    reader.assert_sees("Ctrl+O opens a file.");
    assert_eq!(reader.count("Open File…"), 1);
    assert_eq!(reader.count("Open Folder…"), 1);
    reader.assert_hides("RECENTLY READ");
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn a_document_that_was_read_is_listed_by_its_title() {
    let reader = after_reading(1);

    reader.assert_sees("RECENTLY READ");
    reader.assert_hides("Nothing read yet.");
    assert_eq!(listed(&reader), ["Story 1"]);
}

#[test]
fn a_row_shows_the_opening_lines_and_the_reading_time() {
    let reader = after_reading(1);

    reader.assert_sees("Once upon a time, number 1.");
    reader.assert_sees("1 min read");
}

#[test]
fn clicking_a_row_opens_the_document() {
    let mut reader = after_reading(2);

    reader.click("Story 1");

    reader.assert_sees("Once upon a time, number 1.");
    reader.assert_hides("RECENTLY READ");
    assert_eq!(reader.title(), "story-1.md - Markdown Viewer");
}

#[test]
fn the_latest_read_is_listed_first() {
    let mut reader = after_reading(3);
    assert_eq!(listed(&reader), ["Story 3", "Story 2", "Story 1"]);

    reader.click("Story 1");
    reader.press("Ctrl+W");

    assert_eq!(listed(&reader), ["Story 1", "Story 3", "Story 2"]);
}

#[test]
fn six_are_listed_until_show_more_is_asked_for() {
    let mut reader = after_reading(8);
    assert_eq!(listed(&reader).len(), 6);
    assert_eq!(listed(&reader)[0], "Story 8");

    reader.scroll_down(600.0);
    reader.click("Show more…");
    assert_eq!(listed(&reader).len(), 8);

    reader.scroll_down(600.0);
    reader.click("Show less");
    assert_eq!(listed(&reader).len(), 6);
}

#[test]
fn a_short_list_has_no_show_more() {
    let reader = after_reading(6);

    assert_eq!(listed(&reader).len(), 6);
    assert_eq!(reader.count("Show more…"), 0);
}

#[test]
fn what_was_read_is_remembered() {
    let reader = after_reading(2);

    let reader = reader.restart(&[]);

    reader.assert_sees("RECENTLY READ");
    assert_eq!(listed(&reader), ["Story 2", "Story 1"]);
}

#[test]
fn a_document_named_on_the_command_line_counts_as_read() {
    let mut reader = Reader::with_docs(&[("tale.md", "# Story of the sea\n\nSalt and wind.")]);

    reader.press("Ctrl+W");

    assert_eq!(listed(&reader), ["Story of the sea"]);
}
