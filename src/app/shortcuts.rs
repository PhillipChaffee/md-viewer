//! Keyboard shortcuts and dropped files.

use std::time::Instant;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::layout::{keyboard_scroll_target, zoomed, KeyboardScrollAction, ZOOM_STEP};

impl MarkdownApp {
    /// Read this frame's keyboard shortcuts and act on them. Returns the
    /// history back/forward requests, which the menu bar acts on again.
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) -> (bool, bool) {
        // Handle keyboard shortcuts (suppressed when lightbox is open)
        let mut open_dialog = false;
        let mut toggle_watch = false;
        let mut toggle_dark = false;
        let mut toggle_outline = false;
        let mut toggle_explorer = false;
        let mut quit_app = false;
        let mut zoom_factor_request: Option<f32> = None;
        let mut zoom_reset = false;
        let mut go_back = false;
        let mut go_forward = false;
        let mut close_tab = false;
        let mut new_tab = false;
        let mut next_tab = false;
        let mut prev_tab = false;
        let mut focus_tab: Option<usize> = None;
        let mut open_search = false;
        let mut next_match = false;
        let mut prev_match = false;
        let mut close_search_kb = false;
        let mut keyboard_scroll_action: Option<KeyboardScrollAction> = None;
        let mut save_request = false;

        // Ctrl+/- zoom: applies to lightbox when open, document otherwise.
        // Steps are multiplicative (×1.25) like browsers and the lightbox.
        ctx.input(|i| {
            if i.modifiers.ctrl
                && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals))
            {
                zoom_factor_request = Some(ZOOM_STEP);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Minus) {
                zoom_factor_request = Some(1.0 / ZOOM_STEP);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num0) {
                if let Some(lightbox) = &mut self.lightbox {
                    // Reset lightbox zoom only; the document has its own reset.
                    lightbox.zoom = 1.0;
                } else {
                    zoom_reset = true;
                }
            }
        });

        // Apply zoom to lightbox or document
        if let Some(factor) = zoom_factor_request {
            if let Some(lightbox) = &mut self.lightbox {
                lightbox.zoom = (lightbox.zoom * factor).clamp(0.1, 10.0);
            } else {
                self.zoom_level = zoomed(self.zoom_level, factor);
            }
        }
        if zoom_reset && self.lightbox.is_none() {
            self.zoom_level = 1.0;
        }

        if self.lightbox.is_none() {
            ctx.input(|i| {
                // Ctrl+O: Open file
                if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::O) {
                    open_dialog = true;
                }
                // Ctrl+Shift+O: Toggle outline
                if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::O) {
                    toggle_outline = true;
                }
                // Ctrl+Shift+E: Toggle file explorer
                if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::E) {
                    toggle_explorer = true;
                }
                // Ctrl+W: Close current tab
                if i.modifiers.ctrl && i.key_pressed(egui::Key::W) {
                    close_tab = true;
                }
                // Ctrl+T: Open a file in a new tab
                if i.modifiers.ctrl && i.key_pressed(egui::Key::T) {
                    new_tab = true;
                }
                // Ctrl+Tab: Next tab
                if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Tab) {
                    next_tab = true;
                }
                // Ctrl+Shift+Tab: Previous tab
                if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Tab) {
                    prev_tab = true;
                }
                // Ctrl+1-9: Focus tab by index
                for (idx, key) in [
                    egui::Key::Num1,
                    egui::Key::Num2,
                    egui::Key::Num3,
                    egui::Key::Num4,
                    egui::Key::Num5,
                    egui::Key::Num6,
                    egui::Key::Num7,
                    egui::Key::Num8,
                    egui::Key::Num9,
                ]
                .iter()
                .enumerate()
                {
                    if i.modifiers.ctrl && i.key_pressed(*key) {
                        focus_tab = Some(idx);
                    }
                }
                // Alt+Left: Go back in history
                if i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft) {
                    go_back = true;
                }
                // Alt+Right: Go forward in history
                if i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight) {
                    go_forward = true;
                }
                // Ctrl+D: Toggle dark mode
                if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                    toggle_dark = true;
                }
                // Ctrl+Q: Quit
                if i.modifiers.ctrl && i.key_pressed(egui::Key::Q) {
                    quit_app = true;
                }
                // Ctrl+S: save the active tab's edit buffer to disk
                if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
                    save_request = true;
                }
                // Ctrl + scroll wheel / pinch to zoom (multiplicative, like
                // browsers). The lightbox captures wheel input itself in
                // raw_input_hook, so this only runs for the document.
                let wheel_zoom_factor = i.zoom_delta();
                if wheel_zoom_factor != 1.0 {
                    self.zoom_level = zoomed(self.zoom_level, wheel_zoom_factor);
                }
                // F5: Toggle file watching
                if i.key_pressed(egui::Key::F5) {
                    toggle_watch = true;
                }
                // Ctrl+F: Open find bar (or refocus if already open)
                if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::F) {
                    open_search = true;
                }
                // While the find bar is open, intercept Enter / Shift+Enter / ↑↓ / Esc.
                // Up/Down are safe to bind even when the singleline TextEdit has focus
                // because it doesn't use vertical arrows for cursor movement.
                if self.search.is_open {
                    if i.key_pressed(egui::Key::Enter) {
                        if i.modifiers.shift {
                            prev_match = true;
                        } else {
                            next_match = true;
                        }
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        next_match = true;
                    }
                    if i.key_pressed(egui::Key::ArrowUp) {
                        prev_match = true;
                    }
                    if i.key_pressed(egui::Key::Escape) {
                        close_search_kb = true;
                    }
                }
                // Plain document scroll keys reuse the existing pending-scroll pipeline.
                // Arrow keys stay available for search navigation while the find bar is open.
                let no_scroll_modifier =
                    !i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.command;
                if no_scroll_modifier {
                    if !self.search.is_open {
                        if i.key_pressed(egui::Key::ArrowUp) {
                            keyboard_scroll_action = Some(KeyboardScrollAction::LineUp);
                        }
                        if i.key_pressed(egui::Key::ArrowDown) {
                            keyboard_scroll_action = Some(KeyboardScrollAction::LineDown);
                        }
                    }
                    if i.key_pressed(egui::Key::PageUp) {
                        keyboard_scroll_action = Some(KeyboardScrollAction::PageUp);
                    }
                    if i.key_pressed(egui::Key::PageDown) {
                        keyboard_scroll_action = Some(KeyboardScrollAction::PageDown);
                    }
                }
            });
        } // end lightbox guard

        if open_dialog || new_tab {
            self.open_file_dialog();
        }
        if toggle_watch {
            if self.watcher.is_some() {
                self.stop_watching();
                self.watch_enabled = false;
            } else {
                self.watch_enabled = true;
                self.start_watching();
            }
        }
        if toggle_dark {
            self.dark_mode = !self.dark_mode;
        }
        if toggle_outline {
            self.show_outline = !self.show_outline;
        }
        if toggle_explorer {
            self.show_explorer = !self.show_explorer;
        }
        if quit_app {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // Ctrl+S: persist the active tab's edit buffer. Only meaningful in
        // edit mode with unsaved changes; the flash label in the edit
        // toolbar reports the outcome.
        if save_request {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                if tab.is_editing && tab.content != tab.saved_content {
                    match tab.save_to_disk() {
                        Ok(()) => {
                            log::info!("Saved tab edits: {:?}", tab.path);
                            tab.save_flash = Some((true, Instant::now()));
                        }
                        Err(error) => {
                            log::error!("Failed to save {:?}: {error}", tab.path);
                            self.error_message =
                                Some(format!("Unable to save {}: {error}", tab.path.display()));
                            tab.save_flash = Some((false, Instant::now()));
                        }
                    }
                }
            }
        }
        if close_tab {
            self.close_active_tab();
        }
        if next_tab {
            self.next_tab();
        }
        if prev_tab {
            self.prev_tab();
        }
        if let Some(idx) = focus_tab {
            self.focus_tab(idx);
        }
        if go_back {
            self.navigate_active_history(true);
        }
        if go_forward {
            self.navigate_active_history(false);
        }

        // Search bar actions (Ctrl+F open, Enter/Shift+Enter cycle, Esc close)
        if open_search {
            self.search.is_open = true;
            self.search.focus_requested = true;
        }
        if next_match {
            self.jump_match(1);
        }
        if prev_match {
            self.jump_match(-1);
        }
        if close_search_kb {
            self.close_search();
        }
        if let Some(action) = keyboard_scroll_action {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                let target = keyboard_scroll_target(
                    tab.scroll_offset,
                    tab.last_viewport_height,
                    tab.last_content_height,
                    action,
                );
                tab.pending_scroll_offset = Some(target);
            }
        }

        (go_back, go_forward)
    }

    /// Track drag hover and open dropped markdown files.
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        // Handle drag and drop
        self.is_dragging = false;
        ctx.input(|i| {
            if !i.raw.hovered_files.is_empty() {
                self.is_dragging = true;
            }

            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    if Self::is_markdown_file(path) {
                        self.open_in_new_tab(path.clone());
                    } else {
                        self.error_message = Some(
                            "Unsupported file type. Please drop a markdown file (.md, .markdown, .txt)".to_string(),
                        );
                    }
                }
            }
        });
    }
}
