//! The top bar, clicked and pointed at in the running app.

use eframe::egui::{self, Color32, Key, Modifiers, Rangef, Shape};
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::ui_harness::UiHarness;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

/// Width of the window the harness lays the app out in.
const WINDOW_WIDTH: f32 = 1200.0;

/// One document whose heading is unlike its file name.
fn field_notes() -> UiHarness {
    UiHarness::with_docs(&[("notes.md", "# Field Notes\n\nBody.")])
}

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

/// Where the bar's title, which reads `name`, is: the topmost widget of
/// that name, above the document's own heading.
#[track_caller]
fn title_of(harness: &UiHarness, name: &str) -> egui::Pos2 {
    let named = harness.labelled(name);
    let topmost = named.iter().map(|(center, _)| *center);
    topmost
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .expect("no widget of that name")
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
fn the_bar_names_the_document_by_its_first_heading_else_by_its_file() {
    let harness = field_notes();
    assert_eq!(
        bar_text(&harness, "Field Notes").galley.text(),
        "Field Notes"
    );

    let harness = UiHarness::with_docs(&[("plain.md", "No heading here.")]);
    assert_eq!(bar_text(&harness, "plain.md").galley.text(), "plain.md");
}

#[test]
fn the_bar_has_no_title_without_a_document() {
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
fn the_title_is_small_and_stands_in_the_middle_of_the_bar() {
    let harness = field_notes();

    let title = bar_text(&harness, "Field Notes");
    let size = title.galley.size();
    assert_close(title.pos.x + size.x / 2.0, WINDOW_WIDTH / 2.0);
    assert_close(title.pos.y + size.y / 2.0, BAR_HEIGHT / 2.0);
    assert_eq!(
        title.galley.job.sections[0].format.font_id,
        chrome::label_font()
    );
}

/// One document whose heading is far too long for the bar.
fn long_heading() -> UiHarness {
    let heading = "A heading that goes on and on ".repeat(12);
    UiHarness::with_docs(&[("long.md", &format!("# {heading}\n\nBody."))])
}

/// The title the bar painted for [`long_heading`].
fn long_title(harness: &UiHarness) -> egui::epaint::TextShape {
    let mut texts = bar_shapes(harness).into_iter();
    let title = texts.find_map(|shape| match shape {
        Shape::Text(text) if text.galley.text().starts_with("A heading") => Some(text),
        _ => None,
    });
    title.unwrap()
}

#[test]
fn a_title_too_long_for_the_bar_is_cut_short_between_the_buttons() {
    let harness = long_heading();

    let title = long_title(&harness);
    assert!(title.galley.elided, "it ends in an ellipsis");
    let half_button = BUTTON_SIDE / 2.0;
    let forward = harness.labelled("Go forward")[0].0.x + half_button;
    let appearance = harness.labelled("Appearance")[0].0.x - half_button;
    assert!(title.pos.x >= forward + TITLE_GAP - 0.5, "{}", title.pos.x);
    let right = title.pos.x + title.galley.size().x;
    assert!(right <= appearance - TITLE_GAP + 0.5, "{right}");
}

#[test]
fn a_long_title_stops_short_of_the_edit_controls_of_a_lone_document() {
    let mut harness = long_heading();
    harness.press(Modifiers::CTRL, Key::E);

    let title = long_title(&harness);
    let toggle = harness.ui.get_by_label("Preview").rect();
    let right = title.pos.x + title.galley.size().x;
    assert!(right <= toggle.left() - TITLE_GAP + 0.5, "{right}");
    assert!(toggle.right() < harness.labelled("Appearance")[0].0.x);
}

#[test]
fn the_title_is_muted_until_it_is_pointed_at() {
    let mut harness = field_notes();
    let palette = ThemeId::Sepia.palette();
    assert_eq!(
        bar_text(&harness, "Field Notes").fallback_color,
        palette.muted
    );

    harness.hover_at(title_of(&harness, "Field Notes"));

    assert_eq!(
        bar_text(&harness, "Field Notes").fallback_color,
        palette.text
    );
}

#[test]
fn pointing_at_the_title_says_where_the_file_is_and_whether_it_is_live() {
    let mut harness = field_notes();
    let path = harness.path("notes.md").display().to_string();
    assert!(!harness.shows(&path));

    harness.hover_at(title_of(&harness, "Field Notes"));
    harness.frames(8);
    assert!(harness.shows(&path));
    assert!(!harness.shows("Live"));

    let ctx = harness.ui.ctx.clone();
    harness.app_mut().apply_action(&ctx, AppAction::ToggleWatch);
    harness.frames(8);
    assert!(harness.shows(&path));
    assert!(harness.shows("Live: reloads when the file changes"));
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
fn a_title_with_room_is_centered_on_the_bar() {
    let free = Rangef::new(100.0, 1000.0);
    assert_close(title_center(600.0, free, 200.0), 600.0);
}

#[test]
fn a_title_that_would_reach_the_buttons_moves_just_clear_of_them() {
    // The free span is off-center: the buttons at the right take more room.
    let free = Rangef::new(100.0, 700.0);
    let center = title_center(600.0, free, 300.0);
    assert_close(center, 700.0 - 150.0 - TITLE_GAP);

    let free = Rangef::new(500.0, 1100.0);
    let center = title_center(600.0, free, 300.0);
    assert_close(center, 500.0 + 150.0 + TITLE_GAP);
}

#[test]
fn a_title_wider_than_the_free_span_is_centered_in_it() {
    let free = Rangef::new(100.0, 300.0);
    assert_close(title_center(600.0, free, 400.0), 200.0);
}
