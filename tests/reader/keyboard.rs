//! Keyboard shortcuts: every one in the README's table, from every place
//! the keyboard can be when it is pressed.

use super::docs::long_document;
use super::support::{Reader, Rgb};

/// What can be open or switched on in the window, by the name a failing
/// test calls it.
const SWITCHES: [&str; 5] = [
    "the find bar",
    "edit mode",
    "the files sidebar",
    "the contents sidebar",
    "quit",
];

/// Everything about the window that a shortcut of the README changes.
#[derive(Debug, PartialEq)]
struct Seen {
    paper: Rgb,
    title: String,
    /// The column's width, with the zoom taken out.
    width: i32,
    /// The side of a top bar button, which grows with the zoom.
    zoom: i32,
    /// Whether each of [`SWITCHES`] is open or on.
    switches: [bool; 5],
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "widths are a few thousand pixels at most"
)]
fn seen(reader: &Reader) -> Seen {
    let button = reader.area_of("More").width;
    let unzoomed = reader.column().width * 28.0 / button;
    Seen {
        paper: reader.page_color(),
        title: reader.title().to_owned(),
        width: unzoomed.round() as i32,
        zoom: button.round() as i32,
        switches: [
            reader.count("Close find bar") == 1,
            reader.typed().iter().any(|text| text.starts_with("# ")),
            reader.sees("FILES"),
            reader.count("CONTENTS") == 1,
            reader.has_quit(),
        ],
    }
}

/// The names of what differs between two looks at the window.
fn changes(before: &Seen, after: &Seen) -> Vec<&'static str> {
    let mut changed = Vec::new();
    for (name, differs) in [
        ("the theme", before.paper != after.paper),
        ("the document", before.title != after.title),
        ("the width", before.width != after.width),
        ("the zoom", before.zoom != after.zoom),
    ] {
        changed.extend(differs.then_some(name));
    }
    for (name, (was, is)) in SWITCHES
        .iter()
        .zip(before.switches.iter().zip(after.switches))
    {
        changed.extend((*was != is).then_some(*name));
    }
    changed
}

/// Two tabs: `a.md` behind, and in front a tab that went from `c.md` to
/// `b.md`, so there is a tab to switch to and a step to go back.
fn library() -> Reader {
    // A window with room for both sidebars beside the column at 125%.
    let mut reader = Reader::launch()
        .doc("a.md", "# Alpha\n\n## Part one\n\nThe first page.")
        .doc("b.md", "# Beta\n\n## Part two\n\nThe second page.")
        .doc("c.md", "# Gamma\n\n## Part three\n\nGo [to beta](b.md).")
        .args(&["a.md"])
        .window(1800.0, 1000.0)
        .open();
    reader.drop_file("c.md");
    reader.click("to beta");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    reader
}

/// Each shortcut of the README's table that needs no file dialog: the
/// shortcut, one to press first where it needs a setup, and what it
/// changes.
const SHORTCUTS: [(&str, &str, &str); 16] = [
    ("Ctrl+W", "", "the document"),
    ("Ctrl+Tab", "", "the document"),
    ("Ctrl+Shift+Tab", "", "the document"),
    ("Alt+Left", "", "the document"),
    ("Alt+Right", "Alt+Left", "the document"),
    ("Ctrl+F", "", "the find bar"),
    ("Ctrl+D", "", "the theme"),
    ("Ctrl+Alt+Left", "", "the width"),
    ("Ctrl+Alt+Right", "", "the width"),
    ("Ctrl++", "", "the zoom"),
    ("Ctrl+-", "", "the zoom"),
    ("Ctrl+0", "Ctrl++", "the zoom"),
    ("Ctrl+Shift+E", "", "the files sidebar"),
    ("Ctrl+Shift+O", "", "the contents sidebar"),
    ("Ctrl+E", "", "edit mode"),
    ("Ctrl+Q", "", "quit"),
];

type Place = fn(&mut Reader);

/// Each place the keyboard can be, and how a reader gets it there.
const PLACES: [(&str, Place); 5] = [
    ("the document", |_| {}),
    ("the find bar", |reader| {
        reader.press("Ctrl+F");
        reader.type_text("e");
    }),
    ("an open menu", |reader| reader.click("More")),
    ("the open popover", |reader| reader.click("Appearance")),
    ("edit mode", |reader| {
        reader.press("Ctrl+E");
        reader.click_text_field(0);
    }),
];

/// Press `shortcut` with the keyboard in `place` and return what changed.
fn pressed_in(place: Place, first: &str, shortcut: &str) -> (Vec<&'static str>, Reader) {
    let mut reader = library();
    if !first.is_empty() {
        reader.press(first);
    }
    place(&mut reader);
    let before = seen(&reader);

    reader.press(shortcut);

    (changes(&before, &seen(&reader)), reader)
}

#[test]
fn every_shortcut_does_its_one_thing_from_the_document() {
    for (shortcut, first, change) in SHORTCUTS {
        let (changed, _) = pressed_in(PLACES[0].1, first, shortcut);

        assert_eq!(changed, [change], "{shortcut}");
    }
}

#[test]
fn every_shortcut_does_the_same_with_the_find_bar_open() {
    for (shortcut, first, change) in SHORTCUTS {
        let (changed, reader) = pressed_in(PLACES[1].1, first, shortcut);

        if shortcut == "Ctrl+F" {
            assert!(changed.is_empty(), "the bar is open already: {changed:?}");
            reader.assert_sees("e");
        } else {
            assert_eq!(changed, [change], "{shortcut}");
        }
        assert_eq!(reader.typed()[0], "e", "{shortcut} typed into the find bar");
    }
}

#[test]
fn closing_a_tab_from_the_find_bar_leaves_the_query_alone() {
    let (_, reader) = pressed_in(PLACES[1].1, "", "Ctrl+W");

    assert!(reader.typed()[0] == "e", "Ctrl+W ate the query");
}

#[test]
fn every_shortcut_does_the_same_with_a_menu_or_the_popover_open() {
    for (place, go) in &PLACES[2..4] {
        for (shortcut, first, change) in SHORTCUTS {
            let (changed, _) = pressed_in(*go, first, shortcut);

            assert_eq!(changed, [change], "{shortcut} from {place}");
        }
    }
}

#[test]
fn every_shortcut_does_the_same_from_edit_mode() {
    for (shortcut, first, change) in SHORTCUTS {
        let (mut changed, reader) = pressed_in(PLACES[4].1, first, shortcut);

        // The document that comes to the front is not being edited.
        if change == "the document" {
            changed.retain(|name| *name != "edit mode");
        }
        assert_eq!(changed, [change], "{shortcut}");
        if change != "the document" && change != "edit mode" {
            let source = reader.typed().pop().unwrap();
            assert_eq!(
                source,
                reader.read_file("b.md"),
                "{shortcut} typed into the source"
            );
        }
    }
}

#[test]
fn ctrl_equals_zooms_in_like_ctrl_plus() {
    let (changed, _) = pressed_in(PLACES[0].1, "", "Ctrl+=");

    assert_eq!(changed, ["the zoom"]);
}

#[test]
fn escape_closes_a_menu_and_leaves_the_find_bar_behind_it_open() {
    let mut reader = library();
    reader.press("Ctrl+F");
    reader.click("More");

    reader.press("Escape");
    assert_eq!(reader.count("Quit"), 0, "the menu closed");
    assert_eq!(reader.count("Close find bar"), 1, "the find bar did not");

    reader.click_text_field(0);
    reader.press("Escape");
    assert_eq!(reader.count("Close find bar"), 0);
}

#[test]
fn escape_closes_the_popover_and_nothing_else() {
    let mut reader = library();
    reader.press("Ctrl+E");
    reader.click("Appearance");
    let before = seen(&reader);

    reader.press("Escape");

    assert_eq!(reader.count("Fit window"), 0);
    assert_eq!(seen(&reader), before);
}

#[test]
fn the_shortcuts_that_need_no_document_work_on_the_home_page() {
    let mut reader = Reader::first_run();
    let sepia = reader.page_color();

    reader.press("Ctrl+D");
    assert_ne!(reader.page_color(), sepia);

    let button = reader.area_of("More").width;
    reader.press("Ctrl++");
    assert!(reader.area_of("More").width > button);

    reader.press("Ctrl+Shift+E");
    reader.assert_sees("FILES");

    reader.press("Ctrl+Q");
    assert!(reader.has_quit());
}

#[test]
fn the_shortcuts_that_need_a_document_do_no_harm_on_the_home_page() {
    let mut reader = Reader::first_run();
    let before = reader.names();

    for shortcut in [
        "Ctrl+W",
        "Ctrl+E",
        "Ctrl+Tab",
        "Ctrl+Shift+Tab",
        "Alt+Left",
        "Alt+Right",
        "Ctrl+Alt+Left",
        "Ctrl+Alt+Right",
        "PageDown",
        "Down",
    ] {
        reader.press(shortcut);
        assert_eq!(reader.names(), before, "{shortcut}");
    }
    assert!(!reader.has_quit());
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn the_page_keys_move_the_document_wherever_the_keyboard_is() {
    let places: [(&str, Place); 4] = [
        PLACES[0],
        ("the find bar", |reader| reader.press("Ctrl+F")),
        PLACES[2],
        PLACES[3],
    ];
    for (place, go) in places {
        let mut reader = Reader::with_docs(&[("long.md", &long_document(60))]);
        let opening = "Paragraph 1 mentions the lighthouse once.";
        go(&mut reader);
        assert!(reader.in_view(opening), "{place}");

        reader.press("PageDown");
        assert!(!reader.in_view(opening), "PageDown from {place}");

        reader.press("PageUp");
        assert!(reader.in_view(opening), "PageUp from {place}");
    }
}

#[test]
fn the_arrow_keys_move_the_document_only_while_it_has_the_keyboard() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(60))]);
    let heading = reader.text_area("Section 2").top;

    reader.click("More");
    reader.press("Down");
    reader.press("Escape");
    assert!(
        (reader.text_area("Section 2").top - heading).abs() < 1.0,
        "from a menu"
    );

    reader.press("Down");
    assert!(
        reader.text_area("Section 2").top < heading,
        "from the document"
    );
}

#[test]
fn a_shortcut_held_with_one_modifier_too_many_is_another_shortcut() {
    let (narrowed, reader) = pressed_in(PLACES[0].1, "", "Ctrl+Alt+Left");
    assert_eq!(narrowed, ["the width"]);
    assert_eq!(
        reader.title(),
        "b.md - Markdown Viewer",
        "Alt+Left would go back"
    );

    let (toggled, reader) = pressed_in(PLACES[0].1, "", "Ctrl+Shift+E");
    assert_eq!(toggled, ["the files sidebar"]);
    assert!(reader.typed().is_empty(), "Ctrl+E would open the source");
}
