//! Formulas: where they sit, how large they are, and whose ink they are
//! drawn in.

use super::pages::{each, page, picture_between, pictures_between, read_on, NEAR, THEMES};
use super::support::{Area, Reader};

/// A page with one display formula between `before` and `after`.
fn display(latex: &str) -> Reader {
    page(&format!("# Sums\n\nbefore\n\n$$\n{latex}\n$$\n\nafter"))
}

/// The display formula of a [`display`] page.
fn formula(reader: &mut Reader) -> Area {
    picture_between(reader, "before", "after")
}

/// The formulas written inside the paragraph that starts with `opening`.
fn in_line(reader: &mut Reader, opening: &str) -> Vec<Area> {
    reader.wait_for("the formulas to be drawn", |reader| {
        !reader.pictures().is_empty()
    });
    let line = reader.text_area(opening);
    let mut pictures = reader.pictures();
    pictures.retain(|picture| picture.top < line.bottom() && picture.bottom() > line.top);
    pictures.sort_by(|a, b| a.left.total_cmp(&b.left));
    pictures
}

#[test]
fn a_formula_is_drawn_in_the_ink_of_the_text_on_every_theme() {
    let mut reader = display("\\int_0^1 x^2 \\, dx");

    for theme in THEMES {
        read_on(&mut reader, theme);
        let text = reader.ink_of("before");
        reader.wait_for("the formula to take the theme's ink", |reader| {
            let pictures = reader.pictures();
            pictures
                .iter()
                .any(|picture| reader.picture_ink(*picture) == text)
        });

        let picture = formula(&mut reader);
        let ink = reader.picture_ink(picture);
        let contrast = ink.contrast(reader.page_color());
        assert!(contrast >= 7.0, "{theme}: a formula at {contrast:.2}:1");
    }
}

#[test]
fn a_formula_is_strokes_on_the_page_and_not_a_box() {
    let mut reader = display("\\int_0^1 x^2 \\, dx");

    for theme in THEMES {
        read_on(&mut reader, theme);
        let picture = formula(&mut reader);
        let cover = reader.picture_cover(picture);
        assert!(
            cover > 0.005 && cover < 0.3,
            "{theme}: {cover} of it is inked"
        );
    }
}

#[test]
fn a_display_formula_has_a_row_to_itself() {
    let mut reader = display("a^2 + b^2 = c^2");

    let picture = formula(&mut reader);

    let column = reader.column();
    let middle = picture.left + picture.width / 2.0;
    assert!((middle - (column.left + column.width / 2.0)).abs() < NEAR);
    assert!(picture.top >= reader.text_area("before").bottom());
    assert!(picture.bottom() <= reader.text_area("after").top);
    for source in ["a^2", "$$"] {
        reader.assert_hides(source);
    }
}

#[test]
fn a_tall_formula_takes_the_height_it_needs() {
    let mut reader = display("x + y");
    let short = formula(&mut reader);
    let gap = reader.text_area("after").top - reader.text_area("before").bottom();

    let mut reader = display("\\frac{\\frac{a}{b} + \\sum_{i=1}^{n} i}{\\frac{c}{d}}");
    let tall = formula(&mut reader);
    let tall_gap = reader.text_area("after").top - reader.text_area("before").bottom();

    assert!(tall.height > short.height * 2.0, "{short:?} then {tall:?}");
    assert!(
        tall_gap > gap + (tall.height - short.height) - 4.0,
        "the text made room"
    );
    assert!(tall.bottom() <= reader.text_area("after").top);
}

#[test]
fn a_formula_inside_a_sentence_sits_on_its_line() {
    let mut reader = page("# Sums\n\nThe sum $a^2 + b^2$ is here.");

    let pictures = in_line(&mut reader, "The sum ");

    let [picture] = pictures[..] else {
        panic!("one formula in the sentence, not {pictures:?}");
    };
    let (before, after) = (reader.text_area("The sum "), reader.text_area(" is here."));
    assert!(picture.left >= before.right() - NEAR && picture.right() <= after.left + NEAR);
    assert!(picture.height <= before.height * 1.5, "{picture:?}");
}

#[test]
fn a_tall_formula_inside_a_sentence_does_not_run_into_the_lines_around_it() {
    let mut reader = page(
        "# Sums\n\nthe line above\n\nA ratio $\\frac{\\frac{a}{b}}{\\frac{c}{d}}$ here.\n\nthe line below",
    );

    let pictures = in_line(&mut reader, "A ratio ");

    let picture = pictures[0];
    assert!(picture.top >= reader.text_area("the line above").bottom() - NEAR);
    assert!(picture.bottom() <= reader.text_area("the line below").top + NEAR);
}

#[test]
fn every_formula_of_a_sentence_is_drawn() {
    let mut reader = page("# Sums\n\nFirst $a$ then $x_1$ then $d^2$ and again $a$ done.");

    reader.wait_for("four formulas", |reader| reader.pictures().len() == 4);

    let pictures = in_line(&mut reader, "First ");
    assert!(pictures
        .windows(2)
        .all(|pair| pair[0].right() <= pair[1].left));
    assert!(
        (pictures[0].width - pictures[3].width).abs() < NEAR,
        "the same formula twice"
    );
}

/// Known bug, listed in `tests/README.md`: a formula as plain as `b + c`
/// is taken for prose. It is shown as typed, less its closing dollar sign.
#[test]
#[should_panic(expected = "the sum is shown as text")]
fn a_plain_sum_between_dollar_signs_is_a_formula() {
    let mut reader = page("# Sums\n\nOnly $b + c$ here.");
    reader.idle_for(std::time::Duration::from_millis(500));

    assert!(!reader.sees("$b + c"), "the sum is shown as text");
    assert_eq!(reader.pictures().len(), 1);
}

#[test]
fn dollar_amounts_are_not_formulas() {
    let mut reader = page("# Prices\n\nIt costs $5 and $6 here, or \\$7 there.");
    reader.idle_for(std::time::Duration::from_millis(300));

    assert!(reader.pictures().is_empty(), "{:?}", reader.pictures());
    reader.assert_sees("5 and ");
    reader.assert_sees("$7");
}

#[test]
fn a_formula_in_code_stays_as_typed() {
    let mut reader = page("# Source\n\nWrite `$x^2$` for a square.\n\n```\n$$\ny = mx\n$$\n```");
    reader.idle_for(std::time::Duration::from_millis(300));

    assert!(reader.pictures().is_empty());
    reader.assert_sees("$x^2$");
    reader.assert_sees("y = mx");
}

#[test]
fn a_formula_that_cannot_be_read_is_shown_as_it_was_typed() {
    let mut reader = page("# Broken\n\nbefore\n\n$$\n\\frac{\n$$\n\nafter");
    reader.idle_for(std::time::Duration::from_millis(300));

    reader.assert_sees("\\frac{");
    assert!(reader.text_area("after").top > reader.text_area("before").bottom());
}

#[test]
fn a_formula_grows_with_the_text_size() {
    let mut reader = display("a^2 + b^2 = c^2");
    let normal = formula(&mut reader);

    reader.click("Appearance");
    reader.click("Larger text");
    reader.click("Larger text");
    reader.press("Escape");
    reader.wait_for("the formula to be drawn again", |reader| {
        reader
            .pictures()
            .iter()
            .any(|picture| picture.width > normal.width + 2.0)
    });

    let larger = formula(&mut reader);
    assert!(larger.height > normal.height, "{normal:?} then {larger:?}");
}

#[test]
fn a_formula_grows_with_the_zoom_as_the_text_does() {
    let mut reader = display("a^2 + b^2 = c^2");
    let (formula_before, text_before) = (formula(&mut reader), reader.text_area("before"));

    reader.press("Ctrl++");
    reader.wait_for("the formula at the new zoom", |reader| {
        let wider = formula_before.width * 1.2;
        reader
            .pictures()
            .iter()
            .any(|picture| picture.width > wider)
    });

    let (formula_after, text_after) = (formula(&mut reader), reader.text_area("before"));
    let grew = formula_after.width / formula_before.width;
    let text_grew = text_after.width / text_before.width;
    assert!(
        (grew - text_grew).abs() < 0.05,
        "formula {grew}, text {text_grew}"
    );
}

#[test]
fn a_formula_in_a_table_cell_is_drawn_in_its_cell() {
    let mut reader =
        page("# Table\n\nbefore\n\n| Name | Rule |\n| --- | --- |\n| square | $x^2$ |\n\nafter");

    let pictures = pictures_between(&mut reader, "before", "after");

    let [picture] = pictures[..] else {
        panic!("one formula in the table, not {pictures:?}");
    };
    let (name, rule) = (reader.text_area("square"), reader.text_area("Rule"));
    assert!(
        picture.top < name.bottom() && picture.bottom() > name.top,
        "on the row"
    );
    assert!(
        (picture.left - rule.left).abs() < 12.0,
        "under its heading: {picture:?}"
    );
}

#[test]
fn many_display_formulas_are_all_drawn() {
    let formulas = each(1..=8, |n| format!("$$\nx_{n} = {n}\n$$\n\n"));
    let mut reader = Reader::launch()
        .doc("page.md", &format!("# Many\n\nbefore\n\n{formulas}after"))
        .args(&["page.md"])
        .window(1200.0, 1400.0)
        .open();

    reader.wait_for("eight formulas", |reader| reader.pictures().len() == 8);

    let pictures = pictures_between(&mut reader, "before", "after");
    assert!(pictures
        .windows(2)
        .all(|pair| pair[0].bottom() <= pair[1].top + NEAR));
}
