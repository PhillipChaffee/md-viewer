//! Tables: alignment, rules, fitting the column, and tables written as
//! HTML.

use super::pages::{between, each, page, NEAR};
use super::support::{Area, Reader};

const ALIGNED: &str = "# Stock\n\n\
| Item | Kind | Count |\n\
| :--- | :---: | ---: |\n\
| bolt | steel fastener | 7 |\n\
| washer, extra wide | zinc | 12000 |\n\n\
after the table";

/// A table of `columns` columns whose cells are too long to share one
/// column of text.
fn wide_table(columns: usize) -> String {
    let row = |name: &str| {
        let cells: Vec<String> = (1..=columns).map(|n| format!("{name}{n}")).collect();
        format!("| {} |\n", cells.join(" | "))
    };
    let rule = format!("|{}\n", " --- |".repeat(columns));
    format!(
        "# Wide\n\nbefore\n\n{}{rule}{}\nafter",
        row("unbreakableheading"),
        row("unbreakablecontent")
    )
}

fn middle(area: Area) -> f32 {
    area.left + area.width / 2.0
}

#[test]
fn a_left_column_lines_up_on_its_left_edge() {
    let reader = page(ALIGNED);

    let lefts = ["Item", "bolt", "washer, extra wide"].map(|cell| reader.text_area(cell).left);

    assert!((lefts[0] - lefts[1]).abs() < NEAR, "{lefts:?}");
    assert!((lefts[1] - lefts[2]).abs() < NEAR, "{lefts:?}");
}

#[test]
fn a_right_column_lines_up_on_its_right_edge() {
    let reader = page(ALIGNED);

    let rights = ["Count", "7", "12000"].map(|cell| reader.text_area(cell).right());

    assert!((rights[0] - rights[1]).abs() < NEAR, "{rights:?}");
    assert!((rights[1] - rights[2]).abs() < NEAR, "{rights:?}");
}

#[test]
fn a_centered_column_lines_up_on_its_middle() {
    let reader = page(ALIGNED);

    let middles = ["Kind", "steel fastener", "zinc"].map(|cell| middle(reader.text_area(cell)));

    assert!((middles[0] - middles[1]).abs() < NEAR, "{middles:?}");
    assert!((middles[1] - middles[2]).abs() < NEAR, "{middles:?}");
}

#[test]
fn the_cells_of_a_row_share_a_line_and_rows_follow_each_other() {
    let reader = page(ALIGNED);

    let row = ["bolt", "steel fastener", "7"].map(|cell| reader.text_area(cell));
    let next = reader.text_area("zinc");

    assert!((row[0].top - row[1].top).abs() < NEAR);
    assert!((row[1].top - row[2].top).abs() < NEAR);
    assert!(row[0].right() <= row[1].left && row[1].right() <= row[2].left);
    assert!(reader.text_area("Kind").bottom() <= row[1].top);
    assert!(row[1].bottom() <= next.top);
}

#[test]
fn a_hairline_runs_between_every_two_rows() {
    let reader = page(ALIGNED);

    for (above, below) in [("Item", "bolt"), ("bolt", "washer, extra wide")] {
        let lines = reader.marks_in(between(&reader, above, below));
        let hairline = lines
            .iter()
            .find(|line| line.height < 3.0 && line.width > 100.0);
        assert!(hairline.is_some(), "between {above} and {below}: {lines:?}");
    }
}

#[test]
fn the_table_ends_before_the_text_after_it() {
    let reader = page(ALIGNED);

    let last = reader.text_area("12000");
    let after = reader.text_area("after the table");

    assert!(after.top > last.bottom(), "{after:?} under {last:?}");
    assert!((after.left - reader.column().left).abs() < NEAR);
}

#[test]
fn a_table_stays_inside_the_column() {
    let reader = page(ALIGNED);
    let column = reader.column();

    for cell in ["Item", "Count", "12000", "washer, extra wide"] {
        let area = reader.text_area(cell);
        assert!(area.left >= column.left - NEAR, "{cell}: {area:?}");
        assert!(area.right() <= column.right() + NEAR, "{cell}: {area:?}");
    }
}

#[test]
fn a_narrow_column_wraps_the_cells_instead_of_cutting_them() {
    let mut reader = page(ALIGNED);
    let roomy = reader.text_area("washer, extra wide").height;

    for _ in 0..10 {
        reader.press("Ctrl+Alt+Left");
    }

    let column = reader.column();
    let cramped = reader.text_area("washer, extra wide");
    assert!(cramped.right() <= column.right() + NEAR, "{cramped:?}");
    assert!(reader.text_area("12000").right() <= column.right() + NEAR);
    assert!(cramped.height > roomy, "the cell did not wrap");
}

#[test]
fn a_table_is_fitted_again_when_the_column_gets_its_width_back() {
    let mut reader = page(ALIGNED);
    let cells = [
        "Item",
        "Kind",
        "Count",
        "washer, extra wide",
        "zinc",
        "12000",
    ];
    let before = cells.map(|cell| reader.text_area(cell));

    for _ in 0..10 {
        reader.press("Ctrl+Alt+Left");
    }
    assert_ne!(reader.text_area("12000"), before[5], "narrowing moved it");
    for _ in 0..10 {
        reader.press("Ctrl+Alt+Right");
    }

    assert_eq!(cells.map(|cell| reader.text_area(cell)), before);
}

#[test]
fn a_table_is_fitted_again_after_a_trip_to_the_fitted_window() {
    let mut reader = page(ALIGNED);
    let before = reader.text_area("12000");

    reader.click("More");
    reader.click("Full Width");
    reader.click("More");
    reader.click("Full Width");

    assert_eq!(reader.text_area("12000"), before);
}

#[test]
fn a_table_too_wide_for_the_column_does_not_widen_the_page() {
    let reader = page(&wide_table(14));
    let column = reader.column();

    let first = reader.text_area("unbreakableheading1");
    let after = reader.text_area("after");

    assert!(first.left >= column.left - NEAR, "{first:?}");
    assert!((after.left - column.left).abs() < NEAR, "{after:?}");
    assert!((column.width - 680.0).abs() < NEAR, "{column:?}");
}

#[test]
fn a_table_too_wide_for_the_column_scrolls_sideways_inside_it() {
    let mut reader = page(&wide_table(14));
    let column = reader.column();
    let last = reader.text_area("unbreakablecontent14");
    assert!(
        last.left > column.right(),
        "the last cell is out of the column"
    );
    let first = reader.text_area("unbreakablecontent1");

    reader.sideways_wheel_at(first.left + 40.0, first.top + 10.0, 400.0);

    let moved = reader.text_area("unbreakablecontent14");
    assert!(moved.left < last.left - 100.0, "{last:?} then {moved:?}");
    assert!((reader.text_area("after").left - column.left).abs() < NEAR);
    assert_eq!(reader.column(), column);
}

#[test]
fn a_table_written_as_html_is_laid_out_as_a_table() {
    let reader = page(
        "# Parts\n\n<table>\n<tr><th>Name</th><th>Qty</th></tr>\n\
         <tr><td>bolt</td><td>7</td></tr>\n<tr><td>nut</td><td>9</td></tr>\n</table>\n\nafter",
    );

    let [name, qty, bolt, seven, nut] =
        ["Name", "Qty", "bolt", "7", "nut"].map(|cell| reader.text_area(cell));

    assert!((name.top - qty.top).abs() < NEAR && (bolt.top - seven.top).abs() < NEAR);
    assert!((name.left - bolt.left).abs() < NEAR && (bolt.left - nut.left).abs() < NEAR);
    assert!((qty.left - seven.left).abs() < NEAR);
    assert!(qty.left > name.right() && bolt.top > name.bottom() && nut.top > bolt.bottom());
    for markup in ["<table>", "<tr>", "<td>", "</td>"] {
        reader.assert_hides(markup);
    }
}

#[test]
fn an_html_table_has_hairlines_like_any_other() {
    let reader = page(
        "# Parts\n\n<table><tr><th>Name</th><th>Qty</th></tr>\
         <tr><td>bolt</td><td>7</td></tr></table>\n\nafter",
    );

    let lines = reader.marks_in(between(&reader, "Name", "bolt"));

    assert!(
        lines
            .iter()
            .any(|line| line.height < 3.0 && line.width > 100.0),
        "{lines:?}"
    );
}

#[test]
fn cells_keep_their_inline_styles() {
    let mut reader = Reader::with_docs(&[
        (
            "page.md",
            "# Mixed\n\n| What | How |\n| --- | --- |\n| `cargo test` | see [the guide](guide.md) |\n| ~~old way~~ | **new way** |",
        ),
        ("guide.md", "# Guide\n\nThe guide itself."),
    ]);
    let code = reader.text_area("cargo test");
    let chip = reader.color_at(code.left + 2.0, code.top + code.height / 2.0);

    assert_ne!(chip, reader.page_color(), "inline code keeps its chip");
    assert!(reader.is_struck_out("old way"));
    reader.assert_hides("**");

    reader.click("the guide");
    reader.assert_sees("The guide itself.");
}

#[test]
fn an_escaped_bar_is_a_bar_in_the_cell() {
    let reader = page("# Bars\n\n| Expr | Means |\n| --- | --- |\n| a \\| b | either |");

    reader.assert_hides("\\|");
    let (expr, means) = (reader.text_area("| b"), reader.text_area("either"));
    assert!(
        (expr.top - means.top).abs() < NEAR,
        "still two cells of one row"
    );
}

#[test]
fn an_empty_cell_keeps_the_others_in_their_columns() {
    let reader =
        page("# Gaps\n\n| A | B | C |\n| --- | --- | --- |\n| a1 |  | c1 |\n| a2 | b2 | c2 |");

    let (c_head, c1, c2) = (
        reader.text_area("C").left,
        reader.text_area("c1").left,
        reader.text_area("c2").left,
    );

    assert!((c_head - c1).abs() < NEAR && (c1 - c2).abs() < NEAR);
    assert!(reader.text_area("b2").left < c2);
}

#[test]
fn two_tables_are_fitted_each_on_its_own() {
    let reader = page(
        "# Two\n\n| A | B |\n| --- | --- |\n| short | x |\n\nbetween\n\n\
         | C | D |\n| --- | --- |\n| a much longer first cell | y |",
    );

    let (x, y) = (reader.text_area("x"), reader.text_area("y"));

    assert!(y.top > reader.text_area("between").bottom());
    assert!(
        y.left > x.left + 20.0,
        "the second table has its own columns"
    );
}

#[test]
fn a_table_of_many_rows_keeps_every_row() {
    let rows = each(1..=40, |n| format!("| row {n} | value {n} |\n"));
    let mut reader = page(&format!(
        "# Many\n\n| Name | Value |\n| --- | --- |\n{rows}\nthe end"
    ));

    for _ in 0..12 {
        reader.press("PageDown");
    }

    assert!(reader.in_view("value 40"));
    assert!(reader.in_view("the end"));
    let (name, value) = (reader.text_area("row 40"), reader.text_area("value 40"));
    assert!((name.top - value.top).abs() < NEAR);
}
