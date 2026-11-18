//! The find bar. What it searches, and when, is in `session`.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, QuietButton, STRIP_HEIGHT};
use crate::app::MarkdownApp;
use crate::icons::Icon;

mod session;
#[cfg(test)]
mod tests;

/// Width of the query field.
const QUERY_WIDTH: f32 = 260.0;
/// Side of the magnifier the bar opens with.
const GLYPH_SIDE: f32 = 14.0;

impl MarkdownApp {
    /// Render the find bar while it is open. Returns what its buttons ask
    /// for, in the order to apply it once the panel is done.
    pub(crate) fn render_search_bar(&mut self, ctx: &egui::Context) -> Vec<AppAction> {
        let mut actions = Vec::new();
        if !self.search.is_open {
            return actions;
        }

        let palette = self.theme.palette();
        egui::TopBottomPanel::top("search_bar")
            .exact_height(STRIP_HEIGHT)
            .frame(chrome::bar_frame(palette))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.add_space(6.0);
                    let (glyph, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(GLYPH_SIDE), egui::Sense::hover());
                    Icon::Search.paint(ui.painter(), glyph, palette.muted);
                    self.search_query_input(ui);
                    self.search_match_controls(ui, &mut actions);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_btn = QuietButton::icon(Icon::Close, "Close find bar")
                            .show(ui, palette)
                            .on_hover_text("Close (Esc)");
                        if close_btn.clicked() {
                            // A closed bar has no match to step to.
                            actions = vec![AppAction::CloseSearch];
                        }
                    });
                });
            });

        actions
    }

    /// The query text field, focused on the frame after the bar opens. It
    /// has no box of its own: the bar is the field.
    fn search_query_input(&mut self, ui: &mut egui::Ui) {
        let text_edit = egui::TextEdit::singleline(&mut self.search.query)
            .id(egui::Id::new("search_input"))
            .hint_text("Find in document")
            .font(chrome::label_font())
            .frame(false)
            .desired_width(QUERY_WIDTH);
        let response = ui.add(text_edit);

        if self.search.focus_requested {
            response.request_focus();
            self.search.focus_requested = false;
        }
    }

    /// The "2 / 7" match counter and the previous and next buttons. A step
    /// forward is applied before a step back.
    fn search_match_controls(&self, ui: &mut egui::Ui, actions: &mut Vec<AppAction>) {
        let palette = self.theme.palette();
        let total_matches = self.active_tab_match_count();
        let label_text = if self.search.query.is_empty() {
            String::new()
        } else if total_matches == 0 {
            "0 matches".to_string()
        } else {
            format!("{} / {}", self.search.active_match_index + 1, total_matches)
        };
        chrome::small_label(ui, palette, &label_text);

        let prev_btn = QuietButton::icon(Icon::ChevronUp, "Previous match")
            .enabled(total_matches > 0)
            .show(ui, palette)
            .on_hover_text("Previous match (Shift+Enter / Up)");

        let next_btn = QuietButton::icon(Icon::ChevronDown, "Next match")
            .enabled(total_matches > 0)
            .show(ui, palette)
            .on_hover_text("Next match (Enter / Down)");
        actions.extend(next_btn.clicked().then_some(AppAction::NextMatch));
        actions.extend(prev_btn.clicked().then_some(AppAction::PreviousMatch));
    }
}
