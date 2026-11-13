//! What the app starts with, given the launch arguments and the settings the
//! last session left behind.

use std::path::PathBuf;

use eframe::egui::{self, Color32};

use super::is_virtual_display;
use crate::app::ui_harness::{launch, MemoryStorage, ScratchDir};
use crate::app::MarkdownApp;
use crate::explorer::SortOrder;
use crate::persist::{PersistedState, APP_KEY};
use crate::recent::RecentEntry;
use crate::system_fonts::{FontPreset, TextSizeClass};
use crate::test_support::assert_close;

/// Launch with `persisted` as the last session's settings.
fn relaunch(persisted: &PersistedState, file: Option<PathBuf>) -> MarkdownApp {
    let mut storage = MemoryStorage::default();
    eframe::set_value(&mut storage, APP_KEY, persisted);
    launch(&storage, file, false)
}

fn tab_paths(app: &MarkdownApp) -> Vec<PathBuf> {
    app.tabs.iter().map(|tab| tab.path.clone()).collect()
}

fn recent_paths(app: &MarkdownApp) -> Vec<PathBuf> {
    let recents = app.recent_files.iter();
    recents.map(|entry| entry.path.clone()).collect()
}

#[test]
fn a_first_launch_without_a_file_shows_the_welcome_page_with_default_settings() {
    let app = launch(&MemoryStorage::default(), None, false);

    assert!(app.tabs.is_empty());
    assert!(app.error_message.is_none());
    assert!(app.show_outline);
    assert!(app.show_explorer);
    assert!(!app.full_width_content);
    assert!(!app.smooth_text_rendering);
    assert_close(app.zoom_level, 1.0);
    assert_close(app.math_scale, 1.0);
    assert_eq!(app.highlight_color, None);
    assert_eq!(app.font_preset, FontPreset::Current);
    assert_eq!(app.text_size_class, TextSizeClass::Normal);
    assert!(app.watcher.is_none());
    assert_eq!(
        app.file_explorer.root,
        std::env::current_dir().ok(),
        "with nothing else to go on, the explorer shows the working directory"
    );
}

#[test]
fn a_file_on_the_command_line_wins_over_the_last_session() {
    let dir = ScratchDir::new();
    let elsewhere = ScratchDir::new();
    let a = dir.write("a.md", "# A");
    let persisted = PersistedState {
        open_tabs: Some(vec![elsewhere.write("old.md", "# Old")]),
        active_tab: Some(3),
        explorer_root: Some(elsewhere.root()),
        ..PersistedState::default()
    };

    let app = relaunch(&persisted, Some(a.clone()));

    assert_eq!(tab_paths(&app), std::slice::from_ref(&a));
    assert_eq!(app.active_tab, 0);
    assert_eq!(app.file_explorer.root, Some(dir.root()));
    assert_eq!(recent_paths(&app), [a], "an explicit open is a recent file");
    assert!(app.error_message.is_none());
}

#[test]
fn a_file_that_cannot_be_read_leaves_the_welcome_page_and_says_why() {
    let dir = ScratchDir::new();
    let missing = dir.path("missing.md");

    let app = launch(&MemoryStorage::default(), Some(missing.clone()), false);

    assert!(app.tabs.is_empty());
    let message = app.error_message.unwrap();
    assert!(
        message.contains(&missing.display().to_string()),
        "the message names the file: {message}"
    );
}

#[test]
fn the_last_session_tabs_return_without_the_ones_that_no_longer_open() {
    let dir = ScratchDir::new();
    let (a, c) = (dir.write("a.md", "# A"), dir.write("c.md", "# C"));
    let persisted = PersistedState {
        open_tabs: Some(vec![
            a.clone(),
            dir.path("deleted.md"),
            dir.subdir("now-a-folder.md"),
            c.clone(),
        ]),
        active_tab: Some(3),
        ..PersistedState::default()
    };

    let app = relaunch(&persisted, None);

    assert_eq!(tab_paths(&app), [a, c]);
    assert_eq!(app.active_tab, 1, "the saved position is kept in range");
    assert!(app.error_message.is_none());
    assert!(
        recent_paths(&app).is_empty(),
        "a restored tab is not a fresh open"
    );
    assert_eq!(app.file_explorer.root, Some(dir.root()));
}

#[test]
fn the_saved_explorer_root_is_used_while_it_exists() {
    let docs = ScratchDir::new();
    let saved_root = ScratchDir::new();
    let a = docs.write("a.md", "# A");
    let expanded = saved_root.subdir("guides");
    let session = |root: PathBuf| PersistedState {
        open_tabs: Some(vec![a.clone()]),
        explorer_root: Some(root),
        expanded_dirs: Some(vec![expanded.clone()]),
        explorer_sort_order: Some(SortOrder::DateDesc),
        ..PersistedState::default()
    };

    let app = relaunch(&session(saved_root.root()), None);
    assert_eq!(app.file_explorer.root, Some(saved_root.root()));
    assert!(app.file_explorer.expanded_dirs.contains(&expanded));
    assert!(app.file_explorer.sort_order == SortOrder::DateDesc);

    let app = relaunch(&session(saved_root.path("gone")), None);
    assert_eq!(
        app.file_explorer.root,
        Some(docs.root()),
        "a root that vanished gives way to the first tab's folder"
    );
}

#[test]
fn saved_settings_come_back_and_out_of_range_values_are_pulled_in() {
    let recent = RecentEntry {
        path: PathBuf::from("/notes/todo.md"),
        last_opened: 1_700_000_000,
    };
    let persisted = PersistedState {
        dark_mode: Some(false),
        zoom_level: Some(9.0),
        show_outline: Some(false),
        full_width_content: Some(true),
        math_scale: Some(0.25),
        highlight_color: Some([10, 20, 30, 255]),
        link_color: Some([1, 2, 3, 4]),
        selected_font_family: Some("No Such Family".to_owned()),
        smooth_text_rendering: Some(true),
        font_preset: Some(FontPreset::Github),
        text_size_class: Some(TextSizeClass::Huge),
        show_explorer: Some(false),
        explorer_width: Some(333.0),
        outline_width: Some(f32::NAN),
        recent_files: Some(vec![recent]),
        ..PersistedState::default()
    };

    let app = relaunch(&persisted, None);

    assert!(!app.dark_mode);
    assert_close(app.zoom_level, 3.0);
    assert!(!app.show_outline);
    assert!(app.full_width_content);
    assert_close(app.math_scale, 1.0);
    assert_eq!(app.highlight_color, Some(Color32::from_rgb(10, 20, 30)));
    assert_eq!(
        app.link_color,
        Some(Color32::from_rgba_premultiplied(1, 2, 3, 4))
    );
    assert_eq!(app.selected_font_family.as_deref(), Some("No Such Family"));
    assert!(app.smooth_text_rendering);
    assert_eq!(app.font_preset, FontPreset::Github);
    assert_eq!(app.text_size_class, TextSizeClass::Huge);
    assert!(!app.show_explorer);
    assert_close(app.explorer_width, 333.0);
    assert_close(app.outline_width, crate::layout::OUTLINE_DEFAULT_WIDTH);
    assert_eq!(recent_paths(&app), [PathBuf::from("/notes/todo.md")]);
}

#[test]
fn a_zoom_saved_below_the_range_is_raised_to_the_smallest_zoom() {
    let persisted = PersistedState {
        zoom_level: Some(0.01),
        math_scale: Some(7.0),
        ..PersistedState::default()
    };

    let app = relaunch(&persisted, None);

    assert_close(app.zoom_level, 0.5);
    assert_close(app.math_scale, 2.0);
}

#[test]
fn launching_with_watch_starts_live_reload() {
    let dir = ScratchDir::new();
    let a = dir.write("a.md", "# A");

    let app = launch(&MemoryStorage::default(), Some(a), true);

    assert!(app.watch_enabled);
    assert!(app.watcher.is_some());
}

#[test]
fn launch_takes_over_zoom_keys_and_drops_stale_widget_state() {
    let ctx = egui::Context::default();
    let stale = egui::Id::new("stale widget state");
    ctx.data_mut(|data| data.insert_temp(stale, 7_u32));
    let mut creation = eframe::CreationContext::_new_kittest(ctx.clone());
    creation.storage = None;

    let _app = MarkdownApp::new(&creation, None, false);

    assert!(!ctx.options(|options| options.zoom_with_keyboard));
    assert_eq!(ctx.data(|data| data.get_temp::<u32>(stale)), None);
    let scroll = ctx.style().spacing.scroll;
    assert_close(scroll.floating_width, scroll.bar_width);
}

#[test]
fn only_a_numbered_x_display_other_than_the_first_counts_as_virtual() {
    assert!(is_virtual_display(Some(":99")));
    assert!(is_virtual_display(Some(":1")));
    assert!(!is_virtual_display(Some(":0")));
    assert!(!is_virtual_display(Some(":0.0")));
    assert!(!is_virtual_display(Some("")));
    assert!(!is_virtual_display(None));
}
