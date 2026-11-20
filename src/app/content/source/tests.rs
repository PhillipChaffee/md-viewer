use eframe::egui::accesskit::Role;
use eframe::egui::{vec2, Event, ImeEvent, Key, Modifiers};
use egui_kittest::kittest::Queryable as _;

use crate::app::content::tests::label_harness::LabelHarness;

const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";

/// Emoji written as several characters that the bundled faces draw as one
/// picture: a family, a flag, a skin tone, a keycap and a sign asked for as
/// an emoji.
const JOINED: [&str; 5] = [
    FAMILY,
    "\u{1F1EF}\u{1F1F5}",
    "\u{1F44D}\u{1F3FD}",
    "1\u{FE0F}\u{20E3}",
    "\u{2764}\u{FE0F}",
];

/// The app in edit mode on a document that holds `source`, with the
/// keyboard in the text field.
fn editing(source: &str) -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[("a.md", source)]);
    harness.state_mut().tabs[0].set_edit_mode(true);
    harness.frames(2);
    harness.get_by_role(Role::MultilineTextInput).click();
    harness.frames(2);
    harness
}

/// Ctrl as a keyboard on Linux and Windows reports it.
const CTRL: Modifiers = Modifiers::CTRL.plus(Modifiers::COMMAND);

fn press(harness: &mut LabelHarness, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.frames(2);
}

/// Press each of `keys` with no modifier, starting from the very start of
/// the source (`from_start`) or its very end.
fn press_from(harness: &mut LabelHarness, from_start: bool, keys: &[Key]) {
    let edge = if from_start { Key::Home } else { Key::End };
    press(harness, CTRL, edge);
    for key in keys {
        press(harness, Modifiers::NONE, *key);
    }
}

fn type_text(harness: &mut LabelHarness, text: &str) {
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(text);
    harness.frames(2);
}

fn source(harness: &LabelHarness) -> &str {
    &harness.state().tabs[0].content
}

#[test]
fn the_left_arrow_moves_past_a_joined_emoji_as_past_one_character() {
    for emoji in JOINED {
        let mut harness = editing(&format!("a{emoji}b"));

        press_from(&mut harness, false, &[Key::ArrowLeft, Key::ArrowLeft]);
        type_text(&mut harness, "x");

        assert_eq!(source(&harness), format!("ax{emoji}b"));
    }
}

#[test]
fn the_right_arrow_moves_past_a_joined_emoji_as_past_one_character() {
    for emoji in JOINED {
        let mut harness = editing(&format!("a{emoji}b"));

        press_from(&mut harness, true, &[Key::ArrowRight, Key::ArrowRight]);
        type_text(&mut harness, "x");

        assert_eq!(source(&harness), format!("a{emoji}xb"));
    }
}

#[test]
fn backspace_removes_all_of_a_joined_emoji() {
    for emoji in JOINED {
        let mut harness = editing(&format!("a{emoji}b"));

        press_from(&mut harness, false, &[Key::ArrowLeft, Key::Backspace]);
        assert_eq!(source(&harness), "ab", "{emoji}");

        type_text(&mut harness, "x");
        assert_eq!(source(&harness), "axb", "the cursor is where the emoji was");
    }
}

#[test]
fn delete_removes_all_of_a_joined_emoji() {
    for emoji in JOINED {
        let mut harness = editing(&format!("a{emoji}b"));

        press_from(&mut harness, true, &[Key::ArrowRight, Key::Delete]);
        assert_eq!(source(&harness), "ab", "{emoji}");

        type_text(&mut harness, "x");
        assert_eq!(source(&harness), "axb", "the cursor is where the emoji was");
    }
}

#[test]
fn backspace_and_delete_at_the_ends_of_the_source_remove_nothing() {
    let mut harness = editing(&format!("{FAMILY}b"));

    press_from(&mut harness, true, &[Key::Backspace]);
    press_from(&mut harness, false, &[Key::Delete]);

    assert_eq!(source(&harness), format!("{FAMILY}b"));
}

#[test]
fn shift_and_an_arrow_select_all_of_a_joined_emoji() {
    for (from_start, arrow) in [(false, Key::ArrowLeft), (true, Key::ArrowRight)] {
        let mut harness = editing(&format!("a{FAMILY}b"));

        press_from(&mut harness, from_start, &[arrow]);
        press(&mut harness, Modifiers::SHIFT, arrow);
        type_text(&mut harness, "x");

        assert_eq!(source(&harness), "axb", "{arrow:?}");
    }
}

#[test]
fn home_and_end_go_to_the_ends_of_a_line_that_holds_a_joined_emoji() {
    let mut harness = editing(&format!("{FAMILY}a{FAMILY}"));

    press_from(&mut harness, false, &[Key::ArrowLeft, Key::Home]);
    type_text(&mut harness, "<");
    press(&mut harness, Modifiers::NONE, Key::End);
    type_text(&mut harness, ">");

    assert_eq!(source(&harness), format!("<{FAMILY}a{FAMILY}>"));
}

/// No face has one picture for a man joined to a grinning face. Each is
/// drawn, and each is a character to the keyboard; the joiner takes no room
/// and goes with the man before it.
#[test]
fn a_sequence_drawn_as_its_parts_is_edited_by_its_parts() {
    let (man, face) = ("\u{1F468}", "\u{1F600}");
    let mut harness = editing(&format!("a{man}\u{200D}{face}"));

    press_from(&mut harness, false, &[Key::Backspace]);
    assert_eq!(source(&harness), format!("a{man}\u{200D}"));

    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(source(&harness), "a");
}

#[test]
fn ctrl_z_brings_back_all_of_a_removed_emoji_and_redo_removes_it_again() {
    let mut harness = editing(&format!("a{FAMILY}b"));

    press_from(&mut harness, false, &[Key::ArrowLeft, Key::Backspace]);
    assert_eq!(source(&harness), "ab");

    press(&mut harness, CTRL, Key::Z);
    assert_eq!(source(&harness), format!("a{FAMILY}b"));

    press(&mut harness, CTRL, Key::Y);
    assert_eq!(source(&harness), "ab");
}

#[test]
fn removing_an_emoji_from_a_heading_renames_it_in_the_contents() {
    let mut harness = editing(&format!("# Home {FAMILY}"));

    press_from(&mut harness, false, &[Key::Backspace, Key::Backspace]);

    assert_eq!(source(&harness), "# Home");
    let tab = &harness.state().tabs[0];
    assert_eq!(tab.outline.headers()[0].title, "Home");
}

#[test]
fn a_word_is_still_removed_whole_and_pasted_text_goes_in_at_the_cursor() {
    let mut harness = editing(&format!("one {FAMILY} two"));

    press_from(&mut harness, false, &[]);
    press(&mut harness, CTRL, Key::Backspace);
    assert_eq!(source(&harness), format!("one {FAMILY} "));

    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    harness.event(Event::Paste("2".to_owned()));
    harness.frames(2);
    assert_eq!(source(&harness), format!("one 2{FAMILY} "));
}

#[test]
fn a_click_on_a_joined_emoji_puts_the_cursor_beside_it() {
    let mut harness = editing(FAMILY);
    let field = harness.get_by_role(Role::MultilineTextInput).rect();
    let row_height = 20.0;

    // A little inside the left edge of the picture, on the first row.
    let inside = field.left_top() + vec2(6.0, row_height / 2.0);
    harness.hover_at(inside);
    harness.drag_at(inside);
    harness.drop_at(inside);
    harness.frames(2);
    type_text(&mut harness, "x");

    assert!(
        [format!("x{FAMILY}"), format!("{FAMILY}x")].contains(&source(&harness).to_owned()),
        "{:?}",
        source(&harness)
    );
}

#[test]
fn text_composed_with_an_input_method_goes_in_at_the_cursor() {
    let mut harness = editing(&format!("a{FAMILY}b"));
    press_from(&mut harness, false, &[Key::ArrowLeft]);

    for step in [
        ImeEvent::Enabled,
        ImeEvent::Preedit("ni".to_owned()),
        ImeEvent::Commit("\u{65E5}".to_owned()),
    ] {
        harness.event(Event::Ime(step));
        harness.frames(2);
    }

    assert_eq!(source(&harness), format!("a{FAMILY}\u{65E5}b"));
}

/// The carriage return of a Windows line ending takes no room either, and
/// stays where it is when the character before it is removed.
#[test]
fn delete_leaves_a_windows_line_ending_as_it_is() {
    let mut harness = editing("ab\r\ncd");

    press_from(&mut harness, true, &[Key::ArrowRight, Key::Delete]);

    assert_eq!(source(&harness), "a\r\ncd");
}
