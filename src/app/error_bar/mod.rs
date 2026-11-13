//! The error bar: the last thing that went wrong, under the top bar.

use eframe::egui;

use crate::app::chrome::{self, QuietButton, STRIP_HEIGHT};
use crate::app::MarkdownApp;
use crate::icons::Icon;

#[cfg(test)]
mod tests;

/// Side of the warning sign the error bar opens with.
const WARNING_SIDE: f32 = 16.0;

impl MarkdownApp {
    /// The last thing that went wrong, in the warning color, until it is
    /// dismissed.
    pub(crate) fn render_error_bar(&mut self, ctx: &egui::Context) {
        let Some(error) = &self.error_message else {
            return;
        };
        let palette = self.theme.palette();
        let mut dismissed = false;
        egui::TopBottomPanel::top("error_bar")
            .exact_height(STRIP_HEIGHT)
            .frame(chrome::bar_frame(palette))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.add_space(6.0);
                    let (sign, _) = ui
                        .allocate_exact_size(egui::Vec2::splat(WARNING_SIDE), egui::Sense::hover());
                    Icon::Warning.paint(ui.painter(), sign, palette.warning);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let dismiss = QuietButton::icon(Icon::Close, "Dismiss")
                            .show(ui, palette)
                            .on_hover_text("Dismiss");
                        dismissed = dismiss.clicked();
                        // The message takes what the button leaves, and is
                        // cut short rather than pushing the button out.
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(error)
                                        .font(chrome::label_font())
                                        .color(palette.warning),
                                )
                                .truncate(),
                            );
                        });
                    });
                });
            });
        if dismissed {
            self.error_message = None;
        }
    }
}
