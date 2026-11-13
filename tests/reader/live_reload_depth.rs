//! Live reload, in depth: every way a file changes on disk, and what the
//! page keeps while it follows.

use std::fmt::Write as _;
use std::time::Duration;

use super::docs::long_document;
use super::support::Reader;

/// Longer than a reload has ever taken here, for showing that none comes.
const NO_RELOAD: Duration = Duration::from_millis(1200);

const FIRST: &str = "# Notes\n\nThe first draft.";

fn notes() -> Reader {
    Reader::with_docs(&[("notes.md", FIRST)])
}

/// Wait until the page shows `text`.
#[track_caller]
fn follows(reader: &mut Reader, text: &str) {
    reader.wait_for(&format!("the page to show `{text}`"), |reader| {
        reader.sees(text)
    });
}

#[test]
fn a_file_emptied_on_disk_becomes_an_empty_page() {
    let mut reader = notes();

    reader.write_file("notes.md", "");
    reader.wait_for("the page to empty", |reader| {
        !reader.sees("The first draft.")
    });

    reader.assert_hides("Unable to");
    reader.assert_sees("notes.md \u{b7} 1 min read");
    assert_eq!(reader.title(), "notes.md - Markdown Viewer");
}

#[test]
fn a_deleted_file_is_reported_and_the_page_keeps_what_it_had() {
    let mut reader = notes();

    reader.delete_file("notes.md");
    follows(&mut reader, "Unable to reload");

    reader.assert_sees("notes.md");
    reader.assert_sees("The first draft.");
    assert_eq!(reader.title(), "notes.md - Markdown Viewer");
}

#[test]
fn a_file_deleted_and_made_again_is_followed_again() {
    let mut reader = notes();
    reader.delete_file("notes.md");
    follows(&mut reader, "Unable to reload");
    reader.click("Dismiss");

    reader.write_file("notes.md", "# Notes\n\nBack again.");
    follows(&mut reader, "Back again.");

    reader.assert_hides("The first draft.");
    reader.write_file("notes.md", "# Notes\n\nAnd still followed.");
    follows(&mut reader, "And still followed.");
}

#[test]
fn a_file_moved_away_and_back_is_followed_again() {
    let mut reader = notes();
    reader.rename_file("notes.md", "elsewhere.md");
    follows(&mut reader, "Unable to reload");

    reader.write_file("elsewhere.md", "# Notes\n\nEdited while away.");
    reader.rename_file("elsewhere.md", "notes.md");

    follows(&mut reader, "Edited while away.");
}

#[test]
fn saves_made_by_renaming_a_new_file_over_the_old_are_followed_every_time() {
    let mut reader = notes();

    for round in 1..=4 {
        let text = format!("Saved by rename, round {round}.");
        reader.save_over("notes.md", &format!("# Notes\n\n{text}"));

        follows(&mut reader, &text);
        reader.assert_hides("Unable to reload");
    }
}

#[test]
fn a_burst_of_writes_settles_on_the_last_one() {
    let mut reader = notes();

    for version in 1..=25 {
        reader.write_file(
            "notes.md",
            &format!("# Notes\n\nThis is version {version}."),
        );
    }
    follows(&mut reader, "This is version 25.");

    reader.idle_for(NO_RELOAD);
    reader.assert_sees("This is version 25.");
    reader.assert_hides("This is version 24.");
}

#[test]
fn a_file_that_grows_very_large_is_followed() {
    let mut reader = notes();
    let mut text = String::from("# Notes\n\nNow a very long read.\n\n");
    for line in 1..=6000 {
        let _ = write!(text, "Line {line} of a long list of plain lines.\n\n");
    }

    reader.write_file("notes.md", &text);
    follows(&mut reader, "Now a very long read.");

    reader.assert_sees("Line 1 of a long list of plain lines.");
    reader.assert_hides("1 min read");
}

#[test]
fn a_file_that_stops_being_utf8_is_shown_with_its_bad_bytes_marked() {
    let mut reader = notes();

    reader.write_bytes("notes.md", b"# Notes\n\nNa\xefve on the corner.");
    follows(&mut reader, "Na\u{fffd}ve on the corner.");

    reader.assert_hides("Unable to");
}

#[test]
fn the_title_bar_and_the_reading_time_follow_the_file() {
    let mut reader = notes();
    reader.assert_sees("notes.md \u{b7} 1 min read");
    let mut text = String::from("# Field diary\n\n");
    for _ in 0..400 {
        text.push_str("Seven more words to read right here. ");
    }

    reader.write_file("notes.md", &text);
    follows(&mut reader, "Field diary");

    assert_eq!(reader.count("Notes"), 0, "the old title is gone everywhere");
    reader.assert_hides("1 min read");
    reader.assert_sees("notes.md \u{b7} ");
    assert_eq!(reader.title(), "notes.md - Markdown Viewer");
}

#[test]
fn the_page_keeps_its_place_through_a_reload() {
    let text = long_document(80);
    let mut reader = Reader::with_docs(&[("long.md", &text)]);
    reader.scroll_down(2000.0);
    let place = reader.text_area("Section 20");

    let changed = text.replace("Paragraph 20 mentions", "Paragraph 20 now names");
    reader.write_file("long.md", &changed);
    follows(&mut reader, "Paragraph 20 now names the lighthouse once.");

    assert_eq!(reader.text_area("Section 20"), place);
}

#[test]
fn folds_in_the_contents_do_not_stop_a_reload() {
    let mut reader = Reader::with_docs(&[(
        "book.md",
        "# Book\n\n## Part one\n\n### Chapter\n\nOld text.\n",
    )]);
    reader.click("Collapse Part one");

    reader.write_file(
        "book.md",
        "# Book\n\n## Part one\n\n### Chapter\n\nNew text.\n\n## Part two\n",
    );
    follows(&mut reader, "New text.");

    assert_eq!(reader.count("Part two"), 2, "the heading and its entry");
}

#[test]
fn a_document_reached_by_a_link_is_followed() {
    let mut reader = Reader::with_docs(&[
        ("index.md", "# Index\n\n[the notes](notes.md)"),
        ("notes.md", FIRST),
    ]);
    reader.click("the notes");

    reader.write_file("notes.md", "# Notes\n\nChanged after the link.");

    follows(&mut reader, "Changed after the link.");
}

#[test]
fn a_document_left_by_a_link_no_longer_matters() {
    let mut reader = Reader::with_docs(&[
        ("index.md", "# Index\n\n[the notes](notes.md)"),
        ("notes.md", FIRST),
    ]);
    reader.click("the notes");

    reader.delete_file("index.md");
    reader.idle_for(NO_RELOAD);

    reader.assert_hides("Unable to reload");
    reader.assert_sees("The first draft.");
}

#[test]
fn a_dropped_document_is_followed() {
    let mut reader = Reader::with_docs(&[("index.md", "# Index"), ("notes.md", FIRST)]);
    reader.drop_file("notes.md");

    reader.write_file("notes.md", "# Notes\n\nChanged after the drop.");

    follows(&mut reader, "Changed after the drop.");
}

#[test]
fn every_open_tab_follows_its_own_file() {
    let mut reader = Reader::with_docs(&[
        ("one.md", "# One\n\nOld one."),
        ("two.md", "# Two\n\nOld two."),
    ]);
    reader.drop_file("two.md");

    reader.write_file("one.md", "# One\n\nNew one.");
    reader.write_file("two.md", "# Two\n\nNew two.");

    follows(&mut reader, "New two.");
    reader.click("one.md");
    follows(&mut reader, "New one.");
    reader.assert_hides("Old one.");
}

#[test]
fn a_closed_tabs_file_can_go_without_a_word() {
    let mut reader = Reader::with_docs(&[("index.md", "# Index"), ("notes.md", FIRST)]);
    reader.drop_file("notes.md");
    reader.press("Ctrl+W");

    reader.delete_file("notes.md");
    reader.idle_for(NO_RELOAD);

    reader.assert_hides("Unable to reload");
    reader.assert_sees("Index");
}

#[test]
fn turning_live_reload_on_in_a_no_watch_launch_starts_following() {
    let mut reader = Reader::launch()
        .doc("notes.md", FIRST)
        .args(&["notes.md", "--no-watch"])
        .open();

    reader.press("F5");
    reader.write_file("notes.md", "# Notes\n\nFollowed after all.");

    follows(&mut reader, "Followed after all.");
}

#[test]
fn with_live_reload_switched_off_nothing_is_said_about_a_deleted_file() {
    let mut reader = notes();
    reader.click("More");
    reader.click("Live Reload");

    reader.delete_file("notes.md");
    reader.idle_for(NO_RELOAD);

    reader.assert_hides("Unable to reload");
    reader.assert_sees("The first draft.");
}

#[test]
fn a_reload_error_can_be_dismissed_and_the_next_change_still_arrives() {
    let mut reader = notes();
    reader.delete_file("notes.md");
    follows(&mut reader, "Unable to reload");

    reader.click("Dismiss");
    reader.assert_hides("Unable to reload");

    reader.save_over("notes.md", "# Notes\n\nRestored.");
    follows(&mut reader, "Restored.");
    reader.assert_hides("Unable to reload");
}

#[test]
fn the_live_reload_switch_says_what_it_does() {
    let mut reader = notes();

    reader.click("More");
    reader.hover("Live Reload");

    reader.assert_sees("F5");
    assert!(reader.is_on("Live Reload"));
}
