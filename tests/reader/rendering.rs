//! What a document looks like once rendered: text, structure, pictures.

use super::docs::kitchen_sink;
use super::support::Reader;

/// How far apart two positions may be and still be the same place.
const NEAR: f32 = 1.5;

/// The entries of the contents sidebar, top to bottom.
fn outline(reader: &Reader) -> Vec<String> {
    let names = reader.names();
    let entries = names.iter().skip_while(|name| *name != "CONTENTS").skip(1);
    entries
        .take_while(|name| !name.ends_with(" min read"))
        .filter(|name| !name.starts_with("Collapse ") && !name.starts_with("Expand "))
        .cloned()
        .collect()
}

#[test]
fn headings_are_the_documents_outline() {
    let reader = kitchen_sink();

    assert_eq!(
        outline(&reader),
        [
            "Kitchen Sink",
            "Lists",
            "Table",
            "Code",
            "Math",
            "Diagram",
            "Footnote"
        ]
    );
}

#[test]
fn a_heading_is_set_larger_than_the_text_under_it() {
    let reader = kitchen_sink();

    let heading = reader.text_area("Lists").height;
    let body = reader.text_area("first bullet").height;

    assert!(
        heading > body * 1.15,
        "a heading {heading} over body {body}"
    );
}

#[test]
fn markup_is_rendered_and_not_shown() {
    let reader = kitchen_sink();

    reader.assert_sees("bold words");
    reader.assert_sees("slanted words");
    reader.assert_sees("inline code");
    for markup in [
        "**",
        "`inline",
        "# Kitchen",
        "](guide.md)",
        "- first",
        "| Fruit",
    ] {
        reader.assert_hides(markup);
    }
}

#[test]
fn front_matter_is_shown_as_fields() {
    let reader = kitchen_sink();

    let name = reader.text_area("title");
    let value = reader.text_area("Kitchen sink");

    assert!(value.left > name.right(), "the value is beside its name");
    assert!(value.top < name.bottom() && value.bottom() > name.top);
    assert!(name.bottom() < reader.text_area("Lists").top);
    reader.assert_hides("---");
    reader.assert_hides("title:");
}

#[test]
fn nested_list_items_step_in() {
    let reader = kitchen_sink();

    let column = reader.column().left;
    let item = reader.text_area("second bullet").left;
    let nested = reader.text_area("nested bullet").left;

    assert!(
        item > column,
        "a list item at {item} is not inside {column}"
    );
    assert!(
        nested > item,
        "a nested item at {nested} is not inside {item}"
    );
}

#[test]
fn an_ordered_list_is_numbered() {
    let reader = kitchen_sink();

    reader.assert_sees("1.");
    reader.assert_sees("2.");
    reader.assert_sees("step two");
}

#[test]
fn a_table_honors_the_alignment_its_columns_ask_for() {
    let reader = kitchen_sink();

    let left_head = reader.text_area("Fruit");
    let left_cell = reader.text_area("apple");
    let right_head = reader.text_area("Count");
    let right_cell = reader.text_area("3");

    assert!((left_head.left - left_cell.left).abs() < NEAR);
    assert!((right_head.right() - right_cell.right()).abs() < NEAR);
    assert!(right_cell.left > left_cell.right(), "the columns overlap");
}

#[test]
fn a_code_block_keeps_its_lines_on_a_surface_of_its_own() {
    let reader = kitchen_sink();
    reader.assert_sees("fn answer() -> u32 {\n    42\n}");
    let code = reader.text_area("fn answer() -> u32 {\n    42\n}");

    // Just right of the code's last glyphs, still inside the block.
    let surface = reader.color_at(code.right() + 20.0, code.top + code.height / 2.0);

    assert_ne!(surface, reader.page_color());
}

#[test]
fn a_link_is_inked_differently_from_the_text_around_it() {
    let reader = kitchen_sink();

    assert_ne!(
        reader.ink_of("link to the guide"),
        reader.ink_of("An opening paragraph")
    );
}

#[test]
fn a_quote_steps_in_from_the_column_edge() {
    let mut reader = kitchen_sink();
    reader.click("Code");

    let quote = reader.text_area("A quoted remark.");

    assert!(quote.left > reader.column().left);
}

#[test]
fn an_inline_formula_is_a_picture_between_its_words() {
    let mut reader = kitchen_sink();
    reader.click("Math");

    let before = reader.text_area("Inline ");
    let after = reader.text_area(" in a sentence.");
    let pictures = reader.pictures();
    let formula = pictures.iter().find(|picture| {
        (picture.left - before.right()).abs() < NEAR && (picture.right() - after.left).abs() < NEAR
    });

    assert!(
        formula.is_some(),
        "no formula between the words: {pictures:?}"
    );
    reader.assert_hides("a^2");
}

#[test]
fn a_display_formula_is_a_picture_centered_in_the_column() {
    let mut reader = kitchen_sink();
    reader.click("Math");

    let sentence = reader.text_area(" in a sentence.");
    let heading = reader.text_area("Diagram");
    let column = reader.column();
    let pictures = reader.pictures();
    let formula = pictures
        .iter()
        .find(|picture| picture.top > sentence.bottom() && picture.bottom() < heading.top);

    let Some(formula) = formula else {
        panic!("no formula between the sentence and the next heading: {pictures:?}");
    };
    let middle = formula.left + formula.width / 2.0;
    assert!((middle - (column.left + column.width / 2.0)).abs() < NEAR);
    reader.assert_hides("\\int");
}

#[test]
fn a_mermaid_block_is_drawn_as_a_diagram() {
    let mut reader = kitchen_sink();
    reader.click("Diagram");
    reader.wait_for("the diagram to be drawn", |reader| {
        let heading = reader.text_area("Diagram");
        let pictures = reader.pictures();
        pictures
            .iter()
            .any(|picture| picture.top > heading.bottom())
    });

    let heading = reader.text_area("Diagram");
    let next = reader.text_area("Footnote");
    let column = reader.column();
    let pictures = reader.pictures();
    let diagram = pictures
        .iter()
        .find(|picture| picture.top > heading.bottom() && picture.bottom() < next.top);

    let Some(diagram) = diagram else {
        panic!("no diagram between its heading and the next: {pictures:?}");
    };
    assert!(diagram.width > 100.0, "a two-node flowchart {diagram:?}");
    assert!(diagram.left >= column.left && diagram.right() <= column.right());
    reader.assert_hides("flowchart LR");
}

#[test]
fn footnotes_strikethrough_and_emoji_shortcodes_are_rendered() {
    let mut reader = kitchen_sink();
    reader.click("Footnote");

    reader.assert_sees("The note itself.");
    reader.assert_sees("struck out");
    reader.assert_sees("\u{1f4cc}");
    for markup in ["[^1]", "~~", ":pushpin:"] {
        reader.assert_hides(markup);
    }
}

#[test]
fn text_stays_inside_the_column() {
    let paragraph = "word ".repeat(400);
    let reader = Reader::with_docs(&[("wide.md", &format!("# Wide\n\n{paragraph}"))]);

    let column = reader.column();
    let text = reader.text_area(paragraph.trim_end());

    assert!(text.left >= column.left - NEAR);
    assert!(text.right() <= column.right() + NEAR);
    assert!(text.height > 100.0, "the paragraph did not wrap: {text:?}");
}
