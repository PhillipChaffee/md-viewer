//! The outline sidebar.

use eframe::egui;
use egui_commonmark_extended::header_position_key;

use crate::app::mcp::{self, Mcp};
use crate::app::sidebar::{self, Chevron, Row, Side};
use crate::app::MarkdownApp;
use crate::headers::Header;
use crate::icons::paint_sidebar_resize_affordance;
use crate::layout::{line_fraction, SIDEBAR_MIN_WIDTH};
use crate::tab::Tab;
use crate::theme::Palette;

pub(crate) use room::OutlineRule;
pub(crate) use spy::HeadingSpy;

mod room;
mod spy;
#[cfg(test)]
mod tests;

/// Space left above a heading the document scrolls to.
pub(super) const HEADER_SCROLL_MARGIN: f32 = 50.0;
/// How far each heading level is pushed right of the one above it.
const INDENT: f32 = 14.0;
/// How far the fold control reaches past its chevron on each side.
const FOLD_REACH: f32 = 3.0;

/// What a click in the outline asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutlineClick {
    /// Fold or unfold the header at this index.
    ToggleFold(usize),
    ExpandAll,
    CollapseAll,
    /// Scroll the document to a heading.
    JumpTo {
        /// Byte offset of the heading in the document source.
        source_start: usize,
        line_number: usize,
    },
}

/// State shared by the outline's widgets while one frame is drawn.
struct OutlineUi<'a> {
    palette: &'a Palette,
    /// A pressed pointer may be resizing the panel, so clicks are ignored.
    is_dragging: bool,
    /// The heading the reader is in, or the nearest one above it that no
    /// folded heading hides.
    current: Option<usize>,
    /// The reader moved to another heading, so the list scrolls to show it.
    follow: bool,
    click: Option<OutlineClick>,
    mcp: &'a Mcp,
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
        } else if !tab.outline.has_children(index) {
            Self::Leaf
        } else if tab.outline.is_collapsed(index) {
            Self::Collapsed
        } else {
            Self::Expanded
        }
    }

    const fn chevron(self) -> Chevron {
        match self {
            Self::NoColumn => Chevron::None,
            Self::Leaf => Chevron::Blank,
            Self::Collapsed => Chevron::Closed,
            Self::Expanded => Chevron::Open,
        }
    }

    /// What a click on the fold control does, for a screen reader. `None`
    /// for a row with nothing to fold.
    const fn verb(self) -> Option<&'static str> {
        match self {
            Self::Collapsed => Some("Expand"),
            Self::Expanded => Some("Collapse"),
            Self::NoColumn | Self::Leaf => None,
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
    /// Show or hide the outline, as the reader's own choice from now on.
    pub(crate) const fn toggle_outline(&mut self) {
        self.show_outline = !self.show_outline;
        self.outline_rule = OutlineRule::Reader;
    }

    /// Until the reader has chosen, show the outline while the window has
    /// room for it beside the text.
    fn follow_room(&mut self, ctx: &egui::Context) {
        let explorer = self.show_explorer.then_some(self.explorer_width);
        let window = ctx.content_rect().width();
        let has_room = room::has_room(window, explorer, self.reading_width);
        if let Some(shown) = self.outline_rule.follow(has_room) {
            self.show_outline = shown;
        }
    }

    /// Render the outline sidebar (right panel).
    /// Rendered at top level for proper layout space allocation.
    pub(crate) fn render_outline(&mut self, ctx: &egui::Context) {
        self.follow_room(ctx);
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        if !self.show_outline || tab.outline.headers().is_empty() {
            return;
        }

        let palette = self.theme.palette();
        let (current, follow) = self.heading_spy.look(tab);
        let mut outline = OutlineUi {
            palette,
            is_dragging: ctx.input(|i| i.pointer.any_down()),
            current,
            follow,
            click: None,
            mcp: &self.mcp,
        };

        let margin = egui::Margin {
            left: 8,
            right: 8,
            top: 8,
            bottom: 8,
        };
        let panel = egui::SidePanel::right("outline")
            .resizable(true)
            .show_separator_line(false)
            .default_width(self.outline_width)
            .width_range(SIDEBAR_MIN_WIDTH..=f32::INFINITY)
            .frame(sidebar::frame(palette, margin))
            .show(ctx, |ui| {
                outline_header(ui, tab, &mut outline);
                outline_rows(ui, tab, &mut outline);
            });
        self.outline_width = panel.response.rect.width();
        sidebar::paint_inner_edge(ctx, panel.response.rect, Side::Right, palette);
        // Right panel: the draggable edge is its left border.
        paint_sidebar_resize_affordance(
            ctx,
            panel.response.rect.left(),
            panel.response.rect.y_range(),
            egui::Id::new("outline"),
        );

        if let Some(click) = outline.click {
            apply_click(tab, click);
        }
    }
}

fn apply_click(tab: &mut Tab, click: OutlineClick) {
    match click {
        OutlineClick::ToggleFold(index) => tab.outline.toggle(index),
        OutlineClick::ExpandAll => tab.outline.expand_all(),
        OutlineClick::CollapseAll => tab.outline.collapse_all(),
        OutlineClick::JumpTo {
            source_start,
            line_number,
        } => jump_to_header(tab, source_start, line_number),
    }
}

/// "CONTENTS", and where some header has children, the menu that folds and
/// unfolds all of them.
fn outline_header(ui: &mut egui::Ui, tab: &Tab, outline: &mut OutlineUi<'_>) {
    let can_fold = tab.outline.has_folds();
    let header = sidebar::Header {
        caption: "CONTENTS",
        detail: None,
        menu: can_fold.then_some("Outline options"),
    };
    let Some(more) = sidebar::header(ui, outline.palette, &header) else {
        return;
    };
    mcp::rect(
        outline.mcp,
        format_args!("Outline: Options"),
        "button",
        more.rect,
        None,
    );
    let picked = egui::Popup::menu(&more).show(outline_menu);
    if let Some(click) = picked.and_then(|menu| menu.inner) {
        outline.click = Some(click);
    }
}

/// The outline's menu. Returns what was picked this frame.
fn outline_menu(ui: &mut egui::Ui) -> Option<OutlineClick> {
    let items = [
        ("Expand All", OutlineClick::ExpandAll),
        ("Collapse All", OutlineClick::CollapseAll),
    ];
    sidebar::menu_text(ui);
    let mut picked = None;
    for (text, click) in items {
        if ui.button(text).clicked() {
            picked = Some(click);
        }
    }
    if picked.is_some() {
        ui.close();
    }
    picked
}

/// Distance from the top of the list to the top of its row `row`.
#[expect(
    clippy::cast_precision_loss,
    reason = "a document has far fewer headings than f32 counts exactly, and the result only aims a scroll"
)]
fn row_top(row: usize) -> f32 {
    row as f32 * sidebar::ROW_HEIGHT
}

/// The scrollable list of headers that no collapsed ancestor hides.
fn outline_rows(ui: &mut egui::Ui, tab: &Tab, outline: &mut OutlineUi<'_>) {
    // Pre-compute the visible header indices (skip those hidden
    // by collapsed ancestors). This lets us virtualize via
    // show_rows, paying O(visible-on-screen) instead of
    // O(total-headers) per frame. On a 100k-line doc with ~15k
    // headers this is the difference between visibly laggy and
    // smooth outline interactions.
    let visible_indices = tab.outline.visible();
    let show_fold_column = tab.outline.has_folds();
    // The row that stands for the heading the reader is in: the heading
    // itself, or the folded heading that hides it.
    let current_row = outline.current.and_then(|current| {
        visible_indices
            .partition_point(|&index| index <= current)
            .checked_sub(1)
    });
    outline.current = current_row.map(|row| visible_indices[row]);
    let follow_row = current_row.filter(|_| outline.follow);

    // Rows touch, so that the wash under the pointer never falls between
    // two of them; show_rows adds this spacing to the row height.
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("outline")
        .show_rows(
            ui,
            sidebar::ROW_HEIGHT,
            visible_indices.len(),
            |ui, row_range| {
                if let Some(row) = follow_row {
                    // The list starts `row_range.start` rows above this
                    // ui, which holds only the rows on screen.
                    let top = ui.max_rect().top() + row_top(row) - row_top(row_range.start);
                    let rows = top..=top + sidebar::ROW_HEIGHT;
                    let rect = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), rows);
                    ui.scroll_to_rect(rect, None);
                }
                for &index in &visible_indices[row_range] {
                    let row = OutlineRow {
                        index,
                        header: &tab.outline.headers()[index],
                        fold: Fold::of(tab, index, show_fold_column),
                    };
                    outline_row(ui, &row, outline);
                }
            },
        );
}

fn outline_row(ui: &mut egui::Ui, row: &OutlineRow<'_>, outline: &mut OutlineUi<'_>) {
    let shown = Row {
        text: &row.header.title,
        // Indent based on header level (h2 = 0, h3 = 1 indent, etc.)
        indent: f32::from(row.header.level.saturating_sub(2)) * INDENT,
        current: outline.current == Some(row.index),
        chevron: row.fold.chevron(),
    };
    let laid_out = sidebar::row(ui, outline.palette, &shown);

    mcp::rect(
        outline.mcp,
        format_args!("Header: {}", row.header.title),
        "header",
        laid_out.response.rect,
        Some(&format_args!("h{}", row.header.level)),
    );

    if !outline.is_dragging && laid_out.response.clicked() {
        outline.click = Some(OutlineClick::JumpTo {
            source_start: row.header.source_start,
            line_number: row.header.line_number,
        });
    }
    fold_control(ui, row, laid_out.chevron, outline);
}

/// Makes the chevron of a row that has children a control of its own, laid
/// over the row: a click on it folds, a click beside it jumps.
fn fold_control(
    ui: &egui::Ui,
    row: &OutlineRow<'_>,
    chevron: egui::Rect,
    outline: &mut OutlineUi<'_>,
) {
    let Some(verb) = row.fold.verb() else {
        return;
    };
    let rect = chevron.expand2(egui::vec2(FOLD_REACH, 0.0));
    let response = ui.interact(
        rect,
        ui.id().with(("fold", row.index)),
        egui::Sense::click(),
    );
    // The chevron is painted, not a widget with text, so it names itself
    // for screen readers.
    response.widget_info(|| {
        let name = format!("{verb} {}", row.header.title);
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
    });

    let state = if row.fold == Fold::Collapsed {
        "collapsed"
    } else {
        "expanded"
    };
    mcp::rect(
        outline.mcp,
        format_args!("Toggle: {}", row.header.title),
        "button",
        rect,
        Some(&state),
    );

    if !outline.is_dragging && response.clicked() {
        outline.click = Some(OutlineClick::ToggleFold(row.index));
    }
}

/// Schedules the scroll that brings the clicked header into view.
fn jump_to_header(tab: &mut Tab, source_start: usize, line_number: usize) {
    // The renderer records the same source-stable key, so formatting
    // and duplicate display titles cannot redirect the click.
    let key = header_position_key(source_start);
    let jump = header_jump(
        tab.cache.get_header_position(&key),
        line_number,
        tab.content_lines,
        tab.last_content_height,
    );
    tab.pending_scroll_offset = Some(jump.scroll_offset);
    if jump.is_estimate {
        // The estimate only gets us near the target. Request one
        // complete paint so the corrective pass can measure it.
        tab.pending_header_click_key = Some(key);
    }
}

/// Picks the scroll offset for a header: its measured position when the
/// renderer has painted it, otherwise an estimate from its line number.
/// Before the document is laid out that estimate is the top.
fn header_jump(
    measured_y: Option<f32>,
    line_number: usize,
    content_lines: usize,
    content_height: f32,
) -> HeaderJump {
    let estimated_y = line_fraction(line_number, content_lines) * content_height;
    let header_y = measured_y.unwrap_or(estimated_y);
    HeaderJump {
        scroll_offset: (header_y - HEADER_SCROLL_MARGIN).max(0.0),
        is_estimate: measured_y.is_none(),
    }
}
