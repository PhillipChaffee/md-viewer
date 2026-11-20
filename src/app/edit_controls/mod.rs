//! The edit controls of the document in front: the Preview|Edit toggle,
//! the unsaved-changes marker and the outcome of the last save. They
//! stand in the top row, before the buttons at its right end.

use std::time::Duration;

use eframe::egui;

use crate::app::action::AppAction;
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

impl MarkdownApp {
    /// The edit controls of the document in front, laid out from the
    /// right in the part of the bar `part` is. Returns their left edge, or
    /// `None` with no document in front.
    pub(crate) fn edit_controls(
        &mut self,
        ui: &mut egui::Ui,
        part: egui::UiBuilder,
    ) -> Option<f32> {
        let palette = self.theme.palette();
        let save_hint = self.hints.of(AppAction::Save);
        let front = self.front()?;
        let tab = self.tabs.get_mut(front)?;
        let controls = ui.scope_builder(part, |ui| {
            edit_mode_controls(ui, tab, palette, save_hint);
        });
        Some(controls.response.rect.left())
    }
}

/// The Preview|Edit toggle, the unsaved-changes marker, and the outcome of
/// the last save. The caller lays these out right to left. `save_hint` is
/// the shortcut that saves, as the marker's tooltip names it.
fn edit_mode_controls(ui: &mut egui::Ui, tab: &mut Tab, palette: &Palette, save_hint: &str) {
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
        response.on_hover_ui(|ui| {
            ui.set_max_width(ui.spacing().tooltip_width);
            ui.label(format!("Unsaved changes: press {save_hint} to save"));
        });
    }
    save_flash_label(ui, tab, palette);
}

/// Shows the outcome of the last save until it expires, then forgets it.
fn save_flash_label(ui: &mut egui::Ui, tab: &mut Tab, palette: &Palette) {
    let Some((outcome, at)) = tab.save_flash else {
        return;
    };
    // The outcome leaves the row once it has had its time, which takes a
    // frame nothing else may ask for.
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
