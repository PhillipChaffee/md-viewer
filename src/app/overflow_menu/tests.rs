//! The overflow menu: picking a row changes what its shortcut changes, and
//! the rows report the state the app is in.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app::ui_harness::UiHarness;
use crate::app::MarkdownApp;
use crate::explorer::FileExplorer;
use crate::layout::{LineSpacing, ReadingWidth};
use crate::test_support::assert_close;
use crate::theme::ThemeId;

/// The four on/off settings of the menu's view group, in menu order.
fn view_toggles(app: &MarkdownApp) -> [bool; 4] {
    [
        app.show_explorer,
        app.show_outline,
        app.reading_width.fits_window(),
        app.smooth_text_rendering,
    ]
}

#[test]
fn each_view_toggle_flips_its_own_setting_and_no_other() {
    let mut harness = UiHarness::with_three_docs();
    let items = [
        "Show Explorer",
        "Show Outline",
        "Full Width",
        "Smooth Text Rendering",
    ];

    for (index, item) in items.into_iter().enumerate() {
        let mut expected = view_toggles(harness.app());
        expected[index] = !expected[index];

        harness.pick(item);

        assert_eq!(view_toggles(harness.app()), expected, "picked {item}");
    }
}

#[test]
fn the_menu_ticks_exactly_the_settings_that_are_on() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().show_explorer = true;
    harness.app_mut().show_outline = false;

    harness.open_more();

    assert!(harness.is_on("Show Explorer"));
    assert!(!harness.is_on("Show Outline"));
}

#[test]
fn picking_a_row_closes_the_menu() {
    let mut harness = UiHarness::with_three_docs();
    harness.open_more();
    assert!(harness.shows("Quit"));

    harness.click("Show Outline");

    assert!(!harness.shows("Quit"));
}

#[test]
fn the_dialog_items_open_their_dialogs() {
    let mut harness = UiHarness::with_three_docs();

    harness.pick("Colors");
    assert!(harness.app().show_color_settings_dialog);
    assert!(!harness.app().font_picker.open);

    harness.pick("Fonts");
    assert!(harness.app().font_picker.open);
}

#[test]
fn the_zoom_row_steps_and_resets_the_zoom_it_reads_out_and_stays_open() {
    let mut harness = UiHarness::with_three_docs();
    harness.open_more();
    assert!(harness.shows("Reset Zoom, 100%"));

    harness.click("Zoom In");
    assert_close(harness.app().zoom_level, 1.25);
    assert!(harness.shows("Reset Zoom, 125%"));

    harness.click("Zoom Out");
    harness.click("Zoom Out");
    assert_close(harness.app().zoom_level, 0.8);
    assert!(harness.shows("Reset Zoom, 80%"));

    harness.click("Reset Zoom");
    assert_close(harness.app().zoom_level, 1.0);
    assert!(harness.shows("Reset Zoom, 100%"));
    assert!(harness.shows("Quit"), "the row is pressed again and again");
}

#[test]
fn the_formula_size_submenu_sets_the_scale_and_ticks_it() {
    let mut harness = UiHarness::with_three_docs();
    assert_close(harness.app().math_scale, 1.0);

    harness.pick("Formula Size");
    assert!(harness.is_on("100%"));
    harness.click_exact("150%");
    assert_close(harness.app().math_scale, 1.5);

    harness.pick("Formula Size");
    assert!(harness.is_on("150%"));
    assert!(!harness.is_on("100%"));
}

#[test]
fn the_history_rows_follow_the_active_tab_history() {
    let mut harness = UiHarness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    harness.open_more();
    assert_eq!(harness.enabled("Back"), [false]);
    assert_eq!(harness.enabled("Forward"), [false]);
    harness.open_more();

    harness.app_mut().navigate_active_tab_to(&b);
    harness.settle();

    harness.open_more();
    assert_eq!(harness.enabled("Forward"), [false]);
    harness.click_exact("Back");
    assert_eq!(harness.active_path(), a);

    harness.open_more();
    assert_eq!(harness.enabled("Back"), [false], "nothing lies behind a.md");
    harness.click_exact("Forward");
    assert_eq!(harness.active_path(), b);
}

#[test]
fn the_file_rows_close_the_tab_open_the_find_bar_and_quit() {
    let mut harness = UiHarness::with_three_docs();

    harness.pick("Find…");
    assert!(harness.app().search.is_open);

    assert!(!harness.close_requested());
    harness.pick("Quit");
    assert!(harness.close_requested());

    harness.pick("Close Tab");
    assert!(harness.app().tabs.is_empty());
}

#[test]
fn rows_that_need_a_document_are_dim_without_one() {
    let mut harness = UiHarness::with_docs(&[]);

    harness.open_more();

    assert_eq!(harness.enabled("Close Tab"), [false]);
    assert_eq!(harness.enabled("Edit Document"), [false]);
    assert_eq!(harness.enabled("Open Folder…").last(), Some(&true));
}

#[test]
fn edit_document_switches_to_the_source_and_back_and_is_ticked_meanwhile() {
    let mut harness = UiHarness::with_three_docs();

    harness.pick("Edit Document");
    assert!(harness.app().tabs[0].is_editing);
    harness.open_more();
    assert!(harness.is_on("Edit Document"));

    harness.click("Edit Document");
    assert!(!harness.app().tabs[0].is_editing);
    harness.open_more();
    assert!(!harness.is_on("Edit Document"));
}

#[test]
fn live_reload_turns_file_watching_on_and_off_and_is_ticked_while_on() {
    let mut harness = UiHarness::with_three_docs();
    harness.open_more();
    assert!(!harness.is_on("Live Reload"));

    harness.click("Live Reload");
    assert!(harness.app().live_reload.is_live());
    harness.open_more();
    assert!(harness.is_on("Live Reload"));

    harness.click("Live Reload");
    assert!(!harness.app().live_reload.is_live());
}

#[test]
fn live_reload_is_ticked_while_on_with_nothing_to_watch_and_a_click_unticks_it() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().close_active_tab();
    harness.app_mut().file_explorer = FileExplorer::default();
    harness.open_more();
    harness.click("Live Reload");
    assert!(!harness.app().live_reload.is_live());

    harness.open_more();
    assert!(harness.is_on("Live Reload"));

    harness.click("Live Reload");
    harness.open_more();
    assert!(!harness.is_on("Live Reload"));
}

#[test]
fn open_file_opens_what_the_picker_returns_in_a_new_tab() {
    let mut harness = UiHarness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    let asked_in: Rc<RefCell<Option<PathBuf>>> = Rc::default();
    let (seen, picked) = (Rc::clone(&asked_in), b.clone());
    harness.app_mut().file_picker = Box::new(move |start| {
        *seen.borrow_mut() = start.map(PathBuf::from);
        Ok(Some(picked.clone()))
    });

    harness.pick("Open File");

    let app = harness.app();
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.tabs[app.active_tab].path, b);
    assert_eq!(
        asked_in.borrow().as_deref(),
        a.parent(),
        "the dialog starts beside the active document"
    );
}

#[test]
fn open_folder_points_the_explorer_at_the_picked_folder() {
    let mut harness = UiHarness::with_three_docs();
    let folder = harness.path("notes");
    std::fs::create_dir(&folder).unwrap();
    harness.app_mut().show_explorer = false;
    let picked = folder.clone();
    harness.app_mut().folder_picker = Box::new(move |_| Ok(Some(picked.clone())));

    harness.pick("Open Folder");

    assert_eq!(harness.app().file_explorer.root(), Some(folder.as_path()));
    assert!(harness.app().show_explorer, "the new root is put on screen");
}

#[test]
fn the_theme_submenu_picks_a_theme_by_name_and_ticks_the_one_in_use() {
    let mut harness = UiHarness::with_three_docs();
    assert_eq!(harness.app().theme, ThemeId::Sepia);

    harness.pick("Theme");
    assert!(harness.is_on("Sepia"));
    harness.click_exact("Night");
    assert_eq!(harness.app().theme, ThemeId::Night);

    harness.pick("Theme");
    assert!(harness.is_on("Night"));
    assert!(!harness.is_on("Sepia"));
    harness.click("Next Theme");
    assert_eq!(harness.app().theme, ThemeId::Paper);
}

#[test]
fn the_line_spacing_submenu_picks_a_spacing_and_ticks_the_one_in_use() {
    let mut harness = UiHarness::with_three_docs();
    assert_eq!(harness.app().line_spacing, LineSpacing::Comfortable);

    harness.pick("Line Spacing");
    assert!(harness.is_on("Comfortable"));
    harness.click_exact("Airy");
    assert_eq!(harness.app().line_spacing, LineSpacing::Airy);

    harness.pick("Line Spacing");
    assert!(harness.is_on("Airy"));
    assert!(!harness.is_on("Comfortable"));
    harness.click_exact("Compact");
    assert_eq!(harness.app().line_spacing, LineSpacing::Compact);
}

#[test]
fn full_width_is_ticked_only_while_the_column_fills_the_window() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().reading_width = ReadingWidth::new(900.0);

    harness.open_more();
    assert!(!harness.is_on("Full Width"));
    harness.click("Full Width");
    assert!(harness.app().reading_width.fits_window());

    harness.open_more();
    assert!(harness.is_on("Full Width"));
    harness.click("Full Width");
    assert_eq!(harness.app().reading_width, ReadingWidth::DEFAULT);
}
