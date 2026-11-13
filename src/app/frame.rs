//! The eframe integration: input hook, persistence, and the per-frame update sequence.

use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use eframe::egui;

use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::MarkdownApp;
use crate::persist::{PersistedState, APP_KEY};
use crate::system_fonts::setup_fonts;
use crate::watcher::FLASH_DURATION_MS;

impl eframe::App for MarkdownApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        #[cfg(feature = "mcp")]
        {
            self.mcp_bridge.process_commands();
            self.mcp_bridge.inject_raw_input(raw_input);
        }

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
        let state = PersistedState {
            dark_mode: Some(self.dark_mode),
            zoom_level: Some(self.zoom_level),
            show_outline: Some(self.show_outline),
            full_width_content: Some(self.full_width_content),
            math_scale: Some(self.math_scale),
            highlight_color: self.highlight_color.map(|c| [c.r(), c.g(), c.b(), c.a()]),
            link_color: self.link_color.map(|c| [c.r(), c.g(), c.b(), c.a()]),
            selected_font_family: self.selected_font_family.clone(),
            smooth_text_rendering: Some(self.smooth_text_rendering),
            font_preset: Some(self.font_preset),
            text_size_class: Some(self.text_size_class),
            open_tabs: Some(self.get_open_tab_paths()),
            active_tab: Some(self.active_tab),
            show_explorer: Some(self.show_explorer),
            explorer_root: self.file_explorer.root.clone(),
            expanded_dirs: Some(self.file_explorer.expanded_dirs.iter().cloned().collect()),
            explorer_sort_order: Some(self.file_explorer.sort_order),
            explorer_width: Some(self.explorer_width),
            outline_width: Some(self.outline_width),
            recent_files: Some(self.recent_files.clone()),
        };
        eframe::set_value(storage, APP_KEY, &state);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.begin_frame(ctx);
        self.handle_keyboard_shortcuts(ctx);
        self.handle_dropped_files(ctx);

        // Get ctrl state for link handling
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

        self.render_menu_bar(ctx);
        self.render_top_bars(ctx);
        self.render_panels(ctx, ctrl_held);
        self.render_overlays(ctx);

        // Capture AccessKit output for MCP bridge
        #[cfg(feature = "mcp")]
        self.mcp_bridge.capture_output(ctx);
    }
}

impl MarkdownApp {
    /// Per-frame housekeeping that runs before any input or UI: file
    /// changes, async polls, and theme, font, zoom, and title application.
    fn begin_frame(&mut self, ctx: &egui::Context) {
        // Enable AccessKit for MCP bridge
        #[cfg(feature = "mcp")]
        {
            ctx.enable_accesskit();
            self.mcp_bridge.begin_frame();
            self.mcp_bridge.store_in_context(ctx); // Enable McpUiExt methods
                                                   // Poll for MCP commands at reasonable rate (prevents CPU spin on virtual displays)
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        self.poll_background_work(ctx);

        // Limit frame rate on virtual displays (e.g., Xvfb) which lack vsync
        // request_repaint_after alone doesn't limit actual frame rate, so we sleep
        // This prevents 500%+ CPU usage during E2E testing
        if self.is_virtual_display {
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS cap
        }

        self.apply_theme(ctx);
        self.poll_font_family_scan();
        self.apply_fonts(ctx);

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

    /// Rebuild the egui visuals when dark mode or a color override changed.
    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.last_applied_dark_mode == Some(self.dark_mode)
            && self.last_applied_highlight_color == self.highlight_color
            && self.last_applied_link_color == self.link_color
        {
            return;
        }
        self.last_applied_dark_mode = Some(self.dark_mode);
        self.last_applied_highlight_color = self.highlight_color;
        self.last_applied_link_color = self.link_color;

        let mut visuals = if self.dark_mode {
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::from_rgb(0x12, 0x12, 0x12);
            v.window_fill = egui::Color32::from_rgb(0x12, 0x12, 0x12);
            v.extreme_bg_color = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
            v.override_text_color = Some(egui::Color32::from_rgb(0xE0, 0xE0, 0xE0));
            v
        } else {
            let mut v = egui::Visuals::light();
            v.panel_fill = egui::Color32::from_rgb(0xF8, 0xF8, 0xF8);
            v.window_fill = egui::Color32::from_rgb(0xF8, 0xF8, 0xF8);
            v.extreme_bg_color = egui::Color32::from_rgb(0xF0, 0xF0, 0xF0);
            v.override_text_color = Some(egui::Color32::from_rgb(0x33, 0x33, 0x33));
            v
        };
        if let Some(color) = self.highlight_color {
            visuals.selection.bg_fill = color;
        }
        if let Some(color) = self.link_color {
            visuals.hyperlink_color = color;
        }
        ctx.set_visuals(visuals);
    }

    /// Collect the background font-family scan (async phase: poll the
    /// channel, never block the UI thread). Fills the picker's family
    /// list once, then drops the channel.
    fn poll_font_family_scan(&mut self) {
        let Some(scan) = &self.pending_font_family_scan else {
            return;
        };
        match scan.try_recv() {
            Ok(names) => {
                self.available_font_families_lower =
                    names.iter().map(|name| name.to_ascii_lowercase()).collect();
                self.available_font_families = names;
                self.pending_font_family_scan = None;
                // The list may have landed while the font dialog was
                // already open; repaint so it appears without input.
                self.egui_ctx.request_repaint();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                // Sender dropped without sending (thread spawn raced or
                // panicked) — stop polling, keep whatever list we have.
                self.pending_font_family_scan = None;
            }
        }
    }

    /// Reload fonts only when the selected family, preset, or size class
    /// actually changes — rescanning the system font collection on every
    /// frame would be expensive and is unnecessary (see
    /// `docs/EGUI_WORKFLOW.md`).
    fn apply_fonts(&mut self, ctx: &egui::Context) {
        let font_config = (
            self.selected_font_family.clone(),
            self.font_preset,
            self.text_size_class,
        );
        if self.last_applied_font_config.as_ref() == Some(&font_config) {
            return;
        }
        self.last_applied_font_config = Some(font_config);
        setup_fonts(
            ctx,
            self.selected_font_family.as_deref(),
            self.font_preset,
            self.text_size_class,
        );
    }

    /// Send the window title to the viewport once a tab change made it stale.
    fn apply_window_title(&mut self, ctx: &egui::Context) {
        if !self.title_dirty {
            return;
        }
        self.title_dirty = false;
        let title = self.window_title();
        if title != self.last_window_title {
            self.last_window_title = title;
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.last_window_title.clone()));
        }
    }

    /// The error bar, the find bar, and the tab strip.
    fn render_top_bars(&mut self, ctx: &egui::Context) {
        // Show error message if any
        let mut clear_error = false;
        if let Some(error) = &self.error_message {
            egui::TopBottomPanel::top("error_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚠").color(egui::Color32::from_rgb(255, 200, 100)),
                    );
                    ui.label(
                        egui::RichText::new(error).color(egui::Color32::from_rgb(255, 200, 100)),
                    );
                    if ui.small_button("✕").clicked() {
                        clear_error = true;
                    }
                });
            });
        }
        if clear_error {
            self.error_message = None;
        }

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

        // Tab bar
        let mut tab_to_close: Option<usize> = None;
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            tab_to_close = self.render_tab_bar(ui);
        });

        // Close tab if requested
        if let Some(idx) = tab_to_close {
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

    /// The lightbox, the settings dialogs, and the drag-and-drop overlay.
    fn render_overlays(&mut self, ctx: &egui::Context) {
        // Check if a mermaid diagram was clicked → open lightbox
        // Texture is pre-rasterized at 2x by background thread — no work on click
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let Some((texture, base_size)) = tab.cache.take_clicked_mermaid() {
                self.lightbox_open_count += 1;
                self.lightbox = Some(LightboxState {
                    texture: LightboxTexture::Owned(texture),
                    base_size,
                    zoom: 1.0,
                    open_id: self.lightbox_open_count,
                    scroll_offset: egui::Vec2::ZERO,
                });
            }
            if let Some((tex_id, base_size)) = tab.cache.take_clicked_image() {
                self.lightbox_open_count += 1;
                self.lightbox = Some(LightboxState {
                    texture: LightboxTexture::Loaded(tex_id),
                    base_size,
                    zoom: 1.0,
                    open_id: self.lightbox_open_count,
                    scroll_offset: egui::Vec2::ZERO,
                });
            }
        }

        // Lightbox overlay for enlarged diagrams or images
        self.render_lightbox(ctx);

        // Highlight color picker window
        self.render_color_settings(ctx);
        // Document font picker window
        self.render_font_settings(ctx);

        // Drag and drop overlay
        if self.is_dragging {
            let screen_rect = ctx.available_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drop_overlay"),
            ));

            painter.rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop markdown file here",
                egui::FontId::proportional(24.0),
                egui::Color32::WHITE,
            );
        }
    }
}
