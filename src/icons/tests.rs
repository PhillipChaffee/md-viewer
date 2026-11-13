use super::*;

use egui::{Color32, Pos2, Rect, Shape};

use crate::test_support::{assert_close, painted_shapes};

const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));

/// A resizable sidebar driven frame by frame with a pointer, the way the
/// explorer and outline panels use the affordance.
struct Sidebar {
    ctx: egui::Context,
    id: egui::Id,
    /// Right edge of the panel, where its resize handle is.
    edge: f32,
    resizable: bool,
}

impl Sidebar {
    /// A sidebar the user can resize, laid out once so its edge is known.
    fn new() -> Self {
        Self::laid_out(true)
    }

    /// A sidebar the user cannot resize.
    fn fixed() -> Self {
        Self::laid_out(false)
    }

    fn laid_out(resizable: bool) -> Self {
        let mut sidebar = Self {
            ctx: egui::Context::default(),
            id: egui::Id::new("sidebar-under-test"),
            edge: 0.0,
            resizable,
        };
        sidebar.frame(Vec::new());
        sidebar
    }

    /// Run one frame with `events` and return the affordance color, if the
    /// affordance was painted.
    fn frame(&mut self, events: Vec<egui::Event>) -> Option<Color32> {
        let input = egui::RawInput {
            screen_rect: Some(SCREEN),
            events,
            ..Default::default()
        };
        let (id, resizable, mut strip) = (self.id, self.resizable, Rect::NOTHING);
        let output = self.ctx.run(input, |ctx| {
            let panel = egui::SidePanel::left(id)
                .resizable(resizable)
                .default_width(200.0)
                .show(ctx, |ui| ui.label("sidebar"))
                .response
                .rect;
            paint_sidebar_resize_affordance(ctx, panel.right(), panel.y_range(), id);
            strip =
                Rect::from_x_y_ranges(panel.right() - 1.0..=panel.right() + 1.0, panel.y_range());
        });
        self.edge = strip.center().x;
        painted_shapes(&output)
            .iter()
            .find_map(|shape| match shape {
                Shape::Rect(rect) if rect.rect == strip => Some(rect.fill),
                _ => None,
            })
    }

    fn pointer_at(&mut self, x: f32) -> Option<Color32> {
        self.frame(vec![egui::Event::PointerMoved(Pos2::new(x, 300.0))])
    }

    fn press_at(&mut self, x: f32) -> Option<Color32> {
        self.frame(vec![egui::Event::PointerButton {
            pos: Pos2::new(x, 300.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }])
    }

    fn widget_strokes(&self) -> (Color32, Color32) {
        let widgets = &self.ctx.style().visuals.widgets;
        (
            widgets.hovered.bg_stroke.color,
            widgets.active.bg_stroke.color,
        )
    }
}

#[test]
fn resize_affordance_stays_hidden_away_from_the_sidebar_edge() {
    let mut sidebar = Sidebar::new();
    assert_eq!(sidebar.pointer_at(sidebar.edge + 300.0), None);
    assert_eq!(sidebar.frame(Vec::new()), None);
}

#[test]
fn sidebar_that_cannot_be_resized_never_shows_the_affordance() {
    let mut sidebar = Sidebar::fixed();
    let edge = sidebar.edge;
    assert_eq!(sidebar.pointer_at(edge), None);
    assert_eq!(sidebar.frame(Vec::new()), None);
}

#[test]
fn resize_affordance_lights_up_on_hover_and_brightens_while_dragging() {
    let mut sidebar = Sidebar::new();
    let (hovered, active) = sidebar.widget_strokes();
    assert_ne!(hovered, active);
    let edge = sidebar.edge;

    sidebar.pointer_at(edge);
    assert_eq!(sidebar.frame(Vec::new()), Some(hovered));

    sidebar.press_at(edge);
    sidebar.pointer_at(edge + 20.0);
    assert_eq!(sidebar.pointer_at(edge + 40.0), Some(active));
}

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
