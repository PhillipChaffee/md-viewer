//! The welcome page shown when no tab is open.

use std::path::PathBuf;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::recent::{
    format_relative_time, now_epoch_secs, RecentEntry, RECENT_FILES_CAP, RECENT_SHOWN,
};

impl MarkdownApp {
    /// Render the active tab's content
    /// Render the welcome / idle page shown when no document is open (issue #28).
    pub(crate) fn render_welcome(&mut self, ui: &mut egui::Ui) {
        let mut do_open_file = false;
        let mut do_open_folder = false;
        let mut open_path: Option<PathBuf> = None;
        let mut toggle_show_all = false;
        let now = now_epoch_secs();

        // Snapshot the entries to display so we don't borrow self while acting on clicks.
        let shown = if self.welcome_show_all {
            RECENT_FILES_CAP
        } else {
            RECENT_SHOWN
        };
        let recents: Vec<(RecentEntry, bool)> = self
            .recent_files
            .iter()
            .take(shown)
            .map(|e| (e.clone(), e.path.exists()))
            .collect();
        let total_recent = self.recent_files.len();
        let show_all = self.welcome_show_all;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(64.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Markdown").size(56.0).weak());
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Open a file or folder")
                            .size(22.0)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new("No document is open.").weak());
                });
                ui.add_space(18.0);
                // Center the button row manually: a horizontal inside
                // vertical_centered expands to full width, defeating centering.
                ui.horizontal(|ui| {
                    let avail = ui.available_width();
                    ui.add_space(((avail - 230.0) / 2.0).max(0.0));
                    if ui
                        .button(egui::RichText::new("Open File…").size(15.0))
                        .clicked()
                    {
                        do_open_file = true;
                    }
                    ui.add_space(10.0);
                    if ui
                        .button(egui::RichText::new("Open Folder…").size(15.0))
                        .clicked()
                    {
                        do_open_folder = true;
                    }
                });

                if !recents.is_empty() {
                    ui.add_space(32.0);
                    // Center a fixed-width, left-aligned column for the recent list.
                    let avail = ui.available_width();
                    let col_w = 560.0_f32.min((avail - 16.0).max(200.0));
                    let margin = ((avail - col_w) / 2.0).max(0.0);
                    ui.horizontal(|ui| {
                        ui.add_space(margin);
                        ui.vertical(|ui| {
                            ui.set_max_width(col_w);
                            ui.label(egui::RichText::new("Recent").strong());
                            ui.add_space(2.0);
                            ui.separator();
                            for (entry, exists) in &recents {
                                let name = entry
                                    .path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| entry.path.to_string_lossy().to_string());
                                let dir = entry
                                    .path
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let when = format_relative_time(entry.last_opened, now);
                                ui.horizontal(|ui| {
                                    let label = egui::RichText::new(name.as_str()).size(14.0);
                                    let label = if *exists { label } else { label.weak() };
                                    let resp = ui
                                        .add_enabled(*exists, egui::Button::new(label).frame(false))
                                        .on_hover_text(entry.path.to_string_lossy());
                                    if resp.clicked() {
                                        open_path = Some(entry.path.clone());
                                    }
                                    ui.label(
                                        egui::RichText::new(format!("{dir}   ·   {when}"))
                                            .weak()
                                            .small(),
                                    );
                                });
                            }
                            if total_recent > RECENT_SHOWN {
                                ui.add_space(6.0);
                                let more = if show_all {
                                    "Show less"
                                } else {
                                    "Show more…"
                                };
                                if ui.link(more).clicked() {
                                    toggle_show_all = true;
                                }
                            }
                        });
                    });
                }
            });

        // Deferred actions (run after the borrow on self.recent_files is released).
        if do_open_file {
            self.open_file_dialog();
        }
        if do_open_folder {
            self.open_folder_dialog();
        }
        if let Some(p) = open_path {
            self.open_in_new_tab(p);
        }
        if toggle_show_all {
            self.welcome_show_all = !self.welcome_show_all;
        }
    }
}
