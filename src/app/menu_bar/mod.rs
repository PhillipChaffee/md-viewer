//! The menu bar, its navigation buttons, and the status readout.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::{zoom_percent, MarkdownApp};
use crate::layout::ZOOM_STEP;

#[cfg(test)]
mod tests;

/// Record `action` as the menu bar's pick for this frame and close the menu
/// when `item` was clicked.
fn pick(ui: &egui::Ui, item: &egui::Response, action: AppAction, picked: &mut Option<AppAction>) {
    if item.clicked() {
        *picked = Some(action);
        ui.close();
    }
}

/// The text of an on/off menu item; `on_text` is the one with the check mark.
const fn toggle_text(on: bool, on_text: &'static str, off_text: &'static str) -> &'static str {
    if on {
        on_text
    } else {
        off_text
    }
}

/// The state an on/off menu item reports to the MCP bridge.
#[cfg(feature = "mcp")]
const fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

impl MarkdownApp {
    /// Render the menu bar, then carry out the action picked from it.
    pub(crate) fn render_menu_bar(&mut self, ctx: &egui::Context) {
        let mut picked = None;
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| self.file_menu(ui, &mut picked));
                ui.menu_button("Navigate", |ui| self.navigate_menu(ui, &mut picked));
                let view_menu = ui.menu_button("View", |ui| self.view_menu(ui, &mut picked));
                #[cfg(feature = "mcp")]
                self.mcp_bridge
                    .register_widget("Menu: View", "button", &view_menu.response, None);
                #[cfg(not(feature = "mcp"))]
                let _ = view_menu;

                self.history_buttons(ui, &mut picked);
                self.status_readout(ui);
            });
        });

        // The pick runs after the bar is laid out, so its status readout
        // still shows the state from before the action; repaint to catch up.
        if let Some(action) = picked {
            self.apply_action(ctx, action);
            ctx.request_repaint();
        }
    }

    fn file_menu(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let open = ui.add(egui::Button::new("Open File in New Tab...").shortcut_text("Ctrl+T"));
        pick(ui, &open, AppAction::OpenFile, picked);
        let open_folder = ui.button("Open Folder...");
        pick(ui, &open_folder, AppAction::OpenFolder, picked);
        let close_tab = ui.add(egui::Button::new("Close Tab").shortcut_text("Ctrl+W"));
        pick(ui, &close_tab, AppAction::CloseTab, picked);

        ui.separator();

        let find = ui.add(egui::Button::new("Find...").shortcut_text("Ctrl+F"));
        pick(ui, &find, AppAction::OpenSearch, picked);

        ui.separator();

        let watch_text = toggle_text(self.watcher.is_some(), "✓ Watch Files", "Watch Files");
        let watch = ui.add(egui::Button::new(watch_text).shortcut_text("F5"));
        pick(ui, &watch, AppAction::ToggleWatch, picked);

        ui.separator();

        let quit = ui.add(egui::Button::new("Quit").shortcut_text("Ctrl+Q"));
        pick(ui, &quit, AppAction::Quit, picked);
    }

    /// Whether the active tab has history to step back and forward through.
    fn history_availability(&self) -> (bool, bool) {
        self.tabs
            .get(self.active_tab)
            .map_or((false, false), |tab| {
                (tab.can_go_back(), tab.can_go_forward())
            })
    }

    fn navigate_menu(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let (can_back, can_forward) = self.history_availability();
        let back = ui.add_enabled(can_back, egui::Button::new("← Back").shortcut_text("Alt+←"));
        pick(ui, &back, AppAction::NavigateBack, picked);
        let forward = ui.add_enabled(
            can_forward,
            egui::Button::new("→ Forward").shortcut_text("Alt+→"),
        );
        pick(ui, &forward, AppAction::NavigateForward, picked);
    }

    fn view_menu(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        self.view_menu_layout_items(ui, picked);
        self.view_menu_rendering_items(ui, picked);
        ui.separator();
        // Distinct from zoom: zoom scales the whole UI, leaving
        // formulas and prose in the same ratio. This changes only
        // the formulas, relative to the text around them.
        ui.menu_button("Formula Size", |ui| self.formula_size_menu(ui, picked));
    }

    /// The theme switch and the toggles for the panels around the document.
    fn view_menu_layout_items(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
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
        pick(ui, &theme_btn, AppAction::ToggleDarkMode, picked);

        let explorer_text = toggle_text(self.show_explorer, "✓ Show Explorer", "Show Explorer");
        let explorer_btn = ui.add(egui::Button::new(explorer_text).shortcut_text("Ctrl+Shift+E"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge.register_widget(
            "Menu: View → Show Explorer",
            "button",
            &explorer_btn,
            Some(on_off(self.show_explorer)),
        );
        pick(ui, &explorer_btn, AppAction::ToggleExplorer, picked);

        let outline_text = toggle_text(self.show_outline, "✓ Show Outline", "Show Outline");
        let outline_btn = ui.add(egui::Button::new(outline_text).shortcut_text("Ctrl+Shift+O"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge.register_widget(
            "Menu: View → Show Outline",
            "button",
            &outline_btn,
            Some(on_off(self.show_outline)),
        );
        pick(ui, &outline_btn, AppAction::ToggleOutline, picked);

        let full_width_text = toggle_text(self.full_width_content, "✓ Full Width", "Full Width");
        let full_width_btn = ui.add(egui::Button::new(full_width_text));
        #[cfg(feature = "mcp")]
        self.mcp_bridge.register_widget(
            "Menu: View → Full Width",
            "button",
            &full_width_btn,
            Some(on_off(self.full_width_content)),
        );
        pick(ui, &full_width_btn, AppAction::ToggleFullWidth, picked);
    }

    /// The color and font dialogs, the text-rendering toggle, and the zoom
    /// steps.
    fn view_menu_rendering_items(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let highlight_btn = ui.add(egui::Button::new("Colors…"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Menu: View → Colors", "button", &highlight_btn, None);
        pick(ui, &highlight_btn, AppAction::OpenColorSettings, picked);

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
        pick(ui, &fonts_btn, AppAction::OpenFontSettings, picked);

        let smooth_text_text = toggle_text(
            self.smooth_text_rendering,
            "✓ Smooth Text Rendering",
            "Smooth Text Rendering",
        );
        let smooth_text_btn = ui.add(egui::Button::new(smooth_text_text));
        #[cfg(feature = "mcp")]
        self.mcp_bridge.register_widget(
            "Menu: View → Smooth Text Rendering",
            "button",
            &smooth_text_btn,
            Some(on_off(self.smooth_text_rendering)),
        );
        pick(ui, &smooth_text_btn, AppAction::ToggleSmoothText, picked);

        ui.separator();

        let zoom_in_btn = ui.add(egui::Button::new("Zoom In").shortcut_text("Ctrl++"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Menu: View → Zoom In", "button", &zoom_in_btn, None);
        pick(ui, &zoom_in_btn, AppAction::ZoomBy(ZOOM_STEP), picked);

        let zoom_out_btn = ui.add(egui::Button::new("Zoom Out").shortcut_text("Ctrl+-"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Menu: View → Zoom Out", "button", &zoom_out_btn, None);
        pick(
            ui,
            &zoom_out_btn,
            AppAction::ZoomBy(1.0 / ZOOM_STEP),
            picked,
        );

        let reset_zoom_btn = ui.add(egui::Button::new("Reset Zoom").shortcut_text("Ctrl+0"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Menu: View → Reset Zoom", "button", &reset_zoom_btn, None);
        pick(ui, &reset_zoom_btn, AppAction::ResetZoom, picked);
    }

    fn formula_size_menu(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
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
            pick(ui, &btn, AppAction::SetFormulaScale(scale), picked);
        }
    }

    /// The visible back and forward arrows beside the menus.
    fn history_buttons(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        ui.separator();
        let (can_back, can_forward) = self.history_availability();

        let back_btn = ui.add_enabled(can_back, egui::Button::new("◀").small());
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Navigate Back", "button", &back_btn, None);
        if back_btn.on_hover_text("Back (Alt+←)").clicked() {
            *picked = Some(AppAction::NavigateBack);
        }

        let forward_btn = ui.add_enabled(can_forward, egui::Button::new("▶").small());
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Navigate Forward", "button", &forward_btn, None);
        if forward_btn.on_hover_text("Forward (Alt+→)").clicked() {
            *picked = Some(AppAction::NavigateForward);
        }
    }

    /// Zoom level, live-reload state, and the active file's path, right-aligned.
    fn status_readout(&self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Show zoom level if not at 100%
            if (self.zoom_level - 1.0).abs() > 0.01 {
                ui.label(
                    egui::RichText::new(format!("{}%", zoom_percent(self.zoom_level)))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                ui.separator();
            }

            if self.watcher.is_some() {
                ui.label(
                    egui::RichText::new("● LIVE").color(egui::Color32::from_rgb(100, 200, 100)),
                );
                ui.separator();
            }

            // Show current file path from active tab
            let active_path = self.tabs.get(self.active_tab).map(|tab| &tab.path);
            if let Some(path) = active_path.filter(|path| path.exists()) {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(path.display().to_string())
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                );
            }
        });
    }
}
