//! Edit mode, in depth: exactly what a save writes, when the unsaved mark
//! shows, what a failed save says, and what happens when the file changes
//! on disk under an edit.

use std::time::Duration;

use super::docs::long_document;
use super::support::Reader;

const SOURCE: &str = "# Draft\n\nFirst thoughts.";

/// Longer than a reload has ever taken here, for showing that none comes.
const NO_RELOAD: Duration = Duration::from_millis(1200);

/// `draft.md` open in edit mode with the keyboard in the source.
fn editing(source: &str) -> Reader {
    let mut reader = Reader::with_docs(&[("draft.md", source)]);
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader
}

/// Replace the whole source with `text`.
fn retype(reader: &mut Reader, text: &str) {
    reader.press("Ctrl+A");
    reader.type_text(text);
}

/// Put `text` at the very end of the source.
fn append(reader: &mut Reader, text: &str) {
    reader.press("Ctrl+End");
    reader.type_text(text);
}

/// The size the source is set at, in pixels.
#[track_caller]
fn source_size(reader: &Reader, source: &str) -> f32 {
    let sizes = reader.sizes_of(source);
    assert_eq!(sizes.len(), 1, "the source is painted once");
    sizes[0]
}

#[test]
fn each_larger_text_size_sets_the_source_larger() {
    let source = "Plain words on one line.";
    let mut reader = editing(source);
    reader.click("Appearance");
    reader.click("Smaller text");
    assert!(!reader.is_enabled("Smaller text"), "Small is the smallest");

    let mut sizes = vec![source_size(&reader, source)];
    while reader.is_enabled("Larger text") {
        reader.click("Larger text");
        sizes.push(source_size(&reader, source));
    }

    assert_eq!(sizes.len(), 5, "Small to Huge");
    assert!(
        sizes.is_sorted_by(|smaller, larger| smaller < larger),
        "{sizes:?}"
    );
}

#[test]
fn a_save_writes_exactly_what_was_typed() {
    let cases = [
        "no newline at the end",
        "one newline at the end\n",
        "blank lines kept\n\n\nand trailing ones too\n\n\n",
        "   leading and trailing spaces   \n",
        "\ta tab, then text\n",
        "na\u{ef}ve sm\u{f6}rg\u{e5}sbord \u{2014} \u{6771}\u{4eac} \u{2713} \u{1f642}\n",
        "# Heading\n\n- [ ] task\n\n```rust\nfn main() {}\n```\n",
        "<!-- comment --> &amp; \\* not a bullet\n",
    ];
    let mut reader = editing(SOURCE);

    for text in cases {
        retype(&mut reader, text);
        reader.press("Ctrl+S");

        assert_eq!(reader.typed(), [text]);
        assert_eq!(reader.read_bytes("draft.md"), text.as_bytes(), "{text:?}");
    }
}

#[test]
fn deleting_everything_and_saving_leaves_an_empty_file() {
    let mut reader = editing(SOURCE);

    reader.press("Ctrl+A");
    reader.press("Backspace");
    reader.press("Ctrl+S");

    assert_eq!(reader.read_bytes("draft.md"), b"");
    reader.assert_sees("Saved");
}

#[test]
fn windows_line_endings_are_kept_where_nothing_was_typed() {
    let mut reader = editing("# Draft\r\n\r\nFirst thoughts.\r\n");

    append(&mut reader, "More.");
    reader.press("Ctrl+S");

    assert_eq!(
        reader.read_bytes("draft.md"),
        b"# Draft\r\n\r\nFirst thoughts.\r\nMore."
    );
}

#[test]
fn enter_adds_one_plain_newline() {
    let mut reader = editing(SOURCE);
    reader.press("Ctrl+End");

    reader.press("Enter");
    reader.type_text("Next line.");
    reader.press("Ctrl+S");

    assert_eq!(
        reader.read_file("draft.md"),
        format!("{SOURCE}\nNext line.")
    );
}

#[test]
fn pasted_text_goes_into_the_source() {
    let mut reader = editing(SOURCE);
    reader.press("Ctrl+End");

    reader.paste("\n\nPasted *thoughts*.");
    reader.press("Ctrl+S");

    assert_eq!(
        reader.read_file("draft.md"),
        format!("{SOURCE}\n\nPasted *thoughts*.")
    );
}

/// 👨‍👩‍👧 is five characters drawn as one picture.
const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}";

/// What is in the file once the source is saved.
fn saved(reader: &mut Reader) -> String {
    reader.press("Ctrl+S");
    reader.read_file("draft.md")
}

#[test]
fn the_arrow_keys_step_over_an_emoji_written_as_several_characters() {
    let mut reader = editing(SOURCE);
    append(&mut reader, &format!(" {FAMILY}!"));

    reader.press("Left");
    reader.press("Left");
    reader.type_text("<");
    reader.press("Right");
    reader.type_text(">");

    assert_eq!(saved(&mut reader), format!("{SOURCE} <{FAMILY}>!"));
}

#[test]
fn a_held_arrow_moves_the_caret_by_every_repeat_of_a_slow_frame() {
    let mut reader = editing("one\ntwo\nthree\nfour\nfive\nsix");
    reader.press("Ctrl+Home");

    reader.hold("Down", &[3]);
    reader.type_text(">");

    assert_eq!(saved(&mut reader), "one\ntwo\nthree\nfour\n>five\nsix");
}

#[test]
fn backspace_and_delete_remove_all_of_an_emoji_written_as_several_characters() {
    let mut reader = editing(SOURCE);
    append(&mut reader, &format!(" {FAMILY}{FAMILY}!"));

    reader.press("Left");
    reader.press("Backspace");
    assert_eq!(saved(&mut reader), format!("{SOURCE} {FAMILY}!"));

    reader.press("Left");
    reader.press("Del");
    assert_eq!(saved(&mut reader), format!("{SOURCE} !"));
}

#[test]
fn shift_and_an_arrow_select_all_of_an_emoji_to_copy_or_type_over() {
    let mut reader = editing(SOURCE);
    append(&mut reader, &format!(" {FAMILY}!"));

    reader.press("Left");
    reader.press("Shift+Left");
    reader.copy();
    assert_eq!(reader.clipboard(), Some(FAMILY));

    reader.type_text("family");
    assert_eq!(saved(&mut reader), format!("{SOURCE} family!"));
}

#[test]
fn ctrl_z_brings_back_all_of_an_emoji_backspace_removed() {
    let mut reader = editing(&format!("{SOURCE} {FAMILY}"));
    reader.press("Ctrl+End");

    reader.press("Backspace");
    assert_eq!(reader.typed(), [format!("{SOURCE} ")]);

    reader.press("Ctrl+Z");
    assert_eq!(saved(&mut reader), format!("{SOURCE} {FAMILY}"));
}

/// The same emoji, pasted, is stepped over and removed whole as well.
#[test]
fn a_pasted_emoji_is_one_character_to_the_keyboard_too() {
    let mut reader = editing(SOURCE);
    reader.press("Ctrl+End");
    reader.paste(&format!(" 1\u{fe0f}\u{20e3}\u{1f1ef}\u{1f1f5}{FAMILY}"));

    reader.press("Backspace");
    reader.press("Left");
    reader.type_text("-");
    reader.press("Left");
    reader.press("Backspace");

    assert_eq!(saved(&mut reader), format!("{SOURCE} -\u{1f1ef}\u{1f1f5}"));
}

#[test]
fn removing_the_last_character_of_a_line_keeps_its_windows_line_ending() {
    let mut reader = editing("ab\r\ncd\r\n");

    reader.press("Ctrl+Home");
    reader.press("Right");
    reader.press("Del");
    reader.press("Ctrl+S");

    assert_eq!(reader.read_bytes("draft.md"), b"a\r\ncd\r\n");
}

#[test]
fn a_save_leaves_no_other_file_beside_the_document() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " More.");

    reader.press("Ctrl+S");

    assert_eq!(reader.files_in(""), ["draft.md"]);
}

#[test]
fn the_unsaved_mark_shows_exactly_while_the_source_differs_from_the_file() {
    let mut reader = editing(SOURCE);
    let unsaved = |reader: &Reader| reader.count("Unsaved changes") == 1;
    assert!(!unsaved(&reader));

    append(&mut reader, "x");
    assert!(unsaved(&reader), "after typing");

    reader.press("Backspace");
    assert!(!unsaved(&reader), "after taking the typing back");

    reader.type_text("y");
    reader.press("Ctrl+S");
    assert!(!unsaved(&reader), "after saving");

    reader.type_text("z");
    assert!(unsaved(&reader), "after typing past the save");
}

#[test]
fn the_unsaved_mark_stays_in_the_preview() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " More.");

    reader.press("Ctrl+E");

    reader.assert_sees("First thoughts. More.");
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), SOURCE);
}

#[test]
fn ctrl_s_saves_from_the_preview_too() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " More.");
    reader.press("Ctrl+E");
    assert_eq!(reader.count("Unsaved changes"), 1);

    reader.press("Ctrl+S");

    assert!(
        reader.read_file("draft.md") == format!("{SOURCE} More."),
        "Ctrl+S in the preview did not save the unsaved changes"
    );
}

#[test]
fn the_word_saved_goes_away_by_itself() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " More.");
    reader.press("Ctrl+S");
    reader.assert_sees("Saved");

    reader.wait_for("the word to go away", |reader| !reader.sees("Saved"));

    assert_eq!(reader.typed(), [format!("{SOURCE} More.")]);
}

#[test]
fn a_lone_document_has_the_switch_and_keeps_its_place_when_editing_starts_and_ends() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE)]);
    let reading = reader.column();
    assert!(reader.is_on("Preview"), "the switch is there at rest");
    let switch = reader.area_of("Edit");

    reader.press("Ctrl+E");
    assert_eq!(reader.column(), reading);
    assert!(reader.is_on("Edit"));

    reader.press("Ctrl+E");
    assert_eq!(reader.column(), reading);
    assert!(reader.is_on("Preview"));
    assert_eq!(reader.area_of("Edit"), switch);
}

#[test]
fn a_lone_document_keeps_the_switch_while_it_has_unsaved_changes() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " More.");

    reader.press("Ctrl+E");

    assert!(reader.is_on("Preview"));
    reader.click("Edit");
    assert_eq!(reader.typed(), [format!("{SOURCE} More.")]);
}

#[test]
fn with_several_tabs_the_switch_is_always_there() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE), ("other.md", "# Other")]);
    reader.drop_file("other.md");
    let reading = reader.column();
    assert!(reader.is_on("Preview"));

    reader.click("Edit");

    assert_eq!(reader.typed(), ["# Other"]);
    assert_eq!(reader.column(), reading);
}

#[test]
fn the_page_is_where_it_was_after_a_look_at_the_source() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(80))]);
    reader.scroll_down(2000.0);
    let place = reader.text_area("Section 20");

    reader.press("Ctrl+E");
    assert!(reader.typed()[0].starts_with("# Long read"));
    reader.press("Ctrl+E");
    reader.wait_for("the page to come back", |reader| {
        reader.in_view("Section 20")
    });

    assert_eq!(reader.text_area("Section 20"), place);
}

#[test]
fn a_save_that_fails_keeps_the_text_and_the_mark_and_can_be_retried() {
    let mut reader = Reader::with_docs(&[("notes/draft.md", SOURCE)]);
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    append(&mut reader, " Kept.");
    reader.delete_folder("notes");

    reader.press("Ctrl+S");
    reader.assert_sees("Unable to save");
    reader.assert_sees("draft.md");
    assert_eq!(reader.typed(), [format!("{SOURCE} Kept.")]);
    assert_eq!(reader.count("Unsaved changes"), 1);

    reader.make_folder("notes");
    reader.click("Dismiss");
    reader.press("Ctrl+S");

    assert_eq!(
        reader.read_file("notes/draft.md"),
        format!("{SOURCE} Kept.")
    );
    assert_eq!(reader.count("Unsaved changes"), 0);
    reader.assert_hides("Unable to save");
}

#[test]
fn a_save_onto_a_name_a_folder_has_taken_fails_and_leaves_nothing_behind() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " Lost.");
    reader.delete_file("draft.md");
    reader.write_file("draft.md/inside.md", "# In the way");

    // What is there now cannot be read, so the save asks before it tries.
    reader.press("Ctrl+S");
    reader.click("Overwrite");

    reader.assert_sees("Unable to save");
    reader.assert_sees("Save failed");
    assert_eq!(reader.files_in(""), ["draft.md"]);
    assert_eq!(reader.files_in("draft.md"), ["inside.md"]);
}

#[test]
fn unsaved_edits_are_kept_when_the_file_changes_on_disk() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " Mine.");

    reader.write_file("draft.md", "# Draft\n\nTheirs.");
    reader.idle_for(NO_RELOAD);

    assert_eq!(reader.typed(), [format!("{SOURCE} Mine.")]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), "# Draft\n\nTheirs.");
}

#[test]
fn saving_after_the_file_changed_on_disk_asks_and_then_writes_what_was_typed() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " Mine.");
    reader.write_file("draft.md", "# Draft\n\nTheirs.");
    reader.idle_for(NO_RELOAD);

    reader.press("Ctrl+S");
    assert_eq!(reader.read_file("draft.md"), "# Draft\n\nTheirs.");
    reader.click("Overwrite");

    assert_eq!(reader.read_file("draft.md"), format!("{SOURCE} Mine."));
    assert_eq!(reader.count("Unsaved changes"), 0);
}

#[test]
fn a_source_with_no_edits_follows_the_file_on_disk() {
    let mut reader = editing(SOURCE);

    reader.write_file("draft.md", "# Draft\n\nTheirs.");
    reader.wait_for("the source to reload", |reader| {
        reader.typed() == ["# Draft\n\nTheirs."]
    });

    assert_eq!(reader.count("Unsaved changes"), 0);
    assert!(reader.is_on("Edit"), "still editing");
}

#[test]
fn unsaved_edits_in_a_tab_behind_are_kept_when_its_file_changes() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE), ("other.md", "# Other")]);
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    append(&mut reader, " Mine.");
    reader.drop_file("other.md");

    reader.write_file("draft.md", "# Draft\n\nTheirs.");
    reader.idle_for(NO_RELOAD);
    reader.click("tab:Draft");

    assert_eq!(reader.typed(), [format!("{SOURCE} Mine.")]);
}

#[test]
fn a_save_writes_only_the_document_in_front() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE), ("other.md", "# Other")]);
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    append(&mut reader, " Mine.");
    reader.drop_file("other.md");

    reader.press("Ctrl+S");

    assert_eq!(reader.read_file("draft.md"), SOURCE);
    assert_eq!(reader.read_file("other.md"), "# Other");
    reader.click("tab:Draft");
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn the_source_stays_as_saved_once_the_save_has_settled() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " More.");

    reader.press("Ctrl+S");
    reader.idle_for(NO_RELOAD);

    assert_eq!(reader.typed(), [format!("{SOURCE} More.")]);
    reader.assert_hides("Unable to");
    reader.type_text(" And more.");
    assert_eq!(reader.typed(), [format!("{SOURCE} More. And more.")]);
}

#[test]
fn after_a_save_the_page_and_the_contents_show_the_new_heading() {
    let mut reader = editing(SOURCE);
    append(&mut reader, "\n\n## Afterword\n\nSecond thoughts.");

    reader.press("Ctrl+S");
    reader.press("Ctrl+E");
    reader.wait_for("the contents to list the new heading", |reader| {
        reader.count("Afterword") == 2
    });

    reader.assert_sees("Second thoughts.");
}

#[test]
fn unsaved_edits_are_not_written_or_restored_by_a_restart() {
    let mut reader = editing(SOURCE);
    append(&mut reader, " Never saved.");

    let reader = reader.restart(&[]);

    assert_eq!(reader.read_file("draft.md"), SOURCE);
    reader.assert_sees("First thoughts.");
    reader.assert_hides("Never saved.");
    assert_eq!(reader.count("Unsaved changes"), 0);
}

#[test]
fn edit_mode_is_not_offered_without_a_document() {
    let mut reader = Reader::first_run();

    reader.press("Ctrl+E");
    assert!(reader.typed().is_empty());

    reader.click("More");
    assert!(!reader.is_enabled("Edit Document"));
}

#[test]
fn ctrl_s_with_nothing_to_save_leaves_the_file_as_it_is() {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE)]);

    reader.press("Ctrl+S");

    assert_eq!(reader.read_file("draft.md"), SOURCE);
    assert_eq!(reader.files_in(""), ["draft.md"]);
    reader.assert_hides("Unable to save");
}
