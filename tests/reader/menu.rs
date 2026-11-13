//! The menu behind the top bar's last button: every row does what its
//! label says, and says what state it is in.

use super::pages::{page, picture_between};
use super::support::Reader;

/// Two linked pages, opened on the first, with a formula on it.
fn library() -> Reader {
    Reader::with_docs(&[
        (
            "first.md",
            "# First\n\nGo [to the second](second.md).\n\n## A part\n\nbefore\n\n$$\na^2 + b^2 = c^2\n$$\n\nafter",
        ),
        ("second.md", "# Second\n\nThe second page."),
    ])
}

/// Open the menu, and the submenu `parent` when one is named.
fn open_menu(reader: &mut Reader, parent: Option<&str>) {
    reader.click("More");
    if let Some(parent) = parent {
        reader.click(parent);
    }
}

/// Pick `row` from the menu, or from its submenu `parent`.
fn pick(reader: &mut Reader, parent: Option<&str>, row: &str) {
    open_menu(reader, parent);
    reader.click(row);
}

/// Whether the menu shows `row` as checked.
fn checked(reader: &mut Reader, parent: Option<&str>, row: &str) -> bool {
    open_menu(reader, parent);
    let on = reader.is_on(row);
    reader.press("Escape");
    if parent.is_some() {
        reader.press("Escape");
    }
    on
}

#[test]
fn the_menu_lists_every_command_the_bar_has_no_button_for() {
    let mut reader = library();

    reader.click("More");

    for row in [
        "Open File\u{2026}",
        "Open Folder\u{2026}",
        "Close Tab",
        "Find\u{2026}",
        "Edit Document",
        "Live Reload",
        "Back",
        "Forward",
        "Show Explorer",
        "Show Outline",
        "Full Width",
        "Smooth Text Rendering",
        "Theme",
        "Line Spacing",
        "Formula Size",
        "Zoom Out",
        "Zoom In",
        "Colors\u{2026}",
        "Fonts\u{2026}",
        "Quit",
    ] {
        assert_eq!(reader.count(row), 1, "{row}");
    }
}

#[test]
fn the_menu_shows_the_shortcut_of_each_command_that_has_one() {
    let mut reader = library();

    reader.click("More");

    for shortcut in [
        "Ctrl+W",
        "Ctrl+F",
        "Ctrl+E",
        "F5",
        "Alt+Left",
        "Alt+Right",
        "Ctrl+Shift+E",
        "Ctrl+Shift+O",
        "Ctrl+Q",
    ] {
        reader.assert_sees(shortcut);
    }
}

#[test]
fn a_switch_in_the_menu_shows_a_check_once_it_is_picked_and_loses_it_again() {
    let switches = [
        ("Edit Document", false),
        ("Live Reload", true),
        ("Show Explorer", false),
        ("Show Outline", true),
        ("Full Width", false),
        ("Smooth Text Rendering", false),
    ];

    for (row, starts_on) in switches {
        let mut reader = library();
        assert_eq!(checked(&mut reader, None, row), starts_on, "{row} at first");

        pick(&mut reader, None, row);
        assert_eq!(checked(&mut reader, None, row), !starts_on, "{row} picked");

        pick(&mut reader, None, row);
        assert_eq!(
            checked(&mut reader, None, row),
            starts_on,
            "{row} picked twice"
        );
    }
}

#[test]
fn picking_a_row_closes_the_menu() {
    for row in [
        "Find\u{2026}",
        "Show Explorer",
        "Full Width",
        "Colors\u{2026}",
    ] {
        let mut reader = library();

        pick(&mut reader, None, row);

        assert_eq!(reader.count("Quit"), 0, "{row} left the menu open");
    }
}

#[test]
fn the_menu_closes_without_doing_anything() {
    type WayOut = fn(&mut Reader);
    let ways: [(&str, WayOut); 3] = [
        ("Escape", |reader| reader.press("Escape")),
        ("the button again", |reader| reader.click("More")),
        ("a click on the page", |reader| {
            let page = reader.column();
            reader.click_point(page.left + 20.0, page.bottom() - 20.0);
        }),
    ];

    for (name, leave) in ways {
        let mut reader = library();
        let before = reader.names();
        reader.click("More");

        leave(&mut reader);

        assert_eq!(reader.names(), before, "{name}");
    }
}

#[test]
fn rows_that_need_a_document_are_grayed_out_on_the_home_page() {
    let mut reader = Reader::first_run();

    reader.click("More");

    for row in ["Close Tab", "Edit Document", "Back", "Forward"] {
        assert!(!reader.is_enabled(row), "{row}");
    }
    for row in [
        "Show Explorer",
        "Theme",
        "Colors\u{2026}",
        "Fonts\u{2026}",
        "Quit",
    ] {
        assert!(reader.is_enabled(row), "{row}");
    }
}

#[test]
fn edit_document_shows_the_source_and_shows_the_page_again() {
    let mut reader = library();

    pick(&mut reader, None, "Edit Document");
    assert_eq!(reader.typed().len(), 1, "the source is in a text field");
    assert!(reader.typed()[0].starts_with("# First"));

    pick(&mut reader, None, "Edit Document");
    assert!(reader.typed().is_empty());
}

#[test]
fn full_width_fits_the_column_to_the_window_and_gives_the_width_back() {
    let mut reader = library();
    let before = reader.column();

    pick(&mut reader, None, "Full Width");
    let fitted = reader.column();
    pick(&mut reader, None, "Full Width");

    assert!(
        fitted.width > before.width + 150.0,
        "{before:?} then {fitted:?}"
    );
    assert_eq!(reader.column(), before);
}

#[test]
fn smooth_text_rendering_is_remembered() {
    let mut reader = library();
    pick(&mut reader, None, "Smooth Text Rendering");

    let mut reader = reader.restart(&["first.md"]);

    assert!(checked(&mut reader, None, "Smooth Text Rendering"));
}

#[test]
fn the_theme_rows_check_the_theme_in_use_and_switch_to_the_one_picked() {
    let mut reader = library();
    assert!(checked(&mut reader, Some("Theme"), "Sepia"));

    for theme in ["Paper", "Dusk", "Night", "Sepia"] {
        let before = reader.page_color();

        pick(&mut reader, Some("Theme"), theme);

        assert_ne!(reader.page_color(), before, "{theme}");
        assert!(checked(&mut reader, Some("Theme"), theme), "{theme}");
    }
}

#[test]
fn next_theme_in_the_menu_steps_as_the_shortcut_does() {
    let mut by_menu = library();
    let mut by_key = library();

    for _ in 0..4 {
        pick(&mut by_menu, Some("Theme"), "Next Theme");
        by_key.press("Ctrl+D");

        assert_eq!(by_menu.page_color(), by_key.page_color());
    }
}

#[test]
fn a_formula_size_in_the_menu_sets_formulas_larger_than_their_text() {
    let mut reader = library();
    let normal = picture_between(&mut reader, "before", "after");
    let text = reader.text_area("before");
    assert!(checked(&mut reader, Some("Formula Size"), "100%"));

    let mut widths = vec![normal.width];
    for size in ["110%", "125%", "150%"] {
        let last = *widths.last().unwrap();
        pick(&mut reader, Some("Formula Size"), size);
        reader.wait_for("the formula at its new size", |reader| {
            reader
                .pictures()
                .iter()
                .any(|picture| picture.width > last + 1.0)
        });
        widths.push(picture_between(&mut reader, "before", "after").width);
        assert!(checked(&mut reader, Some("Formula Size"), size), "{size}");
    }

    assert!((widths[3] / widths[0] - 1.5).abs() < 0.1, "{widths:?}");
    assert!((widths[2] / widths[0] - 1.25).abs() < 0.1, "{widths:?}");
    assert_eq!(reader.text_area("before"), text, "the text is as it was");
}

#[test]
fn the_formula_size_is_remembered() {
    let mut reader = library();
    let normal = picture_between(&mut reader, "before", "after").width;
    pick(&mut reader, Some("Formula Size"), "150%");

    let mut reader = reader.restart(&["first.md"]);

    assert!(checked(&mut reader, Some("Formula Size"), "150%"));
    let large = picture_between(&mut reader, "before", "after").width;
    assert!(large > normal * 1.4, "{normal} then {large}");
}

#[test]
fn colors_and_fonts_each_open_a_dialog_that_its_close_button_shuts() {
    let dialogs = [
        ("Colors\u{2026}", "Highlight Color:"),
        ("Fonts\u{2026}", "Preset"),
    ];

    for (row, words) in dialogs {
        let mut reader = library();
        reader.assert_hides(words);

        pick(&mut reader, None, row);
        reader.assert_sees(words);

        reader.click("Close window");
        reader.assert_hides(words);
    }
}

#[test]
fn both_dialogs_can_be_open_at_once() {
    let mut reader = library();

    pick(&mut reader, None, "Colors\u{2026}");
    pick(&mut reader, None, "Fonts\u{2026}");

    reader.assert_sees("Link Color:");
    reader.assert_sees("Preset");
    assert_eq!(reader.count("Close window"), 2);
}

#[test]
fn the_title_says_where_the_document_is_and_that_it_is_live() {
    let mut reader = page("words with no heading");
    reader.assert_hides("Live:");

    reader.hover("page.md");

    reader.assert_sees(&reader.path("page.md").display().to_string());
    reader.assert_sees("Live: reloads when the file changes");
}

#[test]
fn the_title_stops_saying_live_once_live_reload_is_switched_off() {
    let mut reader = page("words with no heading");

    pick(&mut reader, None, "Live Reload");
    reader.hover("page.md");

    reader.assert_sees(&reader.path("page.md").display().to_string());
    reader.assert_hides("Live:");
}

#[test]
fn a_document_opened_without_watching_is_not_called_live() {
    let mut reader = Reader::launch()
        .doc("page.md", "words with no heading")
        .args(&["page.md", "--no-watch"])
        .open();

    reader.hover("page.md");
    reader.assert_hides("Live:");

    reader.press("F5");
    reader.hover("Appearance");
    reader.hover("page.md");
    reader.assert_sees("Live: reloads when the file changes");
}
