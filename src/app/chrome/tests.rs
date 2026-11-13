//! The parts the chrome is built from, each laid out alone in an empty
//! panel: where it goes, what it paints, and what it answers to a pointer.

use eframe::egui::accesskit::{Role, Toggled};
use eframe::egui::{self, Color32, Event, Pos2, Rect, Shape};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
const PALETTE: &Palette = ThemeId::Sepia.palette();

/// An empty panel that lays one piece of chrome out frame after frame.
struct Stage {
    ctx: egui::Context,
}

impl Stage {
    fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
        }
    }

    /// Run one frame on `events`: what `add` returned and what was painted.
    fn frame<R>(
        &self,
        events: Vec<Event>,
        add: &mut impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, Vec<Shape>) {
        let input = egui::RawInput {
            screen_rect: Some(SCREEN),
            events,
            ..Default::default()
        };
        let mut returned = None;
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| returned = Some(add(ui)));
        });
        (returned.unwrap(), painted_shapes(&output))
    }

    /// Lay `add` out with the pointer away from the window.
    fn show<R>(&self, mut add: impl FnMut(&mut egui::Ui) -> R) -> (R, Vec<Shape>) {
        self.frame(Vec::new(), &mut add)
    }

    /// Lay `add` out, then again with the pointer resting on `pos`.
    fn pointing_at<R>(
        &self,
        pos: Pos2,
        mut add: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, Vec<Shape>) {
        self.frame(Vec::new(), &mut add);
        self.frame(vec![Event::PointerMoved(pos)], &mut add);
        self.frame(Vec::new(), &mut add)
    }

    /// Lay `add` out, then press the primary button on `pos`. Returns the
    /// frame the button is held down in.
    fn pressing<R>(&self, pos: Pos2, mut add: impl FnMut(&mut egui::Ui) -> R) -> (R, Vec<Shape>) {
        self.frame(Vec::new(), &mut add);
        self.frame(vec![Event::PointerMoved(pos)], &mut add);
        self.frame(vec![button(pos, true)], &mut add)
    }

    /// Lay `add` out, then click on `pos`. Returns the frame the button
    /// comes up in.
    fn clicking<R>(&self, pos: Pos2, mut add: impl FnMut(&mut egui::Ui) -> R) -> (R, Vec<Shape>) {
        self.frame(Vec::new(), &mut add);
        self.frame(vec![Event::PointerMoved(pos)], &mut add);
        self.frame(vec![button(pos, true)], &mut add);
        self.frame(vec![button(pos, false)], &mut add)
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// The colors of the lines painted, in paint order.
fn line_colors(shapes: &[Shape]) -> Vec<Color32> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Path(path) => match &path.stroke.color {
                egui::epaint::ColorMode::Solid(color) => Some(*color),
                egui::epaint::ColorMode::UV(_) => None,
            },
            Shape::LineSegment { stroke, .. } => Some(stroke.color),
            _ => None,
        })
        .collect()
}

/// The filled rectangles painted, in paint order.
fn fills(shapes: &[Shape]) -> Vec<(Rect, Color32)> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill != Color32::TRANSPARENT => Some((rect.rect, rect.fill)),
            _ => None,
        })
        .collect()
}

/// The outlined rectangles painted, in paint order.
fn outlines(shapes: &[Shape]) -> Vec<(Rect, Color32)> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if !rect.stroke.is_empty() => Some((rect.rect, rect.stroke.color)),
            _ => None,
        })
        .collect()
}

/// The one painted text that reads `text`.
#[track_caller]
fn text_of<'a>(shapes: &'a [Shape], text: &str) -> &'a egui::epaint::TextShape {
    let mut found = shapes.iter().filter_map(|shape| match shape {
        Shape::Text(shape) if shape.galley.text() == text => Some(shape),
        _ => None,
    });
    let first = found.next().expect("the text was not painted");
    assert!(found.next().is_none(), "{text:?} was painted twice");
    first
}

/// Where the painted text `text` is on screen.
#[track_caller]
fn text_rect(shapes: &[Shape], text: &str) -> Rect {
    let shape = text_of(shapes, text);
    Rect::from_min_size(shape.pos, shape.galley.size())
}

#[test]
fn a_quiet_icon_button_is_a_square_drawn_muted_until_it_is_pointed_at() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| QuietButton::icon(Icon::Search, "Find").show(ui, PALETTE);

    let (response, shapes) = stage.show(show);
    assert_close(response.rect.width(), BUTTON_SIDE);
    assert_close(response.rect.height(), BUTTON_SIDE);
    assert_eq!(line_colors(&shapes), [PALETTE.muted]);

    let (_, shapes) = stage.pointing_at(response.rect.center(), show);
    assert_eq!(line_colors(&shapes), [PALETTE.text]);
}

#[test]
fn a_quiet_icon_is_painted_in_the_middle_of_its_button() {
    let stage = Stage::new();
    let (response, shapes) =
        stage.show(|ui| QuietButton::icon(Icon::Plus, "Open").show(ui, PALETTE));

    let bounds = shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Path(_)))
        .fold(Rect::NOTHING, |all, mark| {
            all.union(mark.visual_bounding_rect())
        });
    assert_close(bounds.center().x, response.rect.center().x);
    assert_close(bounds.center().y, response.rect.center().y);
    assert!(bounds.width() <= ICON_SIDE, "{bounds:?}");
}

#[test]
fn a_quiet_text_button_hugs_its_text_and_is_no_narrower_than_asked() {
    let stage = Stage::new();
    let ((short, long, wide), shapes) = stage.show(|ui| {
        (
            QuietButton::text("A").show(ui, PALETTE),
            QuietButton::text("Fit window").show(ui, PALETTE),
            QuietButton::text("80%").min_width(120.0).show(ui, PALETTE),
        )
    });

    assert_close(short.rect.width(), BUTTON_SIDE);
    let label = text_rect(&shapes, "Fit window");
    assert_close(
        long.rect.width(),
        2.0_f32.mul_add(BUTTON_PADDING, label.width()),
    );
    assert_close(label.center().x, long.rect.center().x);
    assert_close(label.center().y, long.rect.center().y);
    assert_close(wide.rect.width(), 120.0);
    assert_close(text_rect(&shapes, "80%").center().x, wide.rect.center().x);
}

#[test]
fn a_snug_quiet_button_takes_the_square_an_icon_takes() {
    let stage = Stage::new();
    let ((roomy, snug), shapes) = stage.show(|ui| {
        (
            QuietButton::text_sized("Wide", 15.0).show(ui, PALETTE),
            QuietButton::text_sized("Aa", 15.0).snug().show(ui, PALETTE),
        )
    });

    assert!(roomy.rect.width() > BUTTON_SIDE);
    assert!(text_rect(&shapes, "Aa").width() > 2.0_f32.mul_add(-BUTTON_PADDING, BUTTON_SIDE));
    assert_close(snug.rect.width(), BUTTON_SIDE);
}

#[test]
fn a_quiet_text_button_is_set_at_the_size_asked() {
    let stage = Stage::new();
    let ((), shapes) = stage.show(|ui| {
        QuietButton::text("Reset").show(ui, PALETTE);
        QuietButton::text_sized("Aa", 19.0).show(ui, PALETTE);
    });

    let size = |text| {
        text_of(&shapes, text).galley.job.sections[0]
            .format
            .font_id
            .size
    };
    assert_close(size("Reset"), LABEL_SIZE);
    assert_close(size("Aa"), 19.0);
}

#[test]
fn a_quiet_button_reports_a_click_unless_it_is_disabled() {
    let stage = Stage::new();
    let (at_rest, _) = stage.show(|ui| QuietButton::text("Reset").show(ui, PALETTE));
    let center = at_rest.rect.center();

    let (enabled, _) = stage.clicking(center, |ui| QuietButton::text("Reset").show(ui, PALETTE));
    assert!(enabled.clicked());

    let stage = Stage::new();
    let disabled = |ui: &mut egui::Ui| QuietButton::text("Reset").enabled(false).show(ui, PALETTE);
    let (response, _) = stage.clicking(center, disabled);
    assert!(!response.clicked());
    // Pale, and not lit by the pointer resting on it.
    let (_, shapes) = stage.pointing_at(center, disabled);
    assert_eq!(
        text_of(&shapes, "Reset").fallback_color,
        PALETTE.muted.gamma_multiply(0.4)
    );
}

#[test]
fn a_lit_quiet_button_is_drawn_in_the_text_color_at_rest() {
    let stage = Stage::new();
    let ((), shapes) = stage.show(|ui| {
        QuietButton::text("Aa").show(ui, PALETTE);
        QuietButton::text("Bb").lit(true).show(ui, PALETTE);
    });

    assert_eq!(text_of(&shapes, "Aa").fallback_color, PALETTE.muted);
    assert_eq!(text_of(&shapes, "Bb").fallback_color, PALETTE.text);
}

#[test]
fn a_pressed_quiet_button_is_washed_and_a_focused_one_is_ringed() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| QuietButton::icon(Icon::Minus, "Zoom Out").show(ui, PALETTE);
    let (at_rest, shapes) = stage.show(show);
    assert_eq!(fills(&shapes), []);
    assert_eq!(outlines(&shapes), []);

    let (response, shapes) = stage.pressing(at_rest.rect.center(), show);
    assert_eq!(fills(&shapes), [(response.rect, PALETTE.raised)]);

    let stage = Stage::new();
    let (response, _) = stage.show(show);
    response.request_focus();
    let (response, shapes) = stage.show(show);
    assert_eq!(outlines(&shapes), [(response.rect, PALETTE.accent)]);
    assert_eq!(line_colors(&shapes), [PALETTE.text], "focus lights it");
}

#[test]
fn a_quiet_button_is_named_for_assistive_technology() {
    let harness = egui_kittest::Harness::new_ui(|ui| {
        QuietButton::icon(Icon::Search, "Find in document").show(ui, PALETTE);
        QuietButton::text("Aa")
            .labelled("Appearance")
            .show(ui, PALETTE);
        QuietButton::text("Reset").enabled(false).show(ui, PALETTE);
    });

    for label in ["Find in document", "Appearance", "Reset"] {
        let node = harness.get_by_label(label);
        assert_eq!(node.accesskit_node().role(), Role::Button, "{label}");
        assert_eq!(node.accesskit_node().is_disabled(), label == "Reset");
    }
    assert!(harness.query_by_label("Aa").is_none());
}

#[test]
fn a_hairline_is_one_pixel_of_the_rule_color_across_the_width() {
    let stage = Stage::new();
    let (taken, shapes) = stage.show(|ui| {
        let top = ui.cursor().top();
        hairline(ui, PALETTE);
        ui.cursor().top() - top
    });

    let lines: Vec<_> = shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, stroke } => Some((*points, *stroke)),
            _ => None,
        })
        .collect();
    let [(points, stroke)] = lines[..] else {
        panic!("one line: {lines:?}");
    };
    assert_close(points[0].x, SCREEN.left());
    assert_close(points[1].x, SCREEN.right());
    assert_close(points[0].y, points[1].y);
    assert_close(stroke.width, 1.0);
    assert_eq!(stroke.color, PALETTE.rule);
    assert!(taken >= 9.0, "air above and below: {taken}");
}

#[test]
fn a_small_label_is_small_muted_text() {
    let stage = Stage::new();
    let (_, shapes) = stage.show(|ui| small_label(ui, PALETTE, "Line spacing"));

    let format = &text_of(&shapes, "Line spacing").galley.job.sections[0].format;
    assert_eq!(format.font_id, small_font());
    assert_eq!(format.color, PALETTE.muted);
}

#[test]
fn a_control_row_centers_a_label_on_the_button_beside_it() {
    let stage = Stage::new();
    let ((label, quiet), _) = stage.show(|ui| {
        control_row(ui, |ui| {
            (
                small_label(ui, PALETTE, "Text size"),
                QuietButton::text("A").show(ui, PALETTE),
            )
        })
    });

    assert_close(label.rect.center().y, quiet.rect.center().y);
    assert_close(quiet.rect.top(), SCREEN.top());
}

#[test]
fn a_menu_row_spans_the_menu_and_is_washed_under_the_pointer() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| MenuRow::new("Quit").show(ui, PALETTE);

    let (response, shapes) = stage.show(show);
    assert_close(response.rect.width(), SCREEN.width());
    assert_close(response.rect.height(), MENU_ROW_HEIGHT);
    assert_eq!(fills(&shapes), []);
    assert_eq!(text_of(&shapes, "Quit").fallback_color, PALETTE.text);

    let (response, shapes) = stage.pointing_at(response.rect.center(), show);
    assert_eq!(fills(&shapes), [(response.rect, PALETTE.raised)]);
}

#[test]
fn a_ticked_row_draws_a_check_mark_in_the_column_every_label_clears() {
    let stage = Stage::new();
    let (_, plain) = stage.show(|ui| MenuRow::new("Outline").show(ui, PALETTE));
    let (_, off) = stage.show(|ui| MenuRow::new("Outline").checked(false).show(ui, PALETTE));
    let (row, on) = stage.show(|ui| MenuRow::new("Outline").checked(true).show(ui, PALETTE));
    let (_, flush) = stage.show(|ui| MenuRow::new("Outline").flush().show(ui, PALETTE));

    assert_eq!(line_colors(&plain), []);
    assert_eq!(line_colors(&off), []);
    assert_eq!(line_colors(&on), [PALETTE.text]);
    let mark = on
        .iter()
        .find(|shape| matches!(shape, Shape::Path(_)))
        .unwrap()
        .visual_bounding_rect();
    let label_left = |shapes: &[Shape]| text_rect(shapes, "Outline").left();
    assert!(mark.right() < label_left(&on), "the mark stands before it");
    assert!(row.rect.contains_rect(mark), "{mark:?} in {:?}", row.rect);
    assert!((mark.center().y - row.rect.center().y).abs() < 1.0);
    assert_close(label_left(&on), row.rect.left() + MENU_LABEL_INSET);
    assert_close(label_left(&off), label_left(&on));
    assert_close(label_left(&plain), label_left(&on));
    assert_close(label_left(&flush), row.rect.left() + MENU_ROW_PADDING);
}

#[test]
fn a_row_ends_in_its_shortcut_or_in_a_submenu_arrow() {
    let stage = Stage::new();
    let (row, shapes) = stage.show(|ui| MenuRow::new("Find…").shortcut("Ctrl+F").show(ui, PALETTE));

    let shortcut = text_of(&shapes, "Ctrl+F");
    assert_eq!(shortcut.fallback_color, PALETTE.muted);
    assert_eq!(shortcut.galley.job.sections[0].format.font_id, small_font());
    let shortcut = text_rect(&shapes, "Ctrl+F");
    assert_close(shortcut.right(), row.rect.right() - MENU_ROW_PADDING);
    assert_close(shortcut.center().y, row.rect.center().y);

    let (row, shapes) = stage.show(|ui| MenuRow::new("Theme").submenu().show(ui, PALETTE));
    assert_eq!(line_colors(&shapes), [PALETTE.muted]);
    let arrow = shapes
        .iter()
        .find(|shape| matches!(shape, Shape::Path(_)))
        .unwrap()
        .visual_bounding_rect();
    assert!(arrow.left() > row.rect.right() - MENU_ROW_PADDING - MENU_MARK_SIDE - 1.0);
    assert!(arrow.right() <= row.rect.right() - MENU_ROW_PADDING + 1.0);
}

#[test]
fn a_disabled_row_is_pale_takes_no_clicks_and_is_not_washed() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| {
        MenuRow::new("Back")
            .shortcut("Alt+Left")
            .enabled(false)
            .show(ui, PALETTE)
    };
    let (at_rest, _) = stage.show(show);
    let center = at_rest.rect.center();

    let (response, shapes) = stage.clicking(center, show);
    assert!(!response.clicked());
    assert_eq!(fills(&shapes), []);
    let pale = PALETTE.muted.gamma_multiply(0.4);
    assert_eq!(text_of(&shapes, "Back").fallback_color, pale);
    assert_eq!(text_of(&shapes, "Alt+Left").fallback_color, pale);

    let stage = Stage::new();
    let (response, _) = stage.clicking(center, |ui| MenuRow::new("Back").show(ui, PALETTE));
    assert!(response.clicked());
}

#[test]
fn a_menu_row_tells_assistive_technology_whether_it_is_a_toggle_and_on() {
    let harness = egui_kittest::Harness::new_ui(|ui| {
        MenuRow::new("Quit").show(ui, PALETTE);
        MenuRow::new("Outline").checked(true).show(ui, PALETTE);
        MenuRow::new("Explorer").checked(false).show(ui, PALETTE);
        MenuRow::new("Back").enabled(false).show(ui, PALETTE);
    });
    let node = |label| harness.get_by_label(label).accesskit_node();

    assert_eq!(node("Quit").role(), Role::Button);
    assert_eq!(node("Quit").toggled(), None);
    assert_eq!(node("Outline").role(), Role::CheckBox);
    assert_eq!(node("Outline").toggled(), Some(Toggled::True));
    assert_eq!(node("Explorer").toggled(), Some(Toggled::False));
    assert!(node("Back").is_disabled());
    assert!(!node("Quit").is_disabled());
}

const MODES: [(u8, &str); 3] = [(0, "Compact"), (1, "Comfortable"), (2, "Airy")];

#[test]
fn a_segmented_control_reports_the_choice_clicked_but_not_the_one_in_force() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| segmented(ui, PALETTE, &MODES, 1, true);
    let (_, shapes) = stage.show(show);
    let center_of = |label| text_rect(&shapes, label).center();

    let (switched, _) = stage.clicking(center_of("Airy"), show);
    assert_eq!(switched, Some(2));

    let stage = Stage::new();
    let (switched, _) = stage.clicking(center_of("Comfortable"), show);
    assert_eq!(switched, None);
}

#[test]
fn segments_share_the_width_when_filling_and_hug_their_labels_otherwise() {
    let stage = Stage::new();
    let (_, filling) = stage.show(|ui| segmented(ui, PALETTE, &MODES, 0, true));
    let (_, hugging) = stage.show(|ui| segmented(ui, PALETTE, &MODES, 0, false));

    // The track is painted first, then the chosen segment on it.
    let (track, chosen) = (fills(&filling)[0].0, fills(&filling)[1].0);
    assert_close(track.width(), SCREEN.width());
    assert_close(
        track.height(),
        2.0_f32.mul_add(SEGMENT_INSET, SEGMENT_HEIGHT),
    );
    assert_close(chosen.width(), segment_share(SCREEN.width(), 3));
    assert_close(chosen.left(), track.left() + SEGMENT_INSET);
    let centers: Vec<f32> = MODES
        .iter()
        .map(|(_, label)| text_rect(&filling, label).center().x)
        .collect();
    assert_close(centers[1] - centers[0], chosen.width());
    assert_close(centers[2] - centers[1], chosen.width());

    let (track, chosen) = (fills(&hugging)[0].0, fills(&hugging)[1].0);
    let label = text_rect(&hugging, "Compact");
    assert_close(
        chosen.width(),
        2.0_f32.mul_add(SEGMENT_PADDING, label.width()),
    );
    let natural: f32 = MODES
        .iter()
        .map(|(_, label)| 2.0_f32.mul_add(SEGMENT_PADDING, text_rect(&hugging, label).width()))
        .sum();
    assert_close(track.width(), 2.0_f32.mul_add(SEGMENT_INSET, natural));
    assert!(track.width() < SCREEN.width(), "{track:?}");
    assert!(
        text_rect(&hugging, "Comfortable").width() > label.width(),
        "each segment is as wide as its own label"
    );
}

#[test]
fn the_chosen_segment_is_lifted_onto_the_page_and_the_rest_stay_muted() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| segmented(ui, PALETTE, &MODES, 2, true);
    let (_, shapes) = stage.show(show);

    let painted = fills(&shapes);
    assert_eq!(painted.len(), 2, "a track and one lifted segment");
    assert_eq!(painted[0].1, PALETTE.raised);
    assert_eq!(painted[1].1, PALETTE.page);
    assert!(painted[1].0.contains(text_rect(&shapes, "Airy").center()));
    assert_eq!(text_of(&shapes, "Airy").fallback_color, PALETTE.text);
    assert_eq!(text_of(&shapes, "Compact").fallback_color, PALETTE.muted);

    let (_, shapes) = stage.pointing_at(text_rect(&shapes, "Compact").center(), show);
    assert_eq!(text_of(&shapes, "Compact").fallback_color, PALETTE.text);
    assert_eq!(
        text_of(&shapes, "Comfortable").fallback_color,
        PALETTE.muted
    );
}

#[test]
fn segments_are_radio_buttons_for_assistive_technology() {
    let harness = egui_kittest::Harness::new_ui(|ui| {
        segmented(
            ui,
            PALETTE,
            &[(false, "Preview"), (true, "Edit")],
            true,
            false,
        );
    });
    let node = |label| harness.get_by_label(label).accesskit_node();

    assert_eq!(node("Edit").role(), Role::RadioButton);
    assert_eq!(node("Edit").toggled(), Some(Toggled::True));
    assert_eq!(node("Preview").toggled(), Some(Toggled::False));
}

#[test]
fn segments_share_what_the_track_inset_leaves() {
    assert_close(segment_share(304.0, 3), 100.0);
    assert_close(segment_share(104.0, 1), 100.0);
    // No width to share, and nothing to share it between.
    assert_close(segment_share(2.0, 3), 0.0);
    assert_close(segment_share(104.0, 0), 100.0);
}

#[test]
fn a_bar_is_the_page_color_with_room_at_its_ends() {
    for theme in ThemeId::ALL {
        let frame = bar_frame(theme.palette());
        assert_eq!(frame.fill, theme.palette().page);
        assert_eq!(frame.stroke, egui::Stroke::NONE);
        assert!(frame.inner_margin.left > 0 && frame.inner_margin.right > 0);
        assert_eq!(frame.inner_margin.top, 0);
    }
}

#[test]
fn a_popup_is_a_rounded_sheet_of_the_surface_color_set_in_the_chrome_text_size() {
    let mut style = egui::Style::default();
    popup_style(ThemeId::Dusk.palette())(&mut style);

    let dusk = ThemeId::Dusk.palette();
    assert_eq!(style.visuals.window_fill, dusk.surface);
    assert_eq!(style.visuals.window_stroke.color, dusk.rule);
    assert_eq!(
        style.visuals.menu_corner_radius,
        egui::CornerRadius::same(10)
    );
    assert_eq!(style.override_font_id, Some(label_font()));
    assert_close(style.spacing.item_spacing.y, 0.0);
}

#[test]
fn a_button_can_tell_that_the_popup_it_is_about_to_anchor_is_open() {
    let stage = Stage::new();
    let show = |ui: &mut egui::Ui| {
        let open = next_widget_popup_is_open(ui);
        let anchor = QuietButton::text("More").lit(open).show(ui, PALETTE);
        egui::Popup::menu(&anchor).show(|ui| ui.label("inside"));
        (open, anchor)
    };

    let ((open, anchor), _) = stage.show(show);
    assert!(!open);

    stage.clicking(anchor.rect.center(), show);
    let ((open, _), shapes) = stage.show(show);
    assert!(open);
    assert_eq!(text_of(&shapes, "More").fallback_color, PALETTE.text);
}
