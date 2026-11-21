//! The error bar: the last thing that went wrong, under the top bar.

use eframe::egui;

use crate::app::chrome::{self, QuietButton};
use crate::app::MarkdownApp;
use crate::icons::Icon;

#[cfg(test)]
mod tests;

impl MarkdownApp {
    /// The last thing that went wrong, in the warning color, until it is
    /// dismissed.
    pub(crate) fn render_error_bar(&mut self, ctx: &egui::Context) {
        let Some(error) = &self.error_message else {
            return;
        };
        let palette = self.theme.palette();
        let mut dismissed = false;
        let message = (error.as_str(), palette.warning);
        chrome::warning_bar(ctx, "error_bar", palette, message, |ui| {
            let dismiss = QuietButton::icon(Icon::Close, "Dismiss")
                .show(ui, palette)
                .on_hover_text("Dismiss");
            dismissed = dismiss.clicked();
        });
        if dismissed {
            self.error_message = None;
        }
    }
}
