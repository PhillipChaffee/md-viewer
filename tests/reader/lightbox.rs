//! The enlarged view of a picture or diagram: opening it, zooming,
//! panning, closing, and keeping the window behind it still.

use super::docs::long_document;
use super::pages::{illustrated, picture_between, NEAR};
use super::support::{Area, Reader, Rgb};

/// What the close button in the corner of the enlarged view is called.
const CLOSE: &str = "\u{2715}";

/// The window every launch here has.
const WINDOW: (f32, f32) = (1200.0, 800.0);

/// A page with no headings, so no sidebar, and one picture `file` on it.
fn gallery(file: &str) -> Reader {
    illustrated(&format!(
        "words before\n\n![a picture]({file})\n\nwords after, with [a link](other.md)"
    ))
    .doc("other.md", "# Other\n\nThe other page.")
    .open()
}

/// The picture on a [`gallery`] page, as it sits in the text.
fn in_the_text(reader: &mut Reader) -> Area {
    picture_between(reader, "words before", "words after, with ")
}

fn middle(area: Area) -> (f32, f32) {
    (area.left + area.width / 2.0, area.top + area.height / 2.0)
}

/// Click the picture on a [`gallery`] page, which enlarges it.
fn enlarge(reader: &mut Reader) {
    let (x, y) = middle(in_the_text(reader));
    reader.click_point(x, y);
    assert_eq!(reader.count(CLOSE), 1, "the picture did not enlarge");
}

/// A gallery page with its picture enlarged.
fn enlarged(file: &str) -> Reader {
    let mut reader = gallery(file);
    enlarge(&mut reader);
    reader
}

/// The enlarged picture: of the two painted, the one that reaches
/// further right than the one in the text column.
#[track_caller]
fn large(reader: &Reader) -> Area {
    assert_eq!(reader.count(CLOSE), 1, "nothing is enlarged");
    let pictures = reader.pictures();
    assert_eq!(pictures.len(), 2, "one in the text and one enlarged");
    *pictures
        .iter()
        .max_by(|a, b| (a.left + a.width).total_cmp(&(b.left + b.width)))
        .unwrap()
}

/// The middle of the room under the top bar, where an enlarged picture
/// rests.
fn room_middle(reader: &Reader) -> (f32, f32) {
    let bar = reader.area_of("More").bottom() + 6.0;
    (WINDOW.0 / 2.0, bar + (WINDOW.1 - bar) / 2.0)
}

#[track_caller]
fn assert_centered(reader: &Reader) {
    let (x, y) = middle(large(reader));
    let (room_x, room_y) = room_middle(reader);
    assert!(
        (x - room_x).abs() < 2.0 && (y - room_y).abs() < 2.0,
        "({x}, {y})"
    );
}

#[test]
fn clicking_a_picture_enlarges_it_over_a_dimmed_window() {
    let mut reader = gallery("blue.png");
    let corner = (20.0, WINDOW.1 - 20.0);
    let lit = reader.color_at(corner.0, corner.1);
    assert_eq!(reader.count(CLOSE), 0);

    enlarge(&mut reader);

    let dimmed = reader.color_at(corner.0, corner.1);
    assert!(
        dimmed.luminance() < lit.luminance() / 2.0,
        "{lit:?} then {dimmed:?}"
    );
    assert_eq!(
        reader.pictures().len(),
        2,
        "the one in the text and the large one"
    );
}

#[test]
fn the_enlarged_picture_is_centered_at_its_own_size() {
    let reader = enlarged("blue.png");

    let picture = large(&reader);

    assert!((picture.width - 120.0).abs() < NEAR && (picture.height - 80.0).abs() < NEAR);
    assert_centered(&reader);
    assert_eq!(reader.picture_ink(picture), Rgb::hex("#3366CC"));
    reader.assert_hides("100%");
}

#[test]
fn a_picture_that_was_scaled_to_fit_the_column_is_enlarged_to_its_real_size() {
    let mut reader = gallery("wide.png");
    let small = in_the_text(&mut reader);

    enlarge(&mut reader);

    let picture = large(&reader);
    assert!(
        picture.width > small.width * 1.5,
        "{small:?} then {picture:?}"
    );
    assert!((picture.width / picture.height - 8.0).abs() < 0.1);
}

#[test]
fn every_way_out_closes_the_enlarged_view() {
    type WayOut = fn(&mut Reader);
    let ways: [(&str, WayOut); 4] = [
        ("Escape", |reader| reader.press("Escape")),
        ("the close button", |reader| reader.click(CLOSE)),
        ("a click beside the picture", |reader| {
            let picture = large(reader);
            reader.click_point(picture.left - 150.0, picture.top);
        }),
        ("a click in the far corner", |reader| {
            reader.click_point(30.0, WINDOW.1 - 30.0);
        }),
    ];

    for (name, leave) in ways {
        let mut reader = enlarged("blue.png");

        leave(&mut reader);

        assert_eq!(reader.count(CLOSE), 0, "{name} left it open");
        assert_eq!(reader.pictures().len(), 1, "{name}");
        reader.assert_sees("words before");
    }
}

#[test]
fn a_click_on_the_enlarged_picture_keeps_it_open() {
    let mut reader = enlarged("blue.png");
    let (x, y) = middle(large(&reader));

    reader.click_point(x, y);

    assert_eq!(reader.count(CLOSE), 1);
    assert_centered(&reader);
}

#[test]
fn closing_puts_the_window_back_as_it_was() {
    let mut reader = gallery("blue.png");
    let corner = (20.0, WINDOW.1 - 20.0);
    let lit = reader.color_at(corner.0, corner.1);
    let before = reader.names();

    enlarge(&mut reader);
    reader.press("Escape");

    assert_eq!(reader.color_at(corner.0, corner.1), lit);
    assert_eq!(reader.names(), before);
}

#[test]
fn the_zoom_keys_step_the_enlarged_picture_by_a_quarter() {
    let mut reader = enlarged("blue.png");

    reader.press("Ctrl++");
    let picture = large(&reader);
    assert!((picture.width - 150.0).abs() < NEAR, "{picture:?}");
    reader.assert_sees("125%");

    reader.press("Ctrl+-");
    reader.press("Ctrl+-");
    let picture = large(&reader);
    assert!((picture.width - 96.0).abs() < NEAR, "{picture:?}");
    reader.assert_sees("80%");
    assert_centered(&reader);
}

#[test]
fn ctrl_zero_puts_the_enlarged_picture_back_to_its_own_size() {
    let mut reader = enlarged("blue.png");
    for _ in 0..3 {
        reader.press("Ctrl++");
    }
    reader.assert_sees("195%");

    reader.press("Ctrl+0");

    assert!((large(&reader).width - 120.0).abs() < NEAR);
    reader.assert_hides("%");
}

#[test]
fn the_wheel_zooms_the_enlarged_picture_in_and_out() {
    let mut reader = enlarged("blue.png");
    let (x, y) = middle(large(&reader));

    reader.wheel_up_at(x, y, 3.0);
    let bigger = large(&reader).width;
    reader.wheel_down_at(x, y, 6.0);
    let smaller = large(&reader).width;

    assert!(bigger > 120.0 + NEAR, "wheel up left it {bigger} wide");
    assert!(smaller < 120.0 - NEAR, "wheel down left it {smaller} wide");
    reader.assert_sees("%");
}

#[test]
fn the_enlarged_picture_zooms_between_a_tenth_and_ten_times() {
    let mut reader = enlarged("blue.png");

    for _ in 0..14 {
        reader.press("Ctrl++");
    }
    reader.assert_sees("1000%");
    assert!((large(&reader).height - 800.0).abs() < NEAR);

    for _ in 0..30 {
        reader.press("Ctrl+-");
    }
    reader.assert_sees("10%");
    assert!((large(&reader).width - 12.0).abs() < NEAR);
}

#[test]
fn a_picture_larger_than_the_room_is_moved_by_dragging_it() {
    let mut reader = enlarged("wide.png");
    let before = large(&reader);
    assert!(before.width > WINDOW.0, "wider than the window: {before:?}");
    let (x, y) = room_middle(&reader);

    reader.drag_point(x, y, -200.0, 0.0);

    let after = large(&reader);
    assert!(
        (before.left - after.left - 200.0).abs() < 3.0,
        "{before:?} then {after:?}"
    );
    assert!(
        (after.top - before.top).abs() < NEAR,
        "it only moves sideways"
    );
    assert_eq!(reader.count(CLOSE), 1, "a drag is not a click");
}

#[test]
fn dragging_stops_at_the_edge_of_the_picture() {
    let mut reader = enlarged("wide.png");
    let (x, y) = room_middle(&reader);

    for _ in 0..4 {
        reader.drag_point(x, y, 500.0, 0.0);
    }

    let picture = large(&reader);
    assert!(picture.left <= 60.0 && picture.left >= 0.0, "{picture:?}");
}

#[test]
fn a_double_click_puts_the_picture_back_in_the_middle_at_its_own_size() {
    let mut reader = enlarged("blue.png");
    for _ in 0..10 {
        reader.press("Ctrl++");
    }
    let (x, y) = room_middle(&reader);
    reader.drag_point(x, y, -150.0, -80.0);
    let moved = large(&reader);

    reader.double_click_point(x, y);

    let picture = large(&reader);
    assert_ne!(picture, moved);
    assert!((picture.width - 120.0).abs() < NEAR, "{picture:?}");
    assert_centered(&reader);
    reader.assert_hides("%");
}

#[test]
fn enlarging_again_starts_at_the_pictures_own_size() {
    let mut reader = enlarged("blue.png");
    for _ in 0..3 {
        reader.press("Ctrl++");
    }
    reader.press("Escape");

    enlarge(&mut reader);

    assert!((large(&reader).width - 120.0).abs() < NEAR);
    assert_centered(&reader);
    reader.assert_hides("%");
}

#[test]
fn the_enlarged_picture_is_centered_again_after_the_window_is_resized() {
    let mut reader = enlarged("blue.png");
    let bar = room_middle(&reader).1.mul_add(2.0, -WINDOW.1);

    reader.resize_window(900.0, 600.0);
    // The picture takes its new place on the next thing the reader does.
    reader.hover_point(30.0, 500.0);

    let (x, y) = middle(large(&reader));
    assert!((x - 450.0).abs() < 2.0, "{x}");
    assert!((y - f32::midpoint(bar, 600.0)).abs() < 2.0, "{y}");
}

/// Each shortcut that acts on the document, and a question whose answer
/// changes when it does.
const DOCUMENT_SHORTCUTS: [&str; 12] = [
    "Ctrl+D",
    "Ctrl+F",
    "Ctrl+E",
    "Ctrl+W",
    "Ctrl+Shift+E",
    "Ctrl+Shift+O",
    "Ctrl+Alt+Left",
    "Ctrl+Alt+Right",
    "Ctrl+Q",
    "F5",
    "PageDown",
    "Down",
];

/// Everything about the window behind the enlarged view that a shortcut
/// could change.
fn behind(reader: &mut Reader) -> (Vec<String>, Rgb, Area, String, bool) {
    reader.click("More");
    let live = reader.is_on("Live Reload");
    reader.press("Escape");
    (
        reader.names(),
        reader.page_color(),
        reader.column(),
        reader.title().to_owned(),
        live || reader.has_quit(),
    )
}

#[test]
fn the_document_shortcuts_do_nothing_while_a_picture_is_enlarged() {
    for shortcut in DOCUMENT_SHORTCUTS {
        let mut reader = gallery("blue.png");
        let before = behind(&mut reader);
        enlarge(&mut reader);

        reader.press(shortcut);
        assert_eq!(reader.count(CLOSE), 1, "{shortcut} closed the picture");
        reader.press("Escape");

        assert_eq!(behind(&mut reader), before, "{shortcut} reached the window");
        assert!(!reader.has_quit(), "{shortcut}");
    }
}

#[test]
fn zooming_the_enlarged_picture_leaves_the_documents_zoom_alone() {
    let mut reader = gallery("blue.png");
    let before = reader.text_area("words before");
    enlarge(&mut reader);

    reader.press("Ctrl++");
    reader.press("Ctrl++");
    reader.press("Escape");

    assert_eq!(reader.text_area("words before"), before);
}

#[test]
fn the_wheel_does_not_scroll_the_page_behind_an_enlarged_picture() {
    let text = format!(
        "words before\n\n![a picture](blue.png)\n\n{}",
        long_document(40)
    );
    let mut reader = illustrated(&text).open();
    let picture = picture_between(&mut reader, "words before", "Section 1");
    reader.click_point(picture.left + 60.0, picture.top + 40.0);
    assert_eq!(reader.count(CLOSE), 1);

    reader.wheel_down_at(600.0, 400.0, 600.0);
    reader.press("Escape");

    assert!(reader.in_view("words before"), "the page moved");
}

#[test]
fn a_click_on_the_dimmed_page_does_not_reach_what_is_under_it() {
    let mut reader = enlarged("blue.png");
    let link = reader.text_area("a link");

    reader.click_point(link.left + 8.0, link.top + link.height / 2.0);

    assert_eq!(reader.count(CLOSE), 0, "the click closed the picture");
    assert_eq!(
        reader.title(),
        "page.md - Markdown Viewer",
        "and followed no link"
    );
    assert!(!reader.is_enabled("Go back"));
}

#[test]
fn the_buttons_of_the_top_bar_do_not_act_while_a_picture_is_enlarged() {
    let mut reader = enlarged("blue.png");

    reader.click("Find in document");

    assert!(
        reader.count("Close find bar") == 0,
        "the find bar opened behind the picture"
    );
}

#[test]
fn a_diagram_is_enlarged_on_a_sheet_of_the_page() {
    let mut reader = illustrated(
        "words before\n\n```mermaid\nflowchart LR\n    Start --> Finish\n```\n\nwords after",
    )
    .open();
    let small = picture_between(&mut reader, "words before", "words after");
    let paper = reader.page_color();

    reader.click_point(
        small.left + small.width / 2.0,
        small.top + small.height / 2.0,
    );

    let diagram = large(&reader);
    assert_centered(&reader);
    assert!(
        diagram.width >= small.width - NEAR,
        "{small:?} then {diagram:?}"
    );
    assert_eq!(
        reader.color_at(diagram.left + 1.0, diagram.top + 1.0),
        paper
    );
    assert_ne!(reader.color_at(diagram.left - 40.0, diagram.top), paper);
}

#[test]
fn a_formula_is_not_something_to_enlarge() {
    let mut reader = illustrated("words before\n\n$$\nx^2 + y^2\n$$\n\nwords after").open();
    let formula = picture_between(&mut reader, "words before", "words after");

    reader.click_point(
        formula.left + formula.width / 2.0,
        formula.top + formula.height / 2.0,
    );

    assert_eq!(reader.count(CLOSE), 0);
}
