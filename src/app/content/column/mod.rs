//! The text column in the document pane: where it sits, the two strips that
//! resize it, and the width readout shown while one is dragged.

use eframe::egui;

use crate::layout::{centered_column_left, ReadingWidth, READING_COLUMN_MARGIN};
use crate::theme::Palette;

#[cfg(test)]
mod tests;

/// Width of the strip beside each column edge that takes the pointer. It
/// lies in the pane's margin, clear of the text, the sidebars' own resize
/// handles and the scrollbar.
const STRIP_WIDTH: f32 = 12.0;
/// How far outside the column edge its hairline is drawn, so the line does
/// not touch the text.
const HAIRLINE_OFFSET: f32 = 6.0;
/// Gap between the top of the pane and the width readout.
const READOUT_TOP: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Edge {
    Left,
    Right,
}

impl Edge {
    /// What a screen reader calls the strip.
    const fn label(self) -> &'static str {
        match self {
            Self::Left => "Resize text column, left edge",
            Self::Right => "Resize text column, right edge",
        }
    }

    /// The direction away from the column: -1 to the left, +1 to the right.
    const fn outward(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }
}

/// The text column as laid out this frame: `width` px, centered in `pane`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Column {
    pub(super) pane: egui::Rect,
    pub(super) width: f32,
}

impl Column {
    /// The column a reader who asked for `width` gets in `pane`.
    pub(super) fn in_pane(pane: egui::Rect, width: ReadingWidth) -> Self {
        Self {
            pane,
            width: width.resolve(pane.width()),
        }
    }

    /// Distance from the pane's left edge to the column's.
    pub(super) fn inset(self) -> f32 {
        centered_column_left(0.0, self.pane.width(), self.width)
    }

    /// The widest a figure may be shown: the pane less its margins, which
    /// is what the column itself can be widened to.
    pub(super) fn figure_room(self) -> f32 {
        2.0_f32
            .mul_add(-READING_COLUMN_MARGIN, self.pane.width())
            .max(self.width)
    }

    fn edge_x(self, edge: Edge) -> f32 {
        let left = self.pane.left() + self.inset();
        match edge {
            Edge::Left => left,
            Edge::Right => left + self.width,
        }
    }

    /// The two strips with their names, for the MCP bridge.
    pub(super) fn strips(self) -> [(&'static str, egui::Rect); 2] {
        [Edge::Left, Edge::Right].map(|edge| (edge.label(), self.strip(edge)))
    }

    /// The strip that resizes from `edge`: the pane's full height, just
    /// outside the column.
    fn strip(self, edge: Edge) -> egui::Rect {
        let inner = self.edge_x(edge);
        let outer = edge.outward().mul_add(STRIP_WIDTH, inner);
        egui::Rect::from_x_y_ranges(inner.min(outer)..=inner.max(outer), self.pane.y_range())
    }
}

/// Lay the two resize strips over the pane and report the width the user
/// asked for this frame: dragging an edge moves both edges about the pane's
/// center, and a double click goes back to the default width.
///
/// `grab` lasts across the frames of one drag: how far from the edge the
/// pointer took hold, so the edge does not jump to the pointer.
pub(super) fn resize_strips(
    ui: &egui::Ui,
    column: Column,
    grab: &mut Option<f32>,
    palette: &Palette,
) -> Option<ReadingWidth> {
    let mut requested = None;
    for edge in [Edge::Left, Edge::Right] {
        let response = ui
            .interact(
                column.strip(edge),
                ui.id().with(("reading-column-edge", edge)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, edge.label()));
        requested = edge_request(ui, &response, column, edge, grab).or(requested);
        if grab.is_none() && response.hovered() {
            paint_hairline(ui, column, edge, palette);
        }
    }
    if grab.is_some() {
        // Both edges move, so both show, and the readout says how far.
        for edge in [Edge::Left, Edge::Right] {
            paint_hairline(ui, column, edge, palette);
        }
        paint_width_readout(ui, column, palette);
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    requested
}

/// The width one strip's input asks for this frame, if any.
fn edge_request(
    ui: &egui::Ui,
    response: &egui::Response,
    column: Column,
    edge: Edge,
    grab: &mut Option<f32>,
) -> Option<ReadingWidth> {
    if response.double_clicked() {
        return Some(ReadingWidth::DEFAULT);
    }
    if response.drag_started() {
        // Where the button went down, not where the pointer is by the time
        // egui calls it a drag: the edge then tracks the pointer exactly.
        let origin = ui.input(|input| input.pointer.press_origin());
        *grab = origin.map(|origin| origin.x - column.edge_x(edge));
    }
    if response.drag_stopped() {
        *grab = None;
        return None;
    }
    let (pointer, offset) = response.interact_pointer_pos().zip(*grab)?;
    response
        .dragged()
        .then(|| ReadingWidth::from_edge(pointer.x - offset, column.pane.center().x))
}

/// A hairline just outside `edge`, the pane's full height. It is the muted
/// color at half strength: present enough to aim at, quiet beside the text.
fn paint_hairline(ui: &egui::Ui, column: Column, edge: Edge, palette: &Palette) {
    ui.painter().vline(
        edge.outward().mul_add(HAIRLINE_OFFSET, column.edge_x(edge)),
        column.pane.y_range(),
        egui::Stroke::new(1.0_f32, palette.muted.gamma_multiply(0.5)),
    );
}

/// The column width in px, in a pill at the top of the column.
fn paint_width_readout(ui: &egui::Ui, column: Column, palette: &Palette) {
    let galley = ui.painter().layout_no_wrap(
        format!("{:.0} px", column.width),
        egui::FontId::proportional(12.0),
        palette.text,
    );
    let pill = egui::Rect::from_center_size(
        egui::pos2(
            column.pane.center().x,
            column.pane.top() + READOUT_TOP + galley.size().y / 2.0,
        ),
        galley.size() + egui::vec2(20.0, 8.0),
    );
    ui.painter().rect(
        pill,
        egui::CornerRadius::same(u8::MAX),
        palette.surface,
        egui::Stroke::new(1.0_f32, palette.rule),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(pill.center() - galley.size() / 2.0, galley, palette.text);
}
