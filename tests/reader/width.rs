//! The width of the text column.

use super::support::Reader;

/// How far a width may be off and still be the width asked for.
const NEAR: f32 = 1.5;

const LEFT_EDGE: &str = "Resize text column, left edge";
const RIGHT_EDGE: &str = "Resize text column, right edge";

fn article() -> Reader {
    let body = "The sea keeps its own hours. ".repeat(40);
    Reader::with_docs(&[("article.md", &format!("# Tides\n\n{body}"))])
}

#[track_caller]
fn assert_width(reader: &Reader, expected: f32) {
    let width = reader.column().width;
    assert!(
        (width - expected).abs() < NEAR,
        "the column is {width} wide, not {expected}"
    );
}

#[test]
fn the_column_starts_680_wide_and_centered() {
    let mut reader = article();
    reader.press("Ctrl+Shift+O");

    assert_width(&reader, 680.0);
    let column = reader.column();
    let middle = column.left + column.width / 2.0;
    assert!((middle - 600.0).abs() < 8.0, "centered at {middle} of 1200");
}

#[test]
fn ctrl_alt_arrows_widen_and_narrow_one_step() {
    let mut reader = article();

    reader.press("Ctrl+Alt+Right");
    assert_width(&reader, 720.0);

    reader.press("Ctrl+Alt+Left");
    reader.press("Ctrl+Alt+Left");
    assert_width(&reader, 640.0);
}

#[test]
fn the_column_never_narrows_below_280() {
    let mut reader = article();

    for _ in 0..15 {
        reader.press("Ctrl+Alt+Left");
    }

    assert_width(&reader, 280.0);
    reader.click("Appearance");
    reader.assert_sees("280 px");
}

#[test]
fn widening_stops_at_the_window() {
    let mut reader = article();

    for _ in 0..40 {
        reader.press("Ctrl+Alt+Right");
    }

    let widest = reader.column();
    assert!(widest.width > 900.0, "{widest:?}");
    assert!(widest.left >= 0.0 && widest.right() <= 1200.0, "{widest:?}");
    reader.press("Ctrl+Alt+Right");
    assert_eq!(reader.column(), widest, "there is no wider");
}

#[test]
fn the_slider_sets_the_width() {
    let mut reader = article();
    reader.click("Appearance");
    reader.assert_sees("680 px");
    assert!((reader.number_of("Text column width") - 680.0).abs() < 1.0);

    reader.slide("Text column width", 0.0);

    assert_width(&reader, 280.0);
    reader.assert_sees("280 px");
    assert!((reader.number_of("Text column width") - 280.0).abs() < 1.0);

    reader.slide("Text column width", 0.5);
    let halfway = reader.column().width;
    assert!(halfway > 400.0 && halfway < 900.0, "halfway is {halfway}");
}

#[test]
fn fit_window_fills_the_pane_and_reset_goes_back() {
    let mut reader = article();
    reader.press("Ctrl+Shift+O");
    reader.click("Appearance");

    reader.click("Fit window");
    reader.assert_sees("Fits the window");
    assert!(reader.column().width > 1100.0, "{:?}", reader.column());

    reader.click("Reset width");
    reader.assert_sees("680 px");
    assert_width(&reader, 680.0);
}

#[test]
fn full_width_in_the_menu_is_the_same_switch() {
    let mut reader = article();
    reader.click("More");
    assert!(!reader.is_on("Full Width"));

    reader.click("Full Width");
    assert!(reader.column().width > 900.0, "{:?}", reader.column());
    reader.click("More");
    assert!(reader.is_on("Full Width"));

    reader.click("Full Width");
    assert_width(&reader, 680.0);
}

#[test]
fn dragging_an_edge_resizes_the_column() {
    let mut reader = article();

    reader.drag(RIGHT_EDGE, 60.0, 0.0);
    let wider = reader.column().width;
    assert!(wider > 700.0, "dragging out left it {wider} wide");

    reader.drag(LEFT_EDGE, 150.0, 0.0);
    let narrower = reader.column().width;
    assert!(
        narrower < wider - 100.0,
        "dragging in left it {narrower} wide"
    );
}

#[test]
fn double_clicking_an_edge_goes_back_to_the_default() {
    let mut reader = article();
    for _ in 0..5 {
        reader.press("Ctrl+Alt+Left");
    }
    assert_width(&reader, 480.0);

    reader.double_click(RIGHT_EDGE);

    assert_width(&reader, 680.0);
}

#[test]
fn the_text_is_laid_out_again_for_the_new_width() {
    let mut reader = article();
    let body = "The sea keeps its own hours. ".repeat(40);
    let tall = reader.text_area(body.trim_end()).height;

    for _ in 0..10 {
        reader.press("Ctrl+Alt+Left");
    }

    let column = reader.column();
    let text = reader.text_area(body.trim_end());
    assert!(
        text.right() <= column.right() + NEAR,
        "{text:?} in {column:?}"
    );
    assert!(text.height > tall, "a narrower column is not a taller text");
}

#[test]
fn the_width_is_remembered() {
    let mut reader = article();
    for _ in 0..4 {
        reader.press("Ctrl+Alt+Left");
    }
    assert_width(&reader, 520.0);

    let reader = reader.restart(&["article.md"]);

    assert_width(&reader, 520.0);
}

#[test]
fn fitting_the_window_is_remembered_and_follows_the_window() {
    let mut reader = article();
    reader.click("Appearance");
    reader.click("Fit window");
    let before = reader.column().width;

    let mut reader = reader.restart(&["article.md"]);
    assert!((reader.column().width - before).abs() < NEAR);

    reader.resize_window(900.0, 700.0);
    let column = reader.column();
    assert!(column.width < before - 50.0, "{column:?}");
    assert!(column.right() <= 900.0, "{column:?}");
    assert!(column.width > 680.0, "{column:?}");
}
