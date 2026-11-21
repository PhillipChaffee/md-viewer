//! Hover texts: whatever one names, a button of the top row, a file of
//! the sidebar or a picture of the document, it is set as the window's
//! own text is, and no larger than the control it names.

use super::pages::{illustrated, picture_between};
use super::support::Reader;

/// A file name too long for a row of the files sidebar to show whole.
const LONG_NAME: &str =
    "the-minutes-of-every-meeting-held-in-the-east-wing-since-the-year-the-roof-was-mended.md";
const DESCRIPTION: &str = "the east wing, seen from the yard";
const SITE: &str = "https://example.com/the-east-wing";
/// How far apart, in pixels, two sizes may be and still be one size.
const NEAR: f32 = 0.01;

/// A hover text is set at the size of the row.
#[track_caller]
fn assert_same_size(hover_text: f32, row: f32) {
    assert!(
        (hover_text - row).abs() < NEAR,
        "a hover text at {hover_text} px, a row at {row} px"
    );
}

/// A page with a picture and a link, beside a file with a long name, and
/// the files sidebar showing.
fn desk() -> Reader {
    let page = format!(
        "# Desk\n\nbefore\n\n![{DESCRIPTION}](blue.png)\n\nafter\n\nRead [the site]({SITE})."
    );
    let mut reader = illustrated(&page).doc(LONG_NAME, "# Minutes").open();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        !reader.sizes_of("page.md").is_empty()
    });
    reader
}

/// The size a row of the files sidebar is set at, in pixels.
#[track_caller]
fn in_a_row(reader: &Reader) -> f32 {
    let rows = reader.sizes_of("page.md");
    assert_eq!(rows.len(), 1, "page.md is painted once, in its row");
    rows[0]
}

/// The size the hover text that starts with `start` is set at, in pixels,
/// once it shows. A hover text is painted over everything else, so it is
/// the last text that reads so.
#[track_caller]
fn in_hover_text(reader: &mut Reader, start: &str) -> f32 {
    reader.wait_for("the hover text", |reader| {
        reader.all_text().iter().any(|text| text.starts_with(start))
    });
    let texts = reader.all_text().into_iter();
    let text = texts.rev().find(|text| text.starts_with(start)).unwrap();
    *reader.sizes_of(&text).last().unwrap()
}

/// The size of the hover text of the Home button.
#[track_caller]
fn over_the_home_button(reader: &mut Reader) -> f32 {
    reader.hover("Home");
    in_hover_text(reader, "Home (")
}

/// The size of the hover text of the file whose row is too narrow for its
/// name.
#[track_caller]
fn over_the_cut_off_file(reader: &mut Reader) -> f32 {
    reader.hover(LONG_NAME);
    reader.wait_for("the whole name", |reader| {
        reader.sizes_of(LONG_NAME).len() == 2
    });
    reader.sizes_of(LONG_NAME)[1]
}

/// The size of the hover text of the button at the end of a file's row,
/// which shows while the pointer is on the row.
#[track_caller]
fn over_the_row_button(reader: &mut Reader) -> f32 {
    reader.hover("page.md");
    reader.hover("Open page.md in new tab");
    in_hover_text(reader, "Open in new tab (")
}

/// The size of the hover text of the page's picture.
#[track_caller]
fn over_the_picture(reader: &mut Reader) -> f32 {
    let picture = picture_between(reader, "before", "after");
    reader.hover_point(picture.left + 20.0, picture.top + 20.0);
    in_hover_text(reader, DESCRIPTION)
}

/// The size of the hover text of the page's link.
#[track_caller]
fn over_the_link(reader: &mut Reader) -> f32 {
    reader.hover("the site");
    in_hover_text(reader, SITE)
}

#[test]
fn the_hover_text_of_a_top_row_button_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();

    assert_same_size(over_the_home_button(&mut reader), in_a_row(&reader));
}

#[test]
fn the_whole_name_of_a_cut_off_file_is_as_large_as_its_row() {
    let mut reader = desk();

    assert_same_size(over_the_cut_off_file(&mut reader), in_a_row(&reader));
}

#[test]
fn the_hover_text_of_a_rows_button_is_as_large_as_its_row() {
    let mut reader = desk();

    assert_same_size(over_the_row_button(&mut reader), in_a_row(&reader));
}

#[test]
fn the_description_of_a_picture_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();

    assert_same_size(over_the_picture(&mut reader), in_a_row(&reader));
}

#[test]
fn the_target_of_a_link_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();

    assert_same_size(over_the_link(&mut reader), in_a_row(&reader));
}

#[test]
fn the_place_of_a_tabs_file_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();
    let place = reader.path("page.md").display().to_string();

    reader.hover("tab:Desk");

    assert_same_size(in_hover_text(&mut reader, &place), in_a_row(&reader));
    reader.assert_sees("Live: reloads when the file changes");
}

#[test]
fn the_place_of_a_recent_document_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();
    let place = reader.path("page.md").display().to_string();
    reader.click("Home");

    reader.hover("Desk");

    assert_same_size(in_hover_text(&mut reader, &place), in_a_row(&reader));
}

#[test]
fn a_larger_text_size_leaves_hover_texts_as_large_as_a_sidebar_row() {
    let mut reader = desk();
    reader.click("Appearance");
    reader.click("Larger text");
    reader.click("Larger text");
    reader.press("Escape");

    assert_same_size(over_the_home_button(&mut reader), in_a_row(&reader));
    assert_same_size(over_the_picture(&mut reader), in_a_row(&reader));
}

#[test]
fn zooming_scales_hover_texts_with_the_sidebar_rows() {
    let mut reader = desk();
    let at_full_size = in_a_row(&reader);

    reader.press("Ctrl++");
    reader.press("Ctrl++");

    let row = in_a_row(&reader);
    assert!(row > at_full_size, "zooming in enlarges a row's text");
    assert_same_size(over_the_home_button(&mut reader), row);
    assert_same_size(over_the_cut_off_file(&mut reader), row);
    assert_same_size(over_the_row_button(&mut reader), row);
    assert_same_size(over_the_picture(&mut reader), row);
    assert_same_size(over_the_link(&mut reader), row);
}
