//! The appearance popover under the top bar's "Aa": the theme, the text
//! size, the width of the text column, the line spacing, and the way to
//! the font dialog. Every control takes effect as it is used.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, MenuRow, QuietButton, BUTTON_SIDE};
use crate::app::MarkdownApp;
use crate::layout::{LineSpacing, ReadingWidth};
use crate::theme::{Palette, ThemeId};

#[cfg(test)]
mod tests;

const POPOVER_WIDTH: f32 = 280.0;
/// Space between the popover's edge and its sections.
const POPOVER_PADDING: i8 = 8;
/// Space above and below the rule between two sections.
const SECTION_GAP: f32 = 6.0;
const SWATCH_DIAMETER: f32 = 40.0;
/// Height of a swatch with its name under it.
const SWATCH_CELL_HEIGHT: f32 = 70.0;
/// Width of the text size's name between its two buttons.
const SIZE_NAME_WIDTH: f32 = 84.0;

/// The widest the slider goes in a document pane `pane_width` px wide: the
/// column that fits the window.
fn widest_column(pane_width: f32) -> f32 {
    ReadingWidth::FIT_WINDOW
        .resolve(pane_width)
        .max(ReadingWidth::MIN + ReadingWidth::STEP)
}

/// One theme to pick: a disc of its paper with "Aa" in its ink, ringed
/// while it is the theme in use, and its name underneath.
fn theme_swatch(
    ui: &mut egui::Ui,
    theme: ThemeId,
    chosen: bool,
    width: f32,
    chrome_palette: &Palette,
) -> egui::Response {
    let response =
        ui.allocate_response(egui::vec2(width, SWATCH_CELL_HEIGHT), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, theme.label())
    });
    let shown = theme.palette();
    let radius = SWATCH_DIAMETER / 2.0;
    let center = egui::pos2(response.rect.center().x, response.rect.top() + radius + 5.0);
    let pointed = response.hovered() || response.has_focus();
    let painter = ui.painter();
    painter.circle_filled(center, radius, shown.page);
    let edge = if pointed {
        chrome_palette.muted
    } else {
        chrome_palette.rule
    };
    painter.circle_stroke(center, radius, egui::Stroke::new(1.0_f32, edge));
    if chosen {
        painter.circle_stroke(
            center,
            radius + 3.0,
            egui::Stroke::new(2.0_f32, chrome_palette.accent),
        );
    }
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        "Aa",
        egui::FontId::proportional(15.0),
        shown.text,
    );
    let name_color = if chosen || pointed {
        chrome_palette.text
    } else {
        chrome_palette.muted
    };
    painter.text(
        egui::pos2(center.x, response.rect.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        theme.label(),
        chrome::small_font(),
        name_color,
    );
    response
}

/// The rule between two sections of the popover, with air around it.
fn section_break(ui: &mut egui::Ui, palette: &Palette) {
    ui.add_space(SECTION_GAP);
    chrome::hairline(ui, palette);
    ui.add_space(SECTION_GAP);
}

impl MarkdownApp {
    /// Show the appearance popover under `button` while it is open; a
    /// click on the button opens and closes it, and so does Enter while
    /// the button has the keyboard focus. A click elsewhere or Escape
    /// closes it.
    pub(crate) fn appearance_popover(
        &self,
        button: &egui::Response,
        picked: &mut Option<AppAction>,
    ) {
        let palette = self.theme.palette();
        egui::Popup::from_toggle_button_response(button)
            .align(egui::RectAlign::BOTTOM_END)
            .gap(4.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .sense(chrome::POPUP_SENSE)
            .style(chrome::popup_style(palette))
            .show(|ui| {
                ui.set_width(POPOVER_WIDTH);
                egui::Frame::NONE
                    .inner_margin(egui::Margin::same(POPOVER_PADDING))
                    .show(ui, |ui| {
                        self.theme_swatches(ui, picked);
                        section_break(ui, palette);
                        self.text_size_row(ui, picked);
                        section_break(ui, palette);
                        self.width_section(ui, picked);
                        section_break(ui, palette);
                        self.line_spacing_section(ui, picked);
                    });
                chrome::hairline(ui, palette);
                let fonts = MenuRow::new("Fonts…").flush().show(ui, palette);
                #[cfg(feature = "mcp")]
                self.mcp_bridge
                    .register_widget("Appearance: Fonts", "button", &fonts, None);
                if fonts.clicked() {
                    *picked = Some(AppAction::OpenFontSettings);
                    ui.close();
                }
            });
    }

    fn theme_swatches(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            #[expect(
                clippy::cast_precision_loss,
                reason = "there are four themes, far inside f32's exact integers"
            )]
            let cell = ui.available_width() / ThemeId::ALL.len() as f32;
            for theme in ThemeId::ALL {
                let chosen = self.theme == theme;
                let swatch = theme_swatch(ui, theme, chosen, cell, palette);
                #[cfg(feature = "mcp")]
                self.mcp_bridge.register_widget(
                    &format!("Appearance: Theme: {}", theme.label()),
                    "button",
                    &swatch,
                    Some(if chosen { "selected" } else { "unselected" }),
                );
                if swatch.clicked() {
                    *picked = Some(AppAction::SetTheme(theme));
                }
            }
        });
    }

    /// A smaller and a larger "A" either side of the name of the size in
    /// use. Each steps one size class and goes dim at the end of the scale.
    fn text_size_row(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        let size = self.text_size_class;
        chrome::control_row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            chrome::small_label(ui, palette, "Text size");
            let controls = 2.0_f32.mul_add(BUTTON_SIDE, SIZE_NAME_WIDTH);
            ui.add_space((ui.available_width() - controls).max(0.0));

            let smaller = QuietButton::text_sized("A", 12.0)
                .labelled("Smaller text")
                .enabled(size.smaller().is_some())
                .show(ui, palette);
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Appearance: Smaller Text", "button", &smaller, None);
            if let Some(size) = size.smaller().filter(|_| smaller.clicked()) {
                *picked = Some(AppAction::SetTextSize(size));
            }

            ui.allocate_ui_with_layout(
                egui::vec2(SIZE_NAME_WIDTH, BUTTON_SIDE),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| chrome::small_label(ui, palette, size.label()),
            );

            let larger = QuietButton::text_sized("A", 19.0)
                .labelled("Larger text")
                .enabled(size.larger().is_some())
                .show(ui, palette);
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Appearance: Larger Text", "button", &larger, None);
            if let Some(size) = size.larger().filter(|_| larger.clicked()) {
                *picked = Some(AppAction::SetTextSize(size));
            }
        });
    }

    /// The width of the text column: a readout, a slider from the
    /// narrowest column to the one that fits the window, and the two
    /// widths worth a button of their own.
    fn width_section(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        let shown = self.reading_width.resolve(self.document_pane_width);
        let readout = if self.reading_width.fits_window() {
            "Fits the window".to_owned()
        } else {
            format!("{shown:.0} px")
        };
        ui.horizontal(|ui| {
            chrome::small_label(ui, palette, "Width");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                chrome::small_label(ui, palette, &readout);
            });
        });

        ui.add_space(2.0);
        ui.spacing_mut().slider_width = ui.available_width();
        ui.visuals_mut().slider_trailing_fill = true;
        ui.visuals_mut().selection.bg_fill = palette.muted;
        let mut width = shown;
        let range = ReadingWidth::MIN..=widest_column(self.document_pane_width);
        let slider = ui.add(
            egui::Slider::new(&mut width, range)
                .show_value(false)
                .step_by(1.0),
        );
        slider
            .widget_info(|| egui::WidgetInfo::slider(true, f64::from(width), "Text column width"));
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Appearance: Width", "slider", &slider, Some(&readout));
        if slider.changed() {
            *picked = Some(AppAction::SetReadingWidth(width));
        }

        ui.horizontal(|ui| {
            let fit = QuietButton::text("Fit window").show(ui, palette);
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Appearance: Fit Window", "button", &fit, None);
            if fit.clicked() {
                *picked = Some(AppAction::SetReadingWidth(ReadingWidth::MAX));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let reset = QuietButton::text("Reset")
                    .labelled("Reset width")
                    .show(ui, palette);
                #[cfg(feature = "mcp")]
                self.mcp_bridge
                    .register_widget("Appearance: Reset Width", "button", &reset, None);
                if reset.clicked() {
                    *picked = Some(AppAction::SetReadingWidth(ReadingWidth::DEFAULT.get()));
                }
            });
        });
    }

    fn line_spacing_section(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        chrome::small_label(ui, palette, "Line spacing");
        ui.add_space(6.0);
        let options = LineSpacing::ALL.map(|spacing| (spacing, spacing.label()));
        if let Some(spacing) = chrome::segmented(ui, palette, &options, self.line_spacing, true) {
            *picked = Some(AppAction::SetLineSpacing(spacing));
        }
    }
}
