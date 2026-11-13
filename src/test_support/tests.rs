use super::*;

use egui::{Color32, FontId, Pos2, Shape};

#[test]
fn scratch_dir_holds_nested_entries_and_removes_itself_when_dropped() {
    let dir = ScratchDir::new();
    let root = dir.path().to_path_buf();

    let file = dir.file("a/b/note.md", "text");
    let folder = dir.dir("c/d");

    assert_eq!(fs::read_to_string(&file).unwrap(), "text");
    assert_eq!(file, root.join("a/b/note.md"));
    assert!(folder.is_dir());

    drop(dir);
    assert!(!root.exists());
}

#[test]
fn every_scratch_dir_is_a_directory_of_its_own() {
    let (first, second) = (ScratchDir::new(), ScratchDir::new());
    assert_ne!(first.path(), second.path());
}

#[test]
fn wait_until_returns_as_soon_as_the_condition_holds() {
    let mut polls = 0;
    let held = wait_until(Duration::from_secs(20), || {
        polls += 1;
        polls == 3
    });
    assert!(held);
    assert_eq!(polls, 3);
}

#[test]
fn wait_until_gives_up_when_the_time_is_over() {
    assert!(!wait_until(Duration::from_millis(20), || false));
}

#[test]
fn painted_shapes_take_grouped_shapes_apart_in_paint_order() {
    let ctx = egui::Context::default();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(egui::LayerId::background());
        let text = |text: &str| {
            let galley = painter.layout_no_wrap(text.to_owned(), FontId::default(), Color32::WHITE);
            Shape::galley(Pos2::ZERO, galley, Color32::WHITE)
        };
        painter.add(Shape::Vec(vec![
            text("grouped"),
            Shape::Vec(vec![text("nested")]),
        ]));
        painter.add(text("alone"));
    });

    let texts: Vec<String> = painted_shapes(&output)
        .iter()
        .map(|shape| match shape {
            Shape::Text(text) => text.galley.text().to_owned(),
            other => panic!("only text was painted, got {other:?}"),
        })
        .collect();
    assert_eq!(texts, ["grouped", "nested", "alone"]);
}

#[test]
fn log_arguments_are_evaluated_at_every_level() {
    evaluate_log_arguments();
    assert_eq!(log::max_level(), log::LevelFilter::Trace);
}

#[test]
fn close_floats_pass_and_distant_ones_do_not() {
    assert_close(0.1 + 0.2, 0.3);
    let distant = std::panic::catch_unwind(|| assert_close(0.31, 0.3));
    assert!(distant.is_err());
}

/// A frame that paints `texts` one under the other, among other shapes.
fn frame_painting(texts: &[&str]) -> egui::FullOutput {
    egui::Context::default().run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            for text in texts {
                ui.label(*text);
            }
        });
    })
}

#[test]
fn painted_text_is_found_by_how_it_starts_in_paint_order() {
    let output = frame_painting(&["first line", "second line", "second thought"]);

    assert_eq!(
        painted_text_starting(&output, "second").galley.text(),
        "second line"
    );
    let first = painted_text_starting(&output, "first");
    let third = painted_text_starting(&output, "second t");
    assert!(first.pos.y < third.pos.y);
}

#[test]
#[should_panic(expected = "no painted text starts with the prefix")]
fn looking_for_text_nobody_painted_fails_the_test() {
    let output = frame_painting(&["first line"]);

    painted_text_starting(&output, "missing");
}
