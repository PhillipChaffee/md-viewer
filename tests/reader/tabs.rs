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

/// The name of the tab in front.
#[track_caller]
fn active_tab(reader: &Reader) -> String {
    reader.tab_in_front().expect("no tab is in front")
}

#[test]
fn a_lone_document_has_a_tab_and_the_new_tab_button_after_it() {
    let reader = three_linked();

    assert_eq!(reader.tabs(), ["Alpha"]);
    assert_eq!(active_tab(&reader), "Alpha");
    let (tab, new_tab) = (reader.area_of("tab:Alpha"), reader.area_of("New tab"));
    assert!(tab.left >= reader.area_of("Go forward").right(), "{tab:?}");
    assert!(new_tab.left >= tab.right(), "{new_tab:?} after {tab:?}");
    assert!(new_tab.left - tab.right() < 8.0, "right after it");
    assert!(new_tab.right() <= reader.area_of("Preview").left);
}

#[test]
fn the_top_row_reads_buttons_tabs_new_tab_switch_buttons() {
    let reader = three_tabs();

    let row = [
        "Files sidebar",
        "Go back",
        "Go forward",
        "tab:Alpha",
        "tab:Beta",
        "tab:Gamma",
        "New tab",
        "Preview",
        "Edit",
        "Appearance",
        "Find in document",
        "Contents sidebar",
        "More",
    ]
    .map(|control| reader.area_of(control));
    for pair in row.windows(2) {
        assert!(pair[0].right() <= pair[1].left, "{pair:?}");
        assert!(pair[0].bottom() <= 40.0 && pair[1].bottom() <= 40.0);
    }
}

#[test]
fn a_tab_is_named_by_the_first_heading_of_its_document_else_by_its_file() {
    let mut reader = Reader::with_docs(&[
        ("notes.md", "Intro.\n\n# Field Notes\n\nBody."),
        ("plain.md", "No heading here."),
    ]);
    reader.drop_file("plain.md");

    assert_eq!(reader.tabs(), ["Field Notes", "plain.md"]);
    assert_eq!(reader.title(), "plain.md - Markdown Viewer");
    reader.click("tab:Field Notes");
    assert_eq!(reader.title(), "notes.md - Markdown Viewer");
    reader.assert_sees("notes.md · 1 min read");
}

#[test]
fn a_tab_takes_the_new_heading_when_its_document_is_edited() {
    let mut reader = Reader::with_docs(&[("notes.md", "# Draft")]);
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.press("Ctrl+End");

    reader.type_text("ed twice");

    assert_eq!(reader.tabs(), ["Drafted twice"]);
}

#[test]
fn pointing_at_a_tab_shows_where_its_file_is() {
    let mut reader = three_tabs();
    let path = reader.path("a.md").display().to_string();
    reader.assert_hides(&path);

    reader.hover("tab:Alpha");

    reader.wait_for("the hover text", |reader| reader.sees(&path));
    reader.assert_sees("Live: reloads when the file changes");
}

#[test]
fn nothing_under_the_row_moves_when_a_second_document_opens() {
    let mut reader = three_linked();
    let (column, heading) = (reader.column(), reader.text_area("Alpha"));

    reader.drop_file("b.md");
    let moved = reader.text_area("Beta").top - heading.top;
    assert!(moved.abs() < 0.5, "the page moved {moved} px");

    reader.click("tab:Alpha");
    assert_eq!(reader.column(), column);
    assert_eq!(reader.text_area("Alpha"), heading);
}

#[test]
fn ctrl_clicking_a_link_opens_it_in_a_new_tab() {
    let mut reader = three_linked();

    reader.ctrl_click("to beta");

    reader.assert_sees("Beta");
    assert_eq!(active_tab(&reader), "Beta");
    assert_eq!(reader.tabs(), ["Alpha", "Beta"], "the first keeps its tab");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn clicking_a_tab_shows_its_document() {
    let mut reader = three_tabs();
    reader.assert_sees("Gamma");

    reader.click("tab:Alpha");

    reader.assert_sees("Alpha");
    reader.assert_hides("The end of the line.");
    assert_eq!(active_tab(&reader), "Alpha");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn ctrl_tab_goes_to_the_next_tab_and_wraps() {
    let mut reader = three_tabs();

    reader.press("Ctrl+Tab");
    assert_eq!(active_tab(&reader), "Alpha");

    reader.press("Ctrl+Tab");
    assert_eq!(active_tab(&reader), "Beta");
    reader.assert_sees("Beta");
}

#[test]
fn ctrl_shift_tab_goes_to_the_previous_tab_and_wraps() {
    let mut reader = three_tabs();

    reader.press("Ctrl+Shift+Tab");
    assert_eq!(active_tab(&reader), "Beta");

    reader.press("Ctrl+Shift+Tab");
    reader.press("Ctrl+Shift+Tab");
    assert_eq!(active_tab(&reader), "Gamma");
}

#[test]
fn ctrl_and_a_digit_goes_to_that_tab() {
    let mut reader = three_tabs();

    reader.press("Ctrl+1");
    assert_eq!(active_tab(&reader), "Alpha");

    reader.press("Ctrl+2");
    assert_eq!(active_tab(&reader), "Beta");

    reader.press("Ctrl+9");
    assert_eq!(active_tab(&reader), "Beta", "there is no ninth tab");
}

#[test]
fn ctrl_w_closes_the_tab_in_front() {
    let mut reader = three_tabs();

    reader.press("Ctrl+W");

    assert_eq!(reader.count("tab:Gamma"), 0);
    assert_eq!(active_tab(&reader), "Beta");
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

    assert_eq!(reader.count("tab:Gamma"), 0);
    assert_eq!(active_tab(&reader), "Beta");
}

#[test]
fn middle_clicking_a_tab_closes_it() {
    let mut reader = three_tabs();

    reader.middle_click("tab:Alpha");

    assert_eq!(reader.count("tab:Alpha"), 0);
    assert_eq!(active_tab(&reader), "Gamma");
    reader.assert_sees("Gamma");
}

#[test]
fn a_tab_under_the_pointer_has_a_close_button() {
    let mut reader = three_tabs();

    reader.hover("tab:Beta");
    reader.click("Close Beta");

    assert_eq!(reader.count("tab:Beta"), 0);
    assert_eq!(reader.count("tab:Alpha"), 1);
    assert_eq!(reader.count("tab:Gamma"), 1);
}

#[test]
fn a_tabs_menu_closes_the_others() {
    let mut reader = three_tabs();

    reader.right_click("tab:Beta");
    reader.click("Close Others");

    reader.assert_sees("Beta");
    assert_eq!(reader.count("tab:Alpha"), 0);
    assert_eq!(reader.count("tab:Gamma"), 0);
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn opening_a_document_that_has_a_tab_goes_to_that_tab() {
    let mut reader = three_tabs();

    reader.drop_file("a.md");

    assert_eq!(reader.count("tab:Alpha"), 1);
    assert_eq!(active_tab(&reader), "Alpha");
}

#[test]
fn each_tab_keeps_its_own_place_in_history() {
    let mut reader = three_linked();
    reader.click("to beta");
    reader.drop_file("c.md");
    assert!(!reader.is_enabled("Go back"), "the new tab has no history");

    reader.click("tab:Beta");

    assert!(reader.is_enabled("Go back"));
    reader.press("Alt+Left");
    reader.assert_sees("Alpha");
    assert_eq!(reader.count("tab:Gamma"), 1);
}
