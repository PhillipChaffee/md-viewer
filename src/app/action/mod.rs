//! What the user can ask the app to do, and the one place each request runs.

use eframe::egui;

use crate::app::close_prompt::CloseAnswer;
use crate::app::MarkdownApp;
use crate::app_icon::AppIcon;
use crate::layout::{zoomed, KeyboardScrollAction, LineSpacing, ReadingWidth};
use crate::system_fonts::TextSizeClass;
use crate::theme::ThemeId;

#[cfg(test)]
mod tests;

/// One thing the user asked for, whether through a shortcut, a menu item, or
/// a toolbar button. [`MarkdownApp::apply_action`] is the only code that
/// carries a request out, so every way of asking does the same thing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AppAction {
    /// Show the home page, where a new tab starts: over the open tabs,
    /// which stay open.
    NewTab,
    /// Show the home page, or leave it for the tab that was in front.
    GoHome,
    /// Pick a file and open it in a new tab.
    OpenFile,
    /// Pick the folder the file explorer shows.
    OpenFolder,
    /// Close the active tab, asking first if it has unsaved edits.
    CloseTab,
    /// What the reader answered when asked about unsaved edits, or about
    /// saving over what another program wrote.
    AnswerClosePrompt(CloseAnswer),
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
    /// Write the active tab's unsaved edits to disk, asking first if that
    /// would replace what another program wrote there.
    Save,
    /// Show the active tab's file as it is on disk now, in place of its
    /// unsaved edits.
    ReloadFromDisk,
    /// Keep the active tab's unsaved edits, and take down the notice that
    /// its file changed on disk.
    KeepMine,
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
    /// Make the text column this wide.
    SetReadingWidth(ReadingWidth),
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
    /// Start moving the window with the pointer, as a drag on a title bar
    /// does.
    MoveWindow,
    /// Make the window as large as its screen lets it be, or the size it
    /// was before.
    ToggleMaximized,
    /// Ask the window to close. With unsaved edits open the app refuses
    /// the request when it comes back and asks about them first.
    Quit,
}

impl MarkdownApp {
    /// Carry out one user request.
    pub(crate) fn apply_action(&mut self, ctx: &egui::Context, action: AppAction) {
        match action {
            AppAction::NewTab => self.show_home(),
            AppAction::GoHome => self.toggle_home(),
            AppAction::OpenFile => self.open_file_dialog(),
            AppAction::OpenFolder => self.open_folder_dialog(),
            AppAction::CloseTab => self.close_active_tab(),
            AppAction::AnswerClosePrompt(answer) => self.answer_close_prompt(answer),
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
            AppAction::ReloadFromDisk => self.reload_front_tab(),
            AppAction::KeepMine => {
                if let Some(tab) = self.front_tab_mut() {
                    tab.keep_mine();
                }
            }
            AppAction::ToggleEditMode => {
                if let Some(tab) = self.front_tab_mut() {
                    tab.set_edit_mode(!tab.is_editing);
                }
            }
            AppAction::ToggleWatch => self.toggle_watching(),
            AppAction::NextTheme => self.set_theme(ctx, self.theme.next()),
            AppAction::SetTheme(theme) => self.set_theme(ctx, theme),
            AppAction::ToggleExplorer => self.show_explorer = !self.show_explorer,
            AppAction::ToggleOutline => self.toggle_outline(),
            AppAction::ToggleFullWidth => self.toggle_full_width(),
            AppAction::SetReadingWidth(width) => self.set_reading_width(width),
            AppAction::StepReadingWidth(delta) => {
                let shown_in = self.document_pane_width;
                self.set_reading_width(self.reading_width.stepped(delta, shown_in));
            }
            AppAction::SetLineSpacing(spacing) => self.set_line_spacing(spacing),
            AppAction::SetTextSize(size) => self.fonts.size = size,
            AppAction::ToggleSmoothText => {
                self.smooth_text_rendering = !self.smooth_text_rendering;
            }
            AppAction::OpenColorSettings => self.show_color_settings_dialog = true,
            AppAction::OpenFontSettings => self.font_picker.open = true,
            AppAction::ZoomBy(factor) => self.zoom_level = zoomed(self.zoom_level, factor),
            AppAction::ResetZoom => self.zoom_level = 1.0,
            AppAction::ZoomLightboxBy(factor) => {
                if let Some(lightbox) = &mut self.lightbox.open {
                    lightbox.zoom_by(factor);
                }
            }
            AppAction::ResetLightboxZoom => {
                if let Some(lightbox) = &mut self.lightbox.open {
                    lightbox.zoom = 1.0;
                }
            }
            AppAction::SetFormulaScale(scale) => self.math_scale = scale,
            AppAction::Scroll(step) => self.scroll_active_tab(step),
            AppAction::MoveWindow => ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag),
            AppAction::ToggleMaximized => {
                let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            AppAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    /// Switch the reading theme, and have the window's icon follow it
    /// from a light theme to a dark one or back.
    ///
    /// The window is asked only when the icon differs. An X11 window
    /// takes the new icon at once. A Mac's Dock and a Wayland desktop do
    /// not: winit has nothing to set there, and the Dock shows the icon
    /// of the saved theme from the next launch.
    fn set_theme(&mut self, ctx: &egui::Context, theme: ThemeId) {
        let icon = AppIcon::for_theme(theme);
        if icon != AppIcon::for_theme(self.theme) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(icon.image())));
        }
        self.theme = theme;
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

    /// Persist the active tab's unsaved edits, from the editor or from the
    /// preview of them: the unsaved mark is shown in both. With nothing
    /// unsaved it does nothing; the flash label beside the mode toggle
    /// reports the outcome.
    fn save_active_tab(&mut self) {
        if let Some(front) = self.front() {
            self.request_save(front);
        }
    }

    /// Queue a keyboard scroll step through the pending-scroll pipeline the
    /// outline and search jumps use.
    fn scroll_active_tab(&mut self, step: KeyboardScrollAction) {
        if let Some(tab) = self.front_tab_mut() {
            tab.scroll_by_key(step);
        }
    }
}
