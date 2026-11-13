//! Links and history, in depth: where a link leads, what counts as a step
//! back, and what happens to history when files and tabs go away.

use super::docs::{long_document, three_linked};
use super::support::Reader;

/// A library whose documents link across folders.
fn library() -> Reader {
    Reader::with_docs(&[
        (
            "index.md",
            "# Index\n\n[Setup](guides/setup.md), [my notes](my%20notes.md), \
             [the web](https://example.com/docs?page=2#top), [write](mailto:someone@example.com).",
        ),
        (
            "guides/setup.md",
            "# Setup\n\nThen [tuning](deep/tuning.md) or [home](../index.md).",
        ),
        (
            "guides/deep/tuning.md",
            "# Tuning\n\nBack to [the index](../../index.md) or [setup](../setup.md).",
        ),
        ("my notes.md", "# My notes\n\nA name with a space."),
    ])
}

#[test]
fn a_long_walk_can_be_retraced_step_by_step_in_both_directions() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.click("to alpha");
    reader.click("to gamma");
    let walk = ["a.md", "b.md", "a.md", "c.md"];

    for file in walk.iter().rev().skip(1) {
        reader.press("Alt+Left");
        assert_eq!(reader.title(), format!("{file} - Markdown Viewer"));
    }
    assert!(!reader.is_enabled("Go back"));

    for file in walk.iter().skip(1) {
        reader.press("Alt+Right");
        assert_eq!(reader.title(), format!("{file} - Markdown Viewer"));
    }
    assert!(!reader.is_enabled("Go forward"));
}

#[test]
fn back_and_forward_with_nowhere_to_go_change_nothing() {
    let mut reader = three_linked();

    reader.press("Alt+Left");
    reader.press("Alt+Right");

    reader.assert_sees("Alpha");
    reader.assert_hides("Unable to");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn the_home_page_offers_no_way_back_or_forward() {
    let mut reader = Reader::first_run();

    assert!(!reader.is_enabled("Go back"));
    assert!(!reader.is_enabled("Go forward"));
    reader.press("Alt+Left");
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn links_lead_into_folders_and_back_out_of_them() {
    let mut reader = library();

    for (link, title, text) in [
        ("Setup", "setup.md", "Then "),
        ("tuning", "tuning.md", "Back to "),
        ("setup", "setup.md", "Then "),
        ("home", "index.md", "Index"),
    ] {
        reader.click(link);

        assert_eq!(reader.title(), format!("{title} - Markdown Viewer"));
        reader.assert_sees(text);
    }
}

#[test]
fn a_link_two_folders_up_is_found() {
    let mut reader = library();
    reader.click("Setup");
    reader.click("tuning");

    reader.click("the index");

    reader.assert_sees("my notes");
    assert_eq!(reader.title(), "index.md - Markdown Viewer");
}

#[test]
fn a_link_with_an_escaped_space_opens_the_file_with_the_space() {
    let mut reader = library();

    reader.click("my notes");

    reader.assert_sees("A name with a space.");
    assert_eq!(reader.title(), "my notes.md - Markdown Viewer");
}

#[test]
fn a_web_link_goes_to_the_browser_and_leaves_the_page_alone() {
    let mut reader = library();
    assert!(reader.opened_in_browser().is_empty());

    reader.click("the web");

    assert_eq!(
        reader.opened_in_browser(),
        ["https://example.com/docs?page=2#top"]
    );
    reader.assert_sees("Index");
    reader.assert_hides("Unable to open");
    assert!(!reader.is_enabled("Go back"));
    assert_eq!(reader.title(), "index.md - Markdown Viewer");
}

#[test]
fn a_mail_link_is_handed_to_the_desktop() {
    let mut reader = library();

    reader.click("write");

    assert_eq!(reader.opened_in_browser(), ["mailto:someone@example.com"]);
    reader.assert_sees("Index");
}

#[test]
fn a_jump_within_the_page_is_not_a_step_in_history() {
    let text = format!("[skip ahead](#section-50)\n\n{}", long_document(60));
    let mut reader = Reader::with_docs(&[
        ("index.md", "# Index\n\n[the long one](long.md)"),
        ("long.md", &text),
    ]);
    reader.click("the long one");

    reader.click("skip ahead");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Section 50")
    });
    reader.press("Alt+Left");

    assert_eq!(reader.title(), "index.md - Markdown Viewer");
    reader.assert_sees("the long one");
}

#[test]
fn going_back_to_a_document_shows_it_from_its_start() {
    let text = format!("[away](b.md)\n\n{}", long_document(60));
    let mut reader = Reader::with_docs(&[("long.md", &text), ("b.md", "# B\n\nShort.")]);
    reader.click("away");

    reader.press("Alt+Left");

    assert!(reader.in_view("away"));
    assert!(reader.in_view("Section 1"));
}

#[test]
fn going_forward_to_a_heading_link_arrives_at_the_document() {
    let mut reader = Reader::with_docs(&[
        ("index.md", "# Index\n\n[far on](long.md#section-40)"),
        ("long.md", &long_document(60)),
    ]);
    reader.click("far on");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Section 40")
    });

    reader.press("Alt+Left");
    reader.press("Alt+Right");

    assert_eq!(reader.title(), "long.md - Markdown Viewer");
    reader.assert_sees("Long read");
}

#[test]
fn a_heading_link_to_a_heading_that_is_not_there_still_opens_the_document() {
    let mut reader = Reader::with_docs(&[
        (
            "index.md",
            "# Index\n\n[nowhere in it](long.md#no-such-heading)",
        ),
        ("long.md", &long_document(30)),
    ]);

    reader.click("nowhere in it");

    assert_eq!(reader.title(), "long.md - Markdown Viewer");
    assert!(reader.in_view("Section 1"), "it opened at its start");
    reader.assert_hides("Unable to open");
}

#[test]
fn going_back_to_a_file_that_was_deleted_says_so_and_stays_put() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.delete_file("a.md");

    reader.press("Alt+Left");

    reader.assert_sees("Unable to navigate history");
    reader.assert_sees("Beta");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn closing_another_tab_leaves_this_tabs_history_alone() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.click("on to gamma");
    reader.press("Alt+Left");
    reader.drop_file("a.md");
    reader.click("to gamma");

    reader.press("Ctrl+W");

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    assert!(reader.is_enabled("Go back"));
    assert!(reader.is_enabled("Go forward"));
    reader.press("Alt+Right");
    reader.assert_sees("The end of the line.");
    reader.press("Alt+Left");
    reader.press("Alt+Left");
    reader.assert_sees("Alpha");
}

#[test]
fn a_file_opened_from_the_sidebar_is_a_step_that_can_be_undone_and_redone() {
    let mut reader = three_linked();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("c.md") == 1
    });

    reader.click("c.md");
    reader.press("Alt+Left");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");

    reader.press("Alt+Right");
    assert_eq!(reader.title(), "c.md - Markdown Viewer");
}

#[test]
fn a_link_to_the_page_itself_is_not_a_step() {
    let mut reader = Reader::with_docs(&[("self.md", "# Self\n\n[again](self.md)")]);

    reader.click("again");

    reader.assert_sees("Self");
    assert!(!reader.is_enabled("Go back"));
    reader.assert_hides("Unable to open");
}

#[test]
fn a_document_reached_by_a_link_counts_as_read() {
    let mut reader = three_linked();
    reader.click("to beta");

    reader.press("Ctrl+W");

    reader.assert_sees("RECENTLY READ");
    assert_eq!(reader.count("Beta"), 1, "the home page lists it");
}

#[test]
fn history_is_not_kept_across_a_restart() {
    let mut reader = three_linked();
    reader.click("to beta");

    let reader = reader.restart(&[]);

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    assert!(!reader.is_enabled("Go back"));
}
