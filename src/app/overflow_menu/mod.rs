//! The overflow menu behind the top bar's last button: every command the
//! bar has no button for, in groups.

use eframe::egui;
use eframe::egui::containers::menu::SubMenu;

use crate::app::action::AppAction;
use crate::app::chrome::{self, MenuRow, QuietButton, BUTTON_SIDE};
use crate::app::{zoom_percent, MarkdownApp};
use crate::icons::Icon;
use crate::layout::{LineSpacing, ZOOM_STEP};
use crate::theme::ThemeId;

#[cfg(test)]
mod tests;

const MENU_WIDTH: f32 = 252.0;
const SUBMENU_WIDTH: f32 = 176.0;
/// Width of the zoom row's percentage, so the buttons either side of it
/// stay put as the number changes.
const ZOOM_READOUT_WIDTH: f32 = 48.0;

/// The sizes formulas can be set at relative to the text around them.
const FORMULA_SCALES: [(&str, f32); 4] =
    [("100%", 1.0), ("110%", 1.1), ("125%", 1.25), ("150%", 1.5)];

impl MarkdownApp {
    /// Show the overflow menu under `button` while it is open; a click on
    /// the button opens and closes it.
    pub(crate) fn overflow_menu(&self, button: &egui::Response, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        egui::Popup::menu(button)
            .align(egui::RectAlign::BOTTOM_END)
            .gap(4.0)
            // The zoom row is pressed again and again; a picked row closes
            // the menu itself.
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .sense(chrome::POPUP_SENSE)
            .style(chrome::popup_style(palette))
            .show(|ui| {
                ui.set_width(MENU_WIDTH);
                self.file_rows(ui, picked);
                chrome::hairline(ui, palette);
                self.document_rows(ui, picked);
                chrome::hairline(ui, palette);
                self.history_rows(ui, picked);
                chrome::hairline(ui, palette);
                self.view_rows(ui, picked);
                chrome::hairline(ui, palette);
                self.zoom_row(ui, picked);
                chrome::hairline(ui, palette);
                self.item(
                    ui,
                    MenuRow::new("Colors…"),
                    AppAction::OpenColorSettings,
                    picked,
                );
                self.item(
                    ui,
                    MenuRow::new("Fonts…"),
                    AppAction::OpenFontSettings,
                    picked,
                );
                chrome::hairline(ui, palette);
                self.item(
                    ui,
                    MenuRow::new("Quit").shortcut("Ctrl+Q"),
                    AppAction::Quit,
                    picked,
                );
            });
    }

    /// One row of the menu: picking it asks for `action` and closes the
    /// menu.
    fn item(
        &self,
        ui: &mut egui::Ui,
        row: MenuRow<'_>,
        action: AppAction,
        picked: &mut Option<AppAction>,
    ) {
        let response = row.show(ui, self.theme.palette());
        #[cfg(feature = "mcp")]
        self.mcp_bridge.register_widget(
            &format!("Menu: {}", row.label),
            "button",
            &response,
            row.mcp_value(),
        );
        if response.clicked() {
            *picked = Some(action);
            ui.close();
        }
    }

    /// A row that opens a submenu of `rows` beside it.
    fn submenu(&self, ui: &mut egui::Ui, label: &str, rows: impl FnOnce(&mut egui::Ui)) {
        let row = MenuRow::new(label).submenu().show(ui, self.theme.palette());
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget(&format!("Menu: {label}"), "button", &row, None);
        SubMenu::new().show(ui, &row, |ui| {
            ui.set_width(SUBMENU_WIDTH);
            rows(ui);
        });
    }

    fn file_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let open = MenuRow::new("Open File…").shortcut("Ctrl+T");
        self.item(ui, open, AppAction::OpenFile, picked);
        self.item(
            ui,
            MenuRow::new("Open Folder…"),
            AppAction::OpenFolder,
            picked,
        );
        let close = MenuRow::new("Close Tab")
            .shortcut("Ctrl+W")
            .enabled(!self.tabs.is_empty());
        self.item(ui, close, AppAction::CloseTab, picked);
    }

    /// What can be done with the open document: find in it, edit it, and
    /// keep it in step with the file.
    fn document_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let find = MenuRow::new("Find…").shortcut("Ctrl+F");
        self.item(ui, find, AppAction::OpenSearch, picked);
        let active = self.tabs.get(self.active_tab);
        let edit = MenuRow::new("Edit Document")
            .checked(active.is_some_and(|tab| tab.is_editing))
            .shortcut("Ctrl+E")
            .enabled(active.is_some());
        self.item(ui, edit, AppAction::ToggleEditMode, picked);
        let live = MenuRow::new("Live Reload")
            .checked(self.watcher.is_some())
            .shortcut("F5");
        self.item(ui, live, AppAction::ToggleWatch, picked);
    }

    fn history_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let (can_back, can_forward) = self.history_availability();
        let back = MenuRow::new("Back").shortcut("Alt+Left").enabled(can_back);
        self.item(ui, back, AppAction::NavigateBack, picked);
        let forward = MenuRow::new("Forward")
            .shortcut("Alt+Right")
            .enabled(can_forward);
        self.item(ui, forward, AppAction::NavigateForward, picked);
    }

    /// The panels around the document and how the document is set.
    fn view_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let explorer = MenuRow::new("Show Explorer")
            .checked(self.show_explorer)
            .shortcut("Ctrl+Shift+E");
        self.item(ui, explorer, AppAction::ToggleExplorer, picked);
        let outline = MenuRow::new("Show Outline")
            .checked(self.show_outline)
            .shortcut("Ctrl+Shift+O");
        self.item(ui, outline, AppAction::ToggleOutline, picked);
        let full_width = MenuRow::new("Full Width").checked(self.reading_width.fits_window());
        self.item(ui, full_width, AppAction::ToggleFullWidth, picked);
        let smooth = MenuRow::new("Smooth Text Rendering").checked(self.smooth_text_rendering);
        self.item(ui, smooth, AppAction::ToggleSmoothText, picked);

        self.submenu(ui, "Theme", |ui| self.theme_rows(ui, picked));
        self.submenu(ui, "Line Spacing", |ui| self.line_spacing_rows(ui, picked));
        // Distinct from zoom: zoom scales the whole window, leaving
        // formulas and prose in the same ratio. This changes only the
        // formulas, relative to the text around them.
        self.submenu(ui, "Formula Size", |ui| self.formula_size_rows(ui, picked));
    }

    /// The reading themes, and the step through them that Ctrl+D takes.
    fn theme_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let next = MenuRow::new("Next Theme").shortcut("Ctrl+D");
        self.item(ui, next, AppAction::NextTheme, picked);
        chrome::hairline(ui, self.theme.palette());
        for theme in ThemeId::ALL {
            let row = MenuRow::new(theme.label()).checked(self.theme == theme);
            self.item(ui, row, AppAction::SetTheme(theme), picked);
        }
    }

    fn line_spacing_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        for spacing in LineSpacing::ALL {
            let row = MenuRow::new(spacing.label()).checked(self.line_spacing == spacing);
            self.item(ui, row, AppAction::SetLineSpacing(spacing), picked);
        }
    }

    fn formula_size_rows(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        for (label, scale) in FORMULA_SCALES {
            let row = MenuRow::new(label).checked((self.math_scale - scale).abs() < 0.001);
            self.item(ui, row, AppAction::SetFormulaScale(scale), picked);
        }
    }

    /// Zoom out, the zoom in force (a click on it goes back to 100%), and
    /// zoom in. None of the three closes the menu.
    fn zoom_row(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        let percent = format!("{}%", zoom_percent(self.zoom_level));
        let reset_label = format!("Reset Zoom, {percent}");
        chrome::control_row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(chrome::MENU_LABEL_INSET);
            ui.label("Zoom");
            let controls = 2.0_f32.mul_add(BUTTON_SIDE, ZOOM_READOUT_WIDTH);
            ui.add_space((ui.available_width() - controls).max(0.0));

            let out = QuietButton::icon(Icon::Minus, "Zoom Out")
                .show(ui, palette)
                .on_hover_text("Zoom out (Ctrl+-)");
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Menu: Zoom Out", "button", &out, None);
            if out.clicked() {
                *picked = Some(AppAction::ZoomBy(1.0 / ZOOM_STEP));
            }

            let reset = QuietButton::text(&percent)
                .labelled(&reset_label)
                .min_width(ZOOM_READOUT_WIDTH)
                .show(ui, palette)
                .on_hover_text("Reset zoom (Ctrl+0)");
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Menu: Reset Zoom", "button", &reset, Some(&percent));
            if reset.clicked() {
                *picked = Some(AppAction::ResetZoom);
            }

            let zoom_in = QuietButton::icon(Icon::Plus, "Zoom In")
                .show(ui, palette)
                .on_hover_text("Zoom in (Ctrl++)");
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Menu: Zoom In", "button", &zoom_in, None);
            if zoom_in.clicked() {
                *picked = Some(AppAction::ZoomBy(ZOOM_STEP));
            }
        });
    }
}
