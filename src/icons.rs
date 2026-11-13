//! Hand-painted icons and affordances for the sidebars.

use eframe::egui;

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

/// Draw a small vector "document" icon (page with folded corner) for file
/// rows in the explorer. Pure painter drawing — no font or emoji involved.
pub(crate) fn paint_file_icon(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = egui::Stroke::new(1.2, ui.visuals().weak_text_color());
    let body = egui::Rect::from_min_max(
        rect.left_top() + egui::vec2(2.0, 1.0),
        rect.right_bottom() - egui::vec2(2.0, 1.0),
    );
    let fold = body.width() * 0.3;
    // Page outline: top edge stops at the fold, then a diagonal to the right.
    painter.line_segment(
        [
            egui::pos2(body.right_top().x - fold, body.top()),
            body.left_top(),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(body.right_top().x - fold, body.top()),
            body.right_top() + egui::vec2(0.0, fold),
        ],
        stroke,
    );
    painter.line_segment([body.left_top(), body.left_bottom()], stroke);
    painter.line_segment([body.left_bottom(), body.right_bottom()], stroke);
    painter.line_segment(
        [
            body.right_bottom(),
            body.right_top() + egui::vec2(0.0, fold),
        ],
        stroke,
    );
}

/// Draw a small vector folder icon for directory rows in the explorer.
/// An expanded folder is drawn slightly open (raised tab, lighter fill).
pub(crate) fn paint_folder_icon(ui: &mut egui::Ui, size: f32, expanded: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    let stroke = egui::Stroke::new(1.2, visuals.weak_text_color());
    let fill = if expanded {
        visuals.extreme_bg_color
    } else {
        visuals.faint_bg_color
    };
    let tab_w = rect.width() * 0.5;
    let tab_h = rect.height() * 0.25;
    let body_top = rect.top() + tab_h;
    // Tab (only spans the left half, like a real folder).
    painter.rect_filled(
        egui::Rect::from_min_max(
            rect.left_top() + egui::vec2(1.0, 1.0),
            egui::pos2(rect.left() + tab_w, body_top + 1.0),
        ),
        2.0,
        fill,
    );
    painter.rect_stroke(
        egui::Rect::from_min_max(
            rect.left_top() + egui::vec2(1.0, 1.0),
            egui::pos2(rect.left() + tab_w, body_top + 1.0),
        ),
        2.0,
        stroke,
        egui::StrokeKind::Middle,
    );
    // Body.
    let body = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 1.0, body_top),
        rect.right_bottom() - egui::vec2(1.0, 1.0),
    );
    painter.rect_filled(body, 2.0, fill);
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Middle);
}
