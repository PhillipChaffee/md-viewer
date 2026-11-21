//! The notice on a document whose file another program changed while the
//! document had unsaved edits: what happened, and Reload or Keep mine.
//!
//! It is a bar under the top bar, like the error bar, and shows while the
//! tab it belongs to is in front. A document without unsaved edits never
//! has one: it reloads.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, QuietButton};
use crate::app::MarkdownApp;
use crate::tab::DiskChange;

/// What the notice says of `change`.
pub(super) const fn message(change: DiskChange) -> &'static str {
    match change {
        DiskChange::Changed => "This file changed on disk. Your edits are not saved.",
        DiskChange::Gone => "This file was deleted on disk. Saving writes it again.",
    }
}

/// What a tab that waits behind another with `change` is marked as.
pub(super) const fn mark(change: DiskChange) -> &'static str {
    match change {
        DiskChange::Changed => "Changed on disk",
        DiskChange::Gone => "Deleted on disk",
    }
}

impl MarkdownApp {
    /// The notice of the document in front, while it has one. Returns the
    /// answer the reader gave this frame.
    pub(crate) fn render_disk_notice(&self, ctx: &egui::Context) -> Option<AppAction> {
        let change = self.front_tab()?.disk_notice()?;
        let palette = self.theme.palette();
        let mut answer = None;
        let (message, ink) = (message(change), palette.text);
        chrome::warning_bar(ctx, "disk_notice", palette, message, ink, |ui| {
            if QuietButton::text("Keep mine").show(ui, palette).clicked() {
                answer = Some(AppAction::KeepMine);
            }
            // There is nothing to reload of a file that is gone.
            if change == DiskChange::Changed
                && QuietButton::text("Reload").show(ui, palette).clicked()
            {
                answer = Some(AppAction::ReloadFromDisk);
            }
        });
        answer
    }
}
