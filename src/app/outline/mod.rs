//! The outline sidebar.

use eframe::egui;
use egui_commonmark_extended::header_position_key;

use crate::app::MarkdownApp;
use crate::headers::{any_header_has_children, header_has_children, header_is_hidden};
use crate::icons::paint_sidebar_resize_affordance;
use crate::layout::SIDEBAR_MIN_WIDTH;

impl MarkdownApp {
    /// Render the outline sidebar (right panel)
    /// Rendered at top level for proper layout space allocation
    pub(crate) fn render_outline(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        if !self.show_outline || tab.outline_headers.is_empty() {
            return;
        }

        // Handle outline header click (store index to access both title and line_number)
        let mut clicked_header_index: Option<usize> = None;

        // Collect widget data for MCP registration (name, widget_type, rect, value)
        #[cfg(feature = "mcp")]
        let mut widget_data: Vec<(String, &'static str, egui::Rect, Option<String>)> = Vec::new();

        let is_dragging = ctx.input(|i| i.pointer.any_down());

        let panel = egui::SidePanel::right("outline")
            .resizable(true)
            .default_width(self.outline_width)
            .width_range(SIDEBAR_MIN_WIDTH..=f32::INFINITY)
            .frame(
                egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                    left: 8,
                    right: 0,
                    top: 8,
                    bottom: 0,
                }),
            )
            .show(ctx, |ui| {
                // Expand/Collapse All buttons (only if there are nested headers)
                let has_nested = any_header_has_children(&tab.outline_headers);
                if has_nested {
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        let expand_btn = ui.small_button("Expand All");

                        // Collect Expand All button for MCP
                        #[cfg(feature = "mcp")]
                        widget_data.push((
                            "Outline: Expand All".to_string(),
                            "button",
                            expand_btn.rect,
                            None,
                        ));

                        if expand_btn.clicked() {
                            tab.collapsed_headers.clear();
                        }

                        let collapse_btn = ui.small_button("Collapse All");

                        // Collect Collapse All button for MCP
                        #[cfg(feature = "mcp")]
                        widget_data.push((
                            "Outline: Collapse All".to_string(),
                            "button",
                            collapse_btn.rect,
                            None,
                        ));

                        if collapse_btn.clicked() {
                            for i in 0..tab.outline_headers.len() {
                                if header_has_children(&tab.outline_headers, i) {
                                    tab.collapsed_headers.insert(i);
                                }
                            }
                        }
                    });
                    ui.separator();
                }
                // Pre-compute the visible header indices (skip those hidden
                // by collapsed ancestors). This lets us virtualize via
                // show_rows, paying O(visible-on-screen) instead of
                // O(total-headers) per frame. On a 100k-line doc with ~15k
                // headers this is the difference between visibly laggy and
                // smooth outline interactions.
                let visible_indices: Vec<usize> = (0..tab.outline_headers.len())
                    .filter(|&i| !header_is_hidden(&tab.outline_headers, i, &tab.collapsed_headers))
                    .collect();
                let show_fold_indicators = any_header_has_children(&tab.outline_headers);
                // Row height: fold indicator is 20px tall, fold-indicator-less
                // rows fall back to the standard interact_size which is
                // typically 18–20px anyway. A small fudge keeps neighboring
                // rows from clipping into each other.
                let row_height = ui.spacing().interact_size.y.max(20.0);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .id_salt("outline")
                    .show_rows(ui, row_height, visible_indices.len(), |ui, row_range| {
                        let mut toggle_index: Option<usize> = None;
                        for &idx in &visible_indices[row_range] {
                            let header = &tab.outline_headers[idx];

                            let has_children = header_has_children(&tab.outline_headers, idx);
                            let is_collapsed = tab.collapsed_headers.contains(&idx);

                            // Indent based on header level (h2 = 0, h3 = 1 indent, etc.)
                            let indent = (header.level.saturating_sub(2) as usize) * 12;

                            ui.horizontal(|ui| {
                                // Add base indent
                                if indent > 0 {
                                    ui.add_space(indent as f32);
                                }

                                // Fold indicator (fixed width area for alignment)
                                // Only allocate space if any header has children
                                if show_fold_indicators {
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(20.0, 20.0),
                                        egui::Sense::click(),
                                    );
                                    if has_children {
                                        let indicator = if is_collapsed { "+" } else { "-" };
                                        let text_color = if response.hovered() {
                                            ui.visuals().strong_text_color()
                                        } else {
                                            ui.visuals().text_color()
                                        };
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            indicator,
                                            egui::FontId::monospace(16.0),
                                            text_color,
                                        );

                                        // Collect fold indicator for MCP
                                        #[cfg(feature = "mcp")]
                                        widget_data.push((
                                            format!("Toggle: {}", header.title),
                                            "button",
                                            rect,
                                            Some(if is_collapsed {
                                                "collapsed".to_string()
                                            } else {
                                                "expanded".to_string()
                                            }),
                                        ));

                                        if !is_dragging && response.clicked() {
                                            toggle_index = Some(idx);
                                        }
                                    }
                                }

                                let response = ui.add(
                                    egui::Button::selectable(false, &header.title)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                );

                                // Collect header for MCP
                                #[cfg(feature = "mcp")]
                                widget_data.push((
                                    format!("Header: {}", header.title),
                                    "header",
                                    response.rect,
                                    Some(format!("h{}", header.level)),
                                ));

                                if !is_dragging && response.clicked() {
                                    clicked_header_index = Some(idx);
                                }
                            });
                        }
                        // Apply toggle after iteration to avoid borrow issues
                        if let Some(idx) = toggle_index {
                            if tab.collapsed_headers.contains(&idx) {
                                tab.collapsed_headers.remove(&idx);
                            } else {
                                tab.collapsed_headers.insert(idx);
                            }
                        }
                    });
            });
        self.outline_width = panel.response.rect.width();
        // Right panel: the draggable edge is its left border.
        paint_sidebar_resize_affordance(
            ctx,
            panel.response.rect.left(),
            panel.response.rect.y_range(),
            egui::Id::new("outline"),
        );

        // Register all collected widgets with MCP bridge
        #[cfg(feature = "mcp")]
        for (name, widget_type, rect, value) in widget_data {
            self.mcp_bridge
                .register_widget_rect(&name, widget_type, rect, value.as_deref());
        }

        // Calculate scroll target if header was clicked
        if let Some(idx) = clicked_header_index {
            if let Some(header) = tab.outline_headers.get(idx) {
                // The renderer records the same source-stable key, so formatting
                // and duplicate display titles cannot redirect the click.
                let key = header_position_key(header.source_start);
                // Try to get actual rendered position from cache first.
                // With virtualization, the cache may hold a stale value from a
                // partial render — record the key for the post-render
                // corrective step which re-checks after the bootstrap full paint.
                if let Some(y_pos) = tab.cache.get_header_position(&key) {
                    tab.pending_scroll_offset = Some((y_pos - 50.0).max(0.0));
                } else if tab.last_content_height > 0.0 && tab.content_lines > 0 {
                    // Fallback: estimate position based on line number ratio
                    let estimated_y = (header.line_number as f32 / tab.content_lines as f32)
                        * tab.last_content_height;
                    tab.pending_scroll_offset = Some((estimated_y - 50.0).max(0.0));
                    // The estimate only gets us near the target. Request one
                    // complete paint so the corrective pass can measure it.
                    tab.pending_header_click_key = Some(key);
                }
            }
        }
    }
}
