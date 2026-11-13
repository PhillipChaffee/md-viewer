//! What the bars, menus and popovers are built from: quiet buttons, a
//! segmented control, menu rows and hairlines.
//!
//! Everything here is set in fixed sizes rather than in the reading text
//! size, so the controls that change the text do not move under the
//! pointer while they are used.

use eframe::egui::{self, Color32};

use crate::icons::Icon;
use crate::theme::Palette;

#[cfg(test)]
mod tests;

/// Height of the top bar.
pub(super) const BAR_HEIGHT: f32 = 40.0;
/// Height of the slimmer bars under it: the tab strip, the find bar and
/// the error bar.
pub(super) const STRIP_HEIGHT: f32 = 34.0;
/// Size of the text of the chrome.
pub(super) const LABEL_SIZE: f32 = 13.0;
/// Size of its secondary text: shortcuts, counters and section labels.
pub(super) const SMALL_SIZE: f32 = 11.5;
/// Side of the square a quiet button takes at least.
pub(super) const BUTTON_SIDE: f32 = 28.0;
/// Side of the square an icon is painted in.
const ICON_SIDE: f32 = 16.0;
/// Space between the text of a quiet button and its edge.
const BUTTON_PADDING: f32 = 7.0;
const CONTROL_RADIUS: u8 = 6;

const MENU_ROW_HEIGHT: f32 = 28.0;
/// Space between the edge of a menu row and what it holds.
const MENU_ROW_PADDING: f32 = 10.0;
/// Width of the column the check marks of a menu stand in.
const MENU_GUTTER: f32 = 20.0;
/// Where the label of a menu row starts, for a row laid out by hand.
pub(super) const MENU_LABEL_INSET: f32 = MENU_ROW_PADDING + MENU_GUTTER;
const MENU_MARK_SIDE: f32 = 12.0;

const SEGMENT_HEIGHT: f32 = 24.0;
const SEGMENT_PADDING: f32 = 12.0;
/// Space between the track of a segmented control and its segments.
const SEGMENT_INSET: f32 = 2.0;

pub(super) const fn label_font() -> egui::FontId {
    egui::FontId::proportional(LABEL_SIZE)
}

pub(super) const fn small_font() -> egui::FontId {
    egui::FontId::proportional(SMALL_SIZE)
}

/// The frame of a bar: the page color edge to edge. The hairline under it
/// is the panel's own separator.
pub(super) fn bar_frame(palette: &Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.page)
        .inner_margin(egui::Margin::symmetric(10, 0))
}

/// The look of a menu or popover on `palette`: a rounded sheet with a soft
/// shadow, set in the chrome's own text size. The colors of the sheet come
/// from the theme's visuals.
pub(super) fn popup_style(
    palette: &'static Palette,
) -> impl Fn(&mut egui::Style) + Send + Sync + 'static {
    move |style| {
        style.spacing.menu_margin = egui::Margin::same(6);
        style.spacing.item_spacing = egui::vec2(6.0, 0.0);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(10);
        style.visuals.popup_shadow.offset = [0, 6];
        style.visuals.popup_shadow.blur = 22;
        style.visuals.popup_shadow.spread = 0;
        style.visuals.window_fill = palette.surface;
        style.visuals.window_stroke = egui::Stroke::new(1.0_f32, palette.rule);
        style.override_font_id = Some(label_font());
    }
}

/// What a menu or popover senses with its own surface: clicks, so they do
/// not reach what lies under it. Unlike egui's default it is no stop for
/// the Tab key, which would otherwise rest once on nothing visible between
/// a button and the first control of the popup it opened.
pub(super) const POPUP_SENSE: egui::Sense = egui::Sense::CLICK;

/// Whether the popup anchored to the widget `ui` lays out next is open, so
/// that widget can be drawn lit.
pub(super) fn next_widget_popup_is_open(ui: &egui::Ui) -> bool {
    egui::Popup::is_id_open(ui.ctx(), ui.next_auto_id().with("popup"))
}

/// The ink of a control: muted at rest, the text color once it is pointed
/// at, focused or lit, and faded while it cannot be used.
fn ink(palette: &Palette, enabled: bool, strong: bool) -> Color32 {
    if !enabled {
        palette.muted.gamma_multiply(0.4)
    } else if strong {
        palette.text
    } else {
        palette.muted
    }
}

/// The wash under a pressed control and the ring around a focused one.
fn paint_feedback(ui: &egui::Ui, response: &egui::Response, palette: &Palette) {
    if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(response.rect, CONTROL_RADIUS, palette.raised);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            CONTROL_RADIUS,
            egui::Stroke::new(1.0_f32, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
}

/// What a quiet button shows.
#[derive(Clone, Copy, Debug)]
enum Face<'a> {
    Icon(Icon),
    /// Text at this size.
    Text(&'a str, f32),
}

/// A button with no outline and no fill: an icon or a word in the muted
/// color, which turns to the text color under the pointer.
#[derive(Clone, Copy, Debug)]
pub(super) struct QuietButton<'a> {
    face: Face<'a>,
    /// What a screen reader calls it.
    label: &'a str,
    enabled: bool,
    lit: bool,
    min_width: f32,
    /// Space kept between the text of its face and each of its edges.
    padding: f32,
}

impl<'a> QuietButton<'a> {
    pub(super) const fn icon(icon: Icon, label: &'a str) -> Self {
        Self {
            face: Face::Icon(icon),
            label,
            enabled: true,
            lit: false,
            min_width: BUTTON_SIDE,
            padding: BUTTON_PADDING,
        }
    }

    /// A button that reads `text` and is called that by a screen reader.
    pub(super) const fn text(text: &'a str) -> Self {
        Self::text_sized(text, LABEL_SIZE)
    }

    /// A button that reads `text` set at `size` px.
    pub(super) const fn text_sized(text: &'a str, size: f32) -> Self {
        Self {
            face: Face::Text(text, size),
            label: text,
            enabled: true,
            lit: false,
            min_width: BUTTON_SIDE,
            padding: BUTTON_PADDING,
        }
    }

    /// Keep no room around its text beyond the least width: for a glyph
    /// that stands in a row of icons and has to take the square they take.
    pub(super) const fn snug(mut self) -> Self {
        self.padding = 0.0;
        self
    }

    /// Make it at least `width` px wide, for text that changes while its
    /// neighbors should stay put.
    pub(super) const fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// Call it `label` where the text on its face does not say what it does.
    pub(super) const fn labelled(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    pub(super) const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draw it in the text color at rest: what it opens is open.
    pub(super) const fn lit(mut self, lit: bool) -> Self {
        self.lit = lit;
        self
    }

    pub(super) fn show(self, ui: &mut egui::Ui, palette: &Palette) -> egui::Response {
        let galley = match self.face {
            Face::Icon(_) => None,
            Face::Text(text, size) => Some(ui.painter().layout_no_wrap(
                text.to_owned(),
                egui::FontId::proportional(size),
                Color32::PLACEHOLDER,
            )),
        };
        let text_width = galley.as_ref().map_or(0.0, |galley| galley.size().x);
        let size = egui::vec2(
            2.0_f32
                .mul_add(self.padding, text_width)
                .max(self.min_width),
            BUTTON_SIDE,
        );
        let sense = if self.enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let response = ui.allocate_response(size, sense);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, self.enabled, self.label)
        });
        paint_feedback(ui, &response, palette);
        let strong = self.lit || response.hovered() || response.has_focus();
        let color = ink(palette, self.enabled, strong);
        if let Face::Icon(icon) = self.face {
            let square =
                egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(ICON_SIDE));
            icon.paint(ui.painter(), square, color);
        }
        if let Some(galley) = galley {
            let corner = response.rect.center() - galley.size() / 2.0;
            ui.painter().galley(corner, galley, color);
        }
        response
    }
}

/// A one-pixel rule across the width `ui` has left, with a little air
/// above and below.
pub(super) fn hairline(ui: &mut egui::Ui, palette: &Palette) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 9.0), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0_f32, palette.rule),
    );
}

/// A row as tall as a quiet button, with whatever it holds centered on the
/// button's middle. A plain row starts at the height of a line of text and
/// leaves a label above the middle of a taller button beside it.
pub(super) fn control_row<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        ui.set_height(BUTTON_SIDE);
        add_contents(ui)
    })
    .inner
}

/// A line of small muted text: a section label, a counter, a readout.
pub(super) fn small_label(ui: &mut egui::Ui, palette: &Palette, text: &str) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .font(small_font())
                .color(palette.muted),
        )
        .selectable(false),
    )
}

/// What stands at the right end of a menu row.
#[derive(Clone, Copy, Debug)]
enum Trailing<'a> {
    Nothing,
    Shortcut(&'a str),
    /// The arrow of a row that opens a submenu.
    Submenu,
}

/// One row of a menu: a label, with a check mark before it while the
/// setting it toggles is on, and its shortcut or a submenu arrow after it.
#[derive(Clone, Copy, Debug)]
pub(super) struct MenuRow<'a> {
    pub(super) label: &'a str,
    /// `None` for a row that is not a toggle.
    checked: Option<bool>,
    trailing: Trailing<'a>,
    enabled: bool,
    /// Whether the label starts at the row's edge, with no check column.
    flush: bool,
}

impl<'a> MenuRow<'a> {
    pub(super) const fn new(label: &'a str) -> Self {
        Self {
            label,
            checked: None,
            trailing: Trailing::Nothing,
            enabled: true,
            flush: false,
        }
    }

    pub(super) const fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub(super) const fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.trailing = Trailing::Shortcut(shortcut);
        self
    }

    pub(super) const fn submenu(mut self) -> Self {
        self.trailing = Trailing::Submenu;
        self
    }

    pub(super) const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// For a menu none of whose rows is a toggle.
    pub(super) const fn flush(mut self) -> Self {
        self.flush = true;
        self
    }

    /// The state a toggle row reports to the MCP bridge.
    #[cfg(feature = "mcp")]
    pub(super) const fn mcp_value(&self) -> Option<&'static str> {
        match self.checked {
            Some(true) => Some("on"),
            Some(false) => Some("off"),
            None => None,
        }
    }

    pub(super) fn show(self, ui: &mut egui::Ui, palette: &Palette) -> egui::Response {
        let sense = if self.enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let response =
            ui.allocate_response(egui::vec2(ui.available_width(), MENU_ROW_HEIGHT), sense);
        response.widget_info(|| {
            self.checked.map_or_else(
                || egui::WidgetInfo::labeled(egui::WidgetType::Button, self.enabled, self.label),
                |checked| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        self.enabled,
                        checked,
                        self.label,
                    )
                },
            )
        });
        let row = response.rect;
        if self.enabled && (response.hovered() || response.has_focus()) {
            ui.painter()
                .rect_filled(row, CONTROL_RADIUS, palette.raised);
        }
        let painter = ui.painter();
        let label_color = ink(palette, self.enabled, true);
        let mut left = row.left() + MENU_ROW_PADDING;
        if !self.flush {
            if self.checked == Some(true) {
                let mark = egui::Rect::from_center_size(
                    egui::pos2(left + MENU_MARK_SIDE / 2.0, row.center().y),
                    egui::Vec2::splat(MENU_MARK_SIDE),
                );
                Icon::Check.paint(painter, mark, label_color);
            }
            left += MENU_GUTTER;
        }
        painter.text(
            egui::pos2(left, row.center().y),
            egui::Align2::LEFT_CENTER,
            self.label,
            label_font(),
            label_color,
        );
        let right = row.right() - MENU_ROW_PADDING;
        match self.trailing {
            Trailing::Nothing => {}
            Trailing::Shortcut(shortcut) => {
                painter.text(
                    egui::pos2(right, row.center().y),
                    egui::Align2::RIGHT_CENTER,
                    shortcut,
                    small_font(),
                    ink(palette, self.enabled, false),
                );
            }
            Trailing::Submenu => {
                let mark = egui::Rect::from_center_size(
                    egui::pos2(right - MENU_MARK_SIDE / 2.0, row.center().y),
                    egui::Vec2::splat(MENU_MARK_SIDE),
                );
                Icon::ChevronRight.paint(painter, mark, palette.muted);
            }
        }
        response
    }
}

/// A row of mutually exclusive choices on one track, the chosen one lifted
/// off it. Returns the choice the user switched to.
///
/// With `fill` the segments share the width `ui` has left; without it each
/// is as wide as its label.
pub(super) fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    palette: &Palette,
    options: &[(T, &str)],
    current: T,
    fill: bool,
) -> Option<T> {
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, label)| {
            ui.painter()
                .layout_no_wrap((*label).to_owned(), small_font(), Color32::PLACEHOLDER)
        })
        .collect();
    let natural =
        |galley: &std::sync::Arc<egui::Galley>| 2.0_f32.mul_add(SEGMENT_PADDING, galley.size().x);
    let shared = segment_share(ui.available_width(), options.len());
    let widths: Vec<f32> = galleys
        .iter()
        .map(|galley| if fill { shared } else { natural(galley) })
        .collect();
    let total = egui::vec2(
        2.0_f32.mul_add(SEGMENT_INSET, widths.iter().sum::<f32>()),
        2.0_f32.mul_add(SEGMENT_INSET, SEGMENT_HEIGHT),
    );
    let (track, track_response) = ui.allocate_exact_size(total, egui::Sense::hover());
    ui.painter()
        .rect_filled(track, CONTROL_RADIUS + 2, palette.raised);

    let mut switched = None;
    let mut left = track.left() + SEGMENT_INSET;
    for (index, ((value, label), galley)) in options.iter().zip(galleys).enumerate() {
        let segment = egui::Rect::from_min_size(
            egui::pos2(left, track.top() + SEGMENT_INSET),
            egui::vec2(widths[index], SEGMENT_HEIGHT),
        );
        left = segment.right();
        let chosen = *value == current;
        let response = ui.interact(segment, track_response.id.with(index), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, *label)
        });
        if chosen {
            ui.painter()
                .rect_filled(segment, CONTROL_RADIUS, palette.page);
        }
        paint_feedback(ui, &response, palette);
        let color = ink(palette, true, chosen || response.hovered());
        ui.painter()
            .galley(segment.center() - galley.size() / 2.0, galley, color);
        if response.clicked() && !chosen {
            switched = Some(*value);
        }
    }
    switched
}

/// The width of each of `count` segments sharing `available` px with the
/// track's inset.
#[expect(
    clippy::cast_precision_loss,
    reason = "a segmented control has a handful of segments, far inside f32's exact integers"
)]
fn segment_share(available: f32, count: usize) -> f32 {
    (2.0_f32.mul_add(-SEGMENT_INSET, available) / count.max(1) as f32).max(0.0)
}
