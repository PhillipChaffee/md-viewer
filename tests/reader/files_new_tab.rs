//! Opening a document from the files sidebar in a tab of its own: the
//! button on its row, the rows of its menu, a middle click, and the dot
//! that marks the documents that have a tab.

use super::support::{Area, Reader, Rgb};

/// What the button at the end of a document's row is called.
const BUTTON: &str = "Open in new tab";
const LONG_NAME: &str = "a-document-whose-name-is-far-too-long-to-fit-in-the-width-of-a-sidebar.md";

/// A folder of documents, open on `readme.md` with the files sidebar
/// showing.
fn desk() -> Reader {
    let mut reader = Reader::launch()
        .doc("readme.md", "# Read me\n\nStart here.")
        .doc("changes.md", "# Changes\n\nWhat is new.")
        .doc("notes.md", "# Notes\n\nKept short.")
        .doc(LONG_NAME, "# Long\n\nNamed at length.")
        .doc("guides/setup.md", "# Setup\n\nHow to begin.")
        .args(&["readme.md"])
        .open();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });
    reader
}

/// Take the pointer off the sidebar.
fn point_away(reader: &mut Reader) {
    reader.hover("More");
}

/// The dots on the row of `name`.
fn dots(reader: &Reader, name: &str) -> Vec<Area> {
    let dots = reader.dots_in(reader.area_of(name)).into_iter();
    dots.map(|(dot, _)| dot).collect()
}

/// The color of the one dot on the row of `name`.
fn dot_ink(reader: &Reader, name: &str) -> Rgb {
    let dots = reader.dots_in(reader.area_of(name));
    assert_eq!(dots.len(), 1, "{name} has {} dots", dots.len());
    dots[0].1
}

/// The documents whose rows carry a dot.
fn dotted(reader: &Reader) -> Vec<&'static str> {
    let mut rows = vec!["changes.md", "notes.md", "readme.md"];
    rows.retain(|name| !dots(reader, name).is_empty());
    rows
}

#[test]
fn a_documents_menu_starts_with_the_two_ways_to_open_it() {
    let mut reader = desk();

    reader.right_click("changes.md");

    let tops = [
        "Open",
        "Open in New Tab",
        "Copy Contents",
        "Copy Path",
        "Copy File URI",
    ]
    .map(|row| reader.area_of(row).top);
    assert!(tops.is_sorted(), "{tops:?}");
    // A rule parts the rows that open from the rows that copy.
    let new_tab = reader.area_of("Open in New Tab");
    let between = Area {
        left: new_tab.left - 1.0,
        top: new_tab.bottom(),
        width: new_tab.width + 2.0,
        height: reader.area_of("Copy Contents").top - new_tab.bottom(),
    };
    assert_eq!(reader.marks_in(between).len(), 1, "one rule");
}

#[test]
fn open_in_the_menu_shows_the_document_in_the_tab_in_front() {
    let mut reader = desk();

    reader.right_click("changes.md");
    reader.click("Open");

    assert_eq!(reader.tabs(), ["Changes"]);
    reader.assert_sees("What is new.");
    assert!(reader.is_enabled("Go back"), "it opened in place");
    assert_eq!(reader.count("Open in New Tab"), 0, "the menu closed");
}

#[test]
fn open_in_new_tab_in_the_menu_gives_the_document_a_tab_of_its_own() {
    let mut reader = desk();

    reader.right_click("changes.md");
    reader.click("Open in New Tab");

    assert_eq!(reader.tabs(), ["Read me", "Changes"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Changes"));
    reader.assert_sees("What is new.");
    assert_eq!(reader.count("Open in New Tab"), 0, "the menu closed");
}

#[test]
fn open_in_new_tab_in_the_menu_shows_the_tab_a_document_already_has() {
    let mut reader = desk();
    reader.ctrl_click("changes.md");
    reader.click("tab:Read me");

    reader.right_click("changes.md");
    reader.click("Open in New Tab");

    assert_eq!(reader.tabs(), ["Read me", "Changes"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Changes"));
}

#[test]
fn the_menu_names_the_click_that_opens_a_new_tab() {
    let mut reader = desk();
    reader.assert_hides("Ctrl-click");

    reader.right_click("changes.md");

    reader.assert_sees("Ctrl-click");
}

#[test]
fn the_named_click_does_open_a_new_tab() {
    let mut reader = desk();

    reader.ctrl_click("changes.md");

    assert_eq!(reader.tabs(), ["Read me", "Changes"]);
}

#[test]
fn the_button_is_on_a_documents_row_only_while_the_pointer_is() {
    let mut reader = desk();
    assert_eq!(reader.count(BUTTON), 0);

    reader.hover("changes.md");
    assert_eq!(reader.count(BUTTON), 1);
    let (row, button) = (reader.area_of("changes.md"), reader.area_of(BUTTON));
    assert!(button.top >= row.top && button.bottom() <= row.bottom());
    assert!(button.left > row.left + row.width / 2.0, "at the right end");
    assert!(button.right() <= row.right());

    reader.hover("notes.md");
    assert_eq!(reader.count(BUTTON), 1, "one row is pointed at");
    assert!(reader.area_of(BUTTON).top >= reader.area_of("notes.md").top);

    point_away(&mut reader);
    assert_eq!(reader.count(BUTTON), 0);
}

#[test]
fn the_button_opens_a_new_tab_and_leaves_the_tab_in_front_as_it_was() {
    let mut reader = desk();

    reader.hover("changes.md");
    reader.click(BUTTON);

    assert_eq!(reader.tabs(), ["Read me", "Changes"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Changes"));
    reader.assert_sees("What is new.");
    reader.click("tab:Read me");
    reader.assert_sees("Start here.");
    assert!(!reader.is_enabled("Go back"), "nothing was opened in it");
}

#[test]
fn the_button_shows_the_tab_a_document_already_has() {
    let mut reader = desk();
    reader.ctrl_click("changes.md");
    reader.click("tab:Read me");

    reader.hover("changes.md");
    reader.click(BUTTON);

    assert_eq!(reader.tabs(), ["Read me", "Changes"], "no second tab");
    assert_eq!(reader.tab_in_front().as_deref(), Some("Changes"));
}

#[test]
fn clicking_the_name_beside_the_button_still_opens_in_place() {
    let mut reader = desk();

    reader.hover("changes.md");
    assert_eq!(reader.count(BUTTON), 1);
    reader.click("changes.md");

    assert_eq!(reader.tabs(), ["Changes"]);
    assert!(reader.is_enabled("Go back"));
}

#[test]
fn the_button_says_what_it_does_and_which_click_does_the_same() {
    let mut reader = desk();
    reader.hover("changes.md");

    reader.hover(BUTTON);
    reader.wait_for("the button's tooltip", |reader| {
        reader.sees("Open in new tab (")
    });

    reader.assert_sees("Open in new tab (Ctrl-click)");
}

#[test]
fn a_middle_click_on_a_document_opens_it_in_a_new_tab() {
    let mut reader = desk();

    reader.middle_click("changes.md");

    assert_eq!(reader.tabs(), ["Read me", "Changes"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Changes"));
    reader.assert_sees("What is new.");
}

#[test]
fn a_middle_click_on_a_document_with_a_tab_shows_that_tab_and_closes_none() {
    let mut reader = desk();
    reader.ctrl_click("changes.md");

    reader.middle_click("readme.md");

    assert_eq!(reader.tabs(), ["Read me", "Changes"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Read me"));
    reader.assert_sees("Start here.");
}

#[test]
fn a_dot_after_the_name_marks_each_document_that_has_a_tab() {
    let mut reader = desk();
    point_away(&mut reader);
    assert_eq!(dotted(&reader), ["readme.md"]);
    let dot = dots(&reader, "readme.md")[0];
    assert!(dot.left >= reader.end_of("readme.md"), "after the name");
    assert!((dot.width - dot.height).abs() < 0.5 && dot.width < 8.0);

    reader.ctrl_click("changes.md");
    point_away(&mut reader);
    assert_eq!(dotted(&reader), ["changes.md", "readme.md"]);

    reader.middle_click("tab:Read me");
    point_away(&mut reader);
    assert_eq!(dotted(&reader), ["changes.md"]);

    reader.click("notes.md");
    point_away(&mut reader);
    assert_eq!(dotted(&reader), ["notes.md"], "opened in place");
}

#[test]
fn the_dot_stands_out_from_the_sidebar_and_the_names_in_every_theme() {
    let mut reader = desk();
    let mut seen = Vec::new();

    for _ in 0..4 {
        point_away(&mut reader);
        let ink = dot_ink(&reader, "readme.md");
        let row = reader.area_of("readme.md");
        let surface = reader.color_at(row.left + 2.0, row.top + 2.0);
        assert!(ink.contrast(surface) > 2.0, "{ink:?} on {surface:?}");
        for name in ["readme.md", "notes.md"] {
            assert_ne!(reader.ink_of_each(name).last(), Some(&ink), "{name}");
        }
        seen.push(ink);
        reader.press("Ctrl+D");
    }

    seen.dedup();
    assert!(seen.len() > 1, "the dot follows the theme: {seen:?}");
}

#[test]
fn the_button_takes_the_dots_place_while_the_row_is_pointed_at() {
    let mut reader = desk();
    point_away(&mut reader);
    let dot = dots(&reader, "readme.md")[0];

    reader.hover("readme.md");

    assert!(dots(&reader, "readme.md").is_empty(), "the dot gives way");
    let button = reader.area_of(BUTTON);
    assert!(button.left <= dot.left && button.right() >= dot.right());
    assert!(button.top <= dot.top && button.bottom() >= dot.bottom());
}

#[test]
fn a_long_name_is_cut_short_of_the_button_and_of_the_dot() {
    let mut reader = desk();
    point_away(&mut reader);
    let row = reader.area_of(LONG_NAME);
    let at_rest = reader.end_of(LONG_NAME);
    assert!(at_rest <= row.right(), "cut to the row: {at_rest}");

    reader.hover(LONG_NAME);
    let button = reader.area_of(BUTTON);
    assert!(at_rest > button.left, "long enough to reach the button");
    let pointed = reader.end_of(LONG_NAME);
    assert!(pointed <= button.left, "{pointed} runs under {button:?}");

    reader.click(BUTTON);
    point_away(&mut reader);
    let dot = dots(&reader, LONG_NAME)[0];
    let marked = reader.end_of(LONG_NAME);
    assert!(marked <= dot.left, "{marked} runs under {dot:?}");
}

#[test]
fn a_folder_has_no_button_no_dot_and_no_rows_to_open_it() {
    let mut reader = desk();
    reader.click("guides");
    reader.ctrl_click("setup.md");

    reader.hover("guides");
    assert_eq!(reader.count(BUTTON), 0);
    assert!(dots(&reader, "guides").is_empty());

    reader.right_click("guides");
    assert_eq!(reader.count("Open in New Tab"), 0);
    assert_eq!(reader.count("Open"), 0);
    reader.hover("notes.md");
    assert!(dots(&reader, "guides").is_empty(), "nor at rest");
}
