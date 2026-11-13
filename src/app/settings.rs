//! The color and font settings dialogs.

use eframe::egui;

use crate::app::MarkdownApp;
use crate::system_fonts::{FontPreset, TextSizeClass};

impl MarkdownApp {
    pub(crate) fn render_color_settings(&mut self, ctx: &egui::Context) {
        if !self.show_color_settings_dialog {
            return;
        }

        let mut open = true;
        // Set inside the window closure, applied after `.show()` returns —
        // same one-shot-flag pattern the font/tab-close deferred actions use
        // elsewhere in this file. Each row is independently resettable, so
        // each gets its own flag.
        let mut new_highlight_color: Option<Option<egui::Color32>> = None;
        let mut new_link_color: Option<Option<egui::Color32>> = None;

        egui::Window::new("Colors")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Highlight Color:");
                    // Read the currently *effective* color (override or
                    // theme default — update() already applied Visuals for
                    // this frame) rather than re-deriving egui's own
                    // dark/light selection defaults by hand.
                    let mut color = ui.visuals().selection.bg_fill;
                    let color_btn = ui.color_edit_button_srgba(&mut color);
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Colors Dialog: Highlight Color",
                        "color",
                        &color_btn,
                        None,
                    );
                    if color_btn.changed() {
                        new_highlight_color = Some(Some(color));
                    }

                    let reset_btn =
                        ui.add_enabled(self.highlight_color.is_some(), egui::Button::new("Reset"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Colors Dialog: Highlight Color Reset",
                        "button",
                        &reset_btn,
                        None,
                    );
                    if reset_btn.clicked() {
                        new_highlight_color = Some(None);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Link Color:");
                    let mut color = ui.visuals().hyperlink_color;
                    let color_btn = ui.color_edit_button_srgba(&mut color);
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Colors Dialog: Link Color",
                        "color",
                        &color_btn,
                        None,
                    );
                    if color_btn.changed() {
                        new_link_color = Some(Some(color));
                    }

                    let reset_btn =
                        ui.add_enabled(self.link_color.is_some(), egui::Button::new("Reset"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Colors Dialog: Link Color Reset",
                        "button",
                        &reset_btn,
                        None,
                    );
                    if reset_btn.clicked() {
                        new_link_color = Some(None);
                    }
                });
            });

        self.show_color_settings_dialog = open;
        if let Some(color) = new_highlight_color {
            self.highlight_color = color;
        }
        if let Some(color) = new_link_color {
            self.link_color = color;
        }
    }

    pub(crate) fn render_font_settings(&mut self, ctx: &egui::Context) {
        if !self.show_font_dialog {
            return;
        }

        let mut open = true;
        // Set inside the window closure, applied after `.show()` returns —
        // the list rows borrow `available_font_families` while the closure
        // runs, so a picked name can't mutate `self` in place.
        let mut new_selection: Option<Option<String>> = None;

        // Single font dialog: preset, text size, and family picker in one
        // place. Handlers only mutate state — the `last_applied_font_config`
        // diff gate in update() rebuilds the font chains once per change.
        egui::Window::new("Fonts")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                // Markdown font presets. Missing preset faces degrade to the
                // app's own fallback chain.
                ui.label("Preset");
                ui.horizontal_wrapped(|ui| {
                    for preset in FontPreset::ALL {
                        let active = self.font_preset == preset;
                        let preset_btn = ui
                            .selectable_label(active, preset.label())
                            .on_hover_text(preset.description());
                        #[cfg(feature = "mcp")]
                        self.mcp_bridge.register_widget(
                            &format!("Font Dialog: Preset: {}", preset.label()),
                            "button",
                            &preset_btn,
                            Some(if active { "selected" } else { "" }),
                        );
                        if preset_btn.clicked() && !active {
                            self.font_preset = preset;
                        }
                    }
                });
                if self.selected_font_family.is_some() {
                    ui.small(
                        "A picked family leads the body chain; presets still \
                         set the code font and line heights.",
                    );
                }

                // Shared text-size classes: one scale for every preset.
                ui.add_space(2.0);
                ui.label("Text Size");
                ui.horizontal_wrapped(|ui| {
                    for size in TextSizeClass::ALL {
                        let active = self.text_size_class == size;
                        let size_btn = ui
                            .selectable_label(active, size.label())
                            .on_hover_text("Base text size; headings and code scale with it");
                        #[cfg(feature = "mcp")]
                        self.mcp_bridge.register_widget(
                            &format!("Font Dialog: Size: {}", size.label()),
                            "button",
                            &size_btn,
                            Some(if active { "selected" } else { "" }),
                        );
                        if size_btn.clicked() && !active {
                            self.text_size_class = size;
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.label("Family");

                ui.horizontal(|ui| {
                    ui.label("Search:");
                    let filter_edit = ui.text_edit_singleline(&mut self.font_filter);
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Font Dialog: Search",
                        "textbox",
                        &filter_edit,
                        None,
                    );
                    #[cfg(not(feature = "mcp"))]
                    let _ = filter_edit;
                });

                // One canonical name per body-capable family, scanned once in
                // the background at startup. While the scan is running, only
                // "System Default" is offered.
                if self.available_font_families.is_empty()
                    && self.pending_font_family_scan.is_some()
                {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Scanning installed fonts…");
                    });
                }

                // Only the empty-filter (common) case avoids allocating: it
                // reuses `available_font_families` directly. A non-empty
                // filter needs one Vec<usize> of matching indices, built from
                // the precomputed lowercase names (no per-name allocation).
                let filter = self.font_filter.to_ascii_lowercase();
                let filtered_indices: Option<Vec<usize>> = if filter.is_empty() {
                    None
                } else {
                    Some(
                        self.available_font_families_lower
                            .iter()
                            .enumerate()
                            .filter(|(_, lower)| lower.contains(&filter))
                            .map(|(i, _)| i)
                            .collect(),
                    )
                };
                let match_count = filtered_indices
                    .as_ref()
                    .map_or(self.available_font_families.len(), Vec::len);

                let row_height = ui.spacing().interact_size.y;
                // auto_shrink_x = false: egui's default (TRUE) shrinks the
                // area to its widest row, which paints the floating scrollbar
                // mid-dialog instead of at the window's right edge.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(300.0)
                    .show_rows(
                        ui,
                        row_height,
                        match_count + 1, // + 1 for the pinned "System Default" row
                        |ui, rows| {
                            for row in rows {
                                if row == 0 {
                                    let default_selected = self.selected_font_family.is_none();
                                    let default_btn =
                                        ui.selectable_label(default_selected, "System Default");
                                    #[cfg(feature = "mcp")]
                                    self.mcp_bridge.register_widget(
                                        "Font Dialog: System Default",
                                        "button",
                                        &default_btn,
                                        Some(if default_selected { "selected" } else { "" }),
                                    );
                                    if default_btn.clicked() {
                                        new_selection = Some(None);
                                    }
                                    continue;
                                }
                                let font_index = filtered_indices
                                    .as_ref()
                                    .map_or(row - 1, |indices| indices[row - 1]);
                                let name = &self.available_font_families[font_index];
                                let is_selected =
                                    self.selected_font_family.as_deref() == Some(name.as_str());
                                if ui.selectable_label(is_selected, name).clicked() {
                                    new_selection = Some(Some(name.clone()));
                                }
                            }
                        },
                    );
            });

        self.show_font_dialog = open;
        if let Some(choice) = new_selection {
            self.selected_font_family = choice;
        }
    }
}
