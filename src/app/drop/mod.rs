//! Files dragged onto the window.

use std::path::PathBuf;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::links::has_markdown_extension;

#[cfg(test)]
mod tests;

impl MarkdownApp {
    /// Track drag hover and open dropped markdown files.
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        // The files are opened after the input lock is released: opening
        // one reads it from disk.
        let (hovering, dropped) = ctx.input(|input| {
            let dropped = input.raw.dropped_files.iter();
            let paths: Vec<PathBuf> = dropped.filter_map(|file| file.path.clone()).collect();
            (!input.raw.hovered_files.is_empty(), paths)
        });
        self.is_dragging = hovering;

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
    }
}
