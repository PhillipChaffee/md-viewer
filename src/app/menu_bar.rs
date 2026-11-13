//! The menu bar, its navigation buttons, and the status readout.

use eframe::egui;

use crate::app::MarkdownApp;
use crate::layout::{zoomed, ZOOM_STEP};

impl MarkdownApp {
    /// Render the menu bar, then act on history navigation requested by
    /// its buttons or carried in from the keyboard shortcuts.
    pub(crate) fn render_menu_bar(
        &mut self,
        ctx: &egui::Context,
        mut go_back: bool,
        mut go_forward: bool,
    ) {
        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("Open File in New Tab...").shortcut_text("Ctrl+T"))
                        .clicked()
                    {
                        self.open_file_dialog();
                        ui.close();
                    }

                    if ui.button("Open Folder...").clicked() {
                        self.open_folder_dialog();
                        ui.close();
                    }

                    if ui
                        .add(egui::Button::new("Close Tab").shortcut_text("Ctrl+W"))
                        .clicked()
                    {
                        self.close_active_tab();
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add(egui::Button::new("Find...").shortcut_text("Ctrl+F"))
                        .clicked()
                    {
                        self.search.is_open = true;
                        self.search.focus_requested = true;
                        ui.close();
                    }

                    ui.separator();

                    let is_watching = self.watcher.is_some();
                    let watch_text = if is_watching {
                        "✓ Watch Files"
                    } else {
                        "Watch Files"
                    };
                    if ui
                        .add(egui::Button::new(watch_text).shortcut_text("F5"))
                        .clicked()
                    {
                        if is_watching {
                            self.stop_watching();
                            self.watch_enabled = false;
                        } else {
                            self.watch_enabled = true;
                            self.start_watching();
                        }
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add(egui::Button::new("Quit").shortcut_text("Ctrl+Q"))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });

                ui.menu_button("Navigate", |ui| {
                    let can_back = self
                        .tabs
                        .get(self.active_tab)
                        .map(|t| t.can_go_back())
                        .unwrap_or(false);
                    if ui
                        .add_enabled(can_back, egui::Button::new("← Back").shortcut_text("Alt+←"))
                        .clicked()
                    {
                        self.navigate_active_history(true);
                        ui.close();
                    }

                    let can_forward = self
                        .tabs
                        .get(self.active_tab)
                        .map(|t| t.can_go_forward())
                        .unwrap_or(false);
                    if ui
                        .add_enabled(
                            can_forward,
                            egui::Button::new("→ Forward").shortcut_text("Alt+→"),
                        )
                        .clicked()
                    {
                        self.navigate_active_history(false);
                        ui.close();
                    }
                });

                #[cfg_attr(not(feature = "mcp"), allow(unused_variables))]
                let view_menu = ui.menu_button("View", |ui| {
                    let theme_text = if self.dark_mode {
                        "☀ Light Mode" // ☀ is text-presentation → monochrome
                    } else {
                        "☾ Dark Mode" // ☾ (U+263E) is a text symbol, not emoji
                    };
                    let theme_btn = ui.add(egui::Button::new(theme_text).shortcut_text("Ctrl+D"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Dark Mode",
                        "button",
                        &theme_btn,
                        Some(if self.dark_mode { "dark" } else { "light" }),
                    );
                    if theme_btn.clicked() {
                        self.dark_mode = !self.dark_mode;
                        ui.close();
                    }

                    let explorer_text = if self.show_explorer {
                        "✓ Show Explorer"
                    } else {
                        "Show Explorer"
                    };
                    let explorer_btn =
                        ui.add(egui::Button::new(explorer_text).shortcut_text("Ctrl+Shift+E"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Show Explorer",
                        "button",
                        &explorer_btn,
                        Some(if self.show_explorer { "on" } else { "off" }),
                    );
                    if explorer_btn.clicked() {
                        self.show_explorer = !self.show_explorer;
                        ui.close();
                    }

                    let outline_text = if self.show_outline {
                        "✓ Show Outline"
                    } else {
                        "Show Outline"
                    };
                    let outline_btn =
                        ui.add(egui::Button::new(outline_text).shortcut_text("Ctrl+Shift+O"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Show Outline",
                        "button",
                        &outline_btn,
                        Some(if self.show_outline { "on" } else { "off" }),
                    );
                    if outline_btn.clicked() {
                        self.show_outline = !self.show_outline;
                        ui.close();
                    }

                    let full_width_text = if self.full_width_content {
                        "✓ Full Width"
                    } else {
                        "Full Width"
                    };
                    let full_width_btn = ui.add(egui::Button::new(full_width_text));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Full Width",
                        "button",
                        &full_width_btn,
                        Some(if self.full_width_content { "on" } else { "off" }),
                    );
                    if full_width_btn.clicked() {
                        self.full_width_content = !self.full_width_content;
                        ui.close();
                    }

                    let highlight_btn = ui.add(egui::Button::new("Colors…"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Colors",
                        "button",
                        &highlight_btn,
                        None,
                    );
                    if highlight_btn.clicked() {
                        self.show_color_settings_dialog = true;
                        ui.close();
                    }

                    // Single font entry point: opens the Fonts dialog, which
                    // holds the preset, the text size, and the family picker.
                    let fonts_btn = ui.add(egui::Button::new("Fonts…"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Fonts",
                        "button",
                        &fonts_btn,
                        self.selected_font_family.as_deref(),
                    );
                    if fonts_btn.clicked() {
                        self.show_font_dialog = true;
                        ui.close();
                    }

                    let smooth_text_text = if self.smooth_text_rendering {
                        "✓ Smooth Text Rendering"
                    } else {
                        "Smooth Text Rendering"
                    };
                    let smooth_text_btn = ui.add(egui::Button::new(smooth_text_text));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Smooth Text Rendering",
                        "button",
                        &smooth_text_btn,
                        Some(if self.smooth_text_rendering {
                            "on"
                        } else {
                            "off"
                        }),
                    );
                    if smooth_text_btn.clicked() {
                        self.smooth_text_rendering = !self.smooth_text_rendering;
                        ui.close();
                    }

                    ui.separator();

                    let zoom_in_btn = ui.add(egui::Button::new("Zoom In").shortcut_text("Ctrl++"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Zoom In",
                        "button",
                        &zoom_in_btn,
                        None,
                    );
                    if zoom_in_btn.clicked() {
                        self.zoom_level = zoomed(self.zoom_level, ZOOM_STEP);
                        ui.close();
                    }
                    let zoom_out_btn =
                        ui.add(egui::Button::new("Zoom Out").shortcut_text("Ctrl+-"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Zoom Out",
                        "button",
                        &zoom_out_btn,
                        None,
                    );
                    if zoom_out_btn.clicked() {
                        self.zoom_level = zoomed(self.zoom_level, 1.0 / ZOOM_STEP);
                        ui.close();
                    }
                    let reset_zoom_btn =
                        ui.add(egui::Button::new("Reset Zoom").shortcut_text("Ctrl+0"));
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Menu: View → Reset Zoom",
                        "button",
                        &reset_zoom_btn,
                        None,
                    );
                    if reset_zoom_btn.clicked() {
                        self.zoom_level = 1.0;
                        ui.close();
                    }

                    ui.separator();
                    // Distinct from zoom: zoom scales the whole UI, leaving
                    // formulas and prose in the same ratio. This changes only
                    // the formulas, relative to the text around them.
                    ui.menu_button("Formula Size", |ui| {
                        for (label, scale) in [
                            ("100%", 1.0f32),
                            ("110%", 1.1),
                            ("125%", 1.25),
                            ("150%", 1.5),
                        ] {
                            let selected = (self.math_scale - scale).abs() < 0.001;
                            let text = if selected {
                                format!("✓ {label}")
                            } else {
                                format!("   {label}")
                            };
                            let btn = ui.add(egui::Button::new(text));
                            #[cfg(feature = "mcp")]
                            self.mcp_bridge.register_widget(
                                &format!("Menu: View → Formula Size → {label}"),
                                "button",
                                &btn,
                                Some(if selected { "selected" } else { "unselected" }),
                            );
                            if btn.clicked() {
                                self.math_scale = scale;
                                ui.close();
                            }
                        }
                    });
                });
                #[cfg(feature = "mcp")]
                self.mcp_bridge
                    .register_widget("Menu: View", "button", &view_menu.response, None);

                // Navigation buttons (visible arrows for back/forward)
                ui.separator();
                let can_back = self
                    .tabs
                    .get(self.active_tab)
                    .map(|t| t.can_go_back())
                    .unwrap_or(false);
                let back_btn = ui.add_enabled(can_back, egui::Button::new("◀").small());
                #[cfg(feature = "mcp")]
                self.mcp_bridge
                    .register_widget("Navigate Back", "button", &back_btn, None);
                if back_btn.on_hover_text("Back (Alt+←)").clicked() {
                    go_back = true;
                }

                let can_forward = self
                    .tabs
                    .get(self.active_tab)
                    .map(|t| t.can_go_forward())
                    .unwrap_or(false);
                let forward_btn = ui.add_enabled(can_forward, egui::Button::new("▶").small());
                #[cfg(feature = "mcp")]
                self.mcp_bridge
                    .register_widget("Navigate Forward", "button", &forward_btn, None);
                if forward_btn.on_hover_text("Forward (Alt+→)").clicked() {
                    go_forward = true;
                }

                // Show status on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Show zoom level if not at 100%
                    if (self.zoom_level - 1.0).abs() > 0.01 {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}%",
                                (self.zoom_level * 100.0).round() as i32
                            ))
                            .small()
                            .color(ui.visuals().weak_text_color()),
                        );
                        ui.separator();
                    }

                    if self.watcher.is_some() {
                        ui.label(
                            egui::RichText::new("● LIVE")
                                .color(egui::Color32::from_rgb(100, 200, 100)),
                        );
                        ui.separator();
                    }

                    // Show current file path from active tab
                    if let Some(tab) = self.tabs.get(self.active_tab) {
                        if tab.path.exists() {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(tab.path.display().to_string())
                                        .small()
                                        .color(ui.visuals().weak_text_color()),
                                )
                                .truncate(),
                            );
                        }
                    }
                });
            });
        });

        // Handle navigation button clicks (must be after menu bar UI)
        if go_back {
            self.navigate_active_history(true);
        }
        if go_forward {
            self.navigate_active_history(false);
        }
    }
}
