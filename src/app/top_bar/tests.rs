//! The top bar, clicked and pointed at in the running app.

use eframe::egui::{self, Color32, Rangef, Shape};
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::ui_harness::UiHarness;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

/// Width of the window the harness lays the app out in.
const WINDOW_WIDTH: f32 = 1200.0;

/// What the last frame painted inside the bar.
fn bar_shapes(harness: &UiHarness) -> Vec<Shape> {
    painted_shapes(harness.ui.output())
        .into_iter()
        .filter(|shape| shape.visual_bounding_rect().bottom() <= BAR_HEIGHT)
        .collect()
}

/// The text the bar painted that reads `text`.
#[track_caller]
fn bar_text(harness: &UiHarness, text: &str) -> egui::epaint::TextShape {
    let found = bar_shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => Some(shape),
            _ => None,
        });
    found.expect("the bar did not paint the text")
}

/// The colors of the bar's icons: every line stroked in it.
fn icon_colors(harness: &UiHarness) -> Vec<Color32> {
    let mut colors: Vec<Color32> = bar_shapes(harness)
        .iter()
        .filter_map(|shape| match shape {
            Shape::Path(path) => match &path.stroke.color {
                egui::epaint::ColorMode::Solid(color) => Some(*color),
                egui::epaint::ColorMode::UV(_) => None,
            },
            Shape::Circle(circle) => Some(circle.stroke.color),
            Shape::Rect(rect) if !rect.stroke.is_empty() => Some(rect.stroke.color),
            _ => None,
        })
        .collect();
    colors.sort_by_key(Color32::to_array);
    colors.dedup();
    colors
}

#[test]
fn the_buttons_at_the_left_toggle_the_explorer_and_walk_the_history() {
    let mut harness = UiHarness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    let shown = harness.app().show_explorer;

    harness.click_exact("Files sidebar");
    assert_eq!(harness.app().show_explorer, !shown);
    harness.click_exact("Files sidebar");
    assert_eq!(harness.app().show_explorer, shown);

    assert_eq!(harness.enabled("Go back"), [false]);
    assert_eq!(harness.enabled("Go forward"), [false]);
    harness.app_mut().navigate_active_tab_to(&b);
    harness.settle();
    assert_eq!(harness.enabled("Go back"), [true]);
    assert_eq!(harness.enabled("Go forward"), [false]);

    harness.click_exact("Go back");
    assert_eq!(harness.active_path(), a);
    assert_eq!(harness.enabled("Go back"), [false]);
    harness.click_exact("Go forward");
    assert_eq!(harness.active_path(), b);
}

#[test]
fn the_buttons_at_the_right_open_the_find_bar_and_toggle_the_outline() {
    let mut harness = UiHarness::with_three_docs();
    let shown = harness.app().show_outline;

    harness.click_exact("Contents sidebar");
    assert_eq!(harness.app().show_outline, !shown);
    harness.click_exact("Contents sidebar");
    assert_eq!(harness.app().show_outline, shown);

    assert!(!harness.app().search.is_open);
    harness.click_exact("Find in document");
    assert!(harness.app().search.is_open);
}

#[test]
fn the_buttons_read_left_to_right_in_the_order_the_tab_key_walks_them() {
    let harness = UiHarness::with_three_docs();
    let order = [
        "Files sidebar",
        "Go back",
        "Go forward",
        "Appearance",
        "Find in document",
        "Contents sidebar",
        "More",
    ];

    let centers: Vec<egui::Pos2> = order.map(|label| harness.labelled(label)[0].0).into();
    assert!(centers.is_sorted_by(|a, b| a.x < b.x), "{centers:?}");
    for center in &centers {
        assert_close(center.y, BAR_HEIGHT / 2.0);
    }
    // The first and the last stand the same distance from the window's edges.
    assert_close(centers[0].x, WINDOW_WIDTH - centers[6].x);
    assert!(centers[0].x < 30.0, "{centers:?}");
}

#[test]
fn the_buttons_at_the_right_fill_the_room_the_bar_keeps_for_them() {
    let harness = UiHarness::with_three_docs();
    let first = harness.ui.get_by_label("Appearance").rect();
    let last = harness.ui.get_by_label("More").rect();
    let files = harness.ui.get_by_label("Files sidebar").rect();

    let kept = TRAILING_BUTTONS.mul_add(BUTTON_SIDE, (TRAILING_BUTTONS - 1.0) * BUTTON_GAP);
    assert_close(last.right() - first.left(), kept);
    assert_close(WINDOW_WIDTH - last.right(), files.left());
}

#[test]
fn the_bar_holds_only_its_buttons_without_a_document() {
    let harness = UiHarness::with_docs(&[]);

    let texts = bar_shapes(&harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["Aa"], "only the appearance button is text");
}

#[test]
fn the_icons_are_muted_pale_where_there_is_nothing_to_do_and_dark_under_the_pointer() {
    let mut harness = UiHarness::with_three_docs();
    let palette = ThemeId::Sepia.palette();
    let pale = palette.muted.gamma_multiply(0.4);
    let mut at_rest = vec![palette.muted, pale];
    at_rest.sort_by_key(Color32::to_array);
    assert_eq!(icon_colors(&harness), at_rest, "no history: pale arrows");

    harness.hover_at(harness.labelled("Find in document")[0].0);
    assert!(icon_colors(&harness).contains(&palette.text));
}

#[test]
fn the_icons_take_the_ink_of_the_theme_in_use() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().theme = ThemeId::Night;
    harness.settle();

    let night = ThemeId::Night.palette();
    assert!(icon_colors(&harness).contains(&night.muted));
    assert!(!icon_colors(&harness).contains(&ThemeId::Sepia.palette().muted));
}

#[test]
fn the_button_whose_popup_is_open_stays_lit() {
    let mut harness = UiHarness::with_three_docs();
    let palette = ThemeId::Sepia.palette();
    assert_eq!(bar_text(&harness, "Aa").fallback_color, palette.muted);

    harness.click_exact("Appearance");
    harness.hover_at(egui::pos2(300.0, 500.0));

    assert_eq!(bar_text(&harness, "Aa").fallback_color, palette.text);
}

#[test]
fn the_row_starts_after_its_inset_and_ends_with_the_buttons_at_the_right() {
    let row = Row::new(Rangef::new(10.0, 1190.0), 78.0);

    assert_eq!(row.inset, Rangef::new(10.0, 88.0));
    assert_close(row.leading.min, 88.0);
    assert_close(row.leading.span(), buttons_width(LEADING_BUTTONS));
    assert_close(row.trailing.max, 1190.0);
    assert_close(row.trailing.span(), buttons_width(TRAILING_BUTTONS));
    assert_close(row.middle.min, row.leading.max + BUTTON_GAP);
    assert_close(row.middle.max, row.trailing.min - BUTTON_GAP);

    let flush = Row::new(Rangef::new(10.0, 1190.0), 0.0);
    assert_close(flush.inset.span(), 0.0);
    assert_close(flush.middle.span(), row.middle.span() + 78.0);
}

#[test]
fn a_bar_too_narrow_for_its_buttons_has_an_empty_middle() {
    let row = Row::new(Rangef::new(0.0, 200.0), 78.0);

    assert_close(row.middle.span(), 0.0);
    assert!(!row.fits_edit_controls());
    assert_eq!(new_tab_span(row.tab_room(None), 0.0), None);
}

#[test]
fn the_tabs_take_the_middle_up_to_the_edit_controls() {
    let row = Row::new(Rangef::new(0.0, 1000.0), 0.0);

    assert_eq!(row.tab_room(None), row.middle);
    let room = row.tab_room(Some(700.0));
    assert_close(room.min, row.middle.min);
    assert_close(room.max, 700.0 - BUTTON_GAP);
    // Controls that start before the middle leave the tabs no room.
    assert_close(row.tab_room(Some(0.0)).span(), 0.0);
    assert_close(tabs_width(room), room.span() - BUTTON_SIDE);
}

#[test]
fn the_new_tab_button_follows_the_tabs_and_stays_in_their_room() {
    let room = Rangef::new(100.0, 500.0);

    assert_eq!(new_tab_span(room, 0.0), Some(Rangef::new(100.0, 128.0)));
    assert_eq!(new_tab_span(room, 150.0), Some(Rangef::new(250.0, 278.0)));
    let most = tabs_width(room);
    assert_eq!(new_tab_span(room, most), Some(Rangef::new(472.0, 500.0)));
    assert_eq!(new_tab_span(room, 9000.0), Some(Rangef::new(472.0, 500.0)));
    assert_eq!(new_tab_span(Rangef::new(100.0, 120.0), 0.0), None);
}

#[test]
fn the_background_is_the_inset_and_what_the_tabs_leave() {
    let row = Row::new(Rangef::new(0.0, 1000.0), 78.0);
    let room = row.tab_room(Some(700.0));

    let [inset, spare] = row.background(room, 400.0);
    assert_eq!(inset, Rangef::new(0.0, 78.0));
    assert_eq!(spare, Rangef::new(400.0, room.max));
    let [_, none] = row.background(room, 9000.0);
    assert_close(none.span(), 0.0);
}

#[test]
fn the_edit_controls_join_the_row_only_where_it_has_room_for_them() {
    let buttons = buttons_width(LEADING_BUTTONS) + buttons_width(TRAILING_BUTTONS);
    let edge = 2.0_f32.mul_add(BUTTON_GAP, buttons);

    assert!(Row::new(Rangef::new(0.0, edge + EDIT_CONTROLS_ROOM), 0.0).fits_edit_controls());
    assert!(!Row::new(Rangef::new(0.0, edge + EDIT_CONTROLS_ROOM - 1.0), 0.0).fits_edit_controls());
    assert!(!Row::new(Rangef::new(0.0, edge + EDIT_CONTROLS_ROOM), 1.0).fits_edit_controls());
}
