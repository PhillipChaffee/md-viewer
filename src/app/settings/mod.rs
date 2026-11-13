//! The color and font settings dialogs.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui;

use crate::app::mcp;
use crate::app::MarkdownApp;
use crate::system_fonts::{scan_pickable_font_families, FontPreset, TextSizeClass};

#[cfg(test)]
mod tests;

/// The font dialog's own state: whether it shows, and the families it
/// offers. None of it is saved.
#[derive(Default)]
pub(super) struct FontPicker {
    pub(super) open: bool,
    // Body-font families the picker offers, scanned in a background thread:
    // one canonical name per family, only families that can actually serve
    // as the body font. Empty until the scan lands (the dialog shows a
    // scanning note; "System Default" always works).
    families: Vec<String>,
    // Lowercased copy of `families`, precomputed once so the search filter
    // doesn't re-lowercase every name on every frame it's open: never
    // allocate in the render loop what can be cached.
    families_lower: Vec<String>,
    // In-flight background scan producing `families`.
    pub(super) scan: Option<Receiver<Vec<String>>>,
    // The search text.
    filter: String,
}

impl FontPicker {
    /// A closed picker whose family list is being scanned off the UI
    /// thread: probing every family's Latin coverage takes ~0.5s on a stock
    /// Arch system (see `scan_pickable_font_families`). Until the result
    /// lands, the font dialog shows "System Default" plus a scanning note.
    /// Where the thread could not be started, the list stays empty.
    pub(super) fn scanning() -> Self {
        let (font_family_tx, font_family_rx) = mpsc::channel();
        let scan = std::thread::Builder::new()
            .name("font-family-scan".to_owned())
            .spawn(move || {
                let _ = font_family_tx.send(scan_pickable_font_families());
            })
            .is_ok()
            .then_some(font_family_rx);
        Self {
            scan,
            ..Self::default()
        }
    }

    /// Collect the background font-family scan (async phase: poll the
    /// channel, never block the UI thread). Fills the family list once,
    /// then drops the channel.
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        let Some(scan) = &self.scan else {
            return;
        };
        match scan.try_recv() {
            Ok(names) => {
                self.families_lower = names.iter().map(|name| name.to_ascii_lowercase()).collect();
                self.families = names;
                self.scan = None;
                // The list may have landed while the font dialog was
                // already open; repaint so it appears without input.
                ctx.request_repaint();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                // Sender dropped without sending (thread spawn raced or
                // panicked) — stop polling, keep whatever list we have.
                self.scan = None;
            }
        }
    }
}

/// A row clicked in the font dialog's family list.
enum FamilyChoice {
    /// The pinned first row: the auto-detected system sans-serif.
    SystemDefault,
    Family(String),
}

impl MarkdownApp {
    pub(crate) fn render_color_settings(&mut self, ctx: &egui::Context) {
        if !self.show_color_settings_dialog {
            return;
        }

        let mut open = true;
        // Set inside the window closure, applied after `.show()` returns —
        // same one-shot-flag pattern the font/tab-close deferred actions use
        // elsewhere in this file. Each row is independently resettable, so
        // each gets its own flag.
        let mut new_highlight_color: Option<Option<egui::Color32>> = None;
        let mut new_link_color: Option<Option<egui::Color32>> = None;

        egui::Window::new("Colors")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Highlight Color:");
                    // Read the currently *effective* color (override or
                    // theme default — update() already applied Visuals for
                    // this frame) rather than re-deriving egui's own
                    // dark/light selection defaults by hand.
                    let mut color = ui.visuals().selection.bg_fill;
                    let color_btn = ui.color_edit_button_srgba(&mut color);
                    mcp::widget(
                        &self.mcp,
                        format_args!("Colors Dialog: Highlight Color"),
                        "color",
                        &color_btn,
                        None,
                    );
                    if color_btn.changed() {
                        new_highlight_color = Some(Some(color));
                    }

                    let reset_btn =
                        ui.add_enabled(self.colors.highlight.is_some(), egui::Button::new("Reset"));
                    mcp::widget(
                        &self.mcp,
                        format_args!("Colors Dialog: Highlight Color Reset"),
                        "button",
                        &reset_btn,
                        None,
                    );
                    if reset_btn.clicked() {
                        new_highlight_color = Some(None);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Link Color:");
                    let mut color = ui.visuals().hyperlink_color;
                    let color_btn = ui.color_edit_button_srgba(&mut color);
                    mcp::widget(
                        &self.mcp,
                        format_args!("Colors Dialog: Link Color"),
                        "color",
                        &color_btn,
                        None,
                    );
                    if color_btn.changed() {
                        new_link_color = Some(Some(color));
                    }

                    let reset_btn =
                        ui.add_enabled(self.colors.link.is_some(), egui::Button::new("Reset"));
                    mcp::widget(
                        &self.mcp,
                        format_args!("Colors Dialog: Link Color Reset"),
                        "button",
                        &reset_btn,
                        None,
                    );
                    if reset_btn.clicked() {
                        new_link_color = Some(None);
                    }
                });
            });

        self.show_color_settings_dialog = open;
        if let Some(color) = new_highlight_color {
            self.colors.highlight = color;
        }
        if let Some(color) = new_link_color {
            self.colors.link = color;
        }
    }

    pub(crate) fn render_font_settings(&mut self, ctx: &egui::Context) {
        if !self.font_picker.open {
            return;
        }

        let mut open = true;
        // Set inside the window closure, applied after `.show()` returns —
        // the list rows borrow the picker's families while the closure
        // runs, so a picked name can't mutate `self` in place.
        let mut picked_family: Option<FamilyChoice> = None;

        // Single font dialog: preset, text size, and family picker in one
        // place. Handlers only mutate state — the `applied_fonts`
        // diff gate in update() rebuilds the font chains once per change.
        egui::Window::new("Fonts")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                self.font_preset_picker(ui);
                self.text_size_picker(ui);

                ui.add_space(4.0);
                ui.separator();
                ui.label("Family");
                self.font_family_search(ui);
                picked_family = self.font_family_list(ui);
            });

        self.font_picker.open = open;
        match picked_family {
            Some(FamilyChoice::SystemDefault) => self.fonts.family = None,
            Some(FamilyChoice::Family(name)) => self.fonts.family = Some(name),
            None => {}
        }
    }

    /// Markdown font presets. Missing preset faces degrade to the app's own
    /// fallback chain.
    fn font_preset_picker(&mut self, ui: &mut egui::Ui) {
        ui.label("Preset");
        ui.horizontal_wrapped(|ui| {
            for preset in FontPreset::ALL {
                let active = self.fonts.preset == preset;
                let preset_btn = ui
                    .selectable_label(active, preset.label())
                    .on_hover_text(preset.description());
                mcp::widget(
                    &self.mcp,
                    format_args!("Font Dialog: Preset: {}", preset.label()),
                    "button",
                    &preset_btn,
                    Some(if active { "selected" } else { "" }),
                );
                if preset_btn.clicked() && !active {
                    self.fonts.preset = preset;
                }
            }
        });
        if self.fonts.family.is_some() {
            ui.small(
                "A picked family leads the body chain; presets still \
                 set the code font and line heights.",
            );
        }
    }

    /// Shared text-size classes: one scale for every preset.
    fn text_size_picker(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.label("Text Size");
        ui.horizontal_wrapped(|ui| {
            for size in TextSizeClass::ALL {
                let active = self.fonts.size == size;
                let size_btn = ui
                    .selectable_label(active, size.label())
                    .on_hover_text("Base text size; headings and code scale with it");
                mcp::widget(
                    &self.mcp,
                    format_args!("Font Dialog: Size: {}", size.label()),
                    "button",
                    &size_btn,
                    Some(if active { "selected" } else { "" }),
                );
                if size_btn.clicked() && !active {
                    self.fonts.size = size;
                }
            }
        });
    }

    fn font_family_search(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            let filter_edit = ui.text_edit_singleline(&mut self.font_picker.filter);
            mcp::widget(
                &self.mcp,
                format_args!("Font Dialog: Search"),
                "textbox",
                &filter_edit,
                None,
            );
        });
    }

    /// Positions in the picker's families that match the search text, or
    /// `None` when there is no search text and every family is listed.
    ///
    /// Only the empty-filter (common) case avoids allocating: it reuses
    /// the family list directly. A non-empty filter needs one
    /// `Vec<usize>` of matching indices, built from the precomputed lowercase
    /// names (no per-name allocation).
    fn filtered_font_indices(&self) -> Option<Vec<usize>> {
        let filter = self.font_picker.filter.to_ascii_lowercase();
        if filter.is_empty() {
            return None;
        }
        Some(
            self.font_picker
                .families_lower
                .iter()
                .enumerate()
                .filter(|(_, lower)| lower.contains(&filter))
                .map(|(i, _)| i)
                .collect(),
        )
    }

    /// The scrolling family list. Returns the row clicked this frame.
    fn font_family_list(&self, ui: &mut egui::Ui) -> Option<FamilyChoice> {
        // One canonical name per body-capable family, scanned once in
        // the background at startup. While the scan is running, only
        // "System Default" is offered.
        if self.font_picker.families.is_empty() && self.font_picker.scan.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Scanning installed fonts…");
            });
        }

        let filtered_indices = self.filtered_font_indices();
        let match_count = filtered_indices
            .as_ref()
            .map_or(self.font_picker.families.len(), Vec::len);
        let mut clicked = None;

        let row_height = ui.spacing().interact_size.y;
        // auto_shrink_x = false: egui's default (TRUE) shrinks the
        // area to its widest row, which paints the floating scrollbar
        // mid-dialog instead of at the window's right edge.
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(300.0)
            .show_rows(
                ui,
                row_height,
                match_count + 1, // + 1 for the pinned "System Default" row
                |ui, rows| {
                    for row in rows {
                        let Some(family_row) = row.checked_sub(1) else {
                            if self.system_default_row(ui) {
                                clicked = Some(FamilyChoice::SystemDefault);
                            }
                            continue;
                        };
                        let font_index = filtered_indices
                            .as_ref()
                            .map_or(family_row, |indices| indices[family_row]);
                        let name = &self.font_picker.families[font_index];
                        let is_selected = self.fonts.family.as_deref() == Some(name.as_str());
                        if ui.selectable_label(is_selected, name).clicked() {
                            clicked = Some(FamilyChoice::Family(name.clone()));
                        }
                    }
                },
            );
        clicked
    }

    /// The pinned first row of the family list. Returns whether it was clicked.
    fn system_default_row(&self, ui: &mut egui::Ui) -> bool {
        let default_selected = self.fonts.family.is_none();
        let default_btn = ui.selectable_label(default_selected, "System Default");
        mcp::widget(
            &self.mcp,
            format_args!("Font Dialog: System Default"),
            "button",
            &default_btn,
            Some(if default_selected { "selected" } else { "" }),
        );
        default_btn.clicked()
    }
}
