use std::cell::Cell;

use egui::accesskit::Role;
use egui::{Color32, Pos2, Rect, Shape};
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(240.0, 300.0));
const PALETTE: &Palette = ThemeId::Sepia.palette();

/// What one frame of a sidebar painted.
struct Painted {
    shapes: Vec<Shape>,
}

impl Painted {
    /// Every painted text with the color it was painted in and where.
    fn texts(&self) -> Vec<(String, Color32, Pos2)> {
        let texts = self.shapes.iter().filter_map(|shape| match shape {
            Shape::Text(text) => Some(text),
            _ => None,
        });
        texts
            .map(|text| {
                let section = &text.galley.job.sections[0];
                (
                    text.galley.text().to_owned(),
                    section.format.color,
                    text.pos,
                )
            })
            .collect()
    }

    /// The color and left edge of the one text that reads `text`.
    #[track_caller]
    fn text(&self, text: &str) -> (Color32, f32) {
        let texts = self.texts();
        let found = texts.iter().find(|(painted, _, _)| painted == text);
        let (_, color, pos) = found.unwrap_or_else(|| panic!("{text:?} is not in {texts:?}"));
        (*color, pos.x)
    }

    /// The filled rectangles, in paint order.
    fn fills(&self) -> Vec<(Rect, Color32)> {
        let rects = self.shapes.iter().filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill != Color32::TRANSPARENT => Some(rect),
            _ => None,
        });
        rects.map(|rect| (rect.rect, rect.fill)).collect()
    }

    /// The open paths: the chevrons, each with its points and its color.
    fn paths(&self) -> Vec<(Vec<Pos2>, Color32)> {
        let paths = self.shapes.iter().filter_map(|shape| match shape {
            Shape::Path(path) if !path.closed => Some(path),
            _ => None,
        });
        paths
            .map(|path| (path.points.clone(), solid(&path.stroke.color)))
            .collect()
    }

    fn circles(&self) -> Vec<Color32> {
        let circles = self.shapes.iter().filter_map(|shape| match shape {
            Shape::Circle(circle) => Some(circle.fill),
            _ => None,
        });
        circles.collect()
    }
}

/// The one color a path is stroked with.
fn solid(color: &egui::epaint::ColorMode) -> Color32 {
    match color {
        egui::epaint::ColorMode::Solid(color) => *color,
        egui::epaint::ColorMode::UV(_) => Color32::PLACEHOLDER,
    }
}

/// Run frames of `contents` in a bare panel, the pointer resting at
/// `pointer`, and return what the last frame painted. Hover shows a frame
/// after the pointer arrives.
fn paint(pointer: Option<Pos2>, mut contents: impl FnMut(&mut egui::Ui)) -> Painted {
    let ctx = egui::Context::default();
    let mut frame = |events| {
        let input = egui::RawInput {
            screen_rect: Some(SCREEN),
            events,
            ..Default::default()
        };
        ctx.run(input, |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, &mut contents);
        })
    };
    frame(pointer.map(egui::Event::PointerMoved).into_iter().collect());
    Painted {
        shapes: painted_shapes(&frame(Vec::new())),
    }
}

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
    const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));

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
            screen_rect: Some(Self::SCREEN),
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
            paint_resize_affordance(ctx, panel.right(), panel.y_range(), id);
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

#[test]
fn a_panel_keeps_its_width_and_marks_the_edge_it_shares_with_the_page() {
    for (side, shared_edge, hairline) in [(Side::Left, 200.0, 199.5), (Side::Right, 600.0, 600.5)] {
        let panel = Panel {
            side,
            id: "panel-under-test",
            margin: egui::Margin::same(8),
        };
        let ctx = egui::Context::default();
        let frame = |events| {
            let input = egui::RawInput {
                screen_rect: Some(Sidebar::SCREEN),
                events,
                ..Default::default()
            };
            let mut shown = (0.0, 0.0);
            let output = ctx.run(input, |ctx| {
                shown = panel.show(ctx, PALETTE, 200.0, |ui| {
                    // A listing takes all the room, as a scroll area does.
                    ui.allocate_space(ui.available_size()).1.width()
                });
            });
            (painted_shapes(&output), shown)
        };
        let resting = egui::Event::PointerMoved(Pos2::new(shared_edge, 300.0));
        frame(vec![resting]);
        let (shapes, (listing_width, width)) = frame(Vec::new());

        assert_close(width, 200.0);
        assert_close(listing_width, 200.0 - 16.0);
        let fills = shapes.iter().filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.rect.height() > 500.0 => Some(rect.rect),
            _ => None,
        });
        let strip = fills.min_by(|a, b| a.width().total_cmp(&b.width()));
        let strip = strip.expect("the panel paints its surface");
        assert_close(strip.width(), 2.0);
        assert_close(strip.center().x, shared_edge);
        // egui draws a line of its own, in its own color, on a hovered edge.
        let rule = egui::Stroke::new(1.0_f32, PALETTE.rule);
        let lines = shapes.iter().filter_map(|shape| match shape {
            Shape::LineSegment { points, stroke } if *stroke == rule => Some(points[0].x),
            _ => None,
        });
        assert_eq!(lines.collect::<Vec<_>>(), [hairline]);
    }
}

fn plain_row(text: &str, current: bool) -> Row<'_> {
    Row {
        text,
        indent: 0.0,
        current,
        chevron: Chevron::None,
    }
}

#[test]
fn a_sidebar_is_filled_with_the_surface_color_and_has_no_outline() {
    let margin = egui::Margin::same(8);

    let frame = frame(PALETTE, margin);

    assert_eq!(frame.fill, PALETTE.surface);
    assert_eq!(frame.stroke, egui::Stroke::NONE);
    assert_eq!(frame.inner_margin, margin);
}

#[test]
fn the_one_hairline_runs_down_the_edge_a_sidebar_shares_with_the_page() {
    let panel = Rect::from_min_max(Pos2::new(40.0, 0.0), Pos2::new(200.0, 300.0));
    let edge_of = |side| {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            paint_inner_edge(ctx, panel, side, PALETTE);
        });
        let lines: Vec<_> = painted_shapes(&output)
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::LineSegment { points, stroke } => Some((points, stroke)),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 1);
        let (points, stroke) = lines[0];
        assert_eq!(stroke, egui::Stroke::new(1.0_f32, PALETTE.rule));
        assert_close(points[0].y, 0.0);
        assert_close(points[1].y, 300.0);
        assert_close(points[0].x, points[1].x);
        points[0].x
    };

    // Half a pixel inside the panel, so the line is one whole pixel.
    assert_close(edge_of(Side::Left), 199.5);
    assert_close(edge_of(Side::Right), 40.5);
}

#[test]
fn a_caption_is_small_and_its_letters_stand_apart() {
    let job = caption("FILES", PALETTE.muted);

    assert_eq!(job.text, "FILES");
    let format = &job.sections[0].format;
    assert_close(format.font_id.size, 11.0);
    assert!(format.extra_letter_spacing > 0.0);
    assert_eq!(format.color, PALETTE.muted);
}

#[test]
fn a_header_shows_its_caption_and_its_detail_in_the_muted_ink() {
    let painted = paint(None, |ui| {
        let header_line = Header {
            caption: "FILES",
            detail: Some("notes"),
            menu: None,
        };
        assert!(header(ui, PALETTE, &header_line).is_none());
    });

    let (color, _) = painted.text("FILESnotes");
    assert_eq!(color, PALETTE.muted);
    assert!(painted.circles().is_empty(), "no menu, no dots");
}

#[test]
fn a_header_names_itself_and_its_menu_for_a_screen_reader() {
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        let with_detail = Header {
            caption: "FILES",
            detail: Some("notes"),
            menu: Some("Explorer options"),
        };
        header(ui, PALETTE, &with_detail);
        let bare = Header {
            caption: "CONTENTS",
            detail: None,
            menu: None,
        };
        header(ui, PALETTE, &bare);
    });
    harness.run();

    assert!(harness.query_by_label("FILES notes").is_some());
    assert!(harness.query_by_label("CONTENTS").is_some());
    let buttons = harness.query_all_by_role(Role::Button).count();
    assert_eq!(buttons, 1, "only the first header has a menu");
    assert!(harness.query_by_label("Explorer options").is_some());
}

#[test]
fn the_menu_button_is_three_dots_that_light_up_under_the_pointer() {
    let header_line = Header {
        caption: "FILES",
        detail: None,
        menu: Some("Explorer options"),
    };
    let button = Cell::new(Rect::NOTHING);
    let draw = |ui: &mut egui::Ui| {
        button.set(header(ui, PALETTE, &header_line).unwrap().rect);
    };

    let at_rest = paint(None, draw);
    assert_eq!(at_rest.circles(), [PALETTE.muted; 3]);
    assert!(at_rest.fills().is_empty());

    let lit = paint(Some(button.get().center()), draw);
    assert_eq!(lit.circles(), [PALETTE.text; 3]);
    assert_eq!(lit.fills(), [(button.get(), PALETTE.raised)]);
    assert_close(button.get().right(), SCREEN.right());
}

#[test]
fn a_long_header_is_cut_short_of_its_menu_button() {
    let folder = "a-folder-with-a-very-long-name-that-cannot-fit";
    let mut button = Rect::NOTHING;
    let painted = paint(None, |ui| {
        let header_line = Header {
            caption: "FILES",
            detail: Some(folder),
            menu: Some("Explorer options"),
        };
        button = header(ui, PALETTE, &header_line).unwrap().rect;
    });

    let text = painted.shapes.iter().find_map(|shape| match shape {
        Shape::Text(text) => Some(text),
        _ => None,
    });
    let text = text.unwrap();
    assert!(text.galley.elided);
    assert!(text.pos.x + text.galley.size().x <= button.left());
}

#[test]
fn the_current_row_is_inked_like_text_and_the_others_are_muted() {
    let painted = paint(None, |ui| {
        row(ui, PALETTE, &plain_row("here.md", true));
        row(ui, PALETTE, &plain_row("there.md", false));
    });

    assert_eq!(painted.text("here.md").0, PALETTE.text);
    assert_eq!(painted.text("there.md").0, PALETTE.muted);
    assert!(painted.fills().is_empty(), "no row is washed at rest");
}

#[test]
fn the_row_under_the_pointer_gets_the_raised_wash_and_the_text_ink() {
    let second = Cell::new(Rect::NOTHING);
    let draw = |ui: &mut egui::Ui| {
        row(ui, PALETTE, &plain_row("one.md", false));
        second.set(row(ui, PALETTE, &plain_row("two.md", false)).response.rect);
    };
    paint(None, draw);

    let painted = paint(Some(second.get().center()), draw);

    assert_eq!(painted.fills(), [(second.get(), PALETTE.raised)]);
    assert_ne!(PALETTE.raised, PALETTE.selection, "a wash, not a selection");
    assert_eq!(painted.text("two.md").0, PALETTE.text);
    assert_eq!(painted.text("one.md").0, PALETTE.muted);
}

#[test]
fn rows_have_one_height_and_span_the_sidebar() {
    let mut rect = Rect::NOTHING;
    paint(None, |ui| {
        rect = row(ui, PALETTE, &plain_row("a.md", false)).response.rect;
    });

    assert_close(rect.height(), ROW_HEIGHT);
    assert_close(rect.width(), SCREEN.width());
}

#[test]
fn a_fold_column_holds_a_chevron_or_keeps_its_place_empty() {
    let with = |chevron| Row {
        text: "name",
        indent: 0.0,
        current: false,
        chevron,
    };
    let left_of = |chevron| {
        let painted = paint(None, |ui| {
            row(ui, PALETTE, &with(chevron));
        });
        (painted.text("name").1, painted.paths())
    };

    let (no_column, no_chevron) = left_of(Chevron::None);
    let (blank, blank_chevron) = left_of(Chevron::Blank);
    let (closed, closed_chevron) = left_of(Chevron::Closed);
    let (open, open_chevron) = left_of(Chevron::Open);

    assert!(no_chevron.is_empty());
    assert!(blank_chevron.is_empty());
    assert!(blank > no_column, "the column pushes the text right");
    assert_close(closed, blank);
    assert_close(open, blank);
    // Three points each, in the row's ink: one opens to the right, the
    // other downward.
    let (closed_points, closed_ink) = &closed_chevron[0];
    let (open_points, open_ink) = &open_chevron[0];
    assert_eq!((closed_points.len(), open_points.len()), (3, 3));
    assert_eq!((*closed_ink, *open_ink), (PALETTE.muted, PALETTE.muted));
    assert!(closed_points[1].x > closed_points[0].x);
    assert_close(closed_points[0].x, closed_points[2].x);
    assert!(open_points[1].y > open_points[0].y);
    assert_close(open_points[0].y, open_points[2].y);
}

#[test]
fn a_row_reports_where_its_chevron_is() {
    let mut laid_out = None;
    let painted = paint(None, |ui| {
        let shown = Row {
            text: "folder",
            indent: 28.0,
            current: false,
            chevron: Chevron::Closed,
        };
        laid_out = Some(row(ui, PALETTE, &shown));
    });
    let laid_out = laid_out.unwrap();

    let (points, _) = &painted.paths()[0];
    assert!(points.iter().all(|point| laid_out.chevron.contains(*point)));
    assert!(laid_out.response.rect.contains_rect(laid_out.chevron));
    assert!(laid_out.chevron.left() >= 28.0, "the indent comes first");
}

#[test]
fn an_indent_pushes_a_row_right_by_that_much() {
    let left_of = |indent| {
        let painted = paint(None, |ui| {
            let shown = Row {
                indent,
                ..plain_row("name", false)
            };
            row(ui, PALETTE, &shown);
        });
        painted.text("name").1
    };

    assert_close(left_of(42.0) - left_of(0.0), 42.0);
}

#[test]
fn a_name_too_long_for_its_row_is_cut_and_shown_whole_on_hover() {
    let long = "a-file-name-that-is-much-too-long-for-a-narrow-sidebar.md";
    let mut harness = egui_kittest::Harness::builder()
        .with_size(SCREEN.size())
        .build_ui(|ui| {
            row(ui, PALETTE, &plain_row(long, false));
            row(ui, PALETTE, &plain_row("short.md", false));
        });
    harness.run();
    // A tooltip repeats the name under the row that carries it.
    let shown =
        |harness: &egui_kittest::Harness<'_>, name: &str| harness.query_all_by_label(name).count();

    harness.get_by_label("short.md").hover();
    harness.run_steps(40);
    assert_eq!(shown(&harness, "short.md"), 1, "a name that fits has none");

    harness.get_by_label(long).hover();
    harness.run_steps(40);
    assert_eq!(shown(&harness, long), 2);
}

#[test]
fn a_row_scrolled_out_of_view_is_laid_out_but_not_painted() {
    let names: Vec<String> = (0..40).map(|n| format!("file-{n}.md")).collect();
    let mut content_height = 0.0;
    let painted = paint(None, |ui| {
        let scrolled = egui::ScrollArea::vertical().show(ui, |ui| {
            for name in &names {
                row(ui, PALETTE, &plain_row(name, false));
            }
        });
        content_height = scrolled.content_size.y;
    });

    assert!(
        content_height >= 40.0 * ROW_HEIGHT,
        "every row takes its place"
    );
    let texts = painted.texts();
    assert!(texts.iter().any(|(text, _, _)| text == "file-0.md"));
    assert!(texts.iter().all(|(text, _, _)| text != "file-39.md"));
}
