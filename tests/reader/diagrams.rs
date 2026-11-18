//! Mermaid diagrams: every kind is drawn, in the theme's colors, with room
//! beside the column when it needs it, and an error that says so when the
//! source is broken.

use super::pages::{page, picture_between, read_on, NEAR, THEMES};
use super::support::{Area, Reader};

/// One small diagram of each kind the README lists.
const KINDS: [(&str, &str); 7] = [
    ("flowchart", "flowchart TD\n    A[Start] --> B{Choice}\n    B -->|yes| C[Done]\n    B -->|no| A"),
    ("sequence", "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi"),
    ("class", "classDiagram\n    class Animal {\n        +String name\n        +eat()\n    }\n    Animal <|-- Dog"),
    ("state", "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Busy\n    Busy --> [*]"),
    ("ER", "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ ITEM : contains"),
    ("gantt", "gantt\n    title Plan\n    dateFormat YYYY-MM-DD\n    section Build\n    Draft :a1, 2024-01-01, 10d\n    Review :after a1, 5d"),
    ("pie", "pie title Pets\n    \"Dogs\" : 40\n    \"Cats\" : 35\n    \"Birds\" : 25"),
];

const SMALL: &str = "flowchart LR\n    Start --> Finish";

/// A page with one diagram of `source` between `before` and `after`.
fn drawing(source: &str) -> Reader {
    page(&format!(
        "# Drawing\n\nbefore\n\n```mermaid\n{source}\n```\n\nafter"
    ))
}

/// The diagram of a [`drawing`] page.
fn diagram(reader: &mut Reader) -> Area {
    picture_between(reader, "before", "after")
}

/// A left-to-right chain of `stations` boxes.
fn chain(stations: usize) -> String {
    let boxes: Vec<String> = (1..=stations)
        .map(|n| format!("N{n}[Station number {n}]"))
        .collect();
    format!("flowchart LR\n    {}", boxes.join(" --> "))
}

fn middle(area: Area) -> f32 {
    area.left + area.width / 2.0
}

#[test]
fn every_kind_of_diagram_the_readme_lists_is_drawn() {
    for (kind, source) in KINDS {
        let mut reader = drawing(source);

        let picture = diagram(&mut reader);

        assert!(
            picture.width > 60.0 && picture.height > 30.0,
            "{kind}: {picture:?}"
        );
        assert!(
            reader.picture_cover(picture) > 0.01,
            "{kind} is an empty sheet"
        );
        reader.assert_hides("render error");
        let first_line = source.lines().next().unwrap();
        assert!(!reader.sees(first_line), "{kind} still shows its source");
    }
}

#[test]
fn a_diagram_is_drawn_in_the_colors_of_the_theme() {
    let mut reader = drawing(SMALL);

    for theme in THEMES {
        read_on(&mut reader, theme);
        let paper = reader.page_color();
        reader.wait_for("the diagram in the theme's colors", |reader| {
            let pictures = reader.pictures();
            pictures
                .iter()
                .any(|picture| reader.picture_ink(*picture).contrast(paper) < 1.6)
        });

        let picture = diagram(&mut reader);
        let cover = reader.picture_cover(picture);
        assert!(
            cover < 0.9,
            "{theme}: the diagram is a box, {cover} of it painted"
        );
    }
}

#[test]
fn a_diagram_on_a_dark_theme_is_not_a_light_sheet() {
    let mut reader = drawing(SMALL);
    read_on(&mut reader, "Paper");
    let picture = diagram(&mut reader);
    let light = reader.picture_ink(picture);

    read_on(&mut reader, "Night");
    reader.wait_for("the diagram to be drawn for the night", |reader| {
        let pictures = reader.pictures();
        pictures
            .iter()
            .any(|picture| reader.picture_ink(*picture) != light)
    });

    let picture = diagram(&mut reader);
    let dark = reader.picture_ink(picture);
    assert!(light.luminance() > 0.6, "on Paper it is {light:?}");
    assert!(dark.luminance() < 0.1, "on Night it is {dark:?}");
}

#[test]
fn a_small_diagram_is_centered_in_the_column() {
    let mut reader = drawing(SMALL);

    let picture = diagram(&mut reader);

    let column = reader.column();
    assert!(
        (middle(picture) - middle(column)).abs() < NEAR,
        "{picture:?}"
    );
    assert!(picture.left >= column.left && picture.right() <= column.right());
}

#[test]
fn a_diagram_wider_than_the_column_uses_the_room_beside_it() {
    let mut reader = drawing(&chain(5));

    let picture = diagram(&mut reader);

    let column = reader.column();
    assert!(
        picture.width > column.width + 100.0,
        "{picture:?} in {column:?}"
    );
    assert!(
        (middle(picture) - middle(column)).abs() < NEAR,
        "still about the column"
    );
    let contents = reader.text_area("CONTENTS");
    assert!(
        picture.left >= 0.0 && picture.right() < contents.left,
        "{picture:?}"
    );
    assert!((reader.text_area("after").left - column.left).abs() < NEAR);
}

#[test]
fn a_diagram_wider_than_the_window_is_scaled_to_fit_it() {
    let mut reader = drawing(&chain(9));

    let picture = diagram(&mut reader);

    assert!(
        picture.left >= 0.0 && picture.right() <= 1200.0,
        "{picture:?}"
    );
    assert!(picture.width > reader.column().width, "{picture:?}");
    assert!(
        picture.height > 10.0,
        "still tall enough to see: {picture:?}"
    );
}

#[test]
fn a_wide_diagram_shrinks_with_a_narrow_window() {
    let mut reader = Reader::launch()
        .doc(
            "page.md",
            &format!("before\n\n```mermaid\n{}\n```\n\nafter", chain(5)),
        )
        .args(&["page.md"])
        .window(700.0, 600.0)
        .open();

    let picture = diagram(&mut reader);

    assert!(
        picture.left >= 0.0 && picture.right() <= 700.0,
        "{picture:?}"
    );
}

#[test]
fn a_diagram_too_large_for_the_screen_does_not_stop_the_app() {
    let mut reader = drawing(&chain(40));

    let picture = diagram(&mut reader);

    assert!(picture.width > 100.0);
}

#[test]
fn a_diagram_that_cannot_be_read_says_so_and_the_page_goes_on() {
    let mut reader = drawing("this is not a diagram");

    reader.wait_for("the error", |reader| reader.sees("Mermaid render error"));

    assert!(reader.pictures().is_empty());
    let (error, after) = (
        reader.text_area("before").bottom(),
        reader.text_area("after").top,
    );
    assert!(after > error + 20.0, "the error has a row of its own");
}

#[test]
fn a_broken_diagram_does_not_stop_the_ones_after_it() {
    let mut reader = page(&format!(
        "# Two\n\nbefore\n\n```mermaid\nnot a diagram\n```\n\nbetween\n\n```mermaid\n{SMALL}\n```\n\nafter"
    ));

    let picture = picture_between(&mut reader, "between", "after");

    assert!(picture.width > 100.0);
    reader.assert_sees("Mermaid render error");
}

#[test]
fn an_empty_diagram_block_does_not_stop_the_page() {
    let mut reader = drawing("");
    reader.idle_for(std::time::Duration::from_millis(400));

    reader.assert_sees("before");
    reader.assert_sees("after");
}

#[test]
fn two_diagrams_are_each_drawn_in_their_own_place() {
    let mut reader = page(&format!(
        "# Two\n\nbefore\n\n```mermaid\n{SMALL}\n```\n\nbetween\n\n```mermaid\n{}\n```\n\nafter",
        KINDS[6].1
    ));

    let first = picture_between(&mut reader, "before", "between");
    let second = picture_between(&mut reader, "between", "after");

    assert!(second.top > first.bottom());
    assert!(
        (first.height - second.height).abs() > 5.0,
        "two different diagrams"
    );
}

#[test]
fn the_same_diagram_twice_is_drawn_twice() {
    let mut reader = page(&format!(
        "# Two\n\nbefore\n\n```mermaid\n{SMALL}\n```\n\nbetween\n\n```mermaid\n{SMALL}\n```\n\nafter"
    ));

    let first = picture_between(&mut reader, "before", "between");
    let second = picture_between(&mut reader, "between", "after");

    assert!((first.width - second.width).abs() < NEAR);
    assert!((first.left - second.left).abs() < NEAR);
}

#[test]
fn a_diagram_stays_about_the_column_as_the_column_changes() {
    let mut reader = drawing(SMALL);
    let before = diagram(&mut reader);

    for _ in 0..6 {
        reader.press("Ctrl+Alt+Left");
    }

    let after = diagram(&mut reader);
    assert!((middle(after) - middle(reader.column())).abs() < NEAR);
    assert!(
        (after.width - before.width).abs() < NEAR,
        "it still fits: same size"
    );
}

#[test]
fn a_diagram_is_drawn_again_when_its_source_changes() {
    let mut reader = drawing(SMALL);
    let before = diagram(&mut reader);

    reader.write_file(
        "page.md",
        &format!(
            "# Drawing\n\nbefore\n\n```mermaid\n{}\n```\n\nafter",
            KINDS[1].1
        ),
    );
    reader.wait_for("the new diagram", |reader| {
        let pictures = reader.pictures();
        pictures.len() == 1 && (pictures[0].height - before.height).abs() > 5.0
    });
}

#[test]
fn zooming_the_window_zooms_its_diagrams() {
    let mut reader = drawing(SMALL);
    let before = diagram(&mut reader);

    reader.press("Ctrl++");

    let after = diagram(&mut reader);
    assert!(
        (after.width / before.width - 1.25).abs() < 0.03,
        "{before:?} then {after:?}"
    );
}
