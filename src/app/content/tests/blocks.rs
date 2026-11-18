//! The document's blocks as the reader sees them: the rhythm between
//! them, typeset tables, hanging list markers, code on chips, and diagrams
//! that may leave the text column.

use std::time::Duration;

use eframe::egui::{self, Shape};

use super::{column_left, reader, PANE_LEFT, PANE_RIGHT};
use crate::app::tests::Harness;
use crate::layout::READING_COLUMN_MARGIN;
use crate::test_support::{painted_shapes, painted_text_starting, wait_until};
use crate::theme::ThemeId;

/// Where the painted text starting with `prefix` is: its lines, top to
/// bottom, from the left edge of the ui it was laid out in.
fn text_rect(output: &egui::FullOutput, prefix: &str) -> egui::Rect {
    let text = painted_text_starting(output, prefix);
    text.galley.rect.translate(text.pos.to_vec2())
}

/// Space between the bottom of the text starting with `upper` and the top
/// of the one starting with `lower`.
fn gap(output: &egui::FullOutput, upper: &str, lower: &str) -> f32 {
    text_rect(output, lower).top() - text_rect(output, upper).bottom()
}

/// The body font size the document is set in.
fn em(harness: &Harness) -> f32 {
    harness.app.document_look.style.text_styles[&egui::TextStyle::Body].size
}

#[track_caller]
fn assert_about(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 0.6,
        "{what}: {actual} is not {expected}"
    );
}

#[test]
fn blocks_keep_the_readers_rhythm() {
    let mut harness = reader(
        "# Title\n\nFirst para.\n\nSecond para.\n\n## Section\n\nThird para.\n\n\
         - tight one\n- tight two\n\n> Quoted.\n\n## Last\n",
    );
    let output = harness.idle();
    let em = em(&harness);
    let rounded = |ems: f32| (ems * em).round();

    assert_about(
        gap(&output, "First", "Second"),
        rounded(0.9),
        "between paragraphs",
    );
    assert_about(
        gap(&output, "Second", "Section"),
        rounded(1.6),
        "above a heading",
    );
    assert_about(
        gap(&output, "Section", "Third"),
        rounded(0.65),
        "below a heading",
    );
    assert_about(
        gap(&output, "Third", "tight one"),
        rounded(0.9),
        "above a list",
    );
    assert_about(
        gap(&output, "tight one", "tight two"),
        rounded(0.35),
        "between tight items",
    );
    assert_about(
        gap(&output, "tight two", "Quoted"),
        rounded(1.25),
        "above a quote",
    );
    // A heading after a block gets the heading's space, not both.
    assert_about(
        gap(&output, "Quoted", "Last"),
        rounded(1.6),
        "a heading after a quote",
    );
}

/// The horizontal and the vertical lines a frame painted in the column,
/// each as its two end points and its color.
fn lines(output: &egui::FullOutput) -> (Vec<[egui::Pos2; 2]>, Vec<egui::Color32>, usize) {
    let (mut rules, mut colors, mut upright) = (Vec::new(), Vec::new(), 0);
    for shape in painted_shapes(output) {
        let Shape::LineSegment { points, stroke } = shape else {
            continue;
        };
        if stroke.width <= 0.0 || points[0].x < column_left(680.0) - 1.0 {
            continue;
        }
        if (points[0].x - points[1].x).abs() < 0.5 {
            upright += 1;
        } else {
            rules.push(points);
            colors.push(stroke.color);
        }
    }
    (rules, colors, upright)
}

#[test]
fn a_table_is_ruled_between_its_rows_in_the_themes_hairline_and_aligned() {
    let table = "| Circuit | Neurons |\n|---|---:|\n| Safety refusal | 50 |\n\
                 | Language selection | 120 |\n| Sycophancy | 85 |\n";
    let mut harness = reader(table);

    for theme in [ThemeId::Sepia, ThemeId::Dusk] {
        harness.apply(crate::app::action::AppAction::SetTheme(theme));
        harness.idle();
        let output = harness.idle();
        let palette = theme.palette();
        let (rules, colors, upright) = lines(&output);

        assert_eq!(upright, 0, "no grid lines: {theme:?}");
        assert_eq!(colors.len(), 3, "under the header and between rows");
        assert_ne!(colors[0], palette.rule, "the header rule is stronger");
        assert_eq!(colors[1..], [palette.rule, palette.rule]);
        for rule in rules {
            // Each rule runs the width of the text column.
            assert_about(rule[0].x, column_left(680.0), "rule start");
            assert!(rule[1].x - rule[0].x > 676.0, "{rule:?}");
        }
        // The first column starts with the prose; the numbers end together.
        let cell = |prefix: &str| painted_text_starting(&output, prefix);
        assert_about(cell("Safety").pos.x, column_left(680.0), "first column");
        let right = |prefix: &str| text_rect(&output, prefix).right();
        assert!((right("50") - right("120")).abs() <= 1.0);
        assert!((right("Neurons") - right("120")).abs() <= 1.0);
    }
}

/// The shapes painted below the top bar, whose icons are drawn from the
/// same circles and muted strokes the list markers use.
fn document_shapes(output: &egui::FullOutput) -> Vec<Shape> {
    painted_shapes(output)
        .into_iter()
        .filter(|shape| shape.visual_bounding_rect().top() >= crate::app::chrome::BAR_HEIGHT)
        .collect()
}

#[test]
fn a_task_item_shows_a_box_in_the_muted_color_and_no_bullet() {
    let mut harness = reader("- [x] done\n- [ ] open\n");
    let output = harness.idle();
    let muted = ThemeId::Sepia.palette().muted;
    let shapes = document_shapes(&output);

    let boxes = shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Rect(rect) if rect.stroke.color == muted))
        .count();
    let bullets = shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Circle(_)))
        .count();
    assert_eq!((boxes, bullets), (2, 0));
}

#[test]
fn bullets_and_numbers_hang_in_the_muted_color() {
    let mut harness = reader("- bullet\n\n1. numbered\n");
    let output = harness.idle();
    let muted = ThemeId::Sepia.palette().muted;

    let bullet = document_shapes(&output)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Circle(circle) => Some(circle),
            _ => None,
        })
        .expect("a bullet");
    assert_eq!(bullet.fill, muted);
    assert!(bullet.radius < 3.0, "a small bullet: {}", bullet.radius);
    let number = painted_text_starting(&output, "1.");
    assert_eq!(number.galley.job.sections[0].format.color, muted);
    // Both items start their text one slot into the column.
    let slot = (1.5 * em(&harness)).round();
    for item in ["bullet", "numbered"] {
        assert_about(
            painted_text_starting(&output, item).pos.x,
            column_left(680.0) + slot,
            item,
        );
    }
}

#[test]
fn inline_code_sits_on_a_rounded_chip_in_the_raised_color() {
    let mut harness = reader("Some `inline code` in a line.\n");
    let output = harness.idle();
    let raised = ThemeId::Sepia.palette().raised;

    let chip = painted_shapes(&output)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == raised => Some(rect),
            _ => None,
        })
        .expect("a chip behind the code");
    assert_eq!(chip.corner_radius, egui::CornerRadius::same(4));
    let line = text_rect(&output, "inline code");
    assert!(chip.rect.height() < line.height(), "shorter than the line");
    assert!(line.y_range().contains(chip.rect.center().y));
}

/// Four boxes in a row: wider than the default column, narrower than the
/// room the test window's pane leaves a figure.
const WIDE_DIAGRAM: &str = "```mermaid\nflowchart LR\n    A[Prompt text] --> B[Clean pass]\n    \
    B --> C[Perturbed pass]\n    C --> D[Compare logits]\n```\n";

/// Run frames until the document's diagram is painted, and return where.
fn diagram_rect(harness: &mut Harness) -> egui::Rect {
    let mut found = None;
    let painted = wait_until(Duration::from_secs(120), || {
        let output = harness.idle();
        found = painted_shapes(&output)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Rect(rect) if rect.fill_texture_id() != egui::TextureId::default() => {
                    Some(rect.rect)
                }
                _ => None,
            });
        found.is_some()
    });
    assert!(painted, "no diagram within two minutes");
    // The frame after the diagram arrived lays the page out around it.
    harness.idle();
    found.unwrap_or(egui::Rect::NOTHING)
}

#[test]
fn a_diagram_wider_than_the_column_uses_the_pane_centered_on_the_column() {
    let mut harness = reader(WIDE_DIAGRAM);

    let diagram = diagram_rect(&mut harness);

    let room = 2.0_f32.mul_add(-READING_COLUMN_MARGIN, PANE_RIGHT - PANE_LEFT);
    assert!(
        diagram.width() > 700.0 && diagram.width() <= room,
        "wider than the 680 px column, inside the pane's margins: {diagram:?}"
    );
    assert_about(
        diagram.center().x,
        column_left(680.0) + 340.0,
        "centered on the column",
    );

    // A column as wide as the diagram's room holds it at the same size.
    harness.apply(crate::app::action::AppAction::SetReadingWidth(
        crate::layout::ReadingWidth::new(room),
    ));
    harness.idle();
    let in_wide_column = diagram_rect(&mut harness);
    assert_about(in_wide_column.width(), diagram.width(), "its natural size");
}
