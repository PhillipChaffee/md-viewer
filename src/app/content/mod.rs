//! The document pane: rendered markdown, edit mode, and link handling.

use std::sync::Arc;

use eframe::egui;
use egui::scroll_area::ScrollAreaOutput;
use egui::Widget;
use egui_commonmark_extended::CommonMarkViewer;

use crate::app::action::AppAction;
use crate::app::content::column::{resize_strips, Column};
use crate::app::content::look::DocumentLook;
use crate::app::mcp;
use crate::app::MarkdownApp;
use crate::diagram::diagram_colors;
use crate::layout::{whole_pixels, CONTENT_RIGHT_RESIZE_GUTTER};
use crate::tab::scroll::{ScrollCorrections, HEADER_SCROLL_MARGIN, MATCH_INSET};
use crate::tab::{LinkTarget, Tab};

mod column;
pub(crate) mod look;
#[cfg(kani)]
mod proofs;
#[cfg(test)]
pub(super) mod tests;

/// The active search match counts as visible only this far inside the
/// viewport's top and bottom edges.
const SEARCH_VISIBLE_MARGIN: f32 = 20.0;

/// Space between the top of the page and the meta line above the document.
const KICKER_TOP_SPACE: f32 = 40.0;
/// Space between the meta line and the document's first block.
const KICKER_BOTTOM_SPACE: f32 = 6.0;
/// The meta line is set in the chrome's sans at this size, whatever the
/// document's own fonts.
const KICKER_SIZE: f32 = 13.0;

/// App settings the preview pane renders with.
struct PreviewSettings<'look> {
    look: &'look DocumentLook,
    math_scale: f32,
    line_height: f32,
    code_line_height: f32,
    /// Paint the whole document, not just the viewport, so the renderer
    /// measures the position a pending correction needs.
    force_full_render: bool,
}

impl MarkdownApp {
    /// Render the active tab's content. Returns a path and optional
    /// `#fragment` when the renderer hook requests it be opened in a new tab.
    pub(crate) fn render_tab_content(
        &mut self,
        ui: &mut egui::Ui,
        ctrl_held: bool,
    ) -> Option<LinkTarget> {
        // Snapshot search state before taking a mutable borrow on the active tab
        let search_is_open = self.search.is_open;
        let active_idx = self.search.active_match_index;

        // No document open → render the welcome / idle page instead.
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            self.render_welcome(ui);
            return None;
        };

        push_search_ranges(tab, search_is_open, active_idx);

        // A search correction grants exactly one full-paint attempt. Some
        // source matches (for example Markdown syntax markers) have no
        // rendered glyph and therefore never record a Y; do not let those
        // matches disable viewport clipping forever. A header click is
        // corrected once too, in any mode: edit mode drops the request
        // because it has no rendered headings.
        let corrections = tab.take_corrections();
        let settings = PreviewSettings {
            look: &self.document_look,
            math_scale: self.math_scale,
            // Body lines follow the reader's spacing setting; code lines
            // follow the font preset, which owns the code face's metrics.
            line_height: self.line_spacing.line_height(),
            code_line_height: self.font_preset.code_line_height(),
            force_full_render: corrections.header_key.is_some() || corrections.search,
        };

        let (reading_width, palette) = (self.reading_width, self.theme.palette());
        let column_drag_grab = &mut self.column_drag_grab;
        let mut pane_width = self.document_pane_width;
        let mut requested_width = None;
        let mut edited = false;
        let mcp = &self.mcp;

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
                let column = Column::in_pane(content_rect, reading_width);
                pane_width = content_rect.width();

                // (the Preview|Edit toggle lives in the bars above)
                if tab.is_editing {
                    edited = edit_pane(ui, tab, column);
                } else {
                    let mut scroll = preview_pane(ui, tab, &settings, column);
                    let corrected = scroll_correction(tab, scroll.state.offset.y, &corrections);
                    if let Some(offset) = corrected {
                        tab.scroll_to(offset);
                    }
                    wheel_scroll_during_selection(ui, tab, &mut scroll, raw_scroll, content_rect);
                }
                // After the document, so the strips sit on top of it.
                requested_width = resize_strips(ui, column, column_drag_grab, palette);
                for (name, strip) in column.strips() {
                    mcp::rect(mcp, format_args!("{name}"), "slider", strip, None);
                }

                // Request repaint during smooth scrolling
                if ui.ctx().input(|i| i.smooth_scroll_delta.length_sq() > 0.0) {
                    ui.ctx().request_repaint();
                }
            });

        let open_in_new_tab = follow_clicked_link(tab, ctrl_held, &mut self.error_message);
        if edited {
            // The find bar's matches are of the text before the edit. They
            // are found again next frame and counted in the one after.
            self.search.last_tab = None;
            ui.ctx().request_repaint();
        }
        self.document_pane_width = pane_width;
        if let Some(width) = requested_width {
            self.apply_action(ui.ctx(), AppAction::SetReadingWidth(width.get()));
        }
        open_in_new_tab
    }
}

/// Act on the link the renderer reported clicked this frame, if any: a
/// plain click navigates `tab`, and the path of a Ctrl+click is returned
/// for the caller to open in a new tab.
fn follow_clicked_link(
    tab: &mut Tab,
    ctrl_held: bool,
    error_message: &mut Option<String>,
) -> Option<LinkTarget> {
    let link = tab.check_link_hooks()?;
    let followed = if ctrl_held {
        tab.link_target(&link)
    } else {
        tab.navigate_to_link(&link).map(|_| None)
    };
    followed.unwrap_or_else(|error| {
        *error_message = Some(format!("Unable to open {link}: {error}"));
        None
    })
}

/// Hands the find bar's matches to the renderer, which paints the highlights.
fn push_search_ranges(tab: &mut Tab, search_is_open: bool, active_idx: usize) {
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
}

/// Edit mode: a scrollable `TextEdit` holding the raw markdown, as wide as
/// the text column and centered like it. The `TextEdit`'s own `ScrollArea`
/// handles the wheel natively. Returns whether the text was edited this
/// frame.
fn edit_pane(ui: &mut egui::Ui, tab: &mut Tab, column: Column) -> bool {
    let pending_edit = tab.pending_edit_scroll.take();
    let mut edited = false;
    let edit_out = egui::ScrollArea::vertical()
        .id_salt(tab.id.with("edit-scroll"))
        .auto_shrink(false)
        .max_width(column.pane.width())
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(column.inset());
                let frame = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(4_i8, 2_i8));
                frame.show(ui, |ui| {
                    edited = egui::TextEdit::multiline(&mut tab.content)
                        .id_salt(ui.id().with("edit"))
                        .desired_width(column.width - 8.0)
                        .desired_rows(24)
                        .frame(false)
                        .ui(ui)
                        .changed();
                });
            });
        });
    if edited {
        tab.content_edited();
    }
    tab.measured(&edit_out);
    if let Some(ratio) = pending_edit {
        let mut state = edit_out.state;
        state.offset.y = ratio * edit_out.content_size.y.max(1.0);
        state.store(ui.ctx(), edit_out.id);
        ui.ctx().request_repaint();
    }
    edited
}

/// The meta line above the document. It is painted inside the renderer's
/// column, so it scrolls with the text it describes.
fn document_preamble(ui: &mut egui::Ui, kicker: &str, color: egui::Color32) {
    ui.add_space(KICKER_TOP_SPACE);
    ui.label(
        egui::RichText::new(kicker)
            .font(egui::FontId::proportional(KICKER_SIZE))
            .color(color),
    );
    ui.add_space(KICKER_BOTTOM_SPACE);
}

/// View mode: the rendered document, in the text column.
///
/// The renderer owns the `ScrollArea` (via `show_scrollable`), so the scroll
/// source, pending offset, and content version go in through builder
/// methods. The returned output exposes the scroll state for the corrective
/// scrolls and the selection-preserving wheel handling that follow.
fn preview_pane(
    ui: &mut egui::Ui,
    tab: &mut Tab,
    settings: &PreviewSettings<'_>,
    column: Column,
) -> ScrollAreaOutput<()> {
    let pending = tab.take_pending_scroll();
    let look = settings.look;
    // The document's fonts, for everything the renderer lays out below.
    ui.set_style(Arc::clone(&look.style));
    let kicker = tab.kicker.as_str();
    let preamble = |ui: &mut egui::Ui| document_preamble(ui, kicker, look.muted);
    // The renderer centers the column in the pane; tables, code blocks and
    // images keep to it, and a table too wide for it scrolls sideways
    // inside it (#110). A diagram wider than the column may use the pane.
    let column_px = whole_pixels(column.width);
    let mut blocks = look.blocks.clone();
    blocks.figure_max_width = Some(column.figure_room());
    let output = CommonMarkViewer::new()
        .default_implicit_uri_scheme(&tab.base_uri)
        .max_image_width(Some(800))
        .default_width(Some(column_px))
        .table_max_width(Some(column_px))
        .block_style(blocks)
        .preamble(Some(&preamble))
        .indentation_spaces(2)
        .use_strong_font_family(true)
        // Frontmatter is metadata, not prose: show it as a
        // key/value table rather than a thematic break plus a
        // paragraph of raw `key: value` lines (#117).
        .render_frontmatter(true)
        .math_scale(settings.math_scale)
        .mermaid_colors(Some(diagram_colors(ui.visuals())))
        .show_alt_text_on_hover(true)
        .syntax_theme_dark(look.syntax_theme)
        .syntax_theme_light(look.syntax_theme)
        .line_height(settings.line_height)
        .code_line_height(settings.code_line_height)
        .content_version(tab.content_version)
        .pending_scroll_offset(pending)
        .force_full_render(settings.force_full_render)
        .scroll_source(egui::scroll_area::ScrollSource {
            scroll_bar: true,
            drag: false,
            mouse_wheel: true,
        })
        .show_scrollable(tab.id, ui, &mut tab.cache, &tab.content);

    tab.measured(&output);
    output
}

/// The scroll offset for the next frame when a jump made from an estimate
/// missed: the renderer has now measured the target, so the scroll snaps to
/// the measured position. A header correction wins over a search correction.
fn scroll_correction(
    tab: &Tab,
    current_scroll: f32,
    corrections: &ScrollCorrections,
) -> Option<f32> {
    // Gated by the one-shot `ScrollCorrections::search` request (set by
    // `scroll_to_active_match`) so it fires at most once per jump — without
    // this gate the correction would re-trigger every frame the active match
    // is off-screen, fighting the user's wheel input and locking the view
    // (issue #19).
    let for_search = corrections
        .search
        .then(|| tab.cache.active_search_y())
        .flatten()
        .and_then(|y| search_correction(y, current_scroll, tab.last_viewport_height));
    // The full paint forced by the header click recorded every heading's
    // precise y under its composite cache key.
    let for_header = corrections
        .header_key
        .as_deref()
        .and_then(|key| tab.cache.get_header_position(key))
        .and_then(|y| header_correction(y, current_scroll));
    for_header.or(for_search)
}

/// Where to scroll so the active search match is visible, or `None` when it
/// already is. Line-ratio jumps overshoot or undershoot in image-heavy
/// documents; this uses the y the renderer recorded for the match.
fn search_correction(match_y: f32, current_scroll: f32, viewport_height: f32) -> Option<f32> {
    let visible = (current_scroll + SEARCH_VISIBLE_MARGIN)
        ..=(current_scroll + viewport_height - SEARCH_VISIBLE_MARGIN);
    if visible.contains(&match_y) || viewport_height <= 0.0 {
        return None;
    }
    // Place the active match ~35% from the top of the viewport
    let inset = viewport_height * MATCH_INSET;
    worth_scrolling((match_y - inset).max(0.0), current_scroll)
}

/// Where to scroll so a clicked header sits just below the top of the
/// viewport, or `None` when it already does.
fn header_correction(header_y: f32, current_scroll: f32) -> Option<f32> {
    let target = (header_y - HEADER_SCROLL_MARGIN).max(0.0);
    worth_scrolling(target, current_scroll)
}

/// `target`, unless the view is already there (within a frame's drift).
fn worth_scrolling(target: f32, current_scroll: f32) -> Option<f32> {
    ((target - current_scroll).abs() > 2.0).then_some(target)
}

/// Scrolls the document by the mouse wheel during a drag, which is what a
/// text selection in progress is: the scroll area leaves the wheel alone
/// while anything is dragged, and takes it itself at any other time.
///
/// The offset is written after rendering because setting it before the
/// renderer runs breaks an in-progress text selection.
fn wheel_scroll_during_selection(
    ui: &egui::Ui,
    tab: &Tab,
    scroll: &mut ScrollAreaOutput<()>,
    raw_scroll: f32,
    content_rect: egui::Rect,
) {
    let pointer_over_content = ui.ctx().input(|i| {
        i.pointer
            .hover_pos()
            .is_some_and(|pos| content_rect.contains(pos))
    });
    let dragging = ui.ctx().dragged_id().is_some();
    if dragging && raw_scroll.abs() > 0.0 && pointer_over_content {
        let max_scroll = (tab.last_content_height - content_rect.height()).max(0.0);
        if let Some(new_offset) = wheel_scroll_target(scroll.state.offset.y, raw_scroll, max_scroll)
        {
            scroll.state.offset.y = new_offset;
            scroll.state.store(ui.ctx(), scroll.id);
            ui.ctx().request_repaint();
        }
    }
}

/// The offset one wheel step moves to, or `None` when the step would not
/// move the view or would land on either end of the document: storing an
/// offset at the ends can break the selection.
fn wheel_scroll_target(current_offset: f32, wheel_delta: f32, max_scroll: f32) -> Option<f32> {
    let new_offset = (current_offset - wheel_delta).clamp(0.0, max_scroll);
    let would_hit_top = new_offset < 0.5;
    let would_hit_bottom = new_offset > max_scroll - 0.5;
    let offset_changed = (new_offset - current_offset).abs() > 0.5;
    (offset_changed && !would_hit_top && !would_hit_bottom).then_some(new_offset)
}
