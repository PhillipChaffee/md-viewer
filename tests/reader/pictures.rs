//! Pictures in a document: which formats are shown, at what size, and what
//! a picture that cannot be shown leaves in its place.

use super::pages::{illustrated, picture_between, pictures_between, NEAR};
use super::support::{Area, Reader, Rgb};

/// How far a channel of a lossy picture may be off its true color.
const LOSSY: u8 = 8;

/// A page with `markup` between a paragraph `before` and a paragraph
/// `after`.
fn framed(markup: &str) -> Reader {
    illustrated(&format!("# Gallery\n\nbefore\n\n{markup}\n\nafter")).open()
}

/// The one picture on a [`framed`] page.
#[track_caller]
fn shown(reader: &mut Reader) -> Area {
    picture_between(reader, "before", "after")
}

#[track_caller]
fn assert_color(actual: Rgb, expected: &str) {
    let expected = Rgb::hex(expected);
    let off = [
        (actual.0, expected.0),
        (actual.1, expected.1),
        (actual.2, expected.2),
    ]
    .map(|(a, b)| a.abs_diff(b));
    assert!(
        off.iter().all(|off| *off <= LOSSY),
        "{actual:?} is not {expected:?}"
    );
}

#[test]
fn each_kind_of_picture_is_shown_at_its_own_size_in_its_own_colors() {
    let kinds = [
        ("blue.png", (120.0, 80.0), "#3366CC"),
        ("red.jpg", (90.0, 60.0), "#CC3333"),
        ("shape.svg", (200.0, 100.0), "#22AA44"),
    ];

    for (file, (width, height), color) in kinds {
        let mut reader = framed(&format!("![a picture]({file})"));

        let picture = shown(&mut reader);

        assert!((picture.width - width).abs() < NEAR, "{file}: {picture:?}");
        assert!(
            (picture.height - height).abs() < NEAR,
            "{file}: {picture:?}"
        );
        assert_color(reader.picture_ink(picture), color);
        assert!(
            reader.picture_cover(picture) > 0.99,
            "{file} is a filled box"
        );
        reader.assert_hides("a picture");
    }
}

#[test]
fn a_gif_is_shown() {
    let mut reader = framed("![a picture](green.gif)");
    reader.idle_for(std::time::Duration::from_millis(500));

    let pictures = reader.pictures();

    assert!(!pictures.is_empty(), "the GIF is not shown");
    assert_color(reader.picture_ink(pictures[0]), "#33AA55");
}

#[test]
fn a_picture_starts_at_the_left_edge_of_the_column_between_its_neighbors() {
    let mut reader = framed("![a picture](blue.png)");

    let picture = shown(&mut reader);

    assert!(
        (picture.left - reader.column().left).abs() < NEAR,
        "{picture:?}"
    );
    assert!(picture.top > reader.text_area("before").bottom());
    assert!(picture.bottom() < reader.text_area("after").top);
}

#[test]
fn a_picture_wider_than_the_column_is_scaled_down_to_fit_it() {
    let mut reader = framed("![a picture](wide.png)");
    let column = reader.column();

    let picture = shown(&mut reader);

    assert!(
        (picture.width - column.width).abs() < NEAR,
        "{picture:?} in {column:?}"
    );
    assert!(
        (picture.width / picture.height - 8.0).abs() < 0.1,
        "1600 by 200 is 8 to 1"
    );
}

#[test]
fn a_scaled_picture_follows_the_column_as_it_narrows_and_widens() {
    let mut reader = framed("![a picture](wide.png)");
    let before = shown(&mut reader);

    for _ in 0..5 {
        reader.press("Ctrl+Alt+Left");
    }
    let narrow = shown(&mut reader);
    for _ in 0..5 {
        reader.press("Ctrl+Alt+Right");
    }

    assert!(
        (narrow.width - reader.column().width + 200.0).abs() < NEAR,
        "{narrow:?}"
    );
    assert!((narrow.width / narrow.height - 8.0).abs() < 0.1);
    assert_eq!(shown(&mut reader), before);
}

#[test]
fn a_small_picture_is_not_stretched_to_fill_the_column() {
    let mut reader = framed("![a picture](blue.png)");

    reader.click("More");
    reader.click("Full Width");

    let picture = shown(&mut reader);
    assert!((picture.width - 120.0).abs() < NEAR, "{picture:?}");
}

#[test]
fn zooming_the_window_zooms_its_pictures() {
    let mut reader = framed("![a picture](blue.png)");
    let before = shown(&mut reader);

    reader.press("Ctrl++");

    let after = shown(&mut reader);
    assert!(
        (after.width / before.width - 1.25).abs() < 0.02,
        "{after:?}"
    );
    assert!(
        (after.height / before.height - 1.25).abs() < 0.02,
        "{after:?}"
    );
}

#[test]
fn pictures_written_on_one_line_sit_side_by_side() {
    let mut reader = framed("![one](blue.png) ![two](shape.svg)");

    let pictures = pictures_between(&mut reader, "before", "after");

    let [left, right] = pictures[..] else {
        panic!("two pictures, not {pictures:?}");
    };
    let (left, right) = if left.left < right.left {
        (left, right)
    } else {
        (right, left)
    };
    assert!(right.left >= left.right(), "{left:?} then {right:?}");
    assert!(
        left.top < right.bottom() && right.top < left.bottom(),
        "one line"
    );
}

#[test]
fn a_picture_is_found_wherever_its_path_points() {
    let mut reader = illustrated("# Paths\n\nplaceholder")
        .bytes("art/deep/blue.png", super::pages::BLUE_PNG)
        .doc(
            "notes/inner.md",
            "# Inner\n\nbefore\n\n![up](../blue.png)\n\nafter",
        )
        .open();
    let absolute = reader.path("art/deep/blue.png");
    let ways = [
        "![a picture](art/deep/blue.png)".to_owned(),
        "![a picture](./art/../blue.png)".to_owned(),
        format!("![a picture]({})", absolute.display()),
        format!("![a picture](file://{})", absolute.display()),
    ];

    for way in ways {
        reader.write_file("page.md", &format!("# Paths\n\nbefore\n\n{way}\n\nafter"));
        reader.wait_for("the page to reload", |reader| reader.sees("before"));
        let picture = shown(&mut reader);
        assert!((picture.width - 120.0).abs() < NEAR, "{way}: {picture:?}");
    }
}

#[test]
fn a_picture_beside_a_document_in_a_subfolder_is_found_from_there() {
    let mut reader = Reader::launch()
        .doc(
            "notes/inner.md",
            "# Inner\n\nbefore\n\n![up](../blue.png)\n\n![here](local.png)\n\nafter",
        )
        .bytes("blue.png", super::pages::BLUE_PNG)
        .bytes("notes/local.png", super::pages::BLUE_PNG)
        .args(&["notes/inner.md"])
        .open();

    reader.wait_for("both pictures", |reader| reader.pictures().len() == 2);
}

#[test]
fn resting_on_a_picture_shows_what_it_is_a_picture_of() {
    let mut reader = framed("![a blue box, 120 by 80](blue.png)");
    let picture = shown(&mut reader);
    reader.assert_hides("a blue box");

    reader.hover_point(picture.left + 60.0, picture.top + 40.0);

    reader.assert_sees("a blue box, 120 by 80");
}

#[test]
fn a_picture_without_a_description_shows_nothing_when_rested_on() {
    let mut reader = framed("![](blue.png)");
    let picture = shown(&mut reader);
    let before = reader.names();

    reader.hover_point(picture.left + 60.0, picture.top + 40.0);

    assert_eq!(reader.names(), before);
}

#[test]
fn a_picture_that_is_missing_leaves_a_warning_and_the_text_around_it() {
    let mut reader = framed("![a lost picture](nowhere.png)");
    reader.wait_for("the warning sign", |reader| reader.sees("\u{26a0}"));

    assert!(reader.pictures().is_empty());
    let (before, after) = (reader.text_area("before"), reader.text_area("after"));
    assert!(after.top > before.bottom());
}

#[test]
fn resting_on_the_warning_says_which_picture_could_not_be_loaded() {
    let mut reader = framed("![a lost picture](nowhere.png)");
    reader.wait_for("the warning sign", |reader| reader.sees("\u{26a0}"));
    let before = reader.text_area("before");

    reader.hover_point(before.left + 8.0, before.bottom() + 24.0);

    reader.assert_sees("nowhere.png");
    reader.assert_sees("a lost picture");
}

#[test]
fn a_file_that_is_not_a_picture_leaves_a_warning_too() {
    let mut reader = illustrated("# Gallery\n\nbefore\n\n![not a picture](fake.png)\n\nafter")
        .doc("fake.png", "these are words, not pixels")
        .open();

    reader.wait_for("the warning sign", |reader| reader.sees("\u{26a0}"));

    assert!(reader.pictures().is_empty());
}

#[test]
fn a_picture_that_appears_later_is_shown_once_the_page_is_reloaded() {
    let mut reader = framed("![late](late.png)");
    reader.wait_for("the warning sign", |reader| reader.sees("\u{26a0}"));

    std::fs::write(reader.path("late.png"), super::pages::BLUE_PNG).unwrap();
    reader.write_file(
        "page.md",
        "# Gallery\n\nbefore\n\n![late](late.png)\n\nafter, edited",
    );
    reader.wait_for("the page to reload", |reader| reader.sees("after, edited"));
    reader.idle_for(std::time::Duration::from_millis(500));

    assert!(
        !reader.pictures().is_empty(),
        "the picture is still missing"
    );
}

#[test]
fn a_picture_is_loaded_over_http() {
    let site = super::support::serve(&[("blue.png", super::pages::BLUE_PNG)]);
    let mut reader = framed(&format!("![from the web]({site}/art/blue.png)"));

    let picture = shown(&mut reader);

    assert!((picture.width - 120.0).abs() < NEAR, "{picture:?}");
    assert_color(reader.picture_ink(picture), "#3366CC");
}

#[test]
fn a_picture_the_web_does_not_have_leaves_a_warning() {
    let site = super::support::serve(&[]);
    let mut reader = framed(&format!("![from the web]({site}/gone.png)"));

    reader.wait_for("the warning sign", |reader| reader.sees("\u{26a0}"));

    assert!(reader.pictures().is_empty());
}

#[test]
fn many_pictures_are_all_shown() {
    let markup = "![p](blue.png)\n\n".repeat(6);
    let mut reader = illustrated(&format!("# Gallery\n\nbefore\n\n{markup}after"))
        .window(1200.0, 1000.0)
        .open();

    let pictures = pictures_between(&mut reader, "before", "after");

    assert_eq!(pictures.len(), 6);
    let tops: Vec<f32> = pictures.iter().map(|picture| picture.top).collect();
    assert!(
        tops.windows(2).all(|pair| pair[1] >= pair[0] + 80.0),
        "{tops:?}"
    );
}
