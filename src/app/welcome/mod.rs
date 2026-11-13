//! The welcome page shown when no tab is open.

use std::path::PathBuf;

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::MarkdownApp;
use crate::recent::{
    format_relative_time, now_epoch_secs, RecentEntry, RECENT_FILES_CAP, RECENT_SHOWN,
};

/// What the user picked on the welcome page.
#[derive(Debug, PartialEq, Eq)]
enum WelcomeAction {
    OpenFile,
    OpenFolder,
    OpenRecent(PathBuf),
    ToggleShowAll,
}

/// The recent-files list as one frame draws it.
struct RecentList<'a> {
    /// Each shown entry, with whether its file still exists.
    rows: &'a [(RecentEntry, bool)],
    /// More entries exist than the collapsed list shows.
    has_more: bool,
    show_all: bool,
    /// Epoch seconds the "opened ... ago" labels are relative to.
    now: u64,
}

impl MarkdownApp {
    /// Render the welcome / idle page shown when no document is open (issue #28).
    pub(crate) fn render_welcome(&mut self, ui: &mut egui::Ui) {
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
        let list = RecentList {
            rows: &recents,
            has_more: self.recent_files.len() > RECENT_SHOWN,
            show_all: self.welcome_show_all,
            now: now_epoch_secs(),
        };

        // Deferred actions (run after the borrow on self.recent_files is released).
        match welcome_page(ui, &list) {
            Some(WelcomeAction::OpenFile) => self.apply_action(ui.ctx(), AppAction::OpenFile),
            Some(WelcomeAction::OpenFolder) => self.apply_action(ui.ctx(), AppAction::OpenFolder),
            Some(WelcomeAction::OpenRecent(path)) => self.open_in_new_tab(path),
            Some(WelcomeAction::ToggleShowAll) => self.welcome_show_all = !self.welcome_show_all,
            None => {}
        }
    }
}

/// Draws the whole page. Returns what the user picked this frame.
fn welcome_page(ui: &mut egui::Ui, list: &RecentList<'_>) -> Option<WelcomeAction> {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(64.0);
            welcome_heading(ui);
            ui.add_space(18.0);
            let from_buttons = open_buttons(ui);
            let from_recents = recent_files(ui, list);
            from_buttons.or(from_recents)
        })
        .inner
}

fn welcome_heading(ui: &mut egui::Ui) {
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
}

fn open_buttons(ui: &mut egui::Ui) -> Option<WelcomeAction> {
    let mut action = None;
    // Center the button row manually: a horizontal inside
    // vertical_centered expands to full width, defeating centering.
    ui.horizontal(|ui| {
        let avail = ui.available_width();
        ui.add_space(((avail - 230.0) / 2.0).max(0.0));
        if ui
            .button(egui::RichText::new("Open File…").size(15.0))
            .clicked()
        {
            action = Some(WelcomeAction::OpenFile);
        }
        ui.add_space(10.0);
        if ui
            .button(egui::RichText::new("Open Folder…").size(15.0))
            .clicked()
        {
            action = Some(WelcomeAction::OpenFolder);
        }
    });
    action
}

/// The recent-files list, centered. Draws nothing when the list is empty.
fn recent_files(ui: &mut egui::Ui, list: &RecentList<'_>) -> Option<WelcomeAction> {
    if list.rows.is_empty() {
        return None;
    }
    ui.add_space(32.0);
    // Center a fixed-width, left-aligned column for the recent list.
    let avail = ui.available_width();
    let col_w = 560.0_f32.min((avail - 16.0).max(200.0));
    let margin = ((avail - col_w) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(margin);
        ui.vertical(|ui| {
            ui.set_max_width(col_w);
            recent_column(ui, list)
        })
        .inner
    })
    .inner
}

fn recent_column(ui: &mut egui::Ui, list: &RecentList<'_>) -> Option<WelcomeAction> {
    let mut action = None;
    ui.label(egui::RichText::new("Recent").strong());
    ui.add_space(2.0);
    ui.separator();
    for (entry, exists) in list.rows {
        if recent_row(ui, entry, *exists, list.now) {
            action = Some(WelcomeAction::OpenRecent(entry.path.clone()));
        }
    }
    if list.has_more {
        ui.add_space(6.0);
        let more = if list.show_all {
            "Show less"
        } else {
            "Show more…"
        };
        if ui.link(more).clicked() {
            action = Some(WelcomeAction::ToggleShowAll);
        }
    }
    action
}

/// Draws one recent file. Returns whether the user clicked it; a file that
/// no longer exists is shown disabled.
fn recent_row(ui: &mut egui::Ui, entry: &RecentEntry, exists: bool, now: u64) -> bool {
    let name = entry.path.file_name().unwrap_or(entry.path.as_os_str());
    let name = name.to_string_lossy();
    let dir = entry
        .path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let when = format_relative_time(entry.last_opened, now);
    ui.horizontal(|ui| {
        let label = egui::RichText::new(name).size(14.0);
        let label = if exists { label } else { label.weak() };
        let clicked = ui
            .add_enabled(exists, egui::Button::new(label).frame(false))
            .on_hover_text(entry.path.to_string_lossy())
            .clicked();
        ui.label(
            egui::RichText::new(format!("{dir}   ·   {when}"))
                .weak()
                .small(),
        );
        clicked
    })
    .inner
}
