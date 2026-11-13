//! Tabs: one per open document.

use super::docs::three_linked;
use super::support::Reader;

/// `a.md`, `b.md` and `c.md` open in three tabs, in that order, with
/// `c.md` showing.
fn three_tabs() -> Reader {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    reader.drop_file("c.md");
    reader
}

/// Which of the three tabs is the active one.
#[track_caller]
fn active_tab(reader: &Reader) -> &'static str {
    let active: Vec<_> = ["a.md", "b.md", "c.md"]
        .into_iter()
        .filter(|tab| reader.count(tab) == 1 && reader.is_on(tab))
        .collect();
    assert_eq!(active.len(), 1, "one tab is active: {active:?}");
    active[0]
}

#[test]
fn a_lone_document_has_no_tab_strip() {
    let reader = three_linked();

    assert_eq!(reader.count("a.md"), 0);
    reader.assert_sees("Alpha");
}

#[test]
fn ctrl_clicking_a_link_opens_it_in_a_new_tab() {
    let mut reader = three_linked();

    reader.ctrl_click("to beta");

    reader.assert_sees("Beta");
    assert_eq!(active_tab(&reader), "b.md");
    assert_eq!(reader.count("a.md"), 1, "the first document keeps its tab");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn clicking_a_tab_shows_its_document() {
    let mut reader = three_tabs();
    reader.assert_sees("Gamma");

    reader.click("a.md");

    reader.assert_sees("Alpha");
    reader.assert_hides("The end of the line.");
    assert_eq!(active_tab(&reader), "a.md");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn ctrl_tab_goes_to_the_next_tab_and_wraps() {
    let mut reader = three_tabs();

    reader.press("Ctrl+Tab");
    assert_eq!(active_tab(&reader), "a.md");

    reader.press("Ctrl+Tab");
    assert_eq!(active_tab(&reader), "b.md");
    reader.assert_sees("Beta");
}

#[test]
fn ctrl_shift_tab_goes_to_the_previous_tab_and_wraps() {
    let mut reader = three_tabs();

    reader.press("Ctrl+Shift+Tab");
    assert_eq!(active_tab(&reader), "b.md");

    reader.press("Ctrl+Shift+Tab");
    reader.press("Ctrl+Shift+Tab");
    assert_eq!(active_tab(&reader), "c.md");
}

#[test]
fn ctrl_and_a_digit_goes_to_that_tab() {
    let mut reader = three_tabs();

    reader.press("Ctrl+1");
    assert_eq!(active_tab(&reader), "a.md");

    reader.press("Ctrl+2");
    assert_eq!(active_tab(&reader), "b.md");

    reader.press("Ctrl+9");
    assert_eq!(active_tab(&reader), "b.md", "there is no ninth tab");
}

#[test]
fn ctrl_w_closes_the_tab_in_front() {
    let mut reader = three_tabs();

    reader.press("Ctrl+W");

    assert_eq!(reader.count("c.md"), 0);
    assert_eq!(active_tab(&reader), "b.md");
    reader.assert_sees("Beta");
}

#[test]
fn closing_the_last_tab_shows_the_home_page() {
    let mut reader = three_linked();

    reader.press("Ctrl+W");

    reader.assert_sees("RECENTLY READ");
    reader.assert_hides("a.md · 1 min read");
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn close_tab_in_the_menu_closes_the_tab_in_front() {
    let mut reader = three_tabs();

    reader.click("More");
    reader.click("Close Tab");

    assert_eq!(reader.count("c.md"), 0);
    assert_eq!(active_tab(&reader), "b.md");
}

#[test]
fn middle_clicking_a_tab_closes_it() {
    let mut reader = three_tabs();

    reader.middle_click("a.md");

    assert_eq!(reader.count("a.md"), 0);
    assert_eq!(active_tab(&reader), "c.md");
    reader.assert_sees("Gamma");
}

#[test]
fn a_tab_under_the_pointer_has_a_close_button() {
    let mut reader = three_tabs();
    assert_eq!(reader.count("Close tab"), 0);

    reader.hover("b.md");
    reader.click("Close tab");

    assert_eq!(reader.count("b.md"), 0);
    assert_eq!(reader.count("a.md"), 1);
    assert_eq!(reader.count("c.md"), 1);
}

#[test]
fn a_tabs_menu_closes_the_others() {
    let mut reader = three_tabs();

    reader.right_click("b.md");
    reader.click("Close Others");

    reader.assert_sees("Beta");
    assert_eq!(reader.count("a.md"), 0);
    assert_eq!(reader.count("c.md"), 0);
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn opening_a_document_that_has_a_tab_goes_to_that_tab() {
    let mut reader = three_tabs();

    reader.drop_file("a.md");

    assert_eq!(reader.count("a.md"), 1);
    assert_eq!(active_tab(&reader), "a.md");
}

#[test]
fn each_tab_keeps_its_own_place_in_history() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.drop_file("c.md");
    assert!(!reader.is_enabled("Go back"), "the new tab has no history");

    reader.click("b.md");

    assert!(reader.is_enabled("Go back"));
    reader.press("Alt+Left");
    reader.assert_sees("Alpha");
    assert_eq!(reader.count("c.md"), 1);
}
