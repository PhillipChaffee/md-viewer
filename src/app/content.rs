//! The document pane: rendered markdown, edit mode, and link handling.

use std::path::PathBuf;

use eframe::egui;
use egui::Widget;
use egui_commonmark_extended::CommonMarkViewer;

use crate::app::MarkdownApp;
use crate::layout::{content_default_width, content_width_limit, CONTENT_RIGHT_RESIZE_GUTTER};
use crate::links::handle_local_link_click;
use crate::search::take_search_correction_request;

impl MarkdownApp {
    /// Render the active tab's content. Returns a path and optional
    /// `#fragment` when the renderer hook requests it be opened in a new tab.
    pub(crate) fn render_tab_content(
        &mut self,
        ui: &mut egui::Ui,
        ctrl_held: bool,
    ) -> Option<(PathBuf, Option<String>)> {
        let mut open_in_new_tab: Option<(PathBuf, Option<String>)> = None;
        let mut navigation_error = None;

        // Snapshot search state before taking a mutable borrow on the active tab
        let search_is_open = self.search.is_open;
        let active_idx = self.search.active_match_index;

        // No document open → render the welcome / idle page instead.
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            self.render_welcome(ui);
            return None;
        };

        // Push current search match ranges into the cache so the renderer can paint highlights
        if search_is_open && !tab.search_matches.is_empty() {
            let ranges: Vec<_> = tab
                .search_matches
                .iter()
                .map(|m| m.byte_start..m.byte_end)
                .collect();
            let active = tab
                .search_matches
                .get(active_idx)
                .map(|m| m.byte_start..m.byte_end);
            tab.cache.set_search_ranges(ranges);
            tab.cache.set_active_search_range(active);
        } else {
            tab.cache.clear_search_ranges();
        }

        // A search correction grants exactly one full-paint attempt. Some
        // source matches (for example Markdown syntax markers) have no
        // rendered glyph and therefore never record a Y; do not let those
        // matches disable viewport clipping forever.
        let correct_search_this_frame =
            take_search_correction_request(&mut tab.correct_active_search_pending);

        // Content area (no inner CentralPanel needed - we're already in one)
        // Left margin for breathing room, right margin prevents scrollbar/resize-handle overlap jitter
        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: 8,
                right: CONTENT_RIGHT_RESIZE_GUTTER,
                ..Default::default()
            })
            .show(ui, |ui| {
                // Capture scroll input for manual handling during selection
                let raw_scroll = ui.ctx().input(|i| i.raw_scroll_delta.y);
                let content_rect = ui.available_rect_before_wrap();

                // The renderer owns the ScrollArea now (via show_scrollable),
                // so we configure scroll_source / pending offset / content
                // version through builder methods. The returned ScrollAreaOutput
                // exposes state.offset and inner_rect for the post-render
                // selection-preserving wheel hack below.
                // View mode: CommonMarkViewer
                // Edit mode: scrollable TextEdit for raw markdown
                // (the Preview|Edit toggle lives in the tab bar)

                let mut scroll_output: Option<egui::scroll_area::ScrollAreaOutput<()>> = None;

                if tab.is_editing {
                    // Edit mode: scrollable TextEdit with the raw markdown.
                    // The TextEdit's own ScrollArea handles the wheel natively;
                    // scroll_output stays None so the view-mode hacks below skip.
                    let pending_edit = tab.pending_edit_scroll.take();
                    let edit_out = egui::ScrollArea::vertical()
                        .id_salt(tab.id.with("edit-scroll"))
                        .auto_shrink(false)
                        .max_width(content_rect.width())
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                let frame = egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(4_i8, 2_i8));
                                frame.show(ui, |ui| {
                                    egui::TextEdit::multiline(&mut tab.content)
                                        .id_salt(ui.id().with("edit"))
                                        .desired_width(content_rect.width() - 8.0)
                                        .desired_rows(24)
                                        .frame(false)
                                        .ui(ui);
                                });
                            });
                        });
                    tab.scroll_offset = edit_out.state.offset.y;
                    tab.last_viewport_height = edit_out.inner_rect.height();
                    tab.last_content_height = edit_out.content_size.y;
                    if let Some(ratio) = pending_edit {
                        let mut state = edit_out.state;
                        state.offset.y = ratio * edit_out.content_size.y.max(1.0);
                        state.store(ui.ctx(), edit_out.id);
                        ui.ctx().request_repaint();
                    }
                } else {
                let force_full_render =
                    tab.pending_header_click_key.is_some() || correct_search_this_frame;
                let pending = tab.pending_scroll_offset.take();
                let default_width = content_default_width(self.full_width_content);
                let content_width =
                    content_width_limit(self.full_width_content, content_rect.width());
                // Line heights follow the emulated viewer (GitHub 1.5/1.45,
                // VS Code preview 1.6/1.36); the font preset owns the metrics.
                let (preset_line_height, preset_code_line_height) = self.font_preset.line_heights();
                scroll_output = Some(CommonMarkViewer::new()
                    .default_implicit_uri_scheme(&tab.base_uri)
                    .max_image_width(Some(800))
                    .default_width(default_width)
                    // Full Width governs the complete document layout. With
                    // the reading cap active, tables reflow within it; their
                    // horizontal scroller still handles minimum-width or
                    // user-resized overflow (#110).
                    .table_max_width(Some(content_width))
                    .indentation_spaces(2)
                    .use_strong_font_family(true)
                    // Frontmatter is metadata, not prose: show it as a
                    // key/value table rather than a thematic break plus a
                    // paragraph of raw `key: value` lines (#117).
                    .render_frontmatter(true)
                    .math_scale(self.math_scale)
                    .show_alt_text_on_hover(true)
                    .syntax_theme_dark("base16-ocean.dark")
                    .syntax_theme_light("base16-ocean.light")
                    .line_height(preset_line_height)
                    .code_line_height(preset_code_line_height)
                    .paragraph_spacing(2.0)
                    .heading_spacing_above(2.0)
                    .heading_spacing_below(0.75)
                    .content_version(tab.content_version)
                    .pending_scroll_offset(pending)
                    .force_full_render(force_full_render)
                    .scroll_source(egui::scroll_area::ScrollSource {
                        scroll_bar: true,
                        drag: false,
                        mouse_wheel: true,
                    })
                    .show_scrollable(tab.id, ui, &mut tab.cache, &tab.content));

                if let Some(so) = scroll_output.as_ref() {
                    tab.scroll_offset = so.state.offset.y;
                    tab.last_viewport_height = so.inner_rect.height();
                    tab.last_content_height = so.content_size.y;
                }
                }

                // If the renderer recorded an exact y for the active match, check
                // whether the current scroll position keeps it visible. If not,
                // schedule a corrective scroll using the recorded y — this fixes
                // line-ratio overshoot/undershoot in image-heavy documents.
                // Corrective scroll for search-match jump. Gated by the
                // one-shot `correct_active_search_pending` flag (set by
                // `scroll_to_active_match`) so it fires at most once per
                // jump — without this gate the block would re-trigger every
                // frame the active match is off-screen, fighting the user's
                // wheel input and locking the view (issue #19).
                if correct_search_this_frame {
                    if let Some(actual_y) = tab.cache.active_search_y() {
                        // View-mode only: scroll_output is None while editing.
                        if let Some(so) = scroll_output.as_ref() {
                            let current_scroll = so.state.offset.y;
                            let viewport_top = current_scroll;
                            let viewport_bottom = current_scroll + tab.last_viewport_height;
                            // Consider "not visible" if outside the viewport with a small margin
                            let margin_outside = 20.0_f32;
                            let needs_correction = actual_y < viewport_top + margin_outside
                                || actual_y > viewport_bottom - margin_outside;
                            if needs_correction && tab.last_viewport_height > 0.0 {
                                // Place the active match ~35% from the top of the viewport
                                let inset = tab.last_viewport_height * 0.35;
                                let want_scroll = (actual_y - inset).max(0.0);
                                // Avoid stomping if we're already at the target (within a frame)
                                if (want_scroll - current_scroll).abs() > 2.0 {
                                    tab.pending_scroll_offset = Some(want_scroll);
                                }
                            }
                        }
                    }
                }

                // Corrective scroll for outline-click. The bootstrap full paint
                // triggered above by `pending_scroll_offset` recorded every
                // heading's precise y under its composite cache key. If the
                // line-ratio first-frame estimate left the target outside the
                // viewport (common in image-heavy docs and the failure mode for
                // duplicate-titled headers before this), snap the next frame to
                // the recorded y. Mirrors the search-match corrective above.
                if let Some(key) = tab.pending_header_click_key.take() {
                    if let Some(actual_y) = tab.cache.get_header_position(&key) {
                        // View-mode only: scroll_output is None while editing.
                        if let Some(so) = scroll_output.as_ref() {
                            let current_scroll = so.state.offset.y;
                            let want_scroll = (actual_y - 50.0).max(0.0);
                            if (want_scroll - current_scroll).abs() > 2.0 {
                                tab.pending_scroll_offset = Some(want_scroll);
                            }
                        }
                    }
                }

                // Manual scroll handling for mouse wheel during text selection.
                // View-mode only: in edit mode the TextEdit's own ScrollArea
                // handles the wheel natively and scroll_output is None.
                let pointer_over_content = ui.ctx().input(|i| {
                    i.pointer
                        .hover_pos()
                        .is_some_and(|pos| content_rect.contains(pos))
                });
                if raw_scroll.abs() > 0.0 && pointer_over_content {
                    if let Some(so) = scroll_output.as_mut() {
                        let current_offset = so.state.offset.y;
                        let max_scroll = (tab.last_content_height - content_rect.height()).max(0.0);
                        let new_offset = (current_offset - raw_scroll).clamp(0.0, max_scroll);

                        // Don't store at boundaries (can break selection)
                        let would_hit_top = new_offset < 0.5;
                        let would_hit_bottom = new_offset > max_scroll - 0.5;
                        let offset_changed = (new_offset - current_offset).abs() > 0.5;

                        if offset_changed && !would_hit_top && !would_hit_bottom {
                            so.state.offset.y = new_offset;
                            so.state.store(ui.ctx(), so.id);
                            ui.ctx().request_repaint();
                        }
                    }
                }

                // Request repaint during smooth scrolling
                if ui.ctx().input(|i| i.smooth_scroll_delta.length_sq() > 0.0) {
                    ui.ctx().request_repaint();
                }
            });

        // Check for clicked links
        if let Some(clicked_link) = tab.check_link_hooks() {
            (open_in_new_tab, navigation_error) =
                handle_local_link_click(tab, &clicked_link, ctrl_held);
        }

        if let Some(error) = navigation_error {
            self.error_message = Some(error);
        }

        open_in_new_tab
    }
}
