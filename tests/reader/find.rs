//! Find in document.

use super::docs::long_document;
use super::support::Reader;

/// A page where "tent" appears three times, once capitalized.
fn camping() -> Reader {
    Reader::with_docs(&[(
        "camping.md",
        "# Camping\n\nPack the tent.\n\nTent pegs go in last.\n\nAir the tent at home.",
    )])
}

#[test]
fn ctrl_f_opens_the_find_bar_with_the_keyboard_in_it() {
    let mut reader = camping();
    assert_eq!(reader.count("Close find bar"), 0);

    reader.press("Ctrl+F");
    reader.type_text("pegs");

    assert_eq!(reader.typed(), ["pegs"]);
    reader.assert_sees("1 / 1");
}

#[test]
fn the_top_bar_button_and_the_menu_open_it_too() {
    let mut reader = camping();

    reader.click("Find in document");
    assert_eq!(reader.count("Close find bar"), 1);

    reader.click("Close find bar");
    assert_eq!(reader.count("Close find bar"), 0);

    reader.click("More");
    reader.click("Find…");
    assert_eq!(reader.count("Close find bar"), 1);
}

#[test]
fn matches_are_counted_whatever_their_case() {
    let mut reader = camping();
    reader.press("Ctrl+F");

    reader.type_text("TENT");

    reader.assert_sees("1 / 3");
}

#[test]
fn enter_steps_through_the_matches_and_wraps() {
    let mut reader = camping();
    reader.press("Ctrl+F");
    reader.type_text("tent");

    reader.press("Enter");
    reader.assert_sees("2 / 3");
    reader.press("Enter");
    reader.assert_sees("3 / 3");
    reader.press("Enter");
    reader.assert_sees("1 / 3");
}

#[test]
fn shift_enter_steps_backwards() {
    let mut reader = camping();
    reader.press("Ctrl+F");
    reader.type_text("tent");

    reader.press("Shift+Enter");
    reader.assert_sees("3 / 3");
    reader.press("Shift+Enter");
    reader.assert_sees("2 / 3");
}

#[test]
fn the_arrow_keys_and_buttons_step_too() {
    let mut reader = camping();
    reader.press("Ctrl+F");
    reader.type_text("tent");

    reader.press("Down");
    reader.assert_sees("2 / 3");
    reader.press("Up");
    reader.assert_sees("1 / 3");

    reader.click("Next match");
    reader.assert_sees("2 / 3");
    reader.click("Previous match");
    reader.assert_sees("1 / 3");
}

#[test]
fn a_word_that_is_not_there_has_no_matches() {
    let mut reader = camping();
    reader.press("Ctrl+F");

    reader.type_text("canoe");

    reader.assert_sees("0 matches");
    assert!(!reader.is_enabled("Next match"));
    assert!(!reader.is_enabled("Previous match"));
}

#[test]
fn escape_closes_the_find_bar() {
    let mut reader = camping();
    reader.press("Ctrl+F");
    reader.type_text("tent");

    reader.press("Escape");

    assert_eq!(reader.count("Close find bar"), 0);
    reader.assert_hides("1 / 3");
    reader.assert_sees("Pack the tent.");
}

#[test]
fn stepping_to_a_match_brings_it_into_view() {
    let mut text = long_document(80);
    text.push_str("## Appendix\n\nThe only heron in the document.\n");
    let mut reader = Reader::with_docs(&[("long.md", &text)]);
    assert!(!reader.in_view("Appendix"));

    reader.press("Ctrl+F");
    reader.type_text("heron");
    reader.wait_for("the match to scroll into view", |reader| {
        reader.in_view("Appendix")
    });

    reader.assert_sees("1 / 1");
}

#[test]
fn the_count_follows_the_document_in_front() {
    let mut reader = Reader::with_docs(&[
        ("one.md", "# One\n\nA tent."),
        ("two.md", "# Two\n\nA tent beside a tent."),
    ]);
    reader.press("Ctrl+F");
    reader.type_text("tent");
    reader.assert_sees("1 / 1");

    reader.drop_file("two.md");

    reader.assert_sees("1 / 2");
}
