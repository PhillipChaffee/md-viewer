//! The find bar and match navigation.

use eframe::egui;

use crate::app::MarkdownApp;
use crate::search::SearchBarOutcome;

/// How far down the document `line` sits, as a fraction of `total_lines`.
#[expect(
    clippy::cast_precision_loss,
    reason = "a scroll estimate: line counts past f32's 24-bit mantissa lose a rounding error, not a position"
)]
fn line_ratio(line: usize, total_lines: usize) -> f32 {
    line as f32 / total_lines as f32
}

impl MarkdownApp {
    /// Render the find bar while it is open. The returned clicks are applied
    /// by the caller once the panel is done.
    pub(crate) fn render_search_bar(&mut self, ctx: &egui::Context) -> SearchBarOutcome {
        let mut outcome = SearchBarOutcome::default();
        if !self.search.is_open {
            return outcome;
        }

        egui::TopBottomPanel::top("search_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.search_query_input(ui);
                ui.add_space(8.0);
                self.search_match_controls(ui, &mut outcome);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_btn = ui.small_button("✕").on_hover_text("Close (Esc)");
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge
                        .register_widget("Search: Close", "button", &close_btn, None);
                    if close_btn.clicked() {
                        outcome.close_requested = true;
                    }
                });
            });
        });

        outcome
    }

    /// The query text field, focused on the frame after the bar opens.
    fn search_query_input(&mut self, ui: &mut egui::Ui) {
        let text_edit = egui::TextEdit::singleline(&mut self.search.query)
            .id(egui::Id::new("search_input"))
            .hint_text("Find in document")
            .desired_width(280.0);
        let response = ui.add(text_edit);

        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Search: Input", "textbox", &response, None);

        if self.search.focus_requested {
            response.request_focus();
            self.search.focus_requested = false;
        }
    }

    /// The "2 / 7" match counter and the previous and next buttons.
    fn search_match_controls(&self, ui: &mut egui::Ui, outcome: &mut SearchBarOutcome) {
        let total_matches = self.active_tab_match_count();
        let label_text = if self.search.query.is_empty() {
            String::new()
        } else if total_matches == 0 {
            "0 matches".to_string()
        } else {
            format!("{} / {}", self.search.active_match_index + 1, total_matches)
        };
        let label_response = ui.label(
            egui::RichText::new(&label_text)
                .color(ui.style().visuals.weak_text_color())
                .small(),
        );

        #[cfg(feature = "mcp")]
        self.mcp_bridge.register_widget(
            "Search: Match Count",
            "label",
            &label_response,
            Some(&label_text),
        );
        #[cfg(not(feature = "mcp"))]
        let _ = label_response;

        ui.add_space(4.0);
        let prev_btn = ui
            .add_enabled(total_matches > 0, egui::Button::new("↑").small())
            .on_hover_text("Previous match (Shift+Enter / ↑)");
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Search: Previous", "button", &prev_btn, None);
        if prev_btn.clicked() {
            outcome.prev_clicked = true;
        }

        let next_btn = ui
            .add_enabled(total_matches > 0, egui::Button::new("↓").small())
            .on_hover_text("Next match (Enter / ↓)");
        #[cfg(feature = "mcp")]
        self.mcp_bridge
            .register_widget("Search: Next", "button", &next_btn, None);
        if next_btn.clicked() {
            outcome.next_clicked = true;
        }
    }

    fn active_tab_match_count(&self) -> usize {
        self.tabs
            .get(self.active_tab)
            .map_or(0, |tab| tab.search_matches.len())
    }

    /// Show the find bar, or refocus its input when it is already open.
    pub(crate) const fn open_search(&mut self) {
        self.search.is_open = true;
        self.search.focus_requested = true;
    }

    /// Move the active match index forward (dir > 0) or backward (dir < 0), wrapping
    /// around, and request a scroll-to-line via `pending_scroll_offset`.
    pub(crate) fn jump_match(&mut self, dir: i32) {
        let n = self.active_tab_match_count();
        if n == 0 {
            return;
        }
        let new_idx = if dir >= 0 {
            (self.search.active_match_index + 1) % n
        } else if self.search.active_match_index == 0 {
            n - 1
        } else {
            self.search.active_match_index - 1
        };
        self.search.active_match_index = new_idx;
        self.scroll_to_active_match();
    }

    /// Request a scroll-to-line for the current `active_match_index`. No-op when
    /// there's no active tab, no matches, or content height hasn't been measured yet.
    /// Used both by `jump_match` (cycling) and by `maybe_rebuild_search` (so typing a
    /// fresh query or switching tabs lands the view on the first match instead of
    /// leaving the view at wherever the user previously scrolled).
    fn scroll_to_active_match(&mut self) {
        let idx = self.search.active_match_index;
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        let Some(m) = tab.search_matches.get(idx) else {
            return;
        };
        if tab.last_content_height <= 0.0 || tab.content_lines == 0 {
            return;
        }
        // Line-ratio estimate gets the view roughly in the right area. The renderer
        // records the actual y of the active match during paint; render_tab_content
        // schedules a corrective scroll next frame if this estimate was off.
        let estimated_y = line_ratio(m.line_number, tab.content_lines) * tab.last_content_height;
        let margin = if tab.last_viewport_height > 0.0 {
            tab.last_viewport_height * 0.35
        } else {
            100.0
        };
        tab.pending_scroll_offset = Some((estimated_y - margin).max(0.0));
        // Grant the next render one full-paint attempt to record an exact Y.
        // The request is consumed before that render even when the source
        // match has no visible glyph, so later frames return to clipping.
        tab.correct_active_search_pending = true;
    }

    /// Rebuild the active tab's search matches if the query or active tab has changed.
    /// Cheap (no-op) when nothing changed.
    pub(crate) fn maybe_rebuild_search(&mut self) {
        if !self.search.is_open {
            return;
        }
        let tab_idx = self.active_tab;
        let needs_rebuild =
            self.search.query != self.search.last_query || self.search.last_tab != Some(tab_idx);
        if !needs_rebuild {
            return;
        }
        let q = self.search.query.clone();
        if let Some(tab) = self.tabs.get_mut(tab_idx) {
            tab.rebuild_search(&q);
        }
        self.search.last_query = q;
        self.search.last_tab = Some(tab_idx);
        // Always start at the first match after any rebuild — query change, tab
        // change, or watcher reload. Match indices are not semantically comparable
        // across rebuilds; preserving a numeric index across docs is coincidental
        // (matches Firefox / Chrome / VS Code behavior).
        self.search.active_match_index = 0;
        // Scroll to the new first match so the user sees the active highlight
        // immediately rather than wherever they were previously scrolled. Without
        // this, the visible match (somewhere else on screen) gets the regular
        // highlight color, leaving the user with the impression that the active
        // highlight is "stuck on the previous result".
        self.scroll_to_active_match();
    }

    /// Close the find bar and clear highlight state on every tab.
    pub(crate) fn close_search(&mut self) {
        self.search.is_open = false;
        self.search.query.clear();
        self.search.last_query.clear();
        self.search.last_tab = None;
        self.search.focus_requested = false;
        self.search.active_match_index = 0;
        for tab in &mut self.tabs {
            tab.search_matches.clear();
            tab.cache.clear_search_ranges();
        }
    }
}
