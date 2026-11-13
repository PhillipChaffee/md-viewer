//! Where the text column sits: dragging its edges, windows of many sizes,
//! zoom, and how well the text stands out from the page.

use super::pages::{read_on, NEAR, THEMES};
use super::support::{Area, Reader};

const LEFT_EDGE: &str = "Resize text column, left edge";
const RIGHT_EDGE: &str = "Resize text column, right edge";
const SENTENCE: &str = "The sea keeps its own hours. ";

/// A page of one long paragraph under a heading and a subheading.
fn article() -> Launch {
    Reader::launch()
        .doc(
            "article.md",
            &format!(
                "# Tides\n\n{}\n\n## Slack water\n\nlast words",
                SENTENCE.repeat(40)
            ),
        )
        .args(&["article.md"])
}

type Launch = super::support::Launch;

fn body() -> String {
    SENTENCE.repeat(40).trim_end().to_owned()
}

fn middle(area: Area) -> f32 {
    area.left + area.width / 2.0
}

/// Where the middle of the strip `edge` is, to press on.
fn grip(reader: &Reader, edge: &str) -> (f32, f32) {
    let strip = reader.area_of(edge);
    (middle(strip), strip.top + 300.0)
}

#[test]
fn dragging_an_edge_moves_both_edges_about_the_middle() {
    let drags = [
        (RIGHT_EDGE, 60.0, 120.0),
        (RIGHT_EDGE, -70.0, -140.0),
        (LEFT_EDGE, -45.0, 90.0),
        (LEFT_EDGE, 80.0, -160.0),
    ];

    for (edge, by, change) in drags {
        let mut reader = article().open();
        let before = reader.column();

        reader.drag(edge, by, 0.0);

        let after = reader.column();
        assert!(
            (after.width - before.width - change).abs() < 2.0,
            "{edge} by {by}: {before:?} then {after:?}"
        );
        assert!(
            (middle(after) - middle(before)).abs() < NEAR,
            "{edge} by {by}"
        );
    }
}

#[test]
fn a_pill_shows_the_width_while_an_edge_is_dragged_and_goes_when_it_is_let_go() {
    let mut reader = article().open();
    let (x, y) = grip(&reader, RIGHT_EDGE);
    reader.assert_hides(" px");

    reader.press_and_drag(x, y, 50.0, 0.0);

    let width = reader.column().width;
    assert!((width - 780.0).abs() < 2.0, "{width}");
    reader.assert_sees("780 px");

    reader.let_go();
    reader.assert_hides(" px");
    assert!(
        (reader.column().width - width).abs() < NEAR,
        "the width stays"
    );
}

#[test]
fn the_pill_follows_the_pointer_as_the_drag_goes_on() {
    let mut reader = article().open();
    let (x, y) = grip(&reader, LEFT_EDGE);

    reader.press_and_drag(x, y, 100.0, 0.0);

    reader.assert_sees("480 px");
    reader.assert_hides("680 px");
    reader.let_go();
}

#[test]
fn an_edge_cannot_be_dragged_narrower_than_280_or_wider_than_the_window() {
    let mut reader = article().open();

    reader.drag(RIGHT_EDGE, -300.0, 0.0);
    assert!(
        (reader.column().width - 280.0).abs() < NEAR,
        "{:?}",
        reader.column()
    );

    reader.drag(RIGHT_EDGE, 900.0, 0.0);
    let widest = reader.column();
    assert!(widest.width > 800.0 && widest.left >= 0.0, "{widest:?}");
    assert!(widest.right() <= 1200.0, "{widest:?}");
}

#[test]
fn moving_the_pointer_up_and_down_during_a_drag_changes_nothing() {
    let mut reader = article().open();
    let before = reader.column();

    reader.drag(RIGHT_EDGE, 0.0, 200.0);

    assert_eq!(reader.column(), before);
}

#[test]
fn a_double_click_on_either_edge_goes_back_to_680() {
    for edge in [LEFT_EDGE, RIGHT_EDGE] {
        let mut reader = article().open();
        reader.drag(RIGHT_EDGE, 90.0, 0.0);
        assert!(reader.column().width > 800.0);

        reader.double_click(edge);

        assert!((reader.column().width - 680.0).abs() < NEAR, "{edge}");
    }
}

#[test]
fn the_text_is_laid_out_for_the_width_an_edge_was_dragged_to() {
    let mut reader = article().open();
    let tall = reader.text_area(&body()).height;

    reader.drag(LEFT_EDGE, 150.0, 0.0);

    let (column, text) = (reader.column(), reader.text_area(&body()));
    assert!(text.left >= column.left - NEAR && text.right() <= column.right() + NEAR);
    assert!(text.height > tall, "a narrower column is a taller text");
}

#[test]
fn a_dragged_width_is_remembered() {
    let mut reader = article().open();
    reader.drag(RIGHT_EDGE, -80.0, 0.0);
    let width = reader.column().width;

    let reader = reader.restart(&["article.md"]);

    assert!((reader.column().width - width).abs() < NEAR);
}

#[test]
fn resting_on_an_edge_draws_a_line_to_take_hold_of() {
    let mut reader = article().open();
    let strip = reader.area_of(RIGHT_EDGE);
    let around = Area {
        left: strip.left - 8.0,
        top: strip.top - 2.0,
        width: strip.width + 16.0,
        height: strip.height + 4.0,
    };
    assert!(reader.marks_in(around).is_empty());

    reader.hover(RIGHT_EDGE);

    let lines = reader.marks_in(around);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].height > 500.0 && lines[0].width < 4.0, "{lines:?}");
}

/// Window sizes from a large screen down to a sliver.
const WINDOWS: [(f32, f32); 8] = [
    (1920.0, 1080.0),
    (1440.0, 900.0),
    (1200.0, 800.0),
    (1000.0, 700.0),
    (800.0, 600.0),
    (640.0, 480.0),
    (480.0, 360.0),
    (360.0, 300.0),
];

#[test]
fn the_column_is_inside_the_window_and_the_text_inside_the_column_at_every_size() {
    for (width, height) in WINDOWS {
        let reader = article().window(width, height).open();

        let (column, text) = (reader.column(), reader.text_area(&body()));

        assert!(
            column.left >= 0.0 && column.right() <= width,
            "{width}: {column:?}"
        );
        assert!(column.width <= 680.0 + NEAR, "{width}: {column:?}");
        assert!(text.left >= column.left - NEAR, "{width}: {text:?}");
        assert!(text.right() <= column.right() + NEAR, "{width}: {text:?}");
        assert!(column.bottom() <= height + NEAR, "{width}: {column:?}");
    }
}

#[test]
fn the_column_keeps_its_680_wherever_the_window_has_room_for_it() {
    for (width, height) in WINDOWS {
        let reader = article().window(width, height).open();
        let column = reader.column();

        if width >= 800.0 {
            assert!((column.width - 680.0).abs() < NEAR, "{width}: {column:?}");
        } else {
            assert!(column.width < width, "{width}: {column:?}");
            assert!(column.width > width - 120.0, "{width}: it uses the room");
        }
    }
}

#[test]
fn the_column_is_centered_in_the_room_the_sidebars_leave() {
    for (width, height) in &WINDOWS[..3] {
        let mut reader = article().window(*width, *height).open();
        reader.press("Ctrl+Shift+E");
        reader.wait_for("the files to be listed", |reader| {
            reader.count("article.md") >= 1
        });

        let column = reader.column();
        let (files, contents) = (reader.area_of("article.md"), reader.text_area("CONTENTS"));

        assert!(
            column.left >= files.right() - NEAR,
            "{width}: {column:?} {files:?}"
        );
        assert!(
            column.right() <= contents.left + NEAR,
            "{width}: {column:?}"
        );
        if (column.width - 680.0).abs() < NEAR {
            let room = f32::midpoint(files.right(), contents.left);
            assert!((middle(column) - room).abs() < 24.0, "{width}: {column:?}");
        }
    }
}

#[test]
fn the_top_bar_keeps_its_buttons_inside_the_smallest_window() {
    let reader = article().window(360.0, 300.0).open();

    let buttons = [
        "Files sidebar",
        "Go back",
        "Go forward",
        "Appearance",
        "More",
    ]
    .map(|button| reader.area_of(button));

    for pair in buttons.windows(2) {
        assert!(pair[0].right() <= pair[1].left, "{pair:?} overlap");
    }
    assert!(buttons[0].left >= 0.0 && buttons[4].right() <= 360.0);
}

#[test]
fn the_title_stays_between_the_buttons_of_a_narrow_window() {
    let name = "a-file-name-much-too-long-to-fit-in-the-bar-of-a-narrow-window.md";
    let reader = Reader::launch()
        .doc(
            name,
            "words under no heading, so the file name is the title",
        )
        .args(&[name])
        .window(480.0, 400.0)
        .open();

    let title = reader.text_area(name);

    assert!(
        title.left > reader.area_of("Go forward").right(),
        "{title:?}"
    );
    assert!(
        title.right() < reader.area_of("Appearance").left,
        "{title:?}"
    );
    assert!(title.width > 40.0, "some of it is still shown: {title:?}");
}

#[test]
fn a_window_made_small_and_large_again_puts_everything_back() {
    let mut reader = article().open();
    let (column, text) = (reader.column(), reader.text_area(&body()));

    reader.resize_window(400.0, 300.0);
    reader.resize_window(1200.0, 800.0);
    if reader.count("CONTENTS") == 0 {
        reader.press("Ctrl+Shift+O");
    }

    assert_eq!(reader.column(), column);
    assert_eq!(reader.text_area(&body()), text);
}

#[test]
fn at_every_zoom_the_column_and_its_text_stay_inside_the_window() {
    let mut reader = article().open();
    for _ in 0..4 {
        reader.press("Ctrl+-");
    }

    for step in 0..9 {
        let (column, text) = (reader.column(), reader.text_area(&body()));
        assert!(
            column.left >= 0.0 && column.right() <= 1200.0,
            "step {step}: {column:?}"
        );
        assert!(text.left >= column.left - NEAR, "step {step}: {text:?}");
        assert!(
            text.right() <= column.right() + NEAR,
            "step {step}: {text:?}"
        );
        reader.press("Ctrl++");
    }
}

#[test]
fn zoom_scales_the_column_until_the_window_stops_it() {
    let mut reader = article().open();
    reader.press("Ctrl+Shift+O");

    reader.press("Ctrl+-");
    let small = reader.column().width;
    reader.press("Ctrl+0");
    reader.press("Ctrl++");
    let large = reader.column().width;
    for _ in 0..4 {
        reader.press("Ctrl++");
    }
    let largest = reader.column();

    assert!(
        (small - 544.0).abs() < 2.0,
        "680 at 80% is 544, not {small}"
    );
    assert!(
        (large - 850.0).abs() < 2.0,
        "680 at 125% is 850, not {large}"
    );
    assert!(
        largest.width < 1200.0 && largest.width > 850.0,
        "{largest:?}"
    );
}

#[test]
fn the_column_has_its_width_back_when_the_zoom_is_reset() {
    let mut reader = article().open();
    let before = reader.column();
    for _ in 0..5 {
        reader.press("Ctrl++");
    }

    reader.press("Ctrl+0");
    if reader.count("CONTENTS") == 0 {
        reader.press("Ctrl+Shift+O");
    }

    assert_eq!(reader.column(), before);
}

#[test]
fn a_narrowed_column_stays_narrowed_in_proportion_when_zoomed() {
    let mut reader = article().open();
    for _ in 0..5 {
        reader.press("Ctrl+Alt+Left");
    }
    assert!((reader.column().width - 480.0).abs() < NEAR);

    reader.press("Ctrl++");

    assert!(
        (reader.column().width - 600.0).abs() < 2.0,
        "{:?}",
        reader.column()
    );
    reader.click("Appearance");
    reader.assert_sees("480 px");
}

/// A page with one of each kind of body text.
const KINDS_OF_TEXT: &str = "# Kinds\n\nA plain paragraph.\n\n- a list item\n\n\
| Head cell |\n| --- |\n| a table cell |\n\n**bold words** and *slanted words*\n\n\
## A subheading\n\n1. a numbered item";

#[test]
fn every_kind_of_body_text_keeps_seven_to_one_on_every_theme() {
    let mut reader = Reader::with_docs(&[("kinds.md", KINDS_OF_TEXT)]);

    for theme in THEMES {
        read_on(&mut reader, theme);
        let paper = reader.page_color();
        for text in [
            "A plain paragraph.",
            "a list item",
            "Head cell",
            "a table cell",
            "bold words",
            "slanted words",
            "a numbered item",
        ] {
            let contrast = reader.ink_of(text).contrast(paper);
            assert!(contrast >= 7.0, "{theme}: `{text}` at {contrast:.2}:1");
        }
    }
}

#[test]
fn the_quieter_text_can_still_be_read_on_every_theme() {
    let mut reader = Reader::with_docs(&[
        (
            "kinds.md",
            "---\nauthor: R. Carson\n---\n\n# Kinds\n\n## A subheading\n\nSee [the link](other.md).\n\n> a quoted line\n\nA note.[^1]\n\n[^1]: the footnote text",
        ),
        ("other.md", "# Other"),
    ]);

    for theme in THEMES {
        read_on(&mut reader, theme);
        let paper = reader.page_color();
        for text in [
            "min read",
            "A subheading",
            "author",
            "R. Carson",
            "the link",
            "a quoted line",
            "the footnote text",
        ] {
            let contrast = reader.ink_of(text).contrast(paper);
            assert!(contrast >= 4.5, "{theme}: `{text}` at {contrast:.2}:1");
        }
    }
}
