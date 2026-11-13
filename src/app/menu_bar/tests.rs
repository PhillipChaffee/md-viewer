//! The menu bar: picking an item changes what its shortcut changes, and the
//! bar reports the state the app is in.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app::ui_harness::UiHarness;
use crate::app::MarkdownApp;
use crate::layout::{LineSpacing, ReadingWidth};
use crate::test_support::assert_close;
use crate::theme::ThemeId;

/// The four on/off settings of the View menu, in menu order.
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

        harness.pick("View", item);

        assert_eq!(view_toggles(harness.app()), expected, "picked {item}");
    }
}

#[test]
fn the_view_menu_ticks_exactly_the_settings_that_are_on() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().show_explorer = true;
    harness.app_mut().show_outline = false;

    harness.click_exact("View");

    assert!(harness.shows("✓ Show Explorer"));
    assert!(harness.shows("Show Outline"));
    assert!(!harness.shows("✓ Show Outline"));
}

#[test]
fn the_dialog_items_open_their_dialogs() {
    let mut harness = UiHarness::with_three_docs();

    harness.pick("View", "Colors");
    assert!(harness.app().show_color_settings_dialog);
    assert!(!harness.app().show_font_dialog);

    harness.pick("View", "Fonts");
    assert!(harness.app().show_font_dialog);
}

#[test]
fn the_zoom_items_step_and_reset_the_zoom_the_status_readout_shows() {
    let mut harness = UiHarness::with_three_docs();
    assert!(!harness.shows("%"), "100% is not worth a readout");

    harness.pick("View", "Zoom In");
    assert_close(harness.app().zoom_level, 1.25);
    assert!(harness.shows("125%"));

    harness.pick("View", "Zoom Out");
    harness.pick("View", "Zoom Out");
    assert_close(harness.app().zoom_level, 0.8);
    assert!(harness.shows("80%"));

    harness.pick("View", "Reset Zoom");
    assert_close(harness.app().zoom_level, 1.0);
    assert!(!harness.shows("%"));
}

#[test]
fn the_formula_size_submenu_sets_the_scale_and_ticks_it() {
    let mut harness = UiHarness::with_three_docs();
    assert_close(harness.app().math_scale, 1.0);

    harness.click_exact("View");
    harness.click("Formula Size");
    assert!(harness.shows("✓ 100%"));
    harness.click("150%");
    assert_close(harness.app().math_scale, 1.5);

    harness.click_exact("View");
    harness.click("Formula Size");
    assert!(harness.shows("✓ 150%"));
    assert!(!harness.shows("✓ 100%"));
}

#[test]
fn history_items_and_arrows_follow_the_active_tab_history() {
    let mut harness = UiHarness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    let active = |harness: &UiHarness| {
        let app = harness.app();
        app.tabs[app.active_tab].path.clone()
    };
    assert!(!harness.is_enabled("◀"));
    assert!(!harness.is_enabled("▶"));

    harness.app_mut().navigate_active_tab_to(&b);
    harness.settle();
    assert!(harness.is_enabled("◀"));
    assert!(!harness.is_enabled("▶"));

    harness.click("◀");
    assert_eq!(active(&harness), a);
    harness.click("▶");
    assert_eq!(active(&harness), b);

    harness.pick("Navigate", "Back");
    assert_eq!(active(&harness), a);
    harness.click_exact("Navigate");
    assert!(!harness.is_enabled("Back"), "nothing lies behind a.md");
    harness.click("Forward");
    assert_eq!(active(&harness), b);
}

#[test]
fn file_menu_items_close_the_tab_open_the_find_bar_and_quit() {
    let mut harness = UiHarness::with_three_docs();

    harness.pick("File", "Find");
    assert!(harness.app().search.is_open);

    assert!(!harness.close_requested());
    harness.pick("File", "Quit");
    assert!(harness.close_requested());

    harness.pick("File", "Close Tab");
    assert!(harness.app().tabs.is_empty());
}

#[test]
fn watch_files_turns_live_reload_on_and_the_bar_says_so() {
    let mut harness = UiHarness::with_three_docs();
    assert!(!harness.shows("LIVE"));

    harness.pick("File", "Watch Files");
    assert!(harness.app().watcher.is_some());
    assert!(harness.shows("LIVE"));
    harness.click_exact("File");
    assert!(harness.shows("✓ Watch Files"));

    harness.click("Watch Files");
    assert!(harness.app().watcher.is_none());
    assert!(!harness.shows("LIVE"));
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

    harness.pick("File", "Open File");

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

    harness.pick("File", "Open Folder");

    assert_eq!(harness.app().file_explorer.root, Some(folder));
    assert!(harness.app().show_explorer, "the new root is put on screen");
}

#[test]
fn the_status_readout_names_the_active_file_while_it_exists() {
    let mut harness = UiHarness::with_three_docs();
    let a = harness.path("a.md");
    let shown_path = a.display().to_string();
    assert!(harness.shows(&shown_path));

    std::fs::remove_file(&a).unwrap();
    harness.settle();
    assert!(!harness.shows(&shown_path));
}

#[test]
fn the_theme_menu_picks_a_theme_by_name_and_ticks_the_one_in_use() {
    let mut harness = UiHarness::with_three_docs();
    assert_eq!(harness.app().theme, ThemeId::Sepia);

    harness.click_exact("View");
    harness.click("Theme");
    assert!(harness.shows("✓ Sepia"));
    harness.click("Night");
    assert_eq!(harness.app().theme, ThemeId::Night);

    harness.click_exact("View");
    harness.click("Theme");
    assert!(harness.shows("✓ Night"));
    assert!(!harness.shows("✓ Sepia"));
    harness.click("Next Theme");
    assert_eq!(harness.app().theme, ThemeId::Paper);
}

#[test]
fn the_line_spacing_menu_picks_a_spacing_and_ticks_the_one_in_use() {
    let mut harness = UiHarness::with_three_docs();
    assert_eq!(harness.app().line_spacing, LineSpacing::Comfortable);

    harness.click_exact("View");
    harness.click("Line Spacing");
    assert!(harness.shows("✓ Comfortable"));
    harness.click("Airy");
    assert_eq!(harness.app().line_spacing, LineSpacing::Airy);

    harness.click_exact("View");
    harness.click("Line Spacing");
    assert!(harness.shows("✓ Airy"));
    assert!(!harness.shows("✓ Comfortable"));
    harness.click("Compact");
    assert_eq!(harness.app().line_spacing, LineSpacing::Compact);
}

#[test]
fn full_width_is_ticked_only_while_the_column_fills_the_window() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().reading_width = ReadingWidth::new(900.0);

    harness.click_exact("View");
    assert!(!harness.shows("✓ Full Width"));
    harness.click("Full Width");
    assert!(harness.app().reading_width.fits_window());

    harness.click_exact("View");
    assert!(harness.shows("✓ Full Width"));
    harness.click("Full Width");
    assert_eq!(harness.app().reading_width, ReadingWidth::DEFAULT);
}
