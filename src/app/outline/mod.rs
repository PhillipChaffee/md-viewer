//! The outline sidebar.

use eframe::egui;
use egui_commonmark_extended::header_position_key;

use crate::app::MarkdownApp;
use crate::headers::{any_header_has_children, header_has_children, header_is_hidden, Header};
use crate::icons::paint_sidebar_resize_affordance;
use crate::layout::SIDEBAR_MIN_WIDTH;
use crate::tab::Tab;

#[cfg(test)]
mod tests;

/// Space left above a heading the document scrolls to.
pub(super) const HEADER_SCROLL_MARGIN: f32 = 50.0;

/// A widget recorded for the MCP bridge: name, widget type, rect, value.
#[cfg(feature = "mcp")]
type McpWidget = (String, &'static str, egui::Rect, Option<String>);

/// What a click in the outline asks for, by header index.
enum OutlineClick {
    ToggleFold(usize),
    JumpTo(usize),
}

/// State shared by the outline's widgets while one frame is drawn.
struct OutlineUi {
    /// A pressed pointer may be resizing the panel, so clicks are ignored.
    is_dragging: bool,
    click: Option<OutlineClick>,
    #[cfg(feature = "mcp")]
    widgets: Vec<McpWidget>,
}

/// The fold column of one outline row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fold {
    /// No header in the document has children, so there is no column.
    NoColumn,
    /// The column is reserved to keep titles aligned, but this row is a leaf.
    Leaf,
    Collapsed,
    Expanded,
}

impl Fold {
    fn of(tab: &Tab, index: usize, show_column: bool) -> Self {
        if !show_column {
            Self::NoColumn
        } else if !header_has_children(&tab.outline_headers, index) {
            Self::Leaf
        } else if tab.collapsed_headers.contains(&index) {
            Self::Collapsed
        } else {
            Self::Expanded
        }
    }
}

/// One outline entry as a row draws it.
struct OutlineRow<'a> {
    index: usize,
    header: &'a Header,
    fold: Fold,
}

/// Where a click on an outline entry scrolls the document.
#[derive(Debug, PartialEq)]
struct HeaderJump {
    scroll_offset: f32,
    /// The offset comes from line counts, not a measured position, so the
    /// renderer must measure the heading and correct the scroll.
    is_estimate: bool,
}

impl MarkdownApp {
    /// Render the outline sidebar (right panel).
    /// Rendered at top level for proper layout space allocation.
    pub(crate) fn render_outline(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        if !self.show_outline || tab.outline_headers.is_empty() {
            return;
        }

        let mut outline = OutlineUi {
            is_dragging: ctx.input(|i| i.pointer.any_down()),
            click: None,
            #[cfg(feature = "mcp")]
            widgets: Vec::new(),
        };

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
                #[cfg(feature = "mcp")]
                outline_toolbar(ui, tab, &mut outline.widgets);
                #[cfg(not(feature = "mcp"))]
                outline_toolbar(ui, tab);
                outline_rows(ui, tab, &mut outline);
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
        for (name, widget_type, rect, value) in outline.widgets {
            self.mcp_bridge
                .register_widget_rect(&name, widget_type, rect, value.as_deref());
        }

        match outline.click {
            Some(OutlineClick::ToggleFold(index)) => toggle_fold(tab, index),
            Some(OutlineClick::JumpTo(index)) => jump_to_header(tab, index),
            None => {}
        }
    }
}

/// Expand All / Collapse All, shown only when some header has children.
fn outline_toolbar(
    ui: &mut egui::Ui,
    tab: &mut Tab,
    #[cfg(feature = "mcp")] widgets: &mut Vec<McpWidget>,
) {
    if !any_header_has_children(&tab.outline_headers) {
        return;
    }
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let expand_btn = ui.small_button("Expand All");

        // Collect Expand All button for MCP
        #[cfg(feature = "mcp")]
        widgets.push((
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
        widgets.push((
            "Outline: Collapse All".to_string(),
            "button",
            collapse_btn.rect,
            None,
        ));

        if collapse_btn.clicked() {
            let headers = &tab.outline_headers;
            tab.collapsed_headers
                .extend((0..headers.len()).filter(|&index| header_has_children(headers, index)));
        }
    });
    ui.separator();
}

/// The scrollable list of headers that no collapsed ancestor hides.
fn outline_rows(ui: &mut egui::Ui, tab: &Tab, outline: &mut OutlineUi) {
    // Pre-compute the visible header indices (skip those hidden
    // by collapsed ancestors). This lets us virtualize via
    // show_rows, paying O(visible-on-screen) instead of
    // O(total-headers) per frame. On a 100k-line doc with ~15k
    // headers this is the difference between visibly laggy and
    // smooth outline interactions.
    let visible_indices: Vec<usize> = (0..tab.outline_headers.len())
        .filter(|&i| !header_is_hidden(&tab.outline_headers, i, &tab.collapsed_headers))
        .collect();
    let show_fold_column = any_header_has_children(&tab.outline_headers);
    // Row height: fold indicator is 20px tall, fold-indicator-less
    // rows fall back to the standard interact_size which is
    // typically 18–20px anyway. A small fudge keeps neighboring
    // rows from clipping into each other.
    let row_height = ui.spacing().interact_size.y.max(20.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .id_salt("outline")
        .show_rows(ui, row_height, visible_indices.len(), |ui, row_range| {
            for &index in &visible_indices[row_range] {
                let row = OutlineRow {
                    index,
                    header: &tab.outline_headers[index],
                    fold: Fold::of(tab, index, show_fold_column),
                };
                outline_row(ui, &row, outline);
            }
        });
}

fn outline_row(ui: &mut egui::Ui, row: &OutlineRow<'_>, outline: &mut OutlineUi) {
    ui.horizontal(|ui| {
        // Indent based on header level (h2 = 0, h3 = 1 indent, etc.)
        let indent = f32::from(row.header.level.saturating_sub(2)) * 12.0;
        if indent > 0.0 {
            ui.add_space(indent);
        }

        fold_indicator(ui, row, outline);

        let response = ui.add(
            egui::Button::selectable(false, &row.header.title)
                .wrap_mode(egui::TextWrapMode::Extend),
        );

        // Collect header for MCP
        #[cfg(feature = "mcp")]
        outline.widgets.push((
            format!("Header: {}", row.header.title),
            "header",
            response.rect,
            Some(format!("h{}", row.header.level)),
        ));

        if !outline.is_dragging && response.clicked() {
            outline.click = Some(OutlineClick::JumpTo(row.index));
        }
    });
}

/// Draws the fold column of a row: a fixed-width slot, with a clickable
/// `+` or `-` when the header has children.
fn fold_indicator(ui: &mut egui::Ui, row: &OutlineRow<'_>, outline: &mut OutlineUi) {
    if row.fold == Fold::NoColumn {
        return;
    }
    let (rect, response) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
    let indicator = match row.fold {
        Fold::Collapsed => "+",
        Fold::Expanded => "-",
        Fold::NoColumn | Fold::Leaf => return,
    };
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
    outline.widgets.push((
        format!("Toggle: {}", row.header.title),
        "button",
        rect,
        Some(if row.fold == Fold::Collapsed {
            "collapsed".to_string()
        } else {
            "expanded".to_string()
        }),
    ));

    if !outline.is_dragging && response.clicked() {
        outline.click = Some(OutlineClick::ToggleFold(row.index));
    }
}

fn toggle_fold(tab: &mut Tab, index: usize) {
    if !tab.collapsed_headers.remove(&index) {
        tab.collapsed_headers.insert(index);
    }
}

/// Schedules the scroll that brings the clicked header into view.
fn jump_to_header(tab: &mut Tab, index: usize) {
    let Some(header) = tab.outline_headers.get(index) else {
        return;
    };
    // The renderer records the same source-stable key, so formatting
    // and duplicate display titles cannot redirect the click.
    let key = header_position_key(header.source_start);
    let Some(jump) = header_jump(
        tab.cache.get_header_position(&key),
        header.line_number,
        tab.content_lines,
        tab.last_content_height,
    ) else {
        return;
    };
    tab.pending_scroll_offset = Some(jump.scroll_offset);
    if jump.is_estimate {
        // The estimate only gets us near the target. Request one
        // complete paint so the corrective pass can measure it.
        tab.pending_header_click_key = Some(key);
    }
}

/// Picks the scroll offset for a header: its measured position when the
/// renderer has painted it, otherwise an estimate from its line number.
/// `None` when the document has not been laid out yet.
fn header_jump(
    measured_y: Option<f32>,
    line_number: usize,
    content_lines: usize,
    content_height: f32,
) -> Option<HeaderJump> {
    let above = |y: f32| (y - HEADER_SCROLL_MARGIN).max(0.0);
    if let Some(y) = measured_y {
        return Some(HeaderJump {
            scroll_offset: above(y),
            is_estimate: false,
        });
    }
    (content_height > 0.0 && content_lines > 0).then(|| HeaderJump {
        scroll_offset: above(line_fraction(line_number, content_lines) * content_height),
        is_estimate: true,
    })
}

/// How far down the document `line` is, as a fraction of `total` lines.
#[expect(
    clippy::cast_precision_loss,
    reason = "line counts above 2^24 lose precision, which a scroll estimate tolerates"
)]
fn line_fraction(line: usize, total: usize) -> f32 {
    line as f32 / total as f32
}
