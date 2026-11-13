//! Hand-painted icons and affordances for the sidebars.

use eframe::egui;

#[cfg(test)]
mod tests;

/// Draw a visible affordance for a resizable sidebar's drag strip.
///
/// egui's `SidePanel` resize interaction paints nothing — the only hint is a
/// cursor change on hover — and the strip (`±SIDEBAR_RESIZE_GRAB_RADIUS`
/// around the panel border) sits over the content pane's edge, where users
/// reach for table column separators and scrollbars. Pressing there grabs the
/// sidebar resizer instead of the content, and the reflow reads as "the table
/// slid behind the sidebar". A line that brightens on hover and while
/// resizing makes the grab zone discoverable.
pub(crate) fn paint_sidebar_resize_affordance(
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
