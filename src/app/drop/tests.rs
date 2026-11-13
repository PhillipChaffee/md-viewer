use std::path::PathBuf;

use eframe::egui;

use crate::app::tests::Harness;
use crate::app::ui_harness::{frame_with, painted_text};
use crate::theme;

const HINT: &str = "Drop markdown file here";
const REFUSAL: &str = "Unsupported file type. Please drop a markdown file (.md, .markdown, .txt)";

fn dropping(paths: Vec<Option<PathBuf>>) -> egui::RawInput {
    let dropped_files = paths.into_iter().map(|path| egui::DroppedFile {
        path,
        ..Default::default()
    });
    egui::RawInput {
        dropped_files: dropped_files.collect(),
        ..Default::default()
    }
}

#[test]
fn a_dropped_markdown_file_opens_in_a_new_tab() {
    let mut harness = Harness::with_three_docs();
    let b = harness.path("b.md");

    let output = frame_with(&mut harness, dropping(vec![None, Some(b.clone())]));

    assert_eq!(harness.app.tabs.len(), 2);
    assert_eq!(harness.active_path(), b);
    assert!(harness.app.error_message.is_none());
    assert!(!painted_text(&output).contains(&HINT.to_owned()));
}

#[test]
fn a_dropped_file_of_another_kind_is_refused_with_a_message() {
    let mut harness = Harness::with_three_docs();
    let picture = harness.path("picture.png");

    frame_with(&mut harness, dropping(vec![Some(picture)]));

    assert_eq!(harness.app.tabs.len(), 1);
    assert!(harness.app.error_message.is_some());
}

#[test]
fn a_dropped_file_is_judged_by_its_extension_in_any_letter_case() {
    for name in ["notes.md", "NOTES.MD", "guide.markdown", "todo.txt"] {
        let mut harness = Harness::with_docs(&[("a.md", "# A"), (name, "text")]);
        let dropped = harness.path(name);

        frame_with(&mut harness, dropping(vec![Some(dropped.clone())]));

        assert_eq!(harness.active_path(), dropped, "{name}");
        assert_eq!(harness.app.error_message, None, "{name}");
    }
    for name in ["paper.pdf", "md", "archive.md.zip", ".md"] {
        let mut harness = Harness::with_docs(&[("a.md", "# A"), (name, "text")]);
        let dropped = harness.path(name);

        frame_with(&mut harness, dropping(vec![Some(dropped)]));

        assert_eq!(harness.app.tabs.len(), 1, "{name}");
        assert_eq!(
            harness.app.error_message.as_deref(),
            Some(REFUSAL),
            "{name}"
        );
    }
}

#[test]
fn files_dragged_over_the_window_get_a_drop_hint() {
    let mut harness = Harness::with_three_docs();
    let hint = HINT.to_owned();
    let dragging = egui::RawInput {
        hovered_files: vec![egui::HoveredFile::default()],
        ..Default::default()
    };

    let output = frame_with(&mut harness, dragging);
    assert!(painted_text(&output).contains(&hint));

    let output = harness.idle();
    assert!(!painted_text(&output).contains(&hint));
}

#[test]
fn the_drop_target_dims_the_window_with_the_scrim() {
    let mut harness = Harness::with_three_docs();
    let hovering = egui::RawInput {
        hovered_files: vec![egui::HoveredFile::default()],
        ..Default::default()
    };

    let output = frame_with(&mut harness, hovering);

    let dimmed = crate::test_support::painted_shapes(&output)
        .into_iter()
        .any(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == theme::SCRIM));
    assert!(dimmed);
    let prompt = crate::test_support::painted_text_starting(&output, "Drop markdown file");
    assert_eq!(prompt.fallback_color, theme::ON_SCRIM);
}
