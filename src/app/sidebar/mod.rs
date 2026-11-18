//! What the explorer and the outline share: the panel's frame, its one
//! hairline and its resize affordance, the section header, the rows, and the
//! small painted controls.

use eframe::egui;
use egui::text::{LayoutJob, TextFormat, TextWrapping};

use crate::layout::SIDEBAR_MIN_WIDTH;
use crate::theme::Palette;

#[cfg(test)]
mod tests;

/// Height of one row: the text and as much air again.
pub(super) const ROW_HEIGHT: f32 = 28.0;
const ROW_TEXT_SIZE: f32 = 13.0;
/// Corner radius of the hover wash and of the small buttons.
pub(super) const RADIUS: f32 = 6.0;
/// Space between a row's edge and what it shows.
const ROW_INSET: f32 = 8.0;
/// How far each level of a nested listing is pushed right of the one above
/// it.
pub(super) const INDENT: f32 = 14.0;
/// Width of the column a row's chevron sits in.
const CHEVRON_SLOT: f32 = 16.0;
/// Space between a chevron and the row's text.
const CHEVRON_GAP: f32 = 4.0;
const HEADER_HEIGHT: f32 = 30.0;
const CAPTION_SIZE: f32 = 11.0;
/// Extra space between a caption's letters, so capitals this small read.
const CAPTION_TRACKING: f32 = 1.0;
const DETAIL_SIZE: f32 = 12.0;
/// Side of the square the "more" button takes.
const MORE_BUTTON: f32 = 24.0;

/// Which edge of the window a sidebar is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

/// One sidebar: where it sits, what egui knows it as, and the space it
/// keeps around what it lists.
pub(super) struct Panel {
    pub(super) side: Side,
    /// Names the panel to egui, which keeps its dragged width under it.
    pub(super) id: &'static str,
    pub(super) margin: egui::Margin,
}

impl Panel {
    /// Show the sidebar, `width` wide until the reader drags its edge, with
    /// its hairline and its resize affordance on the edge it shares with the
    /// page. Returns what `add` returned and the width the sidebar took.
    pub(super) fn show<R>(
        &self,
        ctx: &egui::Context,
        palette: &Palette,
        width: f32,
        add: impl FnOnce(&mut egui::Ui) -> R,
    ) -> (R, f32) {
        let panel = match self.side {
            Side::Left => egui::SidePanel::left(self.id),
            Side::Right => egui::SidePanel::right(self.id),
        };
        let shown = panel
            .resizable(true)
            .show_separator_line(false)
            .default_width(width)
            .width_range(SIDEBAR_MIN_WIDTH..=f32::INFINITY)
            .frame(frame(palette, self.margin))
            .show(ctx, add);
        let rect = shown.response.rect;
        paint_inner_edge(ctx, rect, self.side, palette);
        let shared_edge = match self.side {
            Side::Left => rect.right(),
            Side::Right => rect.left(),
        };
        paint_resize_affordance(ctx, shared_edge, rect.y_range(), egui::Id::new(self.id));
        (shown.inner, rect.width())
    }
}

/// The frame both sidebars sit in: the surface color and no outline.
fn frame(palette: &Palette, inner_margin: egui::Margin) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.surface)
        .inner_margin(inner_margin)
}

/// The one line a sidebar is drawn with: a hairline down the edge it shares
/// with the page. `panel` is the sidebar's whole rectangle.
fn paint_inner_edge(ctx: &egui::Context, panel: egui::Rect, side: Side, palette: &Palette) {
    let x = match side {
        Side::Left => panel.right() - 0.5,
        Side::Right => panel.left() + 0.5,
    };
    ctx.layer_painter(egui::LayerId::background()).vline(
        x,
        panel.y_range(),
        egui::Stroke::new(1.0_f32, palette.rule),
    );
}

/// Draw a visible affordance for a resizable sidebar's drag strip.
///
/// egui's `SidePanel` resize interaction paints nothing — the only hint is a
/// cursor change on hover — and the strip (`±SIDEBAR_RESIZE_GRAB_RADIUS`
/// around the panel border) sits over the content pane's edge, where users
/// reach for table column separators and scrollbars. Pressing there grabs the
/// sidebar resizer instead of the content, and the reflow reads as "the table
/// slid behind the sidebar". A line that brightens on hover and while
/// resizing makes the grab zone discoverable.
fn paint_resize_affordance(
    ctx: &egui::Context,
    boundary_x: f32,
    y_range: egui::Rangef,
    panel_id: egui::Id,
) {
    // egui reads this same response at the top of `SidePanel::show`, so one
    // frame of latency is already inherent to hover feedback there.
    let Some(resize_response) = ctx.read_response(panel_id.with("__resize")) else {
        return;
    };
    let resizing = resize_response.dragged();
    let hover = resize_response.hovered();
    if !hover && !resizing {
        return;
    }
    let visuals = &ctx.style().visuals;
    let stroke = if resizing {
        visuals.widgets.active.bg_stroke
    } else {
        visuals.widgets.hovered.bg_stroke
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new(panel_id).with("resize_affordance"),
    ));
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(boundary_x - 1.0, y_range.min),
            egui::pos2(boundary_x + 1.0, y_range.max),
        ),
        0.0,
        stroke.color,
    );
}

/// `text` set as a section caption: small, and its letters a little apart.
/// The caller writes it in capitals.
pub(super) fn caption(text: &str, color: egui::Color32) -> LayoutJob {
    LayoutJob::single_section(
        text.to_owned(),
        TextFormat {
            font_id: egui::FontId::proportional(CAPTION_SIZE),
            extra_letter_spacing: CAPTION_TRACKING,
            color,
            ..Default::default()
        },
    )
}

/// The line at the top of a sidebar.
pub(super) struct Header<'a> {
    /// What the sidebar lists, in capitals.
    pub(super) caption: &'a str,
    /// What it lists them of, shown after the caption.
    pub(super) detail: Option<&'a str>,
    /// What a screen reader calls the "more" button. `None` leaves the
    /// button out.
    pub(super) menu: Option<&'a str>,
}

/// Draw a sidebar's header across the panel. Returns the "more" button, for
/// the caller to hang its menu on.
pub(super) fn header(
    ui: &mut egui::Ui,
    palette: &Palette,
    header: &Header<'_>,
) -> Option<egui::Response> {
    let size = egui::vec2(ui.available_width(), HEADER_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let more = header
        .menu
        .map(|label| more_button(ui, palette, rect, label));
    let right = more
        .as_ref()
        .map_or_else(|| rect.right(), |more| more.rect.left() - CHEVRON_GAP);

    response.widget_info(|| {
        let label = header.detail.map_or_else(
            || header.caption.to_owned(),
            |detail| format!("{} {detail}", header.caption),
        );
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, label)
    });
    let mut job = caption(header.caption, palette.muted);
    if let Some(detail) = header.detail {
        let format = TextFormat::simple(egui::FontId::proportional(DETAIL_SIZE), palette.muted);
        job.append(detail, ROW_INSET, format);
    }
    let left = rect.left() + ROW_INSET;
    job.wrap = one_line(right - left);
    let galley = ui.painter().layout_job(job);
    let origin = egui::pos2(left, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(origin, galley, palette.muted);
    more
}

/// Wrapping that keeps text on one line and cuts it off, with an ellipsis,
/// at `width`.
fn one_line(width: f32) -> TextWrapping {
    TextWrapping {
        max_width: width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    }
}

/// The three dots at the right end of a header, which open the sidebar's
/// menu.
fn more_button(
    ui: &egui::Ui,
    palette: &Palette,
    header: egui::Rect,
    label: &str,
) -> egui::Response {
    let center = egui::pos2(header.right() - MORE_BUTTON / 2.0, header.center().y);
    let rect = egui::Rect::from_center_size(center, egui::Vec2::splat(MORE_BUTTON));
    let response = ui.interact(rect, ui.id().with("more"), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let ink = if response.hovered() {
        ui.painter().rect_filled(rect, RADIUS, palette.raised);
        palette.text
    } else {
        palette.muted
    };
    for step in [-1.0_f32, 0.0, 1.0] {
        let dot = center + egui::vec2(step * 4.5, 0.0);
        ui.painter().circle_filled(dot, 1.25, ink);
    }
    response
}

/// Set what a sidebar's menu holds in the size of the sidebar's rows.
pub(super) fn menu_text(ui: &mut egui::Ui) {
    ui.style_mut().override_font_id = Some(egui::FontId::proportional(ROW_TEXT_SIZE));
}

/// What a row shows in its fold column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Chevron {
    /// The sidebar has no fold column.
    None,
    /// The column is kept, so that texts line up, but this row has nothing
    /// to fold.
    Blank,
    Closed,
    Open,
}

/// One line of a sidebar.
pub(super) struct Row<'a> {
    pub(super) text: &'a str,
    /// How far the row's content is pushed right, for nesting.
    pub(super) indent: f32,
    /// The row is where the reader is: the open file, the heading in view.
    pub(super) current: bool,
    pub(super) chevron: Chevron,
}

/// A row as laid out.
pub(super) struct RowResponse {
    /// The whole row, which takes clicks.
    pub(super) response: egui::Response,
    /// Where the chevron is, for a sidebar that makes it a control of its
    /// own.
    pub(super) chevron: egui::Rect,
}

/// Lay one row across the sidebar and paint it. The current row is inked
/// like body text and the others muted; the row under the pointer gets the
/// raised wash.
pub(super) fn row(ui: &mut egui::Ui, palette: &Palette, row: &Row<'_>) -> RowResponse {
    let size = egui::vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, row.text));
    let slot_left = rect.left() + ROW_INSET + row.indent;
    let chevron = egui::Rect::from_x_y_ranges(slot_left..=slot_left + CHEVRON_SLOT, rect.y_range());
    if !ui.is_rect_visible(rect) {
        return RowResponse { response, chevron };
    }

    let lit = response.contains_pointer();
    if lit {
        ui.painter().rect_filled(rect, RADIUS, palette.raised);
    }
    let ink = if lit || row.current {
        palette.text
    } else {
        palette.muted
    };
    let text_left = match row.chevron {
        Chevron::None => slot_left,
        Chevron::Blank => chevron.right() + CHEVRON_GAP,
        Chevron::Closed | Chevron::Open => {
            paint_chevron(ui.painter(), chevron.center(), row.chevron, ink);
            chevron.right() + CHEVRON_GAP
        }
    };
    let font = egui::FontId::proportional(ROW_TEXT_SIZE);
    let mut job = LayoutJob::single_section(row.text.to_owned(), TextFormat::simple(font, ink));
    job.wrap = one_line(rect.right() - ROW_INSET - text_left);
    let galley = ui.painter().layout_job(job);
    let elided = galley.elided;
    let origin = egui::pos2(text_left, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(origin, galley, ink);
    if elided {
        // The whole text, for a row too narrow to show it.
        response = response.on_hover_text(row.text);
    }
    RowResponse { response, chevron }
}

/// A thin chevron centered on `center`: pointing right for a folded row,
/// down for an open one.
fn paint_chevron(
    painter: &egui::Painter,
    center: egui::Pos2,
    chevron: Chevron,
    ink: egui::Color32,
) {
    let arm = 3.5;
    let half = arm / 2.0;
    let points = if chevron == Chevron::Open {
        [
            center + egui::vec2(-arm, -half),
            center + egui::vec2(0.0, half),
            center + egui::vec2(arm, -half),
        ]
    } else {
        [
            center + egui::vec2(-half, -arm),
            center + egui::vec2(half, 0.0),
            center + egui::vec2(-half, arm),
        ]
    };
    painter.line(points.to_vec(), egui::Stroke::new(1.3_f32, ink));
}
