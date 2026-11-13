use super::*;

use egui::{Color32, Pos2, Rect, Shape};

use crate::test_support::painted_shapes;

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
