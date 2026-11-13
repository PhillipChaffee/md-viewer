//! The Colors and Fonts dialogs.

use super::pages::{read_on, THEMES};
use super::support::{Reader, Rgb};

/// A page with a link on it, beside the page the link leads to.
fn linked() -> Reader {
    Reader::with_docs(&[
        (
            "page.md",
            "# Colors\n\nSome words and [a link](other.md) to follow.",
        ),
        ("other.md", "# Other"),
    ])
}

/// The page with the Colors dialog open.
fn colors() -> Reader {
    let mut reader = linked();
    open(&mut reader, "Colors\u{2026}");
    reader
}

/// Open the dialog the menu row `row` leads to.
fn open(reader: &mut Reader, row: &str) {
    reader.click("More");
    reader.click(row);
}

/// The color laid behind selected text: the highlight. Read off the find
/// bar, with a word typed into it and selected.
fn highlight(reader: &mut Reader) -> Rgb {
    reader.press("Ctrl+F");
    reader.type_text("mmmmmmmm");
    reader.press("Ctrl+A");
    let field = reader.text_field_area(0);
    let color = reader.color_at(field.left + 12.0, field.top + field.height / 2.0);
    reader.press("Escape");
    color
}

/// Whether the two `Reset` buttons can be used: the highlight's, then the
/// link's.
fn resets(reader: &Reader) -> [bool; 2] {
    [0, 1].map(|nth| reader.is_enabled_nth("Reset", nth))
}

#[test]
fn the_colors_dialog_offers_a_highlight_and_a_link_color_with_nothing_to_reset() {
    let reader = colors();

    reader.assert_sees("Highlight Color:");
    reader.assert_sees("Link Color:");
    assert_eq!(resets(&reader), [false, false]);
}

#[test]
fn a_new_link_color_inks_the_links_and_nothing_else() {
    let mut reader = colors();
    let (link, prose) = (reader.ink_of("a link"), reader.ink_of("Some words"));

    reader.pick_another_color(1);

    assert_ne!(reader.ink_of("a link"), link);
    assert_eq!(reader.ink_of("Some words"), prose);
    assert_eq!(resets(&reader), [false, true]);
}

#[test]
fn resetting_the_link_color_goes_back_to_the_themes() {
    let mut reader = colors();
    let link = reader.ink_of("a link");
    reader.pick_another_color(1);

    reader.click_nth("Reset", 1);

    assert_eq!(reader.ink_of("a link"), link);
    assert_eq!(resets(&reader), [false, false]);
}

#[test]
fn a_new_highlight_color_is_laid_behind_selected_text() {
    let mut reader = linked();
    let before = highlight(&mut reader);
    let link = reader.ink_of("a link");
    open(&mut reader, "Colors\u{2026}");

    reader.pick_another_color(0);

    assert_eq!(resets(&reader), [true, false]);
    reader.click("Close window");
    assert_ne!(highlight(&mut reader), before);
    assert_eq!(
        reader.ink_of("a link"),
        link,
        "the link color is another setting"
    );
}

#[test]
fn resetting_the_highlight_goes_back_to_the_themes() {
    let mut reader = linked();
    let before = highlight(&mut reader);
    open(&mut reader, "Colors\u{2026}");
    reader.pick_another_color(0);

    reader.click_nth("Reset", 0);

    assert_eq!(resets(&reader), [false, false]);
    reader.click("Close window");
    assert_eq!(highlight(&mut reader), before);
}

#[test]
fn the_two_colors_are_reset_one_at_a_time() {
    let mut reader = colors();
    let link = reader.ink_of("a link");
    reader.pick_another_color(0);
    reader.pick_another_color(1);
    assert_eq!(resets(&reader), [true, true]);

    reader.click_nth("Reset", 0);

    assert_eq!(resets(&reader), [false, true]);
    assert_ne!(
        reader.ink_of("a link"),
        link,
        "the link keeps its new color"
    );
}

#[test]
fn a_chosen_link_color_is_remembered() {
    let mut reader = colors();
    reader.pick_another_color(1);
    let chosen = reader.ink_of("a link");

    let mut reader = reader.restart(&["page.md"]);

    assert_eq!(reader.ink_of("a link"), chosen);
    open(&mut reader, "Colors\u{2026}");
    assert_eq!(resets(&reader), [false, true]);
}

#[test]
fn a_chosen_link_color_is_kept_on_every_theme() {
    let mut reader = colors();
    reader.pick_another_color(1);
    reader.click("Close window");
    let chosen = reader.ink_of("a link");

    for theme in THEMES {
        read_on(&mut reader, theme);
        assert_eq!(reader.ink_of("a link"), chosen, "{theme}");
    }
}

#[test]
fn without_a_chosen_color_the_links_follow_the_theme() {
    let mut reader = linked();
    let mut inks = Vec::new();

    for theme in THEMES {
        read_on(&mut reader, theme);
        let link = reader.ink_of("a link");
        assert_ne!(link, reader.ink_of("Some words"), "{theme}");
        inks.push(link);
    }

    assert_ne!(inks[0], inks[3], "Paper and Night ink links differently");
}

#[test]
fn the_colors_dialog_comes_back_as_it_was_left() {
    let mut reader = colors();
    reader.pick_another_color(1);

    reader.click("Close window");
    reader.assert_hides("Link Color:");
    open(&mut reader, "Colors\u{2026}");

    assert_eq!(resets(&reader), [false, true]);
}

#[test]
fn the_fonts_dialog_offers_four_presets_five_sizes_and_the_families() {
    let mut reader = linked();

    open(&mut reader, "Fonts\u{2026}");

    for choice in [
        "Reader",
        "System",
        "GitHub",
        "VS Code",
        "Small",
        "Normal",
        "Large",
        "Extra large",
        "Huge",
        "System Default",
    ] {
        assert_eq!(reader.count(choice), 1, "{choice}");
    }
    for label in ["Preset", "Text Size", "Family", "Search:"] {
        reader.assert_sees(label);
    }
}

/// Which of `choices` the Fonts dialog shows as picked: the one whose
/// button is filled.
fn lit(reader: &Reader, choices: &[&'static str]) -> Vec<&'static str> {
    let fill = |choice: &str| {
        let button = reader.area_of(choice);
        reader.color_at(button.left + 2.0, button.top + 2.0)
    };
    let fills: Vec<Rgb> = choices.iter().map(|choice| fill(choice)).collect();
    let odd_one_out = |fill: &Rgb| fills.iter().filter(|other| *other == fill).count() == 1;
    choices
        .iter()
        .zip(&fills)
        .filter(|(_, fill)| odd_one_out(fill))
        .map(|(choice, _)| *choice)
        .collect()
}

const PRESETS: [&str; 4] = ["Reader", "System", "GitHub", "VS Code"];

#[test]
fn the_reader_preset_is_the_one_a_first_run_uses() {
    let mut reader = linked();

    open(&mut reader, "Fonts\u{2026}");

    assert_eq!(lit(&reader, &PRESETS), ["Reader"]);
}

#[test]
fn picking_a_preset_marks_it_and_keeps_the_document_on_the_page() {
    let mut reader = linked();
    open(&mut reader, "Fonts\u{2026}");

    for preset in ["System", "GitHub", "VS Code", "Reader"] {
        reader.click(preset);

        assert_eq!(lit(&reader, &PRESETS), [preset]);
        reader.assert_sees("Some words and ");
        let column = reader.column();
        assert!(reader.text_area("Some words and ").left >= column.left - 1.0);
    }
}

#[test]
fn a_picked_preset_is_remembered() {
    let mut reader = linked();
    open(&mut reader, "Fonts\u{2026}");
    reader.click("GitHub");

    let mut reader = reader.restart(&["page.md"]);
    open(&mut reader, "Fonts\u{2026}");

    assert_eq!(lit(&reader, &PRESETS), ["GitHub"]);
}

#[test]
fn the_size_in_use_is_marked_in_the_fonts_dialog() {
    let sizes = ["Small", "Normal", "Large", "Extra large", "Huge"];
    let mut reader = linked();
    open(&mut reader, "Fonts\u{2026}");
    assert_eq!(lit(&reader, &sizes), ["Normal"]);

    reader.click("Extra large");

    assert_eq!(lit(&reader, &sizes), ["Extra large"]);
}

#[test]
fn a_search_that_matches_no_family_leaves_only_the_default() {
    let mut reader = linked();
    open(&mut reader, "Fonts\u{2026}");
    reader.wait_for("the installed fonts to be listed", |reader| {
        !reader.sees("Scanning installed fonts")
    });

    reader.click_text_field(0);
    reader.type_text("no family has this name 12345");

    assert_eq!(reader.typed(), ["no family has this name 12345"]);
    assert_eq!(reader.count("System Default"), 1);
    let below = reader.area_of("System Default").bottom();
    let dialog_bottom = reader.area_of("Fonts").bottom();
    assert!(
        dialog_bottom - below < 40.0,
        "the list is empty under the default"
    );
}

#[test]
fn the_default_family_is_the_one_in_use_until_another_is_picked() {
    let mut reader = linked();
    open(&mut reader, "Fonts\u{2026}");
    let default = reader.area_of("System Default");
    let unlit = reader.color_at(default.right() + 60.0, default.top + 2.0);

    assert_ne!(
        reader.color_at(default.left + 2.0, default.top + 2.0),
        unlit
    );
}
