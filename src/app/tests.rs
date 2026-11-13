//! Whole-frame tests: a keypress goes in through `update`, app state comes out.

use std::fs;
use std::path::PathBuf;

use eframe::egui::{self, Key, Modifiers};
use eframe::App as _;

use super::action::AppAction;
use super::lightbox::{LightboxState, LightboxTexture};
use super::MarkdownApp;
use crate::layout::ReadingWidth;
use crate::test_support::{assert_close, ScratchDir};
use crate::theme::ThemeId;

const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: false,
};

/// A running app over a scratch directory of markdown files, driven one
/// frame at a time without a window.
pub(crate) struct Harness {
    pub(crate) app: MarkdownApp,
    pub(crate) ctx: egui::Context,
    pub(crate) frame: eframe::Frame,
    pub(crate) dir: PathBuf,
    /// Removes `dir` when the harness is dropped.
    _scratch: ScratchDir,
}

impl Harness {
    /// Write each `(name, content)` document and start the app on the first
    /// one, the way `md-viewer <file>` does. File watching starts off.
    pub(crate) fn with_docs(docs: &[(&str, &str)]) -> Self {
        let scratch = ScratchDir::new();
        let dir = scratch.path().to_path_buf();
        for (name, content) in docs {
            scratch.file(name, content);
        }
        let first = docs.first().map(|(name, _)| dir.join(name));

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        Self {
            app: MarkdownApp::new(&creation, first, false),
            ctx,
            frame: eframe::Frame::_new_kittest(),
            dir,
            _scratch: scratch,
        }
    }

    /// Three one-line documents: `a.md`, `b.md`, and `c.md`.
    pub(crate) fn with_three_docs() -> Self {
        Self::with_docs(&[("a.md", "# A"), ("b.md", "# B"), ("c.md", "# C")])
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub(crate) fn active_path(&self) -> PathBuf {
        self.app.tabs[self.app.active_tab].path.clone()
    }

    /// Run one frame in which `key` goes down with `modifiers` held.
    pub(crate) fn press(&mut self, modifiers: Modifiers, key: Key) -> egui::FullOutput {
        self.run_frame(
            modifiers,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
        )
    }

    /// Run one frame with no input.
    pub(crate) fn idle(&mut self) -> egui::FullOutput {
        self.run_frame(Modifiers::NONE, Vec::new())
    }

    /// Carry out `action` between frames, as a menu pick does.
    pub(crate) fn apply(&mut self, action: AppAction) {
        self.app.apply_action(&self.ctx, action);
    }

    fn run_frame(&mut self, modifiers: Modifiers, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            modifiers,
            events,
            ..Default::default()
        };
        self.app.raw_input_hook(&self.ctx, &mut raw_input);
        let (app, frame) = (&mut self.app, &mut self.frame);
        self.ctx.run(raw_input, |ctx| app.update(ctx, frame))
    }
}

#[test]
fn alt_arrow_moves_one_history_step_per_keypress() {
    let mut harness = Harness::with_three_docs();
    let (a, b, c) = (
        harness.path("a.md"),
        harness.path("b.md"),
        harness.path("c.md"),
    );
    harness.app.navigate_active_tab_to(&b);
    harness.app.navigate_active_tab_to(&c);

    harness.press(Modifiers::ALT, Key::ArrowLeft);
    assert_eq!(harness.active_path(), b);
    harness.press(Modifiers::ALT, Key::ArrowLeft);
    assert_eq!(harness.active_path(), a);
    harness.press(Modifiers::ALT, Key::ArrowRight);
    assert_eq!(harness.active_path(), b);
    harness.press(Modifiers::ALT, Key::ArrowRight);
    assert_eq!(harness.active_path(), c);
}

#[test]
fn history_navigation_retitles_the_window_and_moves_the_explorer_highlight() {
    let window_titles = |output: &egui::FullOutput| -> Vec<String> {
        output
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .filter_map(|command| match command {
                egui::ViewportCommand::Title(title) => Some(title.clone()),
                _ => None,
            })
            .collect()
    };
    let mut harness = Harness::with_three_docs();
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));
    harness.app.navigate_active_tab_to(&b);
    harness.app.navigate_active_tab_to(&c);
    harness.idle();

    let mut titles = window_titles(&harness.press(Modifiers::ALT, Key::ArrowLeft));
    titles.extend(window_titles(&harness.idle()));

    assert_eq!(titles, ["b.md - Markdown Viewer"]);
    assert!(harness.app.open_tab_paths.contains(&b));
    assert!(!harness.app.open_tab_paths.contains(&c));
}

#[test]
fn view_shortcuts_flip_their_own_setting() {
    let mut harness = Harness::with_three_docs();
    let (theme, outline, explorer) = (
        harness.app.theme,
        harness.app.show_outline,
        harness.app.show_explorer,
    );

    harness.press(Modifiers::CTRL, Key::D);
    assert_eq!(harness.app.theme, theme.next());
    harness.press(CTRL_SHIFT, Key::O);
    assert_eq!(harness.app.show_outline, !outline);
    harness.press(CTRL_SHIFT, Key::E);
    assert_eq!(harness.app.show_explorer, !explorer);

    harness.press(Modifiers::CTRL, Key::E);
    assert_eq!(
        harness.app.show_explorer, !explorer,
        "Ctrl+E without Shift is not a shortcut"
    );
    assert_eq!(harness.app.theme, theme.next());
    assert_eq!(harness.app.show_outline, !outline);
}

#[test]
fn zoom_shortcuts_step_and_reset_the_document_zoom() {
    let mut harness = Harness::with_three_docs();
    assert!((harness.app.zoom_level - 1.0).abs() < 1e-6);

    harness.press(Modifiers::CTRL, Key::Plus);
    assert!((harness.app.zoom_level - 1.25).abs() < 1e-6);
    harness.press(Modifiers::CTRL, Key::Equals);
    assert!((harness.app.zoom_level - 1.5625).abs() < 1e-6);
    harness.press(Modifiers::CTRL, Key::Minus);
    assert!((harness.app.zoom_level - 1.25).abs() < 1e-6);
    harness.press(Modifiers::CTRL, Key::Num0);
    assert!((harness.app.zoom_level - 1.0).abs() < 1e-6);
}

#[test]
fn tab_shortcuts_switch_and_close_tabs() {
    let mut harness = Harness::with_three_docs();
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));
    harness.app.open_in_new_tab(b.clone());
    harness.app.open_in_new_tab(c.clone());
    assert_eq!(harness.app.active_tab, 2);

    harness.press(Modifiers::CTRL, Key::Tab);
    assert_eq!(harness.app.active_tab, 0, "next wraps to the first tab");
    harness.press(CTRL_SHIFT, Key::Tab);
    assert_eq!(harness.app.active_tab, 2, "previous wraps to the last tab");
    harness.press(Modifiers::CTRL, Key::Num2);
    assert_eq!(harness.active_path(), b);
    harness.press(Modifiers::CTRL, Key::Num9);
    assert_eq!(harness.active_path(), b, "there is no ninth tab");

    harness.press(Modifiers::CTRL, Key::W);
    assert_eq!(harness.app.tabs.len(), 2);
    assert_eq!(harness.active_path(), c);
}

#[test]
fn find_bar_shortcuts_open_cycle_and_close() {
    let mut harness = Harness::with_docs(&[("a.md", "needle\n\nneedle\n\nneedle\n")]);

    harness.press(Modifiers::CTRL, Key::F);
    assert!(harness.app.search.is_open);
    harness.app.search.query = "needle".to_owned();
    harness.idle();
    assert_eq!(harness.app.tabs[0].search_matches.len(), 3);
    assert_eq!(harness.app.search.active_match_index, 0);

    harness.press(Modifiers::NONE, Key::Enter);
    assert_eq!(harness.app.search.active_match_index, 1);
    harness.press(Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.app.search.active_match_index, 2);
    harness.press(Modifiers::SHIFT, Key::Enter);
    assert_eq!(harness.app.search.active_match_index, 1);
    harness.press(Modifiers::NONE, Key::ArrowUp);
    assert_eq!(harness.app.search.active_match_index, 0);
    harness.press(Modifiers::NONE, Key::ArrowUp);
    assert_eq!(
        harness.app.search.active_match_index, 2,
        "wraps to the last match"
    );

    harness.press(Modifiers::NONE, Key::Escape);
    assert!(!harness.app.search.is_open);
    assert!(harness.app.tabs[0].search_matches.is_empty());
}

#[test]
fn scroll_keys_move_the_document_until_the_find_bar_takes_the_arrows() {
    let long_document = "A paragraph of text.\n\n".repeat(400);
    let mut harness = Harness::with_docs(&[("a.md", &long_document)]);
    harness.idle();
    harness.idle();
    assert!(harness.app.tabs[0].scroll_offset.abs() < 1e-6);

    harness.press(Modifiers::NONE, Key::PageDown);
    harness.idle();
    let after_page_down = harness.app.tabs[0].scroll_offset;
    assert!(
        after_page_down > 100.0,
        "PageDown scrolled {after_page_down}"
    );

    harness.press(Modifiers::NONE, Key::ArrowDown);
    harness.idle();
    let after_arrow_down = harness.app.tabs[0].scroll_offset;
    assert!(after_arrow_down > after_page_down);

    harness.press(Modifiers::CTRL, Key::F);
    harness.press(Modifiers::NONE, Key::ArrowDown);
    harness.idle();
    let with_find_bar_open = harness.app.tabs[0].scroll_offset;
    assert!(
        (with_find_bar_open - after_arrow_down).abs() < 1e-3,
        "the arrow key belongs to the find bar while it is open"
    );

    harness.press(Modifiers::NONE, Key::PageUp);
    harness.idle();
    assert!(harness.app.tabs[0].scroll_offset < with_find_bar_open);
}

#[test]
fn an_open_lightbox_takes_the_zoom_keys_and_blocks_the_rest() {
    let mut harness = Harness::with_three_docs();
    let theme = harness.app.theme;
    harness.app.lightbox = Some(LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::default()),
        base_size: egui::vec2(100.0, 100.0),
        zoom: 1.0,
        open_id: 1,
        scroll_offset: egui::Vec2::ZERO,
    });
    let lightbox_zoom = |harness: &Harness| harness.app.lightbox.as_ref().unwrap().zoom;

    harness.press(Modifiers::CTRL, Key::Plus);
    assert!((lightbox_zoom(&harness) - 1.25).abs() < 1e-6);
    assert!((harness.app.zoom_level - 1.0).abs() < 1e-6);
    harness.press(Modifiers::CTRL, Key::Minus);
    harness.press(Modifiers::CTRL, Key::Minus);
    assert!((lightbox_zoom(&harness) - 0.8).abs() < 1e-6);
    harness.press(Modifiers::CTRL, Key::Num0);
    assert!((lightbox_zoom(&harness) - 1.0).abs() < 1e-6);

    harness.press(Modifiers::CTRL, Key::D);
    assert_eq!(harness.app.theme, theme);
    harness.press(Modifiers::CTRL, Key::W);
    assert_eq!(harness.app.tabs.len(), 1);

    harness.press(Modifiers::NONE, Key::Escape);
    assert!(harness.app.lightbox.is_none());
}

#[test]
fn ctrl_s_writes_unsaved_edits_and_nothing_else() {
    let mut harness = Harness::with_docs(&[("a.md", "original")]);
    let path = harness.path("a.md");

    harness.press(Modifiers::CTRL, Key::S);
    assert!(
        harness.app.tabs[0].save_flash.is_none(),
        "nothing was unsaved"
    );

    harness.app.tabs[0].set_edit_mode(true);
    harness.app.tabs[0].content = "edited".to_owned();
    harness.press(Modifiers::CTRL, Key::S);
    assert_eq!(fs::read_to_string(&path).unwrap(), "edited");
    assert!(matches!(harness.app.tabs[0].save_flash, Some((true, _))));

    // Edits left unsaved are saved from the preview of them too.
    harness.app.tabs[0].content = "edited again".to_owned();
    harness.app.tabs[0].set_edit_mode(false);
    harness.press(Modifiers::CTRL, Key::S);
    assert_eq!(fs::read_to_string(&path).unwrap(), "edited again");
}

#[test]
fn ctrl_q_asks_the_window_to_close() {
    let mut harness = Harness::with_three_docs();
    let close_requested = |output: &egui::FullOutput| {
        output.viewport_output.values().any(|viewport| {
            viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        })
    };

    assert!(!close_requested(&harness.idle()));
    assert!(close_requested(&harness.press(Modifiers::CTRL, Key::Q)));
}

#[test]
fn f5_toggles_file_watching() {
    let mut harness = Harness::with_three_docs();
    assert!(harness.app.watcher.is_none());

    harness.press(Modifiers::NONE, Key::F5);
    assert!(harness.app.watcher.is_some());
    assert!(harness.app.watch_enabled);
    harness.press(Modifiers::NONE, Key::F5);
    assert!(harness.app.watcher.is_none());
    assert!(!harness.app.watch_enabled);
}

#[test]
fn ctrl_d_steps_through_every_theme_and_comes_back_round() {
    let mut harness = Harness::with_three_docs();
    let start = harness.app.theme;

    let mut seen = vec![start];
    for _ in 1..ThemeId::ALL.len() {
        harness.press(Modifiers::CTRL, Key::D);
        seen.push(harness.app.theme);
    }
    seen.sort_by_key(|theme| ThemeId::ALL.iter().position(|known| known == theme));
    assert_eq!(seen, ThemeId::ALL, "each press showed a theme not yet seen");

    harness.press(Modifiers::CTRL, Key::D);
    assert_eq!(harness.app.theme, start);
}

#[test]
fn ctrl_alt_arrows_widen_and_narrow_the_text_column_one_step_a_press() {
    let mut harness = Harness::with_three_docs();
    harness.app.show_explorer = false;
    harness.app.show_outline = false;
    harness.idle();
    let ctrl_alt = Modifiers::CTRL | Modifiers::ALT;
    assert_eq!(harness.app.reading_width, ReadingWidth::DEFAULT);

    harness.press(ctrl_alt, Key::ArrowRight);
    assert_close(harness.app.reading_width.get(), 720.0);
    harness.press(ctrl_alt, Key::ArrowLeft);
    harness.press(ctrl_alt, Key::ArrowLeft);
    assert_close(harness.app.reading_width.get(), 640.0);

    // From a column that fills the window, a step starts at what is shown.
    harness.apply(AppAction::SetReadingWidth(ReadingWidth::MAX));
    harness.idle();
    let shown = ReadingWidth::FIT_WINDOW.resolve(harness.app.document_pane_width);
    harness.press(ctrl_alt, Key::ArrowLeft);
    assert_close(harness.app.reading_width.get(), shown - ReadingWidth::STEP);
}

#[test]
fn the_column_shortcuts_and_the_history_shortcuts_do_not_fire_each_other() {
    let mut harness = Harness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    harness.app.navigate_active_tab_to(&b);
    harness.idle();

    harness.press(Modifiers::CTRL | Modifiers::ALT, Key::ArrowLeft);
    assert_eq!(harness.active_path(), b, "Ctrl+Alt+Left is not Back");
    assert_close(harness.app.reading_width.get(), 640.0);

    harness.press(Modifiers::ALT, Key::ArrowLeft);
    assert_eq!(harness.active_path(), a);
    assert_close(harness.app.reading_width.get(), 640.0);
}
