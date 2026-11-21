//! Menus and popovers in a window too short for them: every row can still
//! be reached, with the wheel and with the Tab key. In a window that has
//! room they do not scroll.

use super::support::{Reader, Rgb};

/// The paper of the theme a first run starts in.
const SEPIA: &str = "#F8F1E3";

/// As far as the wheel has to turn to reach the end of any menu.
const TO_THE_END: f32 = 4000.0;

/// A document in a window 800 px tall, zoomed in `steps` times: 125%,
/// 156%, and so on to the 300% the zoom stops at.
fn zoomed_in(steps: usize) -> Reader {
    let mut reader = Reader::with_docs(&[
        ("notes.md", "# Notes\n\nSome text.\n\n## A part\n\nMore."),
        ("other.md", "# Other"),
    ]);
    reader.drop_file("other.md");
    reader.click("tab:Notes");
    reader.resize_window(1200.0, 800.0);
    for _ in 0..steps {
        reader.press("Ctrl++");
    }
    reader
}

/// Turn the wheel toward the reader with the pointer on `row`.
fn wheel_down_on(reader: &mut Reader, row: &str, pixels: f32) {
    let row = reader.area_of(row);
    reader.wheel_down_at(
        row.left + row.width / 2.0,
        row.top + row.height / 2.0,
        pixels,
    );
}

/// Press Tab until the keyboard is in `control`.
fn tab_to(reader: &mut Reader, control: &str) {
    for _ in 0..60 {
        if reader.focused().as_deref() == Some(control) {
            return;
        }
        reader.press("Tab");
    }
    panic!("the Tab key never reached `{control}`");
}

#[test]
fn the_wheel_brings_the_last_row_of_a_menu_too_tall_for_the_window_into_reach() {
    let mut reader = zoomed_in(2);
    reader.click("More");
    assert!(
        !reader.is_within_window("Quit"),
        "the menu fits at this zoom, so this proves nothing"
    );

    wheel_down_on(&mut reader, "Open Folder\u{2026}", TO_THE_END);
    assert!(reader.is_within_window("Quit"));
    assert!(!reader.is_within_window("Open File\u{2026}"));

    reader.click("Quit");
    assert!(reader.has_quit());
}

#[test]
fn the_wheel_brings_the_first_row_back() {
    let mut reader = zoomed_in(2);
    reader.click("More");
    wheel_down_on(&mut reader, "Open Folder\u{2026}", TO_THE_END);
    assert!(reader.is_within_window("Quit"));

    let row = reader.area_of("Colors\u{2026}");
    reader.wheel_up_at(row.left + 40.0, row.top + 4.0, TO_THE_END);

    assert!(reader.is_within_window("Open File\u{2026}"));
    reader.click("Open Folder\u{2026}");
    assert_eq!(reader.count("Quit"), 0, "the row was picked");
}

#[test]
fn the_tab_key_walks_to_the_last_row_of_a_menu_and_brings_it_into_view() {
    let mut reader = zoomed_in(2);
    reader.click("More");

    tab_to(&mut reader, "Quit");
    assert!(reader.is_within_window("Quit"));

    reader.press("Shift+Tab");
    reader.press("Tab");
    reader.press("Enter");
    assert!(reader.has_quit());
}

#[test]
fn shift_tab_walks_back_to_the_first_row_and_brings_it_into_view() {
    let mut reader = zoomed_in(2);
    reader.click("More");
    tab_to(&mut reader, "Quit");
    assert!(!reader.is_within_window("Open File\u{2026}"));

    for _ in 0..60 {
        if reader.focused().as_deref() == Some("Open File\u{2026}") {
            break;
        }
        reader.press("Shift+Tab");
    }

    assert_eq!(reader.focused().as_deref(), Some("Open File\u{2026}"));
    assert!(reader.is_within_window("Open File\u{2026}"));
}

#[test]
fn a_menu_the_window_has_room_for_does_not_scroll() {
    for (steps, height) in [(0, 800.0), (1, 900.0), (2, 1100.0)] {
        let mut reader = zoomed_in(steps);
        reader.resize_window(1200.0, height);
        reader.click("More");
        let rows = ["New Tab", "Theme", "Zoom In", "Quit"];
        let before = rows.map(|row| reader.area_of(row));
        assert!(
            reader.is_within_window("New Tab"),
            "zoomed in {steps} times"
        );
        assert!(reader.is_within_window("Quit"), "zoomed in {steps} times");

        wheel_down_on(&mut reader, "Theme", 300.0);

        assert_eq!(rows.map(|row| reader.area_of(row)), before);
    }
}

#[test]
fn a_menu_stops_scrolling_once_the_window_is_tall_enough_for_it() {
    let mut reader = zoomed_in(2);
    reader.click("More");
    wheel_down_on(&mut reader, "Open Folder\u{2026}", TO_THE_END);

    reader.resize_window(1200.0, 1400.0);

    assert!(reader.is_within_window("Open File\u{2026}"));
    assert!(reader.is_within_window("Quit"));
    let before = reader.area_of("Quit");
    wheel_down_on(&mut reader, "Theme", 300.0);
    assert_eq!(reader.area_of("Quit"), before);
}

#[test]
fn a_menu_that_scrolls_stands_clear_of_the_top_and_the_bottom_of_the_window() {
    let mut reader = zoomed_in(2);
    reader.click("More");
    let first = reader.area_of("New Tab");
    assert!(first.top > 8.0 && first.top < 40.0, "{first:?}");

    wheel_down_on(&mut reader, "Open Folder\u{2026}", TO_THE_END);

    let quit = reader.area_of("Quit");
    assert!(quit.bottom() < 800.0 - 8.0, "{quit:?}");
    assert!(quit.bottom() > 800.0 - 40.0, "{quit:?}");
}

#[test]
fn the_zoom_row_stays_in_reach_when_zooming_makes_the_menu_scroll() {
    let mut reader = zoomed_in(0);
    reader.click("More");

    for percent in ["125%", "156%", "195%", "244%", "300%"] {
        reader.click("Zoom In");
        assert_eq!(reader.count(&format!("Reset Zoom, {percent}")), 1);
        assert!(reader.is_within_window("Zoom In"), "at {percent}");
        assert!(reader.is_within_window("Zoom Out"), "at {percent}");
    }
}

/// A popup of the app other than the More menu.
struct Popup {
    name: &'static str,
    /// The height in px of a window too short for it at the 300% the zoom
    /// stops at.
    window: f32,
    open: fn(&mut Reader),
    last: &'static str,
    /// Whether picking `last` did what it says.
    done: fn(&Reader) -> bool,
}

/// The popups a window can be too short for, beside the More menu. The
/// smallest window the app asks for is 300 px tall; a tab's menu takes a shorter one still, which
/// a window manager is free to give.
const POPUPS: [Popup; 5] = [
    Popup {
        name: "a submenu of the More menu",
        window: 300.0,
        open: |reader| {
            // Wide enough for a submenu to stand beside its menu, clear
            // of the pointer that opens it.
            reader.resize_window(2400.0, 300.0);
            reader.click("More");
            tab_to(reader, "Theme");
            reader.click("Theme");
        },
        last: "Night",
        done: |reader| reader.count("Night") == 0 && reader.page_color() != Rgb::hex(SEPIA),
    },
    Popup {
        name: "the appearance popover",
        window: 300.0,
        open: |reader| reader.click("Appearance"),
        last: "Fonts\u{2026}",
        done: |reader| reader.sees("Preset"),
    },
    Popup {
        name: "the files sidebar's menu",
        window: 300.0,
        open: |reader| {
            reader.press("Ctrl+Shift+E");
            reader.click("Explorer options");
        },
        last: "Newest First",
        done: |reader| reader.count("Newest First") == 0,
    },
    Popup {
        name: "a file's menu",
        window: 300.0,
        open: |reader| {
            reader.press("Ctrl+Shift+E");
            reader.right_click("notes.md");
        },
        last: "Copy File URI",
        done: |reader| {
            let copied = reader.clipboard();
            copied.is_some_and(|text| text.starts_with("file://"))
        },
    },
    Popup {
        name: "a tab's menu",
        window: 150.0,
        open: |reader| reader.right_click("tab:Notes"),
        last: "Close Others",
        done: |reader| reader.tabs().len() == 1,
    },
];

/// One of `POPUPS`, open at 300% in the window too short for it.
fn too_tall(popup: &Popup) -> Reader {
    let mut reader = zoomed_in(5);
    reader.resize_window(1200.0, popup.window);
    (popup.open)(&mut reader);
    assert!(
        !reader.is_within_window(popup.last),
        "{} fits, so this proves nothing",
        popup.name
    );
    reader
}

#[test]
fn the_wheel_reaches_the_last_control_of_every_popup_too_tall_for_the_window() {
    for popup in &POPUPS {
        let mut reader = too_tall(popup);

        let pointer = reader.area_of(popup.last);
        reader.wheel_down_at(pointer.left + 10.0, popup.window / 2.0, TO_THE_END);
        assert!(reader.is_within_window(popup.last), "{}", popup.name);

        reader.click(popup.last);
        assert!((popup.done)(&reader), "{}", popup.name);
    }
}

#[test]
fn the_tab_key_reaches_the_last_control_of_every_popup_too_tall_for_the_window() {
    for popup in &POPUPS {
        let mut reader = too_tall(popup);

        tab_to(&mut reader, popup.last);
        assert!(reader.is_within_window(popup.last), "{}", popup.name);

        reader.press("Enter");
        assert!((popup.done)(&reader), "{}", popup.name);
    }
}
