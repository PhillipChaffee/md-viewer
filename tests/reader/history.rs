//! Links between documents, and back and forward through them.

use super::docs::{long_document, three_linked};
use super::support::Reader;

#[test]
fn a_link_to_another_document_opens_it_in_place() {
    let mut reader = three_linked();

    reader.click("to beta");

    reader.assert_sees("Beta");
    reader.assert_hides("Alpha");
    assert_eq!(reader.count("a.md"), 0, "no tab strip: still one document");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn back_and_forward_are_offered_only_when_there_is_somewhere_to_go() {
    let mut reader = three_linked();
    assert!(!reader.is_enabled("Go back"));
    assert!(!reader.is_enabled("Go forward"));

    reader.click("to beta");
    assert!(reader.is_enabled("Go back"));
    assert!(!reader.is_enabled("Go forward"));

    reader.click("Go back");
    assert!(!reader.is_enabled("Go back"));
    assert!(reader.is_enabled("Go forward"));
}

#[test]
fn the_back_and_forward_buttons_retrace_the_steps() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.click("on to gamma");
    reader.assert_sees("The end of the line.");

    reader.click("Go back");
    reader.assert_sees("Beta");
    reader.click("Go back");
    reader.assert_sees("Alpha");

    reader.click("Go forward");
    reader.assert_sees("Beta");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn alt_arrows_go_back_and_forward() {
    let mut reader = three_linked();
    reader.click("to gamma");

    reader.press("Alt+Left");
    reader.assert_sees("Alpha");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");

    reader.press("Alt+Right");
    reader.assert_sees("Gamma");
    assert_eq!(reader.title(), "c.md - Markdown Viewer");
}

#[test]
fn the_menu_goes_back_and_forward_too() {
    let mut reader = three_linked();
    reader.click("to beta");

    reader.click("More");
    assert!(reader.is_enabled("Back"));
    assert!(!reader.is_enabled("Forward"));
    reader.click("Back");
    reader.assert_sees("Alpha");

    reader.click("More");
    reader.click("Forward");
    reader.assert_sees("Beta");
}

#[test]
fn following_a_link_after_going_back_drops_the_way_forward() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.press("Alt+Left");

    reader.click("to gamma");

    assert!(!reader.is_enabled("Go forward"));
    reader.press("Alt+Left");
    reader.assert_sees("Alpha");
}

#[test]
fn a_link_to_a_missing_document_says_so_and_stays_put() {
    let mut reader = three_linked();

    reader.click("nowhere");

    reader.assert_sees("Unable to open");
    reader.assert_sees("missing.md");
    reader.assert_sees("Alpha");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
    assert!(!reader.is_enabled("Go back"));
}

#[test]
fn a_link_to_a_heading_in_another_document_lands_on_it() {
    let mut reader = Reader::with_docs(&[
        (
            "index.md",
            "# Index\n\nSee [the far section](long.md#section-60).",
        ),
        ("long.md", &long_document(80)),
    ]);

    reader.click("the far section");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Section 60")
    });

    assert_eq!(reader.title(), "long.md - Markdown Viewer");
    assert!(!reader.in_view("Section 1"));
}

#[test]
fn a_link_to_a_heading_in_the_same_document_scrolls_to_it() {
    let text = format!("[skip ahead](#section-70)\n\n{}", long_document(80));
    let mut reader = Reader::with_docs(&[("long.md", &text)]);
    assert!(!reader.in_view("Section 70"));

    reader.click("skip ahead");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Section 70")
    });

    assert!(!reader.in_view("skip ahead"));
}
