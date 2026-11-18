//! Files dragged onto the window: the hint while they hover, and the
//! opening of the ones dropped.

use std::path::PathBuf;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::links::has_markdown_extension;
use crate::theme;

#[cfg(test)]
mod tests;

impl MarkdownApp {
    /// Open the markdown files dropped on the window. Returns whether
    /// files hover over it this frame.
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) -> bool {
        // The files are opened after the input lock is released: opening
        // one reads it from disk.
        let (hovering, dropped) = ctx.input(|input| {
            let dropped = input.raw.dropped_files.iter();
            let paths: Vec<PathBuf> = dropped.filter_map(|file| file.path.clone()).collect();
            (!input.raw.hovered_files.is_empty(), paths)
        });
        for path in dropped {
            if has_markdown_extension(&path) {
                self.open_in_new_tab(path);
            } else {
                self.error_message = Some(
                    "Unsupported file type. Please drop a markdown file (.md, .markdown, .txt)"
                        .to_string(),
                );
            }
        }
        hovering
    }
}

/// Dim the window and say what may be dropped on it.
pub(crate) fn paint_hint(ctx: &egui::Context) {
    let screen_rect = ctx.available_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("drop_overlay"),
    ));

    painter.rect_filled(screen_rect, 0.0, theme::SCRIM);

    painter.text(
        screen_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Drop markdown file here",
        egui::FontId::proportional(24.0),
        theme::ON_SCRIM,
    );
}
