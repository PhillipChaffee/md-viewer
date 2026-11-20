//! Whose the title bar is, as plain values and in the running app with
//! the platform set by hand.

use eframe::egui::{self, ViewportCommand};
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::ui_harness::UiHarness;
use crate::test_support::assert_close;

/// The bar's own margin: where its first button starts without an inset.
const BAR_LEFT: f32 = 10.0;
/// The middle of the top row, in the app's points at 100%.
const ROW_MIDDLE: f32 = BAR_HEIGHT / 2.0;

/// The app as it runs on `platform`, with `a.md`, `b.md` and `c.md` each
/// in a tab named by its file and `c.md` in front. The explorer is
/// hidden, so a file name labels its tab and nothing else.
fn app_on(platform: Platform) -> UiHarness {
    let mut harness =
        UiHarness::with_docs(&[("a.md", "Alpha."), ("b.md", "Beta."), ("c.md", "Gamma.")]);
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));
    let app = harness.app_mut();
    app.platform = platform;
    app.show_explorer = false;
    app.open_in_new_tab(b);
    app.open_in_new_tab(c);
    harness.settle();
    harness.sent.clear();
    harness
}

/// Where a widget labelled `label` starts, in the screen's points from
/// the window's left edge. The accessibility tree reports in those, and
/// they are the points the window's buttons are measured in.
fn left_on_screen(harness: &UiHarness, label: &str) -> f32 {
    harness.ui.get_by_label(label).rect().left()
}

/// What the app has asked of the window, its title aside.
fn asked_of_window(harness: &UiHarness) -> Vec<ViewportCommand> {
    let mut asked = harness.sent.clone();
    asked.retain(|command| !matches!(command, ViewportCommand::Title(_)));
    asked
}

/// A point of the row that holds nothing: right after the new-tab button.
fn empty_room(harness: &UiHarness) -> egui::Pos2 {
    let button = harness.ui.get_by_label("New tab").rect();
    egui::pos2(button.right() + 40.0, ROW_MIDDLE)
}

/// Press at `from`, pull the pointer 30 px to the right and let go.
fn drag_from(harness: &mut UiHarness, from: egui::Pos2) {
    harness.ui.drag_at(from);
    harness.step();
    harness.ui.hover_at(from + egui::vec2(30.0, 0.0));
    harness.step();
    harness.ui.drop_at(from + egui::vec2(30.0, 0.0));
    harness.settle();
}

/// Click twice at `pos`, a frame apart: fast enough to be a double click.
fn double_click(harness: &mut UiHarness, pos: egui::Pos2) {
    harness.ui.hover_at(pos);
    harness.step();
    for _ in 0..2 {
        let clicks = [true, false].map(|pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.ui.input_mut().events.extend(clicks);
        harness.step();
    }
    harness.settle();
}

fn fill_the_screen(harness: &mut UiHarness) {
    let viewport = harness
        .ui
        .input_mut()
        .viewports
        .get_mut(&egui::ViewportId::ROOT);
    viewport.expect("the root viewport").fullscreen = Some(true);
    harness.settle();
}

#[test]
fn only_a_mac_window_that_does_not_fill_the_screen_has_the_row_for_its_title_bar() {
    assert_eq!(
        TitleBar::of(Platform::Mac, false, 1.5),
        TitleBar::Row { zoom: 1.5 }
    );
    assert_eq!(TitleBar::of(Platform::Mac, true, 1.5), TitleBar::Systems);
    assert_eq!(TitleBar::of(Platform::Other, false, 1.5), TitleBar::Systems);
    assert_eq!(TitleBar::of(Platform::Other, true, 1.5), TitleBar::Systems);
}

/// The window's buttons do not grow with the app's zoom, so the room kept
/// for them is the same on the screen at every zoom: fewer of the app's
/// points when those are larger.
#[test]
fn the_row_keeps_the_same_room_on_the_screen_for_the_buttons_at_every_zoom() {
    for zoom in [0.5, 1.0, 1.25, 2.0, 3.0] {
        let inset = TitleBar::of(Platform::Mac, false, zoom).inset(BAR_LEFT);
        assert_close((BAR_LEFT + inset) * zoom, WINDOW_BUTTONS_ROOM);
    }
    // A margin already wider than the room needs nothing added.
    assert_close(TitleBar::Row { zoom: 1.0 }.inset(200.0), 0.0);
}

#[test]
fn the_row_keeps_no_room_in_full_screen_or_off_a_mac() {
    for zoom in [0.5, 1.0, 3.0] {
        assert_close(TitleBar::of(Platform::Mac, true, zoom).inset(BAR_LEFT), 0.0);
        assert_close(
            TitleBar::of(Platform::Other, false, zoom).inset(BAR_LEFT),
            0.0,
        );
        assert_close(
            TitleBar::of(Platform::Other, true, zoom).inset(BAR_LEFT),
            0.0,
        );
    }
}

#[test]
fn the_buttons_are_centred_on_a_row_that_grows_with_the_zoom() {
    let height = |zoom| TitleBar::of(Platform::Mac, false, zoom).row_height_on_screen();

    assert_eq!(height(1.0), Some(40.0));
    assert_eq!(height(1.5), Some(60.0));
    assert_eq!(height(0.5), Some(20.0));
    let full_screen = TitleBar::of(Platform::Mac, true, 1.0);
    assert_eq!(full_screen.row_height_on_screen(), None);
    let elsewhere = TitleBar::of(Platform::Other, false, 1.0);
    assert_eq!(elsewhere.row_height_on_screen(), None);
}

#[test]
fn a_drag_moves_the_window_and_a_double_click_zooms_it_only_by_a_title_bar() {
    let row = TitleBar::Row { zoom: 1.0 };

    assert_eq!(row.pick(true, false), Some(AppAction::MoveWindow));
    assert_eq!(row.pick(false, true), Some(AppAction::ToggleMaximized));
    assert_eq!(row.pick(true, true), Some(AppAction::ToggleMaximized));
    assert_eq!(row.pick(false, false), None);
    for (dragged, double_clicked) in [(true, false), (false, true), (true, true)] {
        assert_eq!(TitleBar::Systems.pick(dragged, double_clicked), None);
    }
}

#[test]
fn on_a_mac_the_row_starts_after_the_windows_buttons() {
    let harness = app_on(Platform::Mac);
    assert_close(
        left_on_screen(&harness, "Files sidebar"),
        WINDOW_BUTTONS_ROOM,
    );

    let elsewhere = app_on(Platform::Other);
    assert_close(left_on_screen(&elsewhere, "Files sidebar"), BAR_LEFT);
}

#[test]
fn zoomed_the_row_still_starts_after_the_windows_buttons() {
    let mut harness = app_on(Platform::Mac);

    harness.app_mut().zoom_level = 2.0;
    harness.settle();

    assert_close(
        left_on_screen(&harness, "Files sidebar"),
        WINDOW_BUTTONS_ROOM,
    );
}

#[test]
fn in_full_screen_the_row_starts_at_the_edge_on_a_mac_too() {
    let mut harness = app_on(Platform::Mac);

    fill_the_screen(&mut harness);

    assert_close(left_on_screen(&harness, "Files sidebar"), BAR_LEFT);
}

#[test]
fn dragging_the_rows_empty_room_moves_a_mac_window() {
    let mut harness = app_on(Platform::Mac);

    let from = empty_room(&harness);
    drag_from(&mut harness, from);
    assert_eq!(asked_of_window(&harness), [ViewportCommand::StartDrag]);

    // The room kept for the window's buttons moves it too.
    harness.sent.clear();
    drag_from(&mut harness, egui::pos2(40.0, ROW_MIDDLE));
    assert_eq!(asked_of_window(&harness), [ViewportCommand::StartDrag]);
}

#[test]
fn dragging_a_tab_or_a_button_does_not_move_the_window() {
    let mut harness = app_on(Platform::Mac);
    let front = harness.active_path();

    for label in ["b.md", "Files sidebar", "New tab", "More"] {
        let on_it = harness.ui.get_by_label(label).rect().center();
        drag_from(&mut harness, on_it);
    }

    assert_eq!(asked_of_window(&harness), []);
    assert_eq!(harness.active_path(), front, "a drag is not a click");
}

#[test]
fn a_tab_still_takes_its_click_on_a_mac() {
    let mut harness = app_on(Platform::Mac);

    harness.click_exact("b.md");

    assert_eq!(harness.active_path(), harness.path("b.md"));
    assert_eq!(asked_of_window(&harness), []);
}

#[test]
fn a_double_click_on_the_rows_empty_room_zooms_a_mac_window_and_back() {
    let mut harness = app_on(Platform::Mac);
    let at = empty_room(&harness);

    double_click(&mut harness, at);
    assert_eq!(
        asked_of_window(&harness),
        [ViewportCommand::Maximized(true)]
    );

    harness.sent.clear();
    let viewport = harness
        .ui
        .input_mut()
        .viewports
        .get_mut(&egui::ViewportId::ROOT);
    viewport.expect("the root viewport").maximized = Some(true);
    double_click(&mut harness, at);
    assert_eq!(
        asked_of_window(&harness),
        [ViewportCommand::Maximized(false)]
    );
}

#[test]
fn off_a_mac_the_rows_empty_room_asks_nothing_of_the_window() {
    let mut harness = app_on(Platform::Other);
    let at = empty_room(&harness);

    drag_from(&mut harness, at);
    double_click(&mut harness, at);

    assert_eq!(asked_of_window(&harness), []);
}

#[test]
fn in_full_screen_the_rows_empty_room_asks_nothing_of_a_mac_window() {
    let mut harness = app_on(Platform::Mac);
    fill_the_screen(&mut harness);
    harness.sent.clear();
    let at = empty_room(&harness);

    drag_from(&mut harness, at);
    double_click(&mut harness, at);

    assert_eq!(asked_of_window(&harness), []);
}
