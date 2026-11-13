//! Tabs, in depth: many of them, two with one name, every way to close
//! one, and what each tab keeps for itself.

use super::docs::{long_document, three_linked};
use super::support::Reader;

/// `count` documents named `doc-01.md` and up, titled `Doc 1` and up, each
/// in a tab, the last one in front.
fn many_tabs(count: usize) -> Reader {
    let mut launch = Reader::launch();
    let names: Vec<String> = (1..=count).map(|n| format!("doc-{n:02}.md")).collect();
    for (number, name) in (1..).zip(&names) {
        launch = launch.doc(name, &format!("# Doc {number}\n\nBody of {number}."));
    }
    let mut reader = launch.args(&["doc-01.md"]).open();
    let rest: Vec<&str> = names.iter().skip(1).map(String::as_str).collect();
    reader.drop_files(&rest);
    reader
}

/// `a.md`, `b.md` and `c.md` in three tabs with `c.md` showing.
fn three_tabs() -> Reader {
    let mut reader = three_linked();
    reader.drop_files(&["b.md", "c.md"]);
    reader
}

/// The tabs that are marked as the one in front, by name.
fn tabs_in_front(reader: &Reader, tabs: &[&str]) -> Vec<String> {
    let front = tabs
        .iter()
        .filter(|tab| reader.count(tab) == 1 && reader.is_on(tab));
    front.map(|tab| (*tab).to_owned()).collect()
}

/// Two documents called `same.md` in two folders, each in a tab, the one
/// from `two/` in front.
fn two_of_one_name() -> Reader {
    let mut reader = Reader::with_docs(&[
        ("one/same.md", "# From one\n\nFirst folder."),
        ("two/same.md", "# From two\n\nSecond folder."),
    ]);
    reader.drop_file("two/same.md");
    reader
}

#[test]
fn files_dropped_together_each_get_a_tab_in_the_order_dropped() {
    let reader = many_tabs(5);

    let names = reader.names();
    let is_tab = |name: &&String| name.starts_with("doc-") && !name.contains(char::is_whitespace);
    let tabs: Vec<&String> = names.iter().filter(is_tab).collect();
    assert_eq!(
        tabs,
        [
            "doc-01.md",
            "doc-02.md",
            "doc-03.md",
            "doc-04.md",
            "doc-05.md"
        ]
    );
    assert!(
        reader.is_on("doc-05.md"),
        "the last one dropped is in front"
    );
    reader.assert_sees("Body of 5.");
}

#[test]
fn ctrl_tab_visits_every_one_of_many_tabs_in_order() {
    let mut reader = many_tabs(12);

    for number in 1..=12 {
        reader.press("Ctrl+Tab");

        reader.assert_sees(&format!("Body of {number}."));
        assert_eq!(
            reader.title(),
            format!("doc-{number:02}.md - Markdown Viewer")
        );
    }
}

#[test]
fn ctrl_shift_tab_visits_them_in_reverse() {
    let mut reader = many_tabs(12);

    for number in (1..=11).rev() {
        reader.press("Ctrl+Shift+Tab");

        reader.assert_sees(&format!("Body of {number}."));
    }
}

#[test]
fn with_more_tabs_than_fit_the_strips_buttons_stay_in_the_window() {
    let reader = many_tabs(30);

    for control in ["Open file in new tab", "Preview", "Edit"] {
        let area = reader.area_of(control);
        assert!(
            area.left >= 0.0 && area.right() <= 1200.0,
            "{control}: {area:?}"
        );
    }
}

/// Known bug, listed in `tests/README.md`: with more tabs than fit, the
/// button that opens a file is drawn over the Preview and Edit switch.
/// This test states the right behavior and is expected to fail until the
/// app has it.
#[test]
#[should_panic(expected = "the open button overlaps the edit switch")]
fn with_more_tabs_than_fit_the_open_button_keeps_clear_of_the_edit_switch() {
    let reader = many_tabs(30);

    let open = reader.area_of("Open file in new tab");
    let switch = reader.area_of("Preview");
    assert!(
        open.right() <= switch.left,
        "the open button overlaps the edit switch: {open:?} and {switch:?}"
    );
}

/// Known bug, listed in `tests/README.md`: with more tabs than fit, the
/// strip does not scroll to the tab in front, so a document just opened
/// has no tab on screen. This test states the right behavior and is
/// expected to fail until the app has it.
#[test]
#[should_panic(expected = "the tab in front is not on screen")]
fn the_tab_in_front_is_scrolled_into_the_strip() {
    let reader = many_tabs(30);

    let front = reader.area_of("doc-30.md");
    let strip_end = reader.area_of("Open file in new tab").left;
    assert!(
        front.left >= 0.0 && front.right() <= strip_end,
        "the tab in front is not on screen"
    );
}

#[test]
fn ctrl_and_a_digit_reaches_the_first_nine_of_many() {
    let mut reader = many_tabs(11);

    for digit in [9, 1, 5] {
        reader.press(&format!("Ctrl+{digit}"));

        reader.assert_sees(&format!("Body of {digit}."));
    }
}

#[test]
fn two_documents_with_one_name_get_a_tab_each() {
    let mut reader = two_of_one_name();
    assert_eq!(reader.count("same.md"), 2);
    reader.assert_sees("Second folder.");

    reader.click_nth("same.md", 0);

    reader.assert_sees("First folder.");
    reader.assert_hides("Second folder.");
    assert_eq!(reader.title(), "same.md - Markdown Viewer");
}

#[test]
fn closing_one_of_two_tabs_with_one_name_leaves_the_other() {
    let mut reader = two_of_one_name();

    reader.middle_click_nth("same.md", 0);

    reader.assert_sees("Second folder.");
    reader.assert_hides("First folder.");
    assert_eq!(reader.count("same.md"), 0, "one document: no tab strip");
}

#[test]
fn a_tabs_menu_closes_that_tab_and_keeps_the_one_in_front() {
    let mut reader = three_tabs();

    reader.right_click("a.md");
    reader.click("Close");

    assert_eq!(reader.count("a.md"), 0);
    assert_eq!(tabs_in_front(&reader, &["b.md", "c.md"]), ["c.md"]);
    reader.assert_sees("The end of the line.");
}

#[test]
fn a_tabs_menu_goes_away_on_escape_and_closes_nothing() {
    let mut reader = three_tabs();
    reader.right_click("b.md");
    assert_eq!(reader.count("Close Others"), 1);

    reader.press("Escape");

    assert_eq!(reader.count("Close Others"), 0);
    for tab in ["a.md", "b.md", "c.md"] {
        assert_eq!(reader.count(tab), 1, "{tab}");
    }
}

#[test]
fn close_others_on_a_tab_behind_brings_it_forward_alone() {
    let mut reader = three_tabs();

    reader.right_click("a.md");
    reader.click("Close Others");

    reader.assert_sees("Alpha");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
    for tab in ["a.md", "b.md", "c.md"] {
        assert_eq!(reader.count(tab), 0, "{tab}: one document has no strip");
    }
}

#[test]
fn closing_a_tab_behind_the_one_in_front_changes_nothing_on_the_page() {
    let mut reader = three_tabs();
    reader.click("b.md");

    reader.middle_click("c.md");

    assert_eq!(tabs_in_front(&reader, &["a.md", "b.md"]), ["b.md"]);
    reader.assert_sees("Beta");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn closing_the_middle_tab_in_front_shows_a_neighbor() {
    let mut reader = three_tabs();
    reader.click("b.md");

    reader.hover("b.md");
    reader.click("Close tab");

    assert_eq!(reader.count("b.md"), 0);
    let front = tabs_in_front(&reader, &["a.md", "c.md"]);
    assert_eq!(front.len(), 1, "{front:?}");
    assert_eq!(reader.title(), format!("{} - Markdown Viewer", front[0]));
    reader.assert_hides("Beta");
}

#[test]
fn ctrl_w_closes_tab_after_tab_down_to_the_home_page() {
    let mut reader = three_tabs();

    for title in [
        "b.md - Markdown Viewer",
        "a.md - Markdown Viewer",
        "Markdown Viewer",
    ] {
        reader.press("Ctrl+W");

        assert_eq!(reader.title(), title);
    }
    reader.assert_sees("RECENTLY READ");
    reader.press("Ctrl+W");
    assert!(!reader.has_quit(), "closing nothing does not quit");
}

#[test]
fn the_tab_keys_do_nothing_with_one_document_or_none() {
    let mut reader = three_linked();

    for key in ["Ctrl+Tab", "Ctrl+Shift+Tab", "Ctrl+2"] {
        reader.press(key);
        assert_eq!(reader.title(), "a.md - Markdown Viewer", "{key}");
    }
    reader.press("Ctrl+W");
    for key in ["Ctrl+Tab", "Ctrl+Shift+Tab", "Ctrl+1"] {
        reader.press(key);
        assert_eq!(reader.title(), "Markdown Viewer", "{key}");
    }
}

#[test]
fn each_tab_keeps_its_own_place_on_the_page() {
    let mut reader = Reader::with_docs(&[
        ("long.md", &long_document(80)),
        ("other.md", &long_document(40)),
    ]);
    reader.drop_file("other.md");
    reader.click("long.md");
    reader.scroll_down(3000.0);
    let place = reader.text_area("Section 30");

    reader.click("other.md");
    assert!(reader.in_view("Section 1"), "the other tab is at its start");
    reader.scroll_down(1000.0);
    reader.click("long.md");

    assert_eq!(reader.text_area("Section 30"), place);
    reader.click("other.md");
    assert!(!reader.in_view("Section 1"), "and it kept its own place");
}

#[test]
fn a_tab_is_renamed_when_a_link_is_followed_in_it() {
    let mut reader = three_linked();
    reader.drop_file("c.md");
    reader.click("a.md");

    reader.click("to beta");

    assert_eq!(reader.count("a.md"), 0);
    assert_eq!(tabs_in_front(&reader, &["b.md", "c.md"]), ["b.md"]);
    reader.press("Alt+Left");
    assert_eq!(tabs_in_front(&reader, &["a.md", "c.md"]), ["a.md"]);
}

#[test]
fn ctrl_clicking_a_link_to_an_open_document_goes_to_its_tab() {
    let mut reader = three_tabs();
    reader.click("a.md");

    reader.ctrl_click("to gamma");

    assert_eq!(reader.count("c.md"), 1, "no second tab for it");
    assert_eq!(tabs_in_front(&reader, &["a.md", "b.md", "c.md"]), ["c.md"]);
}

#[test]
fn a_new_tab_goes_to_the_end_of_the_strip() {
    let mut reader = three_linked();
    reader.drop_file("c.md");
    reader.click("a.md");

    reader.ctrl_click("to beta");

    let (a, c, b) = (
        reader.area_of("a.md").left,
        reader.area_of("c.md").left,
        reader.area_of("b.md").left,
    );
    assert!(a < c && c < b, "a at {a}, c at {c}, b at {b}");
}

#[test]
fn a_ctrl_clicked_link_to_a_heading_opens_the_new_tab_there() {
    let mut reader = Reader::with_docs(&[
        ("index.md", "# Index\n\nSee [far on](long.md#section-60)."),
        ("long.md", &long_document(80)),
    ]);

    reader.ctrl_click("far on");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Section 60")
    });

    assert_eq!(
        tabs_in_front(&reader, &["index.md", "long.md"]),
        ["long.md"]
    );
    assert!(!reader.is_enabled("Go back"), "a new tab has no history");
}

#[test]
fn ctrl_clicking_a_link_to_a_missing_document_opens_no_tab() {
    let mut reader = three_linked();

    reader.ctrl_click("nowhere");

    reader.assert_sees("Unable to open");
    reader.assert_sees("Alpha");
    assert_eq!(reader.count("a.md"), 0, "still one document, no strip");
}

#[test]
fn each_tab_is_in_edit_mode_or_not_on_its_own() {
    let mut reader = three_tabs();
    reader.press("Ctrl+E");
    assert!(reader.is_on("Edit"));

    reader.click("a.md");
    assert!(reader.is_on("Preview"));
    assert!(reader.typed().is_empty());

    reader.click("c.md");
    assert!(reader.is_on("Edit"));
    assert_eq!(reader.typed(), ["# Gamma\n\nThe end of the line."]);
}

#[test]
fn a_document_opened_again_after_its_tab_closed_has_no_history() {
    let mut reader = Reader::with_docs(&[
        (
            "long.md",
            &format!("[away](other.md)\n\n{}", long_document(60)),
        ),
        ("other.md", "# Other\n\nElsewhere."),
    ]);
    reader.click("away");
    reader.drop_file("long.md");
    reader.press("Ctrl+W");
    reader.assert_sees("Elsewhere.");

    reader.drop_file("long.md");

    reader.assert_sees("Long read");
    assert!(!reader.is_enabled("Go back"));
    assert!(!reader.is_enabled("Go forward"));
}

#[test]
fn the_window_title_follows_the_tab_in_front_however_it_got_there() {
    let mut reader = three_tabs();

    for (step, title) in [
        ("Ctrl+1", "a.md - Markdown Viewer"),
        ("Ctrl+Tab", "b.md - Markdown Viewer"),
        ("Ctrl+W", "c.md - Markdown Viewer"),
        ("Ctrl+Shift+Tab", "a.md - Markdown Viewer"),
    ] {
        reader.press(step);

        assert_eq!(reader.title(), title, "after {step}");
    }
}
