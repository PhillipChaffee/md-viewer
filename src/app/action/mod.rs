//! What the user can ask the app to do, and the one place each request runs.

use std::time::Instant;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::layout::{keyboard_scroll_target, zoomed, KeyboardScrollAction};

#[cfg(test)]
mod tests;

/// One thing the user asked for, whether through a shortcut, a menu item, or
/// a toolbar button. [`MarkdownApp::apply_action`] is the only code that
/// carries a request out, so every way of asking does the same thing.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AppAction {
    /// Pick a file and open it in a new tab.
    OpenFile,
    /// Pick the folder the file explorer shows.
    OpenFolder,
    /// Close the active tab.
    CloseTab,
    NextTab,
    PreviousTab,
    /// Switch to the tab at this zero-based position, if there is one.
    FocusTab(usize),
    /// Step the active tab back through its history.
    NavigateBack,
    /// Step the active tab forward through its history.
    NavigateForward,
    /// Show the find bar and put the cursor in it.
    OpenSearch,
    NextMatch,
    PreviousMatch,
    CloseSearch,
    /// Write the active tab's unsaved edits to disk.
    Save,
    ToggleWatch,
    ToggleDarkMode,
    ToggleExplorer,
    ToggleOutline,
    ToggleFullWidth,
    ToggleSmoothText,
    OpenColorSettings,
    OpenFontSettings,
    /// Multiply the document zoom by this factor.
    ZoomBy(f32),
    ResetZoom,
    /// Multiply the open lightbox's zoom by this factor.
    ZoomLightboxBy(f32),
    ResetLightboxZoom,
    /// Render formulas at this size relative to the text around them.
    SetFormulaScale(f32),
    /// Scroll the active document by one keyboard step.
    Scroll(KeyboardScrollAction),
    Quit,
}

impl MarkdownApp {
    /// Carry out one user request.
    pub(crate) fn apply_action(&mut self, ctx: &egui::Context, action: AppAction) {
        match action {
            AppAction::OpenFile => self.open_file_dialog(),
            AppAction::OpenFolder => self.open_folder_dialog(),
            AppAction::CloseTab => self.close_active_tab(),
            AppAction::NextTab => self.next_tab(),
            AppAction::PreviousTab => self.prev_tab(),
            AppAction::FocusTab(index) => self.focus_tab(index),
            AppAction::NavigateBack => self.navigate_active_history(true),
            AppAction::NavigateForward => self.navigate_active_history(false),
            AppAction::OpenSearch => self.open_search(),
            AppAction::NextMatch => self.jump_match(1),
            AppAction::PreviousMatch => self.jump_match(-1),
            AppAction::CloseSearch => self.close_search(),
            AppAction::Save => self.save_active_tab(),
            AppAction::ToggleWatch => self.toggle_watching(),
            AppAction::ToggleDarkMode => self.dark_mode = !self.dark_mode,
            AppAction::ToggleExplorer => self.show_explorer = !self.show_explorer,
            AppAction::ToggleOutline => self.show_outline = !self.show_outline,
            AppAction::ToggleFullWidth => self.full_width_content = !self.full_width_content,
            AppAction::ToggleSmoothText => {
                self.smooth_text_rendering = !self.smooth_text_rendering;
            }
            AppAction::OpenColorSettings => self.show_color_settings_dialog = true,
            AppAction::OpenFontSettings => self.show_font_dialog = true,
            AppAction::ZoomBy(factor) => self.zoom_level = zoomed(self.zoom_level, factor),
            AppAction::ResetZoom => self.zoom_level = 1.0,
            AppAction::ZoomLightboxBy(factor) => {
                if let Some(lightbox) = &mut self.lightbox {
                    lightbox.zoom_by(factor);
                }
            }
            AppAction::ResetLightboxZoom => {
                if let Some(lightbox) = &mut self.lightbox {
                    lightbox.zoom = 1.0;
                }
            }
            AppAction::SetFormulaScale(scale) => self.math_scale = scale,
            AppAction::Scroll(step) => self.scroll_active_tab(step),
            AppAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    fn toggle_watching(&mut self) {
        if self.watcher.is_some() {
            self.stop_watching();
            self.watch_enabled = false;
        } else {
            self.watch_enabled = true;
            self.start_watching();
        }
    }

    /// Persist the active tab's edit buffer. Only meaningful in edit mode
    /// with unsaved changes; the flash label in the edit toolbar reports the
    /// outcome.
    fn save_active_tab(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        if !tab.is_editing || tab.content == tab.saved_content {
            return;
        }
        match tab.save_to_disk() {
            Ok(()) => {
                log::info!("Saved tab edits: {}", tab.path.display());
                tab.save_flash = Some((true, Instant::now()));
            }
            Err(error) => {
                log::error!("Failed to save {}: {error}", tab.path.display());
                self.error_message =
                    Some(format!("Unable to save {}: {error}", tab.path.display()));
                tab.save_flash = Some((false, Instant::now()));
            }
        }
    }

    /// Queue a keyboard scroll step through the pending-scroll pipeline the
    /// outline and search jumps use.
    fn scroll_active_tab(&mut self, step: KeyboardScrollAction) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        tab.pending_scroll_offset = Some(keyboard_scroll_target(
            tab.scroll_offset,
            tab.last_viewport_height,
            tab.last_content_height,
            step,
        ));
    }
}
