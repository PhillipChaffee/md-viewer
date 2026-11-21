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
/// A letter every text of these tests holds.
const LETTER: char = 'e';

/// A page with a picture and a link, beside a file with a long name, and
/// the files sidebar showing.
fn desk() -> Reader {
    let page = format!(
        "# Desk\n\nbefore\n\n![{DESCRIPTION}](blue.png)\n\nafter\n\nRead [the site]({SITE})."
    );
    let mut reader = illustrated(&page).doc(LONG_NAME, "# Minutes").open();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        !reader.letter_heights("page.md", LETTER).is_empty()
    });
    reader
}

/// How tall the letter stands in a row of the files sidebar.
#[track_caller]
fn in_a_row(reader: &Reader) -> u16 {
    let rows = reader.letter_heights("page.md", LETTER);
    assert_eq!(rows.len(), 1, "page.md is painted once, in its row");
    rows[0]
}

/// How tall the letter stands in the hover text that starts with `start`,
/// once it shows. A hover text is painted over everything else, so it is
/// the last text that reads so.
#[track_caller]
fn in_hover_text(reader: &mut Reader, start: &str) -> u16 {
    reader.wait_for("the hover text", |reader| {
        reader.all_text().iter().any(|text| text.starts_with(start))
    });
    let texts = reader.all_text().into_iter();
    let text = texts.rev().find(|text| text.starts_with(start)).unwrap();
    *reader.letter_heights(&text, LETTER).last().unwrap()
}

/// How tall the letter stands in the hover text of the Home button.
#[track_caller]
fn over_the_home_button(reader: &mut Reader) -> u16 {
    reader.hover("Home");
    in_hover_text(reader, "Home (")
}

/// How tall the letter stands in the hover text of the file whose row is
/// too narrow for its name.
#[track_caller]
fn over_the_cut_off_file(reader: &mut Reader) -> u16 {
    reader.hover(LONG_NAME);
    reader.wait_for("the whole name", |reader| {
        reader.letter_heights(LONG_NAME, LETTER).len() == 2
    });
    reader.letter_heights(LONG_NAME, LETTER)[1]
}

/// How tall the letter stands in the hover text of the page's picture.
#[track_caller]
fn over_the_picture(reader: &mut Reader) -> u16 {
    let picture = picture_between(reader, "before", "after");
    reader.hover_point(picture.left + 20.0, picture.top + 20.0);
    in_hover_text(reader, DESCRIPTION)
}

/// How tall the letter stands in the hover text of the page's link.
#[track_caller]
fn over_the_link(reader: &mut Reader) -> u16 {
    reader.hover("the site");
    in_hover_text(reader, SITE)
}

#[test]
fn the_hover_text_of_a_top_row_button_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();

    assert_eq!(over_the_home_button(&mut reader), in_a_row(&reader));
}

#[test]
fn the_whole_name_of_a_cut_off_file_is_as_large_as_its_row() {
    let mut reader = desk();

    assert_eq!(over_the_cut_off_file(&mut reader), in_a_row(&reader));
}

#[test]
fn the_description_of_a_picture_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();

    assert_eq!(over_the_picture(&mut reader), in_a_row(&reader));
}

#[test]
fn the_target_of_a_link_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();

    assert_eq!(over_the_link(&mut reader), in_a_row(&reader));
}

#[test]
fn the_place_of_a_tabs_file_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();
    let place = reader.path("page.md").display().to_string();

    reader.hover("tab:Desk");

    assert_eq!(in_hover_text(&mut reader, &place), in_a_row(&reader));
    reader.assert_sees("Live: reloads when the file changes");
}

#[test]
fn the_place_of_a_recent_document_is_as_large_as_a_sidebar_row() {
    let mut reader = desk();
    let place = reader.path("page.md").display().to_string();
    reader.click("Home");

    reader.hover("Desk");

    assert_eq!(in_hover_text(&mut reader, &place), in_a_row(&reader));
}

#[test]
fn a_larger_text_size_leaves_hover_texts_as_large_as_a_sidebar_row() {
    let mut reader = desk();
    reader.click("Appearance");
    reader.click("Larger text");
    reader.click("Larger text");
    reader.press("Escape");

    assert_eq!(over_the_home_button(&mut reader), in_a_row(&reader));
    assert_eq!(over_the_picture(&mut reader), in_a_row(&reader));
}

#[test]
fn zooming_scales_hover_texts_with_the_sidebar_rows() {
    let mut reader = desk();
    let at_full_size = in_a_row(&reader);

    reader.press("Ctrl++");
    reader.press("Ctrl++");

    let row = in_a_row(&reader);
    assert!(row > at_full_size, "zooming in enlarges a row's letters");
    assert_eq!(over_the_home_button(&mut reader), row);
    assert_eq!(over_the_cut_off_file(&mut reader), row);
    assert_eq!(over_the_picture(&mut reader), row);
    assert_eq!(over_the_link(&mut reader), row);
}
