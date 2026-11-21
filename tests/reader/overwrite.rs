//! Saving over a file another program changed: the question that asks
//! first, from Ctrl+S and from Save in the prompt about unsaved edits, and
//! what each answer does.

use super::disk_change::{
    another_program_writes, changed_under_edits, unsaved_draft, unsaved_draft_beside_another, GONE,
    MINE, NOTICE, NO_NOTICE, SOURCE, THEIRS,
};
use super::support::Reader;
use super::unsaved::edit;

const ASKS: &str = "“Draft” (draft.md) changed on disk. Overwrite it?";

/// `draft.md` opened without live reload, edited, and then changed by
/// another program, which nothing told the app about.
fn changed_unwatched() -> Reader {
    let mut reader = Reader::launch()
        .doc("draft.md", SOURCE)
        .args(&["draft.md", "--no-watch"])
        .open();
    edit(&mut reader, " Mine.");
    reader.write_file("draft.md", THEIRS);
    reader
}

#[test]
fn a_save_after_keep_mine_asks_before_it_writes() {
    let mut reader = changed_under_edits();
    reader.click("Keep mine");

    reader.press("Ctrl+S");

    reader.assert_sees(ASKS);
    for answer in ["Overwrite", "Cancel"] {
        assert_eq!(reader.count(answer), 1, "{answer}");
    }
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn a_save_while_the_notice_shows_asks_as_well() {
    let mut reader = changed_under_edits();

    reader.press("Ctrl+S");

    reader.assert_sees(ASKS);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn overwrite_writes_my_text_and_settles_the_matter() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+S");

    reader.click("Overwrite");

    assert_eq!(reader.read_file("draft.md"), MINE);
    assert_eq!(reader.files_in(""), ["draft.md"]);
    reader.assert_hides(ASKS);
    reader.assert_hides(NOTICE);
    reader.assert_sees("Saved");
    assert_eq!(reader.count("Unsaved changes"), 0);
}

#[test]
fn cancel_writes_nothing_and_leaves_the_notice_and_the_edits() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+S");

    reader.click("Cancel");

    reader.assert_hides(ASKS);
    reader.assert_sees(NOTICE);
    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn escape_answers_the_overwrite_question_as_cancel_does() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+S");

    reader.press("Escape");

    reader.assert_hides(ASKS);
    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn enter_does_not_overwrite_because_the_keyboard_starts_on_cancel() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+S");
    assert_eq!(reader.focused().as_deref(), Some("Cancel"));

    reader.press("Enter");

    reader.assert_hides(ASKS);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn a_move_to_overwrite_and_enter_typed_in_one_frame_overwrite() {
    for to_overwrite in ["Shift+Tab", "Left"] {
        let mut reader = changed_under_edits();
        reader.press("Ctrl+S");

        reader.tap_in_turn(&[to_overwrite, "Enter"]);

        assert_eq!(reader.read_file("draft.md"), MINE, "{to_overwrite}");
        reader.assert_hides(ASKS);
    }
}

#[test]
fn the_shortcuts_wait_while_the_overwrite_question_is_up() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+S");

    reader.press("Ctrl+W");
    reader.press("Ctrl+S");

    reader.assert_sees(ASKS);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn save_in_the_close_prompt_asks_the_overwrite_question_too() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+W");

    reader.click("Save");

    reader.assert_sees(ASKS);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn enter_held_on_save_does_not_answer_the_overwrite_question() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+W");
    assert_eq!(reader.focused().as_deref(), Some("Save"));

    reader.hold("Enter", &[1, 1]);

    assert_eq!(reader.read_file("draft.md"), THEIRS);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
}

#[test]
fn enter_pressed_twice_on_save_does_not_answer_the_overwrite_question() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+W");

    reader.press_twice("Enter");

    assert_eq!(reader.read_file("draft.md"), THEIRS);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
}

#[test]
fn overwrite_in_the_close_prompt_writes_and_then_closes() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+W");
    reader.click("Save");

    reader.click("Overwrite");

    assert_eq!(reader.read_file("draft.md"), MINE);
    assert_eq!(reader.title(), "Markdown Viewer");
    reader.assert_hides(ASKS);
}

#[test]
fn cancel_in_the_close_prompt_closes_nothing_and_writes_nothing() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+W");
    reader.click("Save");

    reader.click("Cancel");

    reader.assert_hides(ASKS);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn after_overwrite_the_prompt_goes_on_to_the_next_unsaved_document() {
    let mut reader = unsaved_draft_beside_another();
    another_program_writes(&mut reader, THEIRS);
    reader.drop_file("other.md");
    edit(&mut reader, " Too.");

    reader.close_window();
    reader.assert_sees("Save changes to “Draft” (draft.md)?");
    reader.click("Save");
    reader.assert_sees(ASKS);
    reader.click("Overwrite");

    assert_eq!(reader.read_file("draft.md"), MINE);
    reader.assert_sees("Save changes to “Other” (other.md)?");
    assert_eq!(reader.tabs(), ["Draft", "Other"]);
}

#[test]
fn discard_in_the_close_prompt_leaves_the_other_programs_text() {
    let mut reader = changed_under_edits();
    reader.press("Ctrl+W");

    reader.click("Discard");

    assert_eq!(reader.title(), "Markdown Viewer");
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn a_save_of_a_file_nobody_else_touched_asks_nothing() {
    let mut reader = unsaved_draft();

    reader.press("Ctrl+S");

    reader.assert_hides("Overwrite");
    assert_eq!(reader.read_file("draft.md"), MINE);
}

#[test]
fn without_live_reload_nothing_is_announced_but_a_save_still_asks() {
    let mut reader = changed_unwatched();
    reader.idle_for(NO_NOTICE);
    reader.assert_hides(NOTICE);

    reader.press("Ctrl+S");

    reader.assert_sees(ASKS);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn without_live_reload_cancel_leaves_the_notice_the_save_found() {
    let mut reader = changed_unwatched();
    reader.press("Ctrl+S");

    reader.click("Cancel");

    reader.assert_sees(NOTICE);
    reader.click("Reload");
    assert_eq!(reader.typed(), [THEIRS]);
}

#[test]
fn without_live_reload_overwrite_writes_my_text() {
    let mut reader = changed_unwatched();
    reader.press("Ctrl+S");

    reader.click("Overwrite");

    assert_eq!(reader.read_file("draft.md"), MINE);
    assert_eq!(reader.count("Unsaved changes"), 0);
}

#[test]
fn a_save_writes_a_deleted_file_again_without_a_question() {
    let mut reader = unsaved_draft();
    reader.delete_file("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(GONE));

    reader.press("Ctrl+S");

    reader.assert_hides("Overwrite");
    reader.assert_hides(GONE);
    assert_eq!(reader.read_file("draft.md"), MINE);
}

#[test]
fn a_file_deleted_and_written_again_by_the_other_program_is_still_asked_about() {
    let mut reader = changed_under_edits();
    reader.click("Keep mine");
    reader.delete_file("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(GONE));

    another_program_writes(&mut reader, THEIRS);
    reader.press("Ctrl+S");

    reader.assert_sees(ASKS);
    assert_eq!(reader.read_file("draft.md"), THEIRS);
}

#[test]
fn a_save_over_a_file_that_cannot_be_read_asks_and_then_says_why_it_failed() {
    let mut reader = unsaved_draft();
    reader.delete_file("draft.md");
    reader.make_folder("draft.md");
    reader.wait_for("the notice", |reader| reader.sees(NOTICE));
    reader.click("Keep mine");

    reader.press("Ctrl+S");
    reader.assert_sees(ASKS);
    reader.click("Overwrite");

    reader.assert_sees("Unable to save");
    assert_eq!(reader.typed(), [MINE]);
    assert_eq!(reader.count("Unsaved changes"), 1);
}
