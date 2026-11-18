use super::*;

use egui::{Color32, Pos2, Rect, Shape};

use crate::test_support::{assert_close, painted_shapes};

const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));

/// The shapes `paint` draws alone in an empty panel.
fn drawn_alone(paint: impl Fn(&mut egui::Ui)) -> Vec<Shape> {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(SCREEN),
        ..Default::default()
    };
    let output = ctx.run(input, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.scope(&paint);
            });
    });
    painted_shapes(&output)
}

const INK: Color32 = Color32::from_rgb(12, 34, 56);

/// What `icon` paints into `rect`, without the panel it is painted in.
fn marks_of(icon: Icon, rect: Rect) -> Vec<Shape> {
    drawn_alone(|ui| icon.paint(ui.painter(), rect, INK))
        .into_iter()
        .filter(|shape| !matches!(shape, Shape::Rect(rect) if rect.stroke.is_empty()))
        .collect()
}

/// The color and width of the line a mark is drawn with, or `None` for a
/// mark that is filled instead.
fn line_of(shape: &Shape) -> Option<(Color32, f32)> {
    match shape {
        Shape::Path(path) => match &path.stroke.color {
            egui::epaint::ColorMode::Solid(color) => Some((*color, path.stroke.width)),
            egui::epaint::ColorMode::UV(_) => None,
        },
        Shape::Circle(circle) if circle.fill == Color32::TRANSPARENT => {
            Some((circle.stroke.color, circle.stroke.width))
        }
        Shape::Rect(rect) => Some((rect.stroke.color, rect.stroke.width)),
        _ => None,
    }
}

#[test]
fn every_icon_draws_thin_lines_in_the_asked_color_inside_its_square() {
    let square = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::Vec2::splat(16.0));
    for icon in Icon::ALL {
        let marks = marks_of(icon, square);
        assert!(!marks.is_empty(), "{icon:?} paints nothing");
        for mark in &marks {
            assert!(
                square.contains_rect(mark.visual_bounding_rect()),
                "{icon:?} leaves its square: {mark:?}"
            );
            if let Some((color, width)) = line_of(mark) {
                assert_eq!(color, INK, "{icon:?}");
                assert_close(width, 1.25);
            } else {
                let Shape::Circle(dot) = mark else {
                    panic!("{icon:?} paints an unexpected {mark:?}");
                };
                assert_eq!(dot.fill, INK, "{icon:?}");
            }
        }
    }
}

#[test]
fn no_two_icons_look_the_same() {
    let square = Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(16.0));
    let drawn: Vec<_> = Icon::ALL.map(|icon| marks_of(icon, square)).into();
    for (index, marks) in drawn.iter().enumerate() {
        for other in &drawn[index + 1..] {
            assert_ne!(marks, other, "{:?} repeats another icon", Icon::ALL[index]);
        }
    }
}

#[test]
fn an_icon_grows_with_its_square_lines_included() {
    let small = marks_of(
        Icon::Search,
        Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(16.0)),
    );
    let large = marks_of(
        Icon::Search,
        Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(32.0)),
    );

    let (Shape::Circle(small_lens), Shape::Circle(large_lens)) = (&small[0], &large[0]) else {
        panic!("the magnifier starts with its lens: {small:?}");
    };
    assert_close(small_lens.radius, 4.5);
    assert_close(large_lens.radius, 9.0);
    assert_close(large_lens.center.x, 2.0 * small_lens.center.x);
    assert_close(large_lens.stroke.width, 2.0 * small_lens.stroke.width);
}

#[test]
fn an_icon_sits_in_the_middle_of_a_rect_that_is_not_square() {
    let wide = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(40.0, 16.0));
    let marks = marks_of(Icon::Plus, wide);

    let bounds = marks.iter().fold(Rect::NOTHING, |all, mark| {
        all.union(mark.visual_bounding_rect())
    });
    assert_close(bounds.center().x, wide.center().x);
    assert_close(bounds.center().y, wide.center().y);
    assert!(
        bounds.width() <= 16.0,
        "scaled by the short side: {bounds:?}"
    );
}

#[test]
fn back_and_forward_mirror_each_other() {
    let square = Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(16.0));
    let tips = |icon| -> Vec<f32> {
        marks_of(icon, square)
            .iter()
            .map(|mark| mark.visual_bounding_rect().center().x)
            .collect()
    };
    let (back, forward) = (tips(Icon::Back), tips(Icon::Forward));
    // The arrow head of each is the second mark: left of the middle on the
    // way back, right of it on the way forward, by the same distance.
    assert!(back[1] < 8.0 && forward[1] > 8.0, "{back:?} {forward:?}");
    assert_close(8.0 - back[1], forward[1] - 8.0);
}

#[test]
fn the_warning_sign_dots_its_exclamation_mark() {
    let square = Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(16.0));
    let marks = marks_of(Icon::Warning, square);
    let filled = marks.iter().filter(|mark| line_of(mark).is_none());
    assert_eq!(filled.count(), 1, "one dot under the stroke: {marks:?}");
    assert_eq!(marks.len(), 3);
}
