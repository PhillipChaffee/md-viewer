//! What the user can ask the app to do, and the one place each request runs.

use std::time::Instant;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::layout::{
    keyboard_scroll_target, zoomed, KeyboardScrollAction, LineSpacing, ReadingWidth,
};
use crate::system_fonts::TextSizeClass;
use crate::theme::ThemeId;

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
    /// Switch the active tab between the rendered document and its source.
    ToggleEditMode,
    ToggleWatch,
    /// Switch to the next reading theme, wrapping around.
    NextTheme,
    SetTheme(ThemeId),
    ToggleExplorer,
    ToggleOutline,
    /// Switch the text column between filling the window and its default
    /// width.
    ToggleFullWidth,
    /// Make the text column this many px wide; `ReadingWidth::MAX` fits the
    /// window.
    SetReadingWidth(f32),
    /// Widen the text column by this many px, or narrow it if negative.
    StepReadingWidth(f32),
    SetLineSpacing(LineSpacing),
    /// Set the document, and the chrome that follows it, in this size class.
    SetTextSize(TextSizeClass),
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
            AppAction::ToggleEditMode => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.set_edit_mode(!tab.is_editing);
                }
            }
            AppAction::ToggleWatch => self.toggle_watching(),
            AppAction::NextTheme => self.theme = self.theme.next(),
            AppAction::SetTheme(theme) => self.theme = theme,
            AppAction::ToggleExplorer => self.show_explorer = !self.show_explorer,
            AppAction::ToggleOutline => self.toggle_outline(),
            AppAction::ToggleFullWidth => self.toggle_full_width(),
            AppAction::SetReadingWidth(px) => self.set_reading_width(ReadingWidth::new(px)),
            AppAction::StepReadingWidth(delta) => {
                let shown_in = self.document_pane_width;
                self.set_reading_width(self.reading_width.stepped(delta, shown_in));
            }
            AppAction::SetLineSpacing(spacing) => self.set_line_spacing(spacing),
            AppAction::SetTextSize(size) => self.text_size_class = size,
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

    fn toggle_full_width(&mut self) {
        self.set_reading_width(if self.reading_width.fits_window() {
            ReadingWidth::DEFAULT
        } else {
            ReadingWidth::FIT_WINDOW
        });
    }

    /// A new column width is a new layout: each open document lays out
    /// again, and its tables fit their columns to the new width instead
    /// of keeping the ones a narrower column squeezed them into.
    fn set_reading_width(&mut self, width: ReadingWidth) {
        if width != self.reading_width {
            self.reading_width = width;
            for tab in &mut self.tabs {
                tab.cache.refit_tables();
            }
        }
    }

    /// Line spacing is a renderer option, not part of the layout the
    /// renderer watches, so each open document is told to lay out again.
    fn set_line_spacing(&mut self, spacing: LineSpacing) {
        self.line_spacing = spacing;
        self.relayout_documents();
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

    /// Persist the active tab's unsaved edits, from the editor or from the
    /// preview of them: the unsaved mark is shown in both. With nothing
    /// unsaved it does nothing; the flash label beside the mode toggle
    /// reports the outcome.
    fn save_active_tab(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        if tab.content == tab.saved_content {
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
