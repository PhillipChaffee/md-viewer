//! Opening documents: from the command line and by dropping them on the
//! window.

use super::docs::long_document;
use super::support::Reader;

/// The minutes in the `name · N min read` line above the title.
fn reading_minutes(reader: &Reader) -> u32 {
    let names = reader.names();
    let line = names.iter().find(|name| name.ends_with(" min read"));
    let minutes = line
        .and_then(|line| line.strip_suffix(" min read"))
        .and_then(|line| line.rsplit(' ').next());
    minutes.unwrap().parse().unwrap()
}

#[test]
fn the_file_named_on_the_command_line_is_shown() {
    let reader = Reader::with_docs(&[("notes.md", "# Trip notes\n\nPack the tent.")]);

    reader.assert_sees("Trip notes");
    reader.assert_sees("Pack the tent.");
    assert_eq!(reader.title(), "notes.md - Markdown Viewer");
}

#[test]
fn a_relative_path_is_read_from_the_working_directory() {
    let reader = Reader::launch()
        .raw_args(&["tests/fixtures/plain.md", "--no-watch"])
        .open();

    reader.assert_sees("A plain fixture");
    reader.assert_sees("read in place from the repository");
    assert_eq!(reader.title(), "plain.md - Markdown Viewer");
}

#[test]
fn a_file_in_a_subfolder_is_shown() {
    let reader = Reader::launch()
        .doc("notes/deep/inner.md", "# Inner notes")
        .args(&["notes/deep/inner.md"])
        .open();

    reader.assert_sees("Inner notes");
    assert_eq!(reader.title(), "inner.md - Markdown Viewer");
}

#[test]
fn a_missing_file_opens_the_home_page_and_says_why() {
    let mut reader = Reader::launch().args(&["absent.md"]).open();

    reader.assert_sees("Unable to open");
    reader.assert_sees("absent.md");
    reader.assert_sees("md-viewer");
    assert_eq!(reader.title(), "Markdown Viewer");

    reader.click("Dismiss");
    reader.assert_hides("Unable to open");
    reader.assert_sees("md-viewer");
}

/// Known bug, listed in `tests/README.md`: the file is put on the recently
/// read list although it was never opened. This test states the right
/// behavior and is expected to fail until the app has it.
#[test]
#[should_panic(expected = "a file that was never read is listed as read")]
fn a_file_that_could_not_be_opened_is_not_listed_as_read() {
    let reader = Reader::launch().args(&["absent.md"]).open();

    assert!(
        !reader.sees("RECENTLY READ"),
        "a file that was never read is listed as read"
    );
}

#[test]
fn a_file_that_is_not_utf8_opens_with_its_bad_bytes_marked() {
    let reader = Reader::launch()
        .bytes("latin.md", b"# Caf\xe9 menu\n\nSoup of the day.")
        .args(&["latin.md"])
        .open();

    reader.assert_hides("Unable to open");
    reader.assert_sees("Caf\u{fffd} menu");
    reader.assert_sees("Soup of the day.");
    assert_eq!(reader.title(), "latin.md - Markdown Viewer");
}

#[test]
fn an_empty_file_opens_as_an_empty_page() {
    let reader = Reader::with_docs(&[("blank.md", "")]);

    reader.assert_hides("Unable to open");
    reader.assert_sees("blank.md · 1 min read");
    assert_eq!(reader.title(), "blank.md - Markdown Viewer");
}

#[test]
fn an_unknown_flag_is_refused() {
    let refusal = Reader::launch().args(&["--bogus"]).try_open().err();

    assert!(
        refusal
            .as_deref()
            .is_some_and(|said| said.contains("--bogus")),
        "{refusal:?}"
    );
}

#[test]
fn two_files_on_the_command_line_are_refused() {
    let launch = Reader::launch().doc("a.md", "# A").doc("b.md", "# B");

    let refusal = launch.args(&["a.md", "b.md"]).try_open().err();

    assert!(refusal.is_some(), "md-viewer takes one file");
}

#[test]
fn the_line_above_the_title_names_the_file_and_its_reading_time() {
    let short = Reader::with_docs(&[("short.md", "# Short\n\nA few words.")]);
    let long = Reader::with_docs(&[("long.md", &long_document(400))]);

    short.assert_sees("short.md · 1 min read");
    assert_eq!(reading_minutes(&short), 1);
    assert!(
        reading_minutes(&long) > 5,
        "2,800 words read in {} min",
        reading_minutes(&long)
    );
}

#[test]
fn dropping_a_markdown_file_opens_it_in_a_new_tab() {
    let mut reader = Reader::with_docs(&[("a.md", "# Alpha"), ("b.md", "# Beta")]);

    reader.drop_file("b.md");

    reader.assert_sees("Beta");
    assert!(
        reader.is_on("b.md"),
        "the dropped file's tab is the active one"
    );
    assert!(!reader.is_on("a.md"));
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn dropping_a_file_onto_the_home_page_opens_it() {
    let mut reader = Reader::launch().doc("a.md", "# Alpha").open();
    reader.assert_sees("Nothing read yet.");

    reader.drop_file("a.md");

    reader.assert_sees("Alpha");
    reader.assert_hides("Nothing read yet.");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn dropping_a_file_that_is_not_markdown_is_refused() {
    let mut reader = Reader::launch()
        .doc("a.md", "# Alpha")
        .bytes("photo.png", &[0x89, 0x50, 0x4e, 0x47])
        .args(&["a.md"])
        .open();

    reader.drop_file("photo.png");

    reader.assert_sees("Unsupported file type");
    reader.assert_sees("Alpha");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn dropping_a_file_that_has_vanished_says_so() {
    let mut reader = Reader::with_docs(&[("a.md", "# Alpha")]);
    let gone = reader.path("gone.md");

    reader.drop_path(gone);

    reader.assert_sees("Unable to open");
    reader.assert_sees("gone.md");
    reader.assert_sees("Alpha");
}

#[test]
fn a_file_held_over_the_window_invites_the_drop() {
    let mut reader = Reader::with_docs(&[("a.md", "# Alpha"), ("b.md", "# Beta")]);
    reader.assert_hides("Drop markdown file here");

    reader.drag_files_over(&["b.md"]);

    reader.assert_sees("Drop markdown file here");
}
