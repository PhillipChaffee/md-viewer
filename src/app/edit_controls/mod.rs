//! The edit controls of the document in front: the Preview|Edit toggle,
//! the unsaved-changes marker and the outcome of the last save. The tab
//! strip holds them; a lone document has no strip, so the top bar does.

use std::time::Duration;

use eframe::egui;

use crate::app::chrome;
use crate::app::explorer_panel::FLASH_DURATION_MS;
use crate::app::MarkdownApp;
use crate::tab::{SaveOutcome, Tab};
use crate::theme::Palette;

#[cfg(test)]
mod tests;

/// How long a save's outcome stays beside the mode toggle.
const SAVE_FLASH: Duration = Duration::from_millis(FLASH_DURATION_MS * 3);

const UNSAVED_DOT: f32 = 7.0;

/// Whether `tab` has something to show beside the mode toggle even when it
/// is the only tab: an edit in progress, unsaved changes, or a save's
/// outcome.
fn has_edit_state(tab: &Tab) -> bool {
    tab.is_editing || tab.is_dirty() || tab.save_flash.is_some()
}

impl MarkdownApp {
    /// The edit controls of a lone document, which has no strip to hold
    /// them, laid out in the top bar from the right end of `room`. Returns
    /// their left edge when there are any.
    pub(crate) fn lone_document_edit_controls(
        &mut self,
        ui: &mut egui::Ui,
        room: egui::Rect,
    ) -> Option<f32> {
        let palette = self.theme.palette();
        let [tab] = self.tabs.as_mut_slice() else {
            return None;
        };
        if !has_edit_state(tab) {
            return None;
        }
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        let builder = egui::UiBuilder::new().max_rect(room).layout(layout);
        let controls = ui.scope_builder(builder, |ui| edit_mode_controls(ui, tab, palette));
        Some(controls.response.rect.left())
    }
}

/// The Preview|Edit toggle, the unsaved-changes marker, and the outcome of
/// the last save. The caller lays these out right to left.
pub(super) fn edit_mode_controls(ui: &mut egui::Ui, tab: &mut Tab, palette: &Palette) {
    ui.spacing_mut().item_spacing.x = 8.0;
    let modes = [(false, "Preview"), (true, "Edit")];
    if let Some(to_edit) = chrome::segmented(ui, palette, &modes, tab.is_editing, false) {
        tab.set_edit_mode(to_edit);
    }
    if tab.is_dirty() {
        let (dot, response) =
            ui.allocate_exact_size(egui::Vec2::splat(UNSAVED_DOT), egui::Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Unsaved changes")
        });
        ui.painter()
            .circle_filled(dot.center(), UNSAVED_DOT / 2.0, palette.warning);
        response.on_hover_text("Unsaved changes: press Ctrl+S to save");
    }
    save_flash_label(ui, tab, palette);
}

/// Shows the outcome of the last save until it expires, then forgets it.
fn save_flash_label(ui: &mut egui::Ui, tab: &mut Tab, palette: &Palette) {
    let Some((outcome, at)) = tab.save_flash else {
        return;
    };
    // The strip may be on screen only for this outcome, so it has to be
    // drawn again once the outcome has had its time.
    let left = SAVE_FLASH.saturating_sub(at.elapsed());
    ui.ctx().request_repaint_after(left);
    if left.is_zero() {
        tab.save_flash = None;
        return;
    }
    let (label, color) = match outcome {
        SaveOutcome::Saved => ("Saved", palette.positive),
        SaveOutcome::Failed => ("Save failed", palette.warning),
    };
    ui.label(
        egui::RichText::new(label)
            .font(chrome::small_font())
            .color(color),
    );
}
