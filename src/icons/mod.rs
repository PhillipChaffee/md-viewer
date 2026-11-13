//! Hand-painted icons and affordances for the sidebars.

use eframe::egui;

#[cfg(test)]
mod tests;

/// The side of the square an [`Icon`] is designed on. Its marks are given
/// in these units and scaled to whatever square the icon is painted in.
const ICON_GRID: f32 = 16.0;
/// Stroke width of an [`Icon`], in grid units: thin enough to read as a
/// drawn line beside 13 px text.
const ICON_STROKE: f32 = 1.25;

/// One mark of an [`Icon`], in grid units.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mark {
    /// An open path through these points.
    Path(&'static [[f32; 2]]),
    /// A circle outline: center and radius.
    Ring([f32; 2], f32),
    /// A filled circle: center and radius.
    Dot([f32; 2], f32),
    /// A rounded rectangle outline: two corners and the corner radius.
    Frame([f32; 2], [f32; 2], f32),
}

/// A line icon of the chrome, painted as vector strokes so it takes the
/// ink of the theme and stays sharp at any zoom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    /// A window with a side panel: the file explorer.
    Sidebar,
    Back,
    Forward,
    Search,
    /// Indented lines: the outline of the document.
    Outline,
    /// Three small rings: everything else.
    More,
    Check,
    Close,
    ChevronUp,
    ChevronDown,
    ChevronRight,
    Plus,
    Minus,
    /// A ringed exclamation mark.
    Warning,
}

impl Icon {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 14] = [
        Self::Sidebar,
        Self::Back,
        Self::Forward,
        Self::Search,
        Self::Outline,
        Self::More,
        Self::Check,
        Self::Close,
        Self::ChevronUp,
        Self::ChevronDown,
        Self::ChevronRight,
        Self::Plus,
        Self::Minus,
        Self::Warning,
    ];

    const fn marks(self) -> &'static [Mark] {
        match self {
            Self::Sidebar => &[
                Mark::Frame([1.5, 2.5], [14.5, 13.5], 2.5),
                Mark::Path(&[[6.0, 2.5], [6.0, 13.5]]),
            ],
            Self::Back => &[
                Mark::Path(&[[13.0, 8.0], [3.0, 8.0]]),
                Mark::Path(&[[7.0, 4.0], [3.0, 8.0], [7.0, 12.0]]),
            ],
            Self::Forward => &[
                Mark::Path(&[[3.0, 8.0], [13.0, 8.0]]),
                Mark::Path(&[[9.0, 4.0], [13.0, 8.0], [9.0, 12.0]]),
            ],
            Self::Search => &[
                Mark::Ring([7.0, 7.0], 4.5),
                Mark::Path(&[[10.3, 10.3], [14.0, 14.0]]),
            ],
            Self::Outline => &[
                Mark::Path(&[[2.0, 4.0], [14.0, 4.0]]),
                Mark::Path(&[[5.0, 8.0], [14.0, 8.0]]),
                Mark::Path(&[[5.0, 12.0], [11.0, 12.0]]),
            ],
            Self::More => &[
                Mark::Ring([2.5, 8.0], 1.5),
                Mark::Ring([8.0, 8.0], 1.5),
                Mark::Ring([13.5, 8.0], 1.5),
            ],
            Self::Check => &[Mark::Path(&[[3.0, 8.5], [6.5, 12.0], [13.0, 4.5]])],
            Self::Close => &[
                Mark::Path(&[[4.0, 4.0], [12.0, 12.0]]),
                Mark::Path(&[[12.0, 4.0], [4.0, 12.0]]),
            ],
            Self::ChevronUp => &[Mark::Path(&[[4.0, 10.0], [8.0, 6.0], [12.0, 10.0]])],
            Self::ChevronDown => &[Mark::Path(&[[4.0, 6.0], [8.0, 10.0], [12.0, 6.0]])],
            Self::ChevronRight => &[Mark::Path(&[[6.0, 4.0], [10.0, 8.0], [6.0, 12.0]])],
            Self::Plus => &[
                Mark::Path(&[[8.0, 3.5], [8.0, 12.5]]),
                Mark::Path(&[[3.5, 8.0], [12.5, 8.0]]),
            ],
            Self::Minus => &[Mark::Path(&[[3.5, 8.0], [12.5, 8.0]])],
            Self::Warning => &[
                Mark::Ring([8.0, 8.0], 6.0),
                Mark::Path(&[[8.0, 4.5], [8.0, 9.0]]),
                Mark::Dot([8.0, 11.3], 0.7),
            ],
        }
    }

    /// Paint the icon in `color`, filling the largest square centered in
    /// `rect`.
    pub(crate) fn paint(self, painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
        let scale = rect.width().min(rect.height()) / ICON_GRID;
        let origin = rect.center() - egui::Vec2::splat(scale * ICON_GRID / 2.0);
        let at = |[x, y]: [f32; 2]| origin + egui::vec2(x, y) * scale;
        let stroke = egui::Stroke::new(ICON_STROKE * scale, color);
        for mark in self.marks() {
            match *mark {
                Mark::Path(points) => {
                    painter.add(egui::Shape::line(
                        points.iter().copied().map(at).collect(),
                        stroke,
                    ));
                }
                Mark::Ring(center, radius) => {
                    painter.circle_stroke(at(center), radius * scale, stroke);
                }
                Mark::Dot(center, radius) => {
                    painter.circle_filled(at(center), radius * scale, color);
                }
                Mark::Frame(min, max, radius) => {
                    painter.rect_stroke(
                        egui::Rect::from_min_max(at(min), at(max)),
                        radius * scale,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }
    }
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
