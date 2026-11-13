//! The eframe integration: input hook, persistence, and the per-frame update sequence.

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;

use crate::app::content::look::DocumentLook;
use crate::app::drop;
use crate::app::explorer_panel::FLASH_DURATION_MS;
use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::mcp;
use crate::app::MarkdownApp;
use crate::persist::APP_KEY;
use crate::system_fonts::{document_fonts, setup_fonts};
use crate::theme;

#[cfg(test)]
mod tests;

impl eframe::App for MarkdownApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        mcp::raw_input(&mut self.mcp, raw_input);

        // When lightbox is open, intercept scroll events before they reach InputState.
        // This prevents the document's ScrollArea from scrolling while letting us
        // use the captured delta for lightbox zoom.
        self.lightbox_scroll = 0.0;
        if self.lightbox.is_some() {
            for event in &raw_input.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    self.lightbox_scroll += delta.y;
                }
            }
            raw_input
                .events
                .retain(|e| !matches!(e, egui::Event::MouseWheel { .. }));
        }
    }

    fn persist_egui_memory(&self) -> bool {
        false // Don't persist egui's internal Memory (widget states, panel sizes, etc.)
              // Our PersistedState handles everything we need; egui's blob grows unbounded (~170KB)
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_KEY, &self.persisted());
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.begin_frame(ctx);
        self.handle_keyboard_shortcuts(ctx);
        let files_hover = self.handle_dropped_files(ctx);

        // Get ctrl state for link handling
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

        self.render_top_bar(ctx);
        self.render_top_bars(ctx);
        self.render_panels(ctx, ctrl_held);
        self.render_overlays(ctx, files_hover);

        // Capture AccessKit output for MCP bridge
        mcp::capture(&mut self.mcp, ctx);
    }
}

impl MarkdownApp {
    /// Per-frame housekeeping that runs before any input or UI: file
    /// changes, async polls, and theme, font, zoom, and title application.
    fn begin_frame(&mut self, ctx: &egui::Context) {
        // Enable AccessKit for MCP bridge
        mcp::begin_frame(&mut self.mcp, ctx);

        self.poll_background_work(ctx);

        // Limit frame rate on virtual displays (e.g., Xvfb) which lack vsync
        // request_repaint_after alone doesn't limit actual frame rate, so we sleep
        // This prevents 500%+ CPU usage during E2E testing
        if self.is_virtual_display {
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS cap
        }

        let theme_changed = self.apply_theme(ctx);
        self.font_picker.poll(&self.egui_ctx);
        let fonts_changed = self.apply_fonts(ctx);
        if theme_changed || fonts_changed {
            self.rebuild_document_look(ctx);
        }

        // Apply display zoom only when it actually changed: set_zoom_factor
        // is not free (it can invalidate glyph caches), and the value only
        // moves on user input or state restore.
        if self.last_applied_zoom_level != Some(self.zoom_level) {
            ctx.set_zoom_factor(self.zoom_level);
            self.last_applied_zoom_level = Some(self.zoom_level);
        }

        // Disabling pixel-snapping lets glyphs render at their exact
        // sub-pixel position instead of each being individually rounded to
        // the physical pixel grid — the rounding is what makes text look
        // unevenly spaced/jagged at the fractional effective pixel ratios
        // this app's zoom levels commonly produce. Trivial cost (one write
        // into egui's own memory), so no last-applied change-gate needed.
        ctx.tessellation_options_mut(|opts| {
            opts.round_text_to_pixels = !self.smooth_text_rendering;
        });

        self.apply_window_title(ctx);
    }

    /// Reload changed files, collect the explorer scan, and expire reload
    /// flashes, asking for the repaints each of those still needs.
    fn poll_background_work(&mut self, ctx: &egui::Context) {
        // Check for file changes and reload affected tabs
        let changed_paths = self.check_file_changes();
        if !changed_paths.is_empty() {
            self.reload_changed_tabs(changed_paths);
        }

        // Poll for asynchronous Explorer root scan completion.
        if self.file_explorer.pending_scan.is_some() {
            if self.file_explorer.poll_pending_scan() {
                log::info!("Explorer directory scan completed");
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Clean up expired flash effects and request repaints while animating
        if !self.flashing_paths.is_empty() {
            let flash_duration = Duration::from_millis(FLASH_DURATION_MS);
            self.flashing_paths
                .retain(|_, start_time| start_time.elapsed() < flash_duration);

            // Request repaints while there are active flashes
            if !self.flashing_paths.is_empty() {
                ctx.request_repaint_after(Duration::from_millis(16)); // ~60fps for smooth animation
            }
        }
    }

    /// Rebuild the egui visuals when the theme or a color override changed.
    /// Returns whether it did.
    fn apply_theme(&mut self, ctx: &egui::Context) -> bool {
        let wanted = Some((self.theme, self.colors));
        if self.applied_visuals == wanted {
            return false;
        }
        self.applied_visuals = wanted;
        ctx.set_visuals(theme::visuals(self.theme.palette(), self.colors));
        true
    }

    /// Rebuild what the document pane renders with from the theme and the
    /// fonts now in force, and have every open document lay out again:
    /// a font change moves every line.
    fn rebuild_document_look(&mut self, ctx: &egui::Context) {
        let fonts = document_fonts(&self.fonts);
        self.document_look = DocumentLook::new(&ctx.style(), self.theme.palette(), &fonts);
        self.relayout_documents();
    }

    /// Tell the renderer that every open document's layout is stale.
    pub(crate) fn relayout_documents(&mut self) {
        for tab in &mut self.tabs {
            tab.cache.mark_layout_changed();
        }
    }

    /// Reload fonts only when the selected family, preset, or size class
    /// actually changes — rescanning the system font collection on every
    /// frame would be expensive and is unnecessary. Returns whether it
    /// reloaded them.
    fn apply_fonts(&mut self, ctx: &egui::Context) -> bool {
        if self.applied_fonts.as_ref() == Some(&self.fonts) {
            return false;
        }
        setup_fonts(ctx, &self.fonts);
        self.applied_fonts = Some(self.fonts.clone());
        true
    }

    /// Send the window title to the viewport once a tab change made it stale.
    fn apply_window_title(&mut self, ctx: &egui::Context) {
        if !self.title_dirty {
            return;
        }
        self.title_dirty = false;
        let title = self.tabs.get(self.active_tab).map_or_else(
            || "Markdown Viewer".to_string(),
            |tab| format!("{} - Markdown Viewer", tab.title()),
        );
        if title != self.last_window_title {
            self.last_window_title = title;
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.last_window_title.clone()));
        }
    }

    /// The error bar, the find bar, and the tab strip.
    fn render_top_bars(&mut self, ctx: &egui::Context) {
        self.render_error_bar(ctx);

        // Find bar (conditional, between error bar and tab bar)
        let search_outcome = self.render_search_bar(ctx);
        // Rebuild matches if query or tab changed (TextEdit may have mutated the query)
        self.maybe_rebuild_search();
        if search_outcome.close_requested {
            self.close_search();
        }
        if search_outcome.next_clicked {
            self.jump_match(1);
        }
        if search_outcome.prev_clicked {
            self.jump_match(-1);
        }

        // Close tab if requested
        if let Some(idx) = self.render_tab_bar(ctx) {
            self.close_tab(idx);
        }
    }

    /// The explorer, the outline, and the document pane, with the
    /// navigation each of them requested.
    fn render_panels(&mut self, ctx: &egui::Context, ctrl_held: bool) {
        // File explorer (left sidebar)
        let explorer_action = self.render_file_explorer(ctx);

        // Explorer matches document links: primary click replaces the active
        // document, while Ctrl/Cmd+click opens (or focuses) a separate tab.
        if let Some(path) = explorer_action.file_to_open {
            if ctrl_held {
                self.open_in_new_tab(path);
            } else {
                self.navigate_active_tab_to(&path);
            }
        }

        // Close tab from explorer (middle-click on open file)
        if let Some(path) = explorer_action.file_to_close {
            if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
                self.close_tab(idx);
            }
        }

        // Outline sidebar (right) - at top level for proper layout
        self.render_outline(ctx);

        // Main content area
        let mut open_in_new_tab: Option<(PathBuf, Option<String>)> = None;
        let path_before_render = self.tabs.get(self.active_tab).map(|tab| tab.path.clone());
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                open_in_new_tab = self.render_tab_content(ui, ctrl_held);
            });

        // A primary-clicked document link navigates inside Tab while it is
        // rendered. Synchronize the app-level state that depends on tab paths.
        let path_after_render = self.tabs.get(self.active_tab).map(|tab| tab.path.clone());
        if path_after_render != path_before_render {
            self.active_tab_navigated();
        }

        // Open link in new tab if requested
        if let Some((path, fragment)) = open_in_new_tab {
            self.open_in_new_tab_with_fragment(path, fragment.as_deref());
        }
    }

    /// Show `texture` enlarged at 100% in a fresh lightbox.
    fn open_lightbox(&mut self, texture: LightboxTexture, base_size: egui::Vec2) {
        self.lightbox_open_count += 1;
        self.lightbox = Some(LightboxState {
            texture,
            base_size,
            zoom: 1.0,
            open_id: self.lightbox_open_count,
            scroll_offset: egui::Vec2::ZERO,
        });
    }

    /// The lightbox, the settings dialogs, and the hint that shows while
    /// files hover over the window.
    fn render_overlays(&mut self, ctx: &egui::Context, files_hover: bool) {
        // Check if a mermaid diagram was clicked → open lightbox
        // Texture is pre-rasterized at 2x by background thread — no work on click
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let mermaid = tab.cache.take_clicked_mermaid();
            let image = tab.cache.take_clicked_image();
            if let Some((texture, base_size)) = mermaid {
                self.open_lightbox(LightboxTexture::Owned(texture), base_size);
            }
            if let Some((texture_id, base_size)) = image {
                self.open_lightbox(LightboxTexture::Loaded(texture_id), base_size);
            }
        }

        // Lightbox overlay for enlarged diagrams or images
        self.render_lightbox(ctx);

        // Highlight color picker window
        self.render_color_settings(ctx);
        // Document font picker window
        self.render_font_settings(ctx);

        if files_hover {
            drop::paint_hint(ctx);
        }
    }
}
