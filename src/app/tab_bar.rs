//! The tab strip.

use eframe::egui;

use crate::app::MarkdownApp;
use crate::watcher::FLASH_DURATION_MS;

impl MarkdownApp {
    /// Render the custom tab bar
    pub(crate) fn render_tab_bar(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        let mut tab_to_close: Option<usize> = None;
        let mut new_active: Option<usize> = None;
        let mut close_others: Option<usize> = None;

        // Collect tab info first to avoid borrow issues
        let tab_info: Vec<(String, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| (tab.title(), idx == self.active_tab))
            .collect();

        let tab_count = tab_info.len();
        let hovered_tab = self.hovered_tab;

        // Collect widget data for MCP registration (name, widget_type, rect, value)
        #[cfg(feature = "mcp")]
        let mut widget_data: Vec<(String, &'static str, egui::Rect, Option<String>)> = Vec::new();

        ui.horizontal(|ui| {
            // Scrollable tab area
            egui::ScrollArea::horizontal()
                .max_width(ui.available_width() - 30.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (idx, (title, is_active)) in tab_info.iter().enumerate() {
                            let is_hovered = hovered_tab == Some(idx);

                            // Tab frame
                            let tab_response = ui.horizontal(|ui| {
                                // Tab button
                                let text = egui::RichText::new(title);
                                let text = if *is_active { text.strong() } else { text };

                                let response = ui.selectable_label(*is_active, text);

                                // Collect tab widget data for MCP
                                #[cfg(feature = "mcp")]
                                widget_data.push((
                                    format!("Tab: {}", title),
                                    "tab",
                                    response.rect,
                                    Some(if *is_active {
                                        "active".to_string()
                                    } else {
                                        "".to_string()
                                    }),
                                ));

                                if response.clicked() {
                                    new_active = Some(idx);
                                }

                                // Middle-click to close
                                if response.middle_clicked() {
                                    tab_to_close = Some(idx);
                                }

                                // Close button (show on hover or active)
                                if *is_active || is_hovered {
                                    let close_btn = ui.small_button("×");

                                    // Collect close button widget data for MCP
                                    #[cfg(feature = "mcp")]
                                    widget_data.push((
                                        format!("Close Tab: {}", title),
                                        "button",
                                        close_btn.rect,
                                        None,
                                    ));

                                    if close_btn.clicked() {
                                        tab_to_close = Some(idx);
                                    }
                                }

                                // Context menu
                                response.context_menu(|ui| {
                                    if ui.button("Close").clicked() {
                                        tab_to_close = Some(idx);
                                        ui.close();
                                    }
                                    if tab_count > 1 && ui.button("Close Others").clicked() {
                                        close_others = Some(idx);
                                        ui.close();
                                    }
                                });

                                response
                            });

                            // Track hover state
                            if tab_response.response.hovered() {
                                self.hovered_tab = Some(idx);
                            }

                            ui.separator();
                        }
                    });
                });

            // Placeholder hint when no document is open (the welcome page is showing)
            if tab_count == 0 {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("No file open").weak());
                ui.add_space(4.0);
            }

            // Opening a file is what creates a new tab; there is no empty-tab state.
            let new_tab_btn = ui
                .button("+")
                .on_hover_text("Open File in New Tab... (Ctrl+T)");

            // Collect the open-file action for MCP.
            #[cfg(feature = "mcp")]
            widget_data.push((
                "Open File in New Tab".to_string(),
                "button",
                new_tab_btn.rect,
                None,
            ));

            if new_tab_btn.clicked() {
                self.open_file_dialog();
            }

            // Preview|Edit segmented toggle for the active tab, right-aligned
            // in the tab bar (DSH-better-sidebar editor-header placement).
            // Borderless on purpose: the tab bar strip already carries a
            // border, so only the filled active segment provides the frame.
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut switch_mode: Option<bool> = None; // Some(true) → edit
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(2))
                        .show(ui, |ui| {
                            ui.set_min_height(ui.text_style_height(&egui::TextStyle::Small));
                            ui.horizontal(|ui| {
                                // Added in Edit, Preview order so the
                                // right-to-left layout renders [Preview|Edit].
                                for (label, is_edit) in [("Edit", true), ("Preview", false)] {
                                    let active = tab.is_editing == is_edit;
                                    let text_color = if active {
                                        ui.visuals().text_color()
                                    } else {
                                        ui.visuals().weak_text_color()
                                    };
                                    let response = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(label)
                                                .small()
                                                .color(text_color),
                                        )
                                        .fill(if active {
                                            ui.visuals()
                                                .widgets
                                                .inactive
                                                .weak_bg_fill
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .corner_radius(egui::CornerRadius::same(4)),
                                    );
                                    if response.clicked() && !active {
                                        switch_mode = Some(is_edit);
                                    }
                                }
                            });
                        });
                    if let Some(to_edit) = switch_mode {
                        tab.set_edit_mode(to_edit);
                    }
                    if tab.content != tab.saved_content {
                        ui.label(
                            egui::RichText::new("●")
                                .small()
                                .color(ui.visuals().warn_fg_color),
                        )
                        .on_hover_text("Unsaved changes — press Ctrl+S to save");
                    }
                    if let Some((ok, at)) = tab.save_flash {
                        if (at.elapsed().as_millis() as u64) < FLASH_DURATION_MS * 3 {
                            let (label, color) = if ok {
                                ("Saved ✓", egui::Color32::from_rgb(90, 200, 120))
                            } else {
                                ("Save failed", ui.visuals().error_fg_color)
                            };
                            ui.label(egui::RichText::new(label).small().color(color));
                        } else {
                            tab.save_flash = None;
                        }
                    }
                });
            }
        });

        // Register all collected widgets with MCP bridge
        #[cfg(feature = "mcp")]
        for (name, widget_type, rect, value) in widget_data {
            self.mcp_bridge
                .register_widget_rect(&name, widget_type, rect, value.as_deref());
        }

        // Apply new active tab
        if let Some(idx) = new_active {
            self.active_tab = idx;
            self.title_dirty = true;
        }

        // Handle close others
        if let Some(keep_idx) = close_others {
            let kept = self.tabs.remove(keep_idx);
            self.tabs.clear();
            self.tabs.push(kept);
            self.active_tab = 0;
            self.title_dirty = true;
            self.refresh_open_tab_paths();
            if self.watch_enabled {
                self.update_watched_paths();
            }
            return None; // Don't close any tab, we already handled it
        }

        tab_to_close
    }
}
