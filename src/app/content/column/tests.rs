use eframe::egui::{Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};

use super::*;
use crate::layout::READING_COLUMN_MARGIN;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

/// A pane 1000 px wide, centered on x = 600.
const PANE: Rect = Rect::from_min_max(egui::pos2(100.0, 50.0), egui::pos2(1100.0, 650.0));
/// The height the pointer works at.
const POINTER_Y: f32 = 300.0;

fn at(x: f32) -> Pos2 {
    egui::pos2(x, POINTER_Y)
}

fn button(x: f32, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at(x),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// The resize strips over `PANE`, driven one frame at a time. A requested
/// width is taken up before the next frame, as the app does.
struct Strips {
    ctx: egui::Context,
    width: ReadingWidth,
    grab: Option<f32>,
    time: f64,
}

impl Strips {
    fn new(width: f32) -> Self {
        let mut strips = Self {
            ctx: egui::Context::default(),
            width: ReadingWidth::new(width),
            grab: None,
            time: 0.0,
        };
        // egui hit-tests against the widgets of the frame before.
        strips.frame(Vec::new());
        strips
    }

    fn column(&self) -> Column {
        Column::in_pane(PANE, self.width)
    }

    fn frame(&mut self, events: Vec<Event>) -> (FullOutput, Option<ReadingWidth>) {
        self.time += 0.05;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 700.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let (column, grab) = (self.column(), &mut self.grab);
        let mut requested = None;
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                requested = resize_strips(ui, column, grab, ThemeId::Sepia.palette());
            });
        });
        if let Some(width) = requested {
            self.width = width;
        }
        (output, requested)
    }

    fn move_to(&mut self, x: f32) -> (FullOutput, Option<ReadingWidth>) {
        self.frame(vec![Event::PointerMoved(at(x))])
    }

    /// Press at `from` and move to `to` with the button held.
    fn drag(&mut self, from: f32, to: f32) -> (FullOutput, Option<ReadingWidth>) {
        self.move_to(from);
        self.frame(vec![button(from, true)]);
        self.move_to(to)
    }

    fn release(&mut self, x: f32) -> (FullOutput, Option<ReadingWidth>) {
        self.frame(vec![button(x, false)])
    }
}

/// The x of every hairline the frame painted over the pane's full height.
fn hairlines(output: &FullOutput) -> Vec<f32> {
    let mut lines: Vec<f32> = painted_shapes(output)
        .iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, .. }
                if (points[1].y - points[0].y - PANE.height()).abs() < 0.5 =>
            {
                Some(points[0].x)
            }
            _ => None,
        })
        .collect();
    lines.sort_by(f32::total_cmp);
    lines
}

#[track_caller]
fn assert_hairlines(output: &FullOutput, expected: &[f32]) {
    let painted = hairlines(output);
    assert_eq!(painted.len(), expected.len(), "{painted:?}");
    for (painted, expected) in painted.into_iter().zip(expected) {
        assert_close(painted, *expected);
    }
}

/// The text of the width readout, when the frame painted one.
fn readout(output: &FullOutput) -> Option<String> {
    painted_shapes(output).iter().find_map(|shape| match shape {
        Shape::Text(text) if text.galley.text().ends_with(" px") => {
            Some(text.galley.text().to_owned())
        }
        _ => None,
    })
}

#[test]
fn the_column_is_centered_in_its_pane() {
    let column = Column::in_pane(PANE, ReadingWidth::new(600.0));

    assert_close(column.width, 600.0);
    assert_close(column.inset(), 200.0);
    assert_close(column.edge_x(Edge::Left), 300.0);
    assert_close(column.edge_x(Edge::Right), 900.0);
}

#[test]
fn a_column_that_fits_the_window_keeps_the_panes_margins() {
    let column = Column::in_pane(PANE, ReadingWidth::FIT_WINDOW);

    assert_close(
        column.width,
        2.0_f32.mul_add(-READING_COLUMN_MARGIN, 1000.0),
    );
    assert_close(column.inset(), READING_COLUMN_MARGIN);
}

#[test]
fn each_strip_lies_just_outside_its_edge_and_inside_the_pane() {
    for width in [ReadingWidth::new(600.0), ReadingWidth::FIT_WINDOW] {
        let column = Column::in_pane(PANE, width);
        let (left, right) = (column.strip(Edge::Left), column.strip(Edge::Right));

        assert_close(left.right(), column.edge_x(Edge::Left));
        assert_close(right.left(), column.edge_x(Edge::Right));
        for strip in [left, right] {
            assert_close(strip.width(), STRIP_WIDTH);
            assert_eq!(strip.y_range(), PANE.y_range());
            // Clear of the sidebars' resize handles and the scrollbar, which
            // sit at the pane's edges.
            assert!(strip.left() >= PANE.left() + 8.0, "{strip:?}");
            assert!(strip.right() <= PANE.right() - 8.0, "{strip:?}");
        }
    }
}

#[test]
fn hovering_a_strip_shows_its_hairline_and_the_resize_cursor() {
    let mut strips = Strips::new(600.0);

    let (elsewhere, _) = strips.move_to(600.0);
    assert!(hairlines(&elsewhere).is_empty());
    assert_ne!(
        elsewhere.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );

    let (over_right, requested) = strips.move_to(905.0);
    assert_hairlines(&over_right, &[900.0 + HAIRLINE_OFFSET]);
    assert_eq!(
        over_right.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    assert_eq!(requested, None, "hovering asks for nothing");
    assert!(readout(&over_right).is_none());

    let (over_left, _) = strips.move_to(295.0);
    assert_hairlines(&over_left, &[300.0 - HAIRLINE_OFFSET]);
}

#[test]
fn dragging_the_right_edge_moves_both_edges_about_the_center() {
    let mut strips = Strips::new(600.0);

    // Took hold 5 px outside the edge; 50 px to the right is 100 px wider.
    let (_, requested) = strips.drag(905.0, 955.0);
    assert_eq!(requested, Some(ReadingWidth::new(700.0)));
    assert_close(strips.column().edge_x(Edge::Left), 250.0);
    assert_close(strips.column().edge_x(Edge::Right), 950.0);

    // And back in, past where it started.
    let (_, requested) = strips.move_to(855.0);
    assert_eq!(requested, Some(ReadingWidth::new(500.0)));
}

#[test]
fn dragging_the_left_edge_does_the_same_from_the_other_side() {
    let mut strips = Strips::new(600.0);

    let (_, requested) = strips.drag(295.0, 245.0);

    assert_eq!(requested, Some(ReadingWidth::new(700.0)));
}

#[test]
fn a_drag_shows_both_hairlines_and_the_width_until_the_button_is_released() {
    let mut strips = Strips::new(600.0);
    strips.drag(905.0, 955.0);

    // The frame after the request lays the column out at its new width.
    let (dragging, _) = strips.frame(Vec::new());
    assert_hairlines(
        &dragging,
        &[250.0 - HAIRLINE_OFFSET, 950.0 + HAIRLINE_OFFSET],
    );
    assert_eq!(readout(&dragging).as_deref(), Some("700 px"));
    assert_eq!(
        dragging.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    assert!(strips.grab.is_some());

    let (released, requested) = strips.release(955.0);
    assert_eq!(requested, None);
    assert_eq!(strips.grab, None);
    assert!(readout(&released).is_none());

    // With the button up, the pointer moves nothing.
    let (_, requested) = strips.move_to(600.0);
    assert_eq!(requested, None);
    assert_eq!(strips.width, ReadingWidth::new(700.0));
}

#[test]
fn a_drag_past_the_center_stops_at_the_narrowest_column() {
    let mut strips = Strips::new(600.0);

    let (_, requested) = strips.drag(905.0, 500.0);

    assert_eq!(requested, Some(ReadingWidth::new(ReadingWidth::MIN)));
}

#[test]
fn a_double_click_on_a_strip_goes_back_to_the_default_width() {
    let mut strips = Strips::new(400.0);
    let on_right_edge = 600.0 + 200.0 + 5.0;
    strips.move_to(on_right_edge);

    strips.frame(vec![button(on_right_edge, true)]);
    let (_, first) = strips.release(on_right_edge);
    assert_eq!(first, None, "one click asks for nothing");
    strips.frame(vec![button(on_right_edge, true)]);
    let (_, second) = strips.release(on_right_edge);

    assert_eq!(second, Some(ReadingWidth::DEFAULT));
    assert_eq!(strips.grab, None);
}

#[test]
fn each_strip_names_its_edge() {
    assert_ne!(Edge::Left.label(), Edge::Right.label());
    assert!(Edge::Left.label().contains("left"));
    assert!(Edge::Right.label().contains("right"));
}

#[test]
fn a_figure_may_use_the_pane_up_to_its_margins() {
    let column = Column::in_pane(PANE, ReadingWidth::new(680.0));

    assert_close(
        column.figure_room(),
        2.0_f32.mul_add(-READING_COLUMN_MARGIN, 1000.0),
    );
}

#[test]
fn a_figure_always_has_the_width_of_the_column() {
    // A pane too narrow for its own margins still shows a column.
    let narrow = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(40.0, 600.0));
    let column = Column {
        pane: narrow,
        width: 30.0,
    };

    assert_close(column.figure_room(), 30.0);
}
