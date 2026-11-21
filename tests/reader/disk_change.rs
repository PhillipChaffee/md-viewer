//! Another program changes a file while its document has unsaved edits:
//! the notice on that document and its two answers.

use std::time::Duration;

use super::support::Reader;
use super::unsaved::edit;

pub(super) const SOURCE: &str = "# Draft\n\nFirst thoughts.";
pub(super) const MINE: &str = "# Draft\n\nFirst thoughts. Mine.";
pub(super) const THEIRS: &str = "# Draft\n\nWritten by another program.";
pub(super) const NOTICE: &str = "This file changed on disk. Your edits are not saved.";
pub(super) const GONE: &str = "This file was deleted on disk. Saving writes it again.";
/// What a tab is marked with while its notice waits behind another tab.
const TAB_MARK: &str = "Changed on disk";

/// Longer than a change has ever taken to arrive here, for showing that
/// none is announced.
pub(super) const NO_NOTICE: Duration = Duration::from_millis(1500);

/// `draft.md` open with an edit that is not saved.
pub(super) fn unsaved_draft() -> Reader {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE)]);
    edit(&mut reader, " Mine.");
    reader
}

/// Have another program write `text` to `draft.md`, and wait for the
/// notice of it.
#[track_caller]
pub(super) fn another_program_writes(reader: &mut Reader, text: &str) {
    reader.write_file("draft.md", text);
    reader.wait_for("the notice", |reader| reader.sees(NOTICE));
}

/// `draft.md` with an unsaved edit, changed by another program, and the
/// notice of that showing.
pub(super) fn changed_under_edits() -> Reader {
    let mut reader = unsaved_draft();
    another_program_writes(&mut reader, THEIRS);
    reader
}

#[test]
fn a_change_under_unsaved_edits_is_announced_and_nothing_is_touched() {
    let reader = changed_under_edits();

    for answer in ["Reload", "Keep mine"] {
        assert_eq!(reader.count(answer), 1, "{answer}");
    }
    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn the_notice_shows_over_the_preview_of_the_edits_too() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+E");

    another_program_writes(&mut reader, THEIRS);

    reader.assert_sees("First thoughts. Mine.");
}

#[test]
fn reload_takes_the_file_and_drops_the_edits() {
    let mut reader = changed_under_edits();

    reader.click("Reload");

    reader.assert_hides(NOTICE);
    assert_eq!(reader.typed(), [THEIRS]);
    assert_eq!(reader.count("Unsaved changes"), 0);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn reload_brings_the_find_bar_up_to_date() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+E");
    another_program_writes(&mut reader, THEIRS);
    reader.press("Ctrl+F");
    reader.type_text("another");

    reader.click("Reload");

    reader.assert_sees("1 / 1");
}

#[test]
fn keep_mine_takes_the_notice_down_and_keeps_the_edits() {
    let mut reader = changed_under_edits();

    reader.click("Keep mine");

    reader.assert_hides(NOTICE);
    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn a_second_change_keeps_one_notice_and_reload_takes_the_latest() {
    let mut reader = changed_under_edits();
    let latest = "# Draft\n\nWritten twice by another program.";

    reader.write_file("draft.md", latest);
    reader.idle_for(NO_NOTICE);

    assert_eq!(reader.count(NOTICE), 1);
    assert_eq!(reader.count("Reload"), 1);
    reader.click("Reload");
    assert_eq!(reader.typed(), [latest]);
}

#[test]
fn a_change_after_keep_mine_is_announced_again() {
    let mut reader = changed_under_edits();
    reader.click("Keep mine");
    reader.assert_hides(NOTICE);

    another_program_writes(&mut reader, "# Draft\n\nWritten twice by another program.");

    assert_eq!(reader.typed(), [MINE]);
}

#[test]
fn the_notice_stays_with_its_tab() {
    let mut reader = Reader::with_docs(&[
        ("draft.md", SOURCE),
        ("other.md", "# Other\n\nSomething else."),
    ]);
    edit(&mut reader, " Mine.");
    another_program_writes(&mut reader, THEIRS);

    reader.drop_file("other.md");
    reader.assert_hides(NOTICE);
    assert_eq!(reader.count(TAB_MARK), 1);

    reader.click("tab:Draft");
    reader.assert_sees(NOTICE);
    assert_eq!(reader.typed(), [MINE]);
}

#[test]
fn pointing_at_a_marked_tab_says_what_became_of_its_file() {
    let mut reader = Reader::with_docs(&[
        ("draft.md", SOURCE),
        ("other.md", "# Other\n\nSomething else."),
    ]);
    edit(&mut reader, " Mine.");
    another_program_writes(&mut reader, THEIRS);
    reader.drop_file("other.md");
    reader.assert_hides(NOTICE);

    reader.hover("tab:Draft");

    reader.wait_for("the hover text", |reader| reader.sees(NOTICE));
}

#[test]
fn a_change_to_a_tab_behind_another_marks_it_and_waits_there() {
    let mut reader = Reader::with_docs(&[
        ("draft.md", SOURCE),
        ("other.md", "# Other\n\nSomething else."),
    ]);
    edit(&mut reader, " Mine.");
    reader.drop_file("other.md");
    assert_eq!(reader.count(TAB_MARK), 0);

    reader.write_file("draft.md", THEIRS);
    reader.wait_for("the tab to be marked", |reader| reader.count(TAB_MARK) == 1);

    reader.assert_hides(NOTICE);
    reader.assert_sees("Something else.");
    reader.click("tab:Draft");
    reader.assert_sees(NOTICE);
}

#[test]
fn a_document_without_unsaved_edits_reloads_without_a_word() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE)]);

    reader.write_file("draft.md", THEIRS);
    reader.wait_for("the page to reload", |reader| {
        reader.sees("Written by another program.")
    });

    reader.assert_hides(NOTICE);
    assert_eq!(reader.count("Reload"), 0);
}

#[test]
fn edits_taken_out_by_hand_do_not_make_a_changed_file_look_saved() {
    let mut reader = changed_under_edits();

    for _ in " Mine.".chars() {
        reader.press("Backspace");
    }

    assert_eq!(reader.typed(), [SOURCE]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    reader.assert_sees(NOTICE);
}

#[test]
fn text_made_the_same_as_the_file_needs_no_answer() {
    let mut reader = changed_under_edits();

    reader.press("Ctrl+A");
    reader.paste(THEIRS);

    assert_eq!(reader.typed(), [THEIRS]);
    reader.assert_hides(NOTICE);
    assert_eq!(reader.count("Unsaved changes"), 0);
}

#[test]
fn another_program_writing_just_what_i_typed_needs_no_answer() {
    let mut reader = unsaved_draft();

    reader.write_file("draft.md", MINE);
    reader.wait_for("the edits to count as saved", |reader| {
        reader.count("Unsaved changes") == 0
    });

    reader.assert_hides(NOTICE);
    assert_eq!(reader.typed(), [MINE]);
}

#[test]
fn a_file_deleted_under_unsaved_edits_is_announced_without_a_reload() {
    let mut reader = unsaved_draft();

    reader.delete_file("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(GONE));

    assert_eq!(reader.count("Reload"), 0);
    assert_eq!(reader.count("Keep mine"), 1);
    assert_eq!(reader.typed(), [MINE]);
    reader.assert_hides("Unable to reload");
}

#[test]
fn keep_mine_takes_the_notice_of_a_deleted_file_down_for_good() {
    let mut reader = unsaved_draft();
    reader.delete_file("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(GONE));

    reader.click("Keep mine");
    reader.type_text(" More.");
    reader.idle_for(NO_NOTICE);

    reader.assert_hides(GONE);
}

#[test]
fn a_deleted_file_that_comes_back_as_it_was_is_no_longer_announced() {
    let mut reader = unsaved_draft();
    reader.delete_file("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(GONE));

    reader.write_file("draft.md", SOURCE);
    reader.wait_for("the notice to go", |reader| !reader.sees(GONE));

    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn a_file_that_can_no_longer_be_read_is_announced_and_reload_says_why() {
    let mut reader = unsaved_draft();
    reader.delete_file("draft.md");
    reader.make_folder("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(NOTICE));

    reader.click("Reload");

    reader.assert_sees("Unable to reload");
    reader.assert_sees(NOTICE);
    assert_eq!(reader.typed(), [MINE]);
}
