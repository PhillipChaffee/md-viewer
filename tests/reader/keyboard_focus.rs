//! Where the keyboard is: the ring round whatever the Tab key reached, and
//! the close button of a tab, which the Tab key reaches after its tab.

use super::docs::three_linked;
use super::support::{Area, Reader, Rgb};

/// `a.md`, `b.md` and `c.md` in three tabs named Alpha, Beta and Gamma,
/// with Gamma in front.
fn three_tabs() -> Reader {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    reader.drop_file("c.md");
    reader
}

/// A folder of documents, open on `readme.md` with the files sidebar
/// showing.
fn desk() -> Reader {
    let mut reader = Reader::launch()
        .doc("readme.md", "# Read me\n\n## Setup\n\nStart here.")
        .doc("changes.md", "# Changes\n\nWhat is new.")
        .doc("notes.md", "# Notes\n\nKept short.")
        .args(&["readme.md"])
        .open();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });
    reader
}

/// Take the pointer off the tabs and the sidebars.
fn point_away(reader: &mut Reader) {
    reader.hover("More");
}

/// The ring a button of the top row has while the keyboard is on it.
#[track_caller]
fn button_ring(reader: &mut Reader) -> Rgb {
    reader.tab_to("More");
    let ring = reader.ring_around("More");
    ring.expect("a top-row button the keyboard is on has a ring")
}

/// What the close button of the tab named `tab` is called.
fn close(tab: &str) -> String {
    format!("Close {tab}")
}

/// Whether each of the three tabs has its close button drawn.
fn closes_drawn(reader: &Reader) -> [bool; 3] {
    ["Alpha", "Beta", "Gamma"].map(|tab| !reader.marks_in(reader.area_of(&close(tab))).is_empty())
}

/// Reach `label` with the Tab key and check that it has the ring of a
/// top-row button then, and none at rest or under the pointer.
#[track_caller]
fn assert_ring_follows_the_keyboard(reader: &mut Reader, label: &str) {
    let ring = button_ring(reader);
    assert_eq!(reader.ring_around(label), None, "{label}, at rest");
    reader.hover(label);
    assert_eq!(reader.ring_around(label), None, "{label}, pointed at");
    point_away(reader);

    reader.tab_to(label);

    assert_eq!(reader.ring_around(label), Some(ring), "{label}, focused");
    reader.press("Tab");
    assert_eq!(reader.ring_around(label), None, "{label}, left behind");
}

#[test]
fn a_file_the_keyboard_is_on_has_the_ring_of_a_top_row_button() {
    let mut reader = desk();

    assert_ring_follows_the_keyboard(&mut reader, "changes.md");
}

#[test]
fn a_heading_of_the_contents_the_keyboard_is_on_has_the_ring_too() {
    let mut reader = desk();

    assert_ring_follows_the_keyboard(&mut reader, "Setup");
}

#[test]
fn the_menu_button_of_each_sidebar_has_the_ring_while_the_keyboard_is_on_it() {
    let mut reader = desk();

    assert_ring_follows_the_keyboard(&mut reader, "Explorer options");
    assert_ring_follows_the_keyboard(&mut reader, "Outline options");
}

#[test]
fn a_tab_the_keyboard_is_on_has_the_ring_of_a_top_row_button() {
    let mut reader = three_tabs();

    assert_ring_follows_the_keyboard(&mut reader, "tab:Beta");
    assert_ring_follows_the_keyboard(&mut reader, "tab:Gamma");
}

#[test]
fn the_ring_is_in_the_accent_of_each_theme() {
    let mut reader = desk();
    let mut seen = Vec::new();

    for _ in 0..4 {
        let ring = button_ring(&mut reader);
        for label in ["changes.md", "Explorer options", "tab:Read me"] {
            reader.tab_to(label);
            assert_eq!(reader.ring_around(label), Some(ring), "{label}");
        }
        seen.push(ring);
        reader.press("Ctrl+D");
    }

    seen.dedup();
    assert!(seen.len() > 1, "the ring follows the theme: {seen:?}");
}

#[test]
fn the_tab_key_goes_from_each_tab_to_its_close_button_and_then_on() {
    let mut reader = three_tabs();
    point_away(&mut reader);
    reader.tab_to("tab:Alpha");

    let mut stops = Vec::new();
    for _ in 0..6 {
        reader.press("Tab");
        stops.push(reader.focused().unwrap_or_default());
    }

    let after = [
        "Close Alpha",
        "Beta",
        "Close Beta",
        "Gamma",
        "Close Gamma",
        "New tab",
    ];
    assert_eq!(stops, after);
    reader.press("Shift+Tab");
    assert_eq!(reader.focused(), Some(close("Gamma")), "and back");
    reader.press("Shift+Tab");
    assert!(reader.keyboard_is_on("tab:Gamma"));
}

#[test]
fn a_close_button_the_keyboard_is_on_is_drawn_with_the_ring() {
    let mut reader = three_tabs();

    assert_ring_follows_the_keyboard(&mut reader, &close("Alpha"));
}

#[test]
fn a_close_button_is_drawn_while_the_pointer_or_the_keyboard_is_on_its_tab_or_on_it() {
    let mut reader = three_tabs();
    point_away(&mut reader);
    assert_eq!(closes_drawn(&reader), [false, false, false]);

    reader.hover("tab:Beta");
    assert_eq!(closes_drawn(&reader), [false, true, false], "pointed at");
    reader.hover(&close("Beta"));
    assert_eq!(closes_drawn(&reader), [false, true, false], "itself");
    point_away(&mut reader);
    assert_eq!(closes_drawn(&reader), [false, false, false]);

    reader.tab_to("tab:Alpha");
    assert_eq!(closes_drawn(&reader), [true, false, false], "its tab");
    reader.press("Tab");
    assert_eq!(reader.focused(), Some(close("Alpha")));
    assert_eq!(closes_drawn(&reader), [true, false, false], "itself");
    reader.press("Tab");
    assert_eq!(closes_drawn(&reader), [false, true, false], "moved on");
}

#[test]
fn enter_and_space_on_a_close_button_close_its_tab_and_no_other() {
    for key in ["Enter", "Space"] {
        let mut reader = three_tabs();
        point_away(&mut reader);

        reader.tab_to(&close("Alpha"));
        reader.press(key);

        assert_eq!(reader.tabs(), ["Beta", "Gamma"], "{key}");
        assert_eq!(reader.tab_in_front().as_deref(), Some("Gamma"), "{key}");
        reader.assert_sees("The end of the line.");
    }
}

#[test]
fn the_tabs_stay_where_they_are_while_the_keyboard_is_on_a_tab_or_its_close_button() {
    let places = |reader: &Reader| -> [Area; 4] {
        ["tab:Alpha", "tab:Beta", "tab:Gamma", "New tab"].map(|name| reader.area_of(name))
    };
    let mut reader = three_tabs();
    point_away(&mut reader);
    let at_rest = places(&reader);

    reader.tab_to("tab:Alpha");
    assert_eq!(places(&reader), at_rest, "on the tab");
    reader.tab_to(&close("Alpha"));
    assert_eq!(places(&reader), at_rest, "on its close button");

    let (tab, button) = (at_rest[0], reader.area_of(&close("Alpha")));
    assert!(button.left > tab.left + tab.width / 2.0, "at the right end");
    assert!(button.right() <= tab.right() && button.top >= tab.top);
    assert!(button.bottom() <= tab.bottom(), "{button:?} in {tab:?}");
}

#[test]
fn a_screen_reader_is_told_which_tab_each_close_button_closes() {
    let mut reader = three_tabs();
    point_away(&mut reader);

    for tab in ["Alpha", "Beta", "Gamma"] {
        assert_eq!(reader.count(&close(tab)), 1, "{tab}");
    }
    assert_eq!(reader.count("Close tab"), 0, "none is nameless");
}

#[test]
fn a_tab_the_keyboard_reaches_outside_the_strip_is_scrolled_into_it() {
    let names: Vec<String> = (1..=30).map(|n| format!("chapter-{n:02}.md")).collect();
    let mut launch = Reader::launch();
    for name in &names {
        launch = launch.doc(name, "Words.");
    }
    let mut reader = launch.args(&[names[0].as_str()]).open();
    for name in &names[1..] {
        reader.drop_file(name);
    }
    point_away(&mut reader);
    // The tab in front is the last one, so the strip shows its far end.
    let in_strip = |reader: &Reader, label: &str| {
        let (tab, from) = (reader.area_of(label), reader.area_of("Go forward"));
        tab.left >= from.right() && tab.right() <= reader.area_of("New tab").left
    };
    assert!(!in_strip(&reader, "tab:chapter-01.md"), "more than fit");

    reader.tab_to("tab:chapter-01.md");
    assert!(in_strip(&reader, "tab:chapter-01.md"), "the tab");
    reader.press("Tab");
    assert_eq!(reader.focused(), Some(close("chapter-01.md")));
    assert!(in_strip(&reader, &close("chapter-01.md")), "its button");

    reader.tab_to("tab:chapter-30.md");
    assert!(
        in_strip(&reader, "tab:chapter-30.md"),
        "and back to the end"
    );
    assert_eq!(reader.tab_in_front().as_deref(), Some("chapter-30.md"));
}

/// Check that the keyboard is on the tab in front, with the ring to show
/// it, and that neither Enter nor Space there closes anything.
#[track_caller]
fn assert_keyboard_rests_on_the_tab_in_front(reader: &mut Reader) {
    let front = reader.tab_in_front().expect("a tab is in front");
    let tab = format!("tab:{front}");
    assert!(reader.keyboard_is_on(&tab), "{:?}", reader.focused());
    assert!(reader.ring_around(&tab).is_some(), "and it shows");
    let open = reader.tabs();
    for key in ["Enter", "Space"] {
        reader.press(key);
        assert_eq!(reader.tabs(), open, "{key} closes nothing more");
    }
    assert!(reader.keyboard_is_on(&tab), "and it stays there");
}

#[test]
fn closing_a_tab_behind_from_the_keyboard_puts_the_keyboard_on_the_tab_in_front() {
    for key in ["Enter", "Space"] {
        let mut reader = three_tabs();
        point_away(&mut reader);
        reader.tab_to(&close("Alpha"));

        reader.press(key);

        assert_eq!(reader.tabs(), ["Beta", "Gamma"], "{key}");
        assert_eq!(reader.tab_in_front().as_deref(), Some("Gamma"), "{key}");
        assert_keyboard_rests_on_the_tab_in_front(&mut reader);
    }
}

#[test]
fn closing_the_tab_in_front_from_the_keyboard_puts_it_on_the_tab_that_comes_forward() {
    let mut reader = three_tabs();
    point_away(&mut reader);
    reader.tab_to(&close("Gamma"));

    reader.press("Enter");

    assert_eq!(reader.tabs(), ["Alpha", "Beta"]);
    assert_keyboard_rests_on_the_tab_in_front(&mut reader);
}

#[test]
fn closing_the_last_tab_from_the_keyboard_puts_the_keyboard_on_the_new_tab_button() {
    let mut reader = three_linked();
    point_away(&mut reader);
    reader.tab_to(&close("Alpha"));

    reader.press("Enter");

    assert!(reader.tabs().is_empty());
    assert_eq!(reader.focused().as_deref(), Some("New tab"));
    assert!(reader.ring_around("New tab").is_some(), "and it shows");
}

#[test]
fn a_key_held_down_on_a_close_button_closes_one_tab() {
    for key in ["Enter", "Space"] {
        let mut reader = three_tabs();
        point_away(&mut reader);
        reader.tab_to(&close("Alpha"));

        reader.hold(key, 12);

        assert_eq!(reader.tabs(), ["Beta", "Gamma"], "{key}");
        assert_eq!(reader.tab_in_front().as_deref(), Some("Gamma"), "{key}");
    }
}

/// Alpha with an edit that is not saved, and Beta in front of it, with the
/// keyboard on Alpha's close button.
fn on_the_close_button_of_unsaved_edits() -> Reader {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    reader.click("tab:Alpha");
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.type_text(" Mine.");
    reader.click("tab:Beta");
    point_away(&mut reader);
    reader.tab_to(&close("Alpha"));
    reader
}

#[test]
fn a_key_held_down_on_the_close_button_of_unsaved_edits_asks_and_answers_nothing() {
    for key in ["Enter", "Space"] {
        let mut reader = on_the_close_button_of_unsaved_edits();
        let written = reader.read_file("a.md");

        reader.hold(key, 12);

        reader.assert_sees("Save changes to “Alpha” (a.md)?");
        assert_eq!(reader.tabs(), ["Alpha", "Beta"], "{key}");
        assert_eq!(reader.read_file("a.md"), written, "{key} saved nothing");
    }
}

#[test]
fn a_tab_closed_from_the_keyboard_after_its_prompt_leaves_the_keyboard_on_the_tab_in_front() {
    let mut reader = on_the_close_button_of_unsaved_edits();

    reader.press("Enter");
    reader.assert_sees("Save changes to “Alpha” (a.md)?");
    reader.click("Discard");

    assert_eq!(reader.tabs(), ["Beta"]);
    assert_keyboard_rests_on_the_tab_in_front(&mut reader);
}

#[test]
fn a_close_from_the_keyboard_that_is_cancelled_moves_the_keyboard_nowhere_later() {
    let mut reader = on_the_close_button_of_unsaved_edits();
    reader.press("Enter");
    reader.press("Escape");
    assert_eq!(reader.tabs(), ["Alpha", "Beta"], "cancelled");

    reader.hover("tab:Alpha");
    reader.click(&close("Alpha"));
    reader.click("Discard");

    assert_eq!(reader.tabs(), ["Beta"]);
    assert_eq!(reader.focused(), None, "the pointer closed it");
}

#[test]
fn a_tab_closed_with_the_pointer_moves_the_keyboard_nowhere() {
    let mut reader = three_tabs();

    reader.hover("tab:Alpha");
    reader.click(&close("Alpha"));

    assert_eq!(reader.tabs(), ["Beta", "Gamma"]);
    assert_eq!(reader.focused(), None);
}
