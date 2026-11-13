//! Edit mode: the source of the document, and saving it.

use super::support::Reader;

const SOURCE: &str = "# Draft\n\nFirst thoughts.";

fn draft() -> Reader {
    Reader::with_docs(&[("draft.md", SOURCE)])
}

/// Put the keyboard at the very end of the source.
fn go_to_the_end(reader: &mut Reader) {
    reader.click_text_field(0);
    reader.press("Ctrl+End");
}

#[test]
fn ctrl_e_shows_the_source_and_goes_back() {
    let mut reader = draft();
    assert!(reader.typed().is_empty());

    reader.press("Ctrl+E");
    assert_eq!(reader.typed(), [SOURCE]);
    assert!(reader.is_on("Edit"));
    assert!(!reader.is_on("Preview"));

    reader.press("Ctrl+E");
    assert!(reader.typed().is_empty());
    reader.assert_sees("First thoughts.");
}

#[test]
fn the_preview_and_edit_switch_does_the_same() {
    let mut reader = draft();
    reader.press("Ctrl+E");

    reader.click("Preview");
    assert!(reader.typed().is_empty());

    reader.click("More");
    reader.click("Edit Document");
    assert_eq!(reader.typed(), [SOURCE]);
    reader.click("More");
    assert!(reader.is_on("Edit Document"));
}

#[test]
fn typing_changes_the_source_and_marks_it_unsaved() {
    let mut reader = draft();
    reader.press("Ctrl+E");
    assert_eq!(reader.count("Unsaved changes"), 0);

    go_to_the_end(&mut reader);
    reader.type_text(" Second thoughts.");

    assert_eq!(reader.typed(), [format!("{SOURCE} Second thoughts.")]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), SOURCE, "nothing is saved yet");
}

#[test]
fn ctrl_s_writes_the_source_to_the_file() {
    let mut reader = draft();
    reader.press("Ctrl+E");
    go_to_the_end(&mut reader);
    reader.type_text(" Second thoughts.");

    reader.press("Ctrl+S");

    assert_eq!(
        reader.read_file("draft.md"),
        format!("{SOURCE} Second thoughts.")
    );
    reader.assert_sees("Saved");
    assert_eq!(reader.count("Unsaved changes"), 0);
}

#[test]
fn the_preview_shows_what_was_typed() {
    let mut reader = draft();
    reader.press("Ctrl+E");
    go_to_the_end(&mut reader);
    reader.type_text("\n\n## Afterword\n\nSecond thoughts.");

    reader.press("Ctrl+E");

    reader.assert_sees("Second thoughts.");
    assert!(reader.in_view("Afterword"));
    reader.assert_hides("## Afterword");
}

/// Known bug, listed in `tests/README.md`: the preview shows the heading
/// that was typed, and the contents sidebar beside it does not. This test
/// states the right behavior and is expected to fail until the app has it.
#[test]
#[should_panic(expected = "the contents sidebar lacks the heading the page shows")]
fn the_contents_follow_what_was_typed() {
    let mut reader = draft();
    reader.press("Ctrl+E");
    go_to_the_end(&mut reader);
    reader.type_text("\n\n## Afterword\n\nSecond thoughts.");

    reader.press("Ctrl+E");

    assert!(reader.in_view("Afterword"));
    assert!(
        reader.count("Afterword") == 2,
        "the contents sidebar lacks the heading the page shows"
    );
}

#[test]
fn unsaved_edits_survive_a_trip_to_another_tab() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE), ("other.md", "# Other")]);
    reader.press("Ctrl+E");
    go_to_the_end(&mut reader);
    reader.type_text(" Kept.");

    reader.drop_file("other.md");
    reader.assert_sees("Other");
    reader.click("draft.md");

    assert_eq!(reader.typed(), [format!("{SOURCE} Kept.")]);
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn a_save_that_cannot_be_written_says_so() {
    let mut reader = Reader::with_docs(&[("notes/draft.md", SOURCE)]);
    reader.press("Ctrl+E");
    go_to_the_end(&mut reader);
    reader.type_text(" Lost.");
    reader.delete_folder("notes");

    reader.press("Ctrl+S");

    reader.assert_sees("Unable to save");
    reader.assert_sees("Save failed");
    assert_eq!(reader.count("Unsaved changes"), 1);
}
