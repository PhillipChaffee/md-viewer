//! The home page shown over open tabs: how to get there, what the window
//! is like meanwhile, and every way back.

use super::docs::three_linked;
use super::support::Reader;

/// `a.md`, `b.md` and `c.md` in three tabs, headed `Alpha`, `Beta` and
/// `Gamma`, with `b.md` in front.
fn three_tabs() -> Reader {
    let mut reader = three_linked();
    reader.drop_files(&["b.md", "c.md"]);
    reader.click("tab:Beta");
    reader
}

/// The three tabs with the home page showing over them.
fn at_home() -> Reader {
    let mut reader = three_tabs();
    reader.click("Home");
    reader
}

/// Fail unless the home page shows, over three tabs none of which is in
/// front.
#[track_caller]
fn assert_at_home(reader: &Reader) {
    reader.assert_sees("RECENTLY READ");
    reader.assert_hides("Go [to alpha]");
    assert_eq!(reader.tabs(), ["Alpha", "Beta", "Gamma"]);
    assert_eq!(reader.tab_in_front(), None);
    assert_eq!(reader.title(), "Markdown Viewer");
}

/// Fail unless `b.md` is back in front.
#[track_caller]
fn assert_back_on_beta(reader: &Reader, way: &str) {
    assert_eq!(reader.tab_in_front().as_deref(), Some("Beta"), "{way}");
    assert_eq!(reader.title(), "b.md - Markdown Viewer", "{way}");
    assert!(!reader.sees("RECENTLY READ"), "{way}");
    assert_eq!(reader.tabs(), ["Alpha", "Beta", "Gamma"], "{way}");
}

#[test]
fn the_home_button_shows_the_home_page_and_keeps_the_tabs() {
    let reader = at_home();

    assert_at_home(&reader);
    let (home, back) = (reader.area_of("Home"), reader.area_of("Go back"));
    assert!(reader.area_of("Files sidebar").right() <= home.left);
    assert!(home.right() <= back.left, "{home:?} then {back:?}");
}

#[test]
fn the_home_page_over_tabs_has_no_document_controls() {
    let mut reader = at_home();

    reader.assert_hides("CONTENTS");
    assert_eq!(reader.count("Preview"), 0, "no edit switch");
    assert!(reader.is_enabled("Go back"), "back leads to the document");
    assert!(!reader.is_enabled("Go forward"));
    reader.click("More");
    for row in ["Close Tab", "Edit Document", "Forward"] {
        assert!(!reader.is_enabled(row), "{row}");
    }
    assert!(reader.is_enabled("Back"));
    assert!(
        reader.is_on("Home Page"),
        "the menu ticks where the reader is"
    );
}

#[test]
fn every_way_back_leads_to_the_tab_that_was_in_front() {
    type Way = fn(&mut Reader);
    let ways: [(&str, Way); 8] = [
        ("the home button again", |reader| reader.click("Home")),
        ("the home shortcut again", |reader| {
            reader.press("Ctrl+Shift+H");
        }),
        ("the back button", |reader| reader.click("Go back")),
        ("Alt+Left", |reader| reader.press("Alt+Left")),
        ("its tab", |reader| reader.click("tab:Beta")),
        ("Ctrl+Tab", |reader| reader.press("Ctrl+Tab")),
        ("Ctrl+Shift+Tab", |reader| reader.press("Ctrl+Shift+Tab")),
        ("the menu", |reader| {
            reader.click("More");
            reader.click("Home Page");
        }),
    ];
    for (way, go) in ways {
        let mut reader = at_home();

        go(&mut reader);

        assert_back_on_beta(&reader, way);
        reader.assert_sees("on to gamma");
    }
}

#[test]
fn back_from_the_home_page_does_not_step_the_tabs_history() {
    let mut reader = three_tabs();
    reader.click("to alpha");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
    reader.click("Home");

    reader.press("Alt+Left");
    assert_eq!(
        reader.title(),
        "a.md - Markdown Viewer",
        "off the home page"
    );

    reader.press("Alt+Left");
    assert_eq!(
        reader.title(),
        "b.md - Markdown Viewer",
        "then one step back"
    );
}

#[test]
fn another_tab_or_its_digit_takes_the_place_of_the_home_page() {
    let mut reader = at_home();
    reader.click("tab:Gamma");
    assert_eq!(reader.tab_in_front().as_deref(), Some("Gamma"));
    reader.assert_sees("The end of the line.");

    reader.click("Home");
    reader.press("Ctrl+1");
    assert_eq!(reader.tab_in_front().as_deref(), Some("Alpha"));
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn the_new_tab_button_and_ctrl_t_show_the_home_page_and_stay_there() {
    for way in ["the button", "Ctrl+T"] {
        let mut reader = three_tabs();
        let go = |reader: &mut Reader| match way {
            "the button" => reader.click("New tab"),
            _ => reader.press("Ctrl+T"),
        };

        go(&mut reader);
        assert_at_home(&reader);

        go(&mut reader);
        assert_at_home(&reader);
    }
}

#[test]
fn ctrl_t_types_nothing_into_the_source_or_the_find_bar() {
    let mut reader = three_tabs();
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.press("Ctrl+T");
    assert_at_home(&reader);
    reader.press("Ctrl+Shift+H");
    assert_eq!(reader.typed(), [reader.read_file("b.md")]);

    reader.press("Ctrl+F");
    reader.type_text("go");
    reader.press("Ctrl+Shift+H");
    assert_at_home(&reader);
    assert_eq!(reader.typed(), ["go"]);
}

#[test]
fn a_recent_document_with_a_tab_goes_to_that_tab() {
    let mut reader = at_home();

    reader.click("Alpha");

    assert_eq!(reader.tabs(), ["Alpha", "Beta", "Gamma"], "no second tab");
    assert_eq!(reader.tab_in_front().as_deref(), Some("Alpha"));
    reader.assert_sees("to gamma");
}

#[test]
fn a_recent_document_without_a_tab_opens_in_a_new_one() {
    let mut reader = three_tabs();
    reader.middle_click("tab:Gamma");
    assert_eq!(reader.tabs(), ["Alpha", "Beta"]);
    reader.click("New tab");

    reader.click("Gamma");

    assert_eq!(reader.tabs(), ["Alpha", "Beta", "Gamma"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Gamma"));
    reader.assert_sees("The end of the line.");
    reader.click("tab:Beta");
    reader.assert_sees("on to gamma");
}

#[test]
fn a_file_picked_in_the_sidebar_from_the_home_page_gets_a_tab_of_its_own() {
    let mut reader = three_linked();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("c.md") == 1
    });
    reader.click("Home");

    reader.click("c.md");

    assert_eq!(reader.tabs(), ["Alpha", "Gamma"], "a.md keeps its tab");
    assert_eq!(reader.tab_in_front().as_deref(), Some("Gamma"));
}

#[test]
fn a_file_dropped_on_the_home_page_opens_in_front() {
    let mut reader = three_linked();
    reader.click("Home");

    reader.drop_file("b.md");

    assert_eq!(reader.tabs(), ["Alpha", "Beta"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Beta"));
}

#[test]
fn closing_from_the_home_page_closes_no_tab() {
    let mut reader = at_home();

    reader.press("Ctrl+W");

    assert_at_home(&reader);
    assert!(!reader.has_quit());
}

#[test]
fn a_tab_can_be_closed_from_under_the_home_page_which_stays() {
    let mut reader = at_home();

    reader.middle_click("tab:Beta");
    reader.assert_sees("RECENTLY READ");
    assert_eq!(reader.tabs(), ["Alpha", "Gamma"]);
    assert_eq!(reader.tab_in_front(), None);

    reader.middle_click("tab:Alpha");
    reader.middle_click("tab:Gamma");
    assert!(reader.tabs().is_empty());
    reader.assert_sees("RECENTLY READ");
    assert!(!reader.is_enabled("Go back"), "nothing to go back to");
}

#[test]
fn unsaved_edits_wait_under_the_home_page_and_ctrl_s_saves_nothing_there() {
    let mut reader = three_tabs();
    let written = reader.read_file("b.md");
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.press("Ctrl+End");
    reader.type_text(" More.");
    reader.click("Home");

    reader.press("Ctrl+S");
    reader.press("Ctrl+E");

    assert_at_home(&reader);
    assert_eq!(reader.read_file("b.md"), written);
    reader.click("Home");
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.typed(), [format!("{written} More.")]);
}

#[test]
fn the_find_bar_finds_nothing_on_the_home_page_and_counts_again_on_the_way_back() {
    let mut reader = three_tabs();
    reader.press("Ctrl+F");
    reader.type_text("gamma");
    reader.assert_sees("1 / 1");

    reader.click("Home");
    reader.assert_sees("0 matches");
    assert_eq!(reader.typed(), ["gamma"]);

    reader.click("Home");
    reader.assert_sees("1 / 1");
}

#[test]
fn a_tab_under_the_home_page_still_follows_its_file() {
    let mut reader = at_home();

    reader.write_file("b.md", "# Beta\n\nRewritten meanwhile.");
    reader.click("Home");

    reader.wait_for("the new text", |reader| reader.sees("Rewritten meanwhile."));
}

#[test]
fn zoom_and_the_theme_work_on_the_home_page_over_tabs() {
    let mut reader = at_home();
    let (paper, button) = (reader.page_color(), reader.area_of("More").width);

    reader.press("Ctrl+D");
    reader.press("Ctrl++");

    assert_ne!(reader.page_color(), paper);
    assert!(reader.area_of("More").width > button);
    assert_at_home(&reader);
}

#[test]
fn a_restart_shows_the_document_that_was_in_front_not_the_home_page() {
    let reader = at_home();

    let reader = reader.restart(&[]);

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    assert_eq!(reader.tab_in_front().as_deref(), Some("Beta"));
    assert_eq!(reader.tabs(), ["Alpha", "Beta", "Gamma"]);
}

#[test]
fn with_no_tab_open_the_home_button_changes_nothing() {
    let mut reader = three_linked();
    reader.press("Ctrl+W");
    let before = reader.names();

    reader.click("Home");
    reader.press("Ctrl+Shift+H");
    reader.press("Ctrl+T");

    assert_eq!(reader.names(), before);
    assert!(!reader.is_enabled("Go back"));
}
