//! Live reload: the page follows the file on disk.

use std::time::Duration;

use super::support::Reader;

/// Longer than a reload has ever taken here, for showing that none comes.
const NO_RELOAD: Duration = Duration::from_millis(1500);

fn notes() -> Reader {
    Reader::with_docs(&[("notes.md", "# Notes\n\nThe first draft.")])
}

#[test]
fn the_page_follows_the_file_when_it_changes() {
    let mut reader = notes();
    reader.assert_sees("The first draft.");

    reader.write_file("notes.md", "# Notes\n\nThe second draft.");
    reader.wait_for("the page to reload", |reader| {
        reader.sees("The second draft.")
    });

    reader.assert_hides("The first draft.");
}

#[test]
fn the_contents_follow_the_file_too() {
    let mut reader = notes();
    assert_eq!(reader.count("Appendix"), 0);

    reader.write_file("notes.md", "# Notes\n\nA draft.\n\n## Appendix\n\nMore.");
    reader.wait_for("the page to reload", |reader| reader.sees("More."));

    assert_eq!(
        reader.count("Appendix"),
        2,
        "a heading and its outline entry"
    );
}

#[test]
fn a_document_in_another_tab_is_reloaded_as_well() {
    let mut reader = Reader::with_docs(&[
        ("notes.md", "# Notes\n\nThe first draft."),
        ("other.md", "# Other\n\nSomething else."),
    ]);
    reader.drop_file("other.md");
    reader.assert_sees("Something else.");

    reader.write_file("notes.md", "# Notes\n\nThe second draft.");
    reader.click("notes.md");
    reader.wait_for("the tab to show the new text", |reader| {
        reader.sees("The second draft.")
    });

    reader.assert_hides("The first draft.");
}

#[test]
fn live_reload_is_on_unless_no_watch_is_given() {
    let mut watching = notes();
    let mut still = Reader::launch()
        .doc("notes.md", "# Notes")
        .args(&["notes.md", "--no-watch"])
        .open();

    watching.click("More");
    still.click("More");

    assert!(watching.is_on("Live Reload"));
    assert!(!still.is_on("Live Reload"));
}

#[test]
fn with_no_watch_the_page_keeps_what_it_read() {
    let mut reader = Reader::launch()
        .doc("notes.md", "# Notes\n\nThe first draft.")
        .args(&["notes.md", "--no-watch"])
        .open();

    reader.write_file("notes.md", "# Notes\n\nThe second draft.");
    reader.idle_for(NO_RELOAD);

    reader.assert_sees("The first draft.");
    reader.assert_hides("The second draft.");
}

#[test]
fn f5_switches_live_reload_off_and_on() {
    let mut reader = notes();

    reader.press("F5");
    reader.click("More");
    assert!(!reader.is_on("Live Reload"));
    reader.press("Escape");
    reader.write_file("notes.md", "# Notes\n\nThe second draft.");
    reader.idle_for(NO_RELOAD);
    reader.assert_sees("The first draft.");

    reader.press("F5");
    reader.click("More");
    assert!(reader.is_on("Live Reload"));
    reader.press("Escape");
    reader.write_file("notes.md", "# Notes\n\nThe third draft.");
    reader.wait_for("the page to reload", |reader| {
        reader.sees("The third draft.")
    });
}

#[test]
fn the_menu_switches_live_reload() {
    let mut reader = notes();

    reader.click("More");
    reader.click("Live Reload");

    reader.click("More");
    assert!(!reader.is_on("Live Reload"));
}
