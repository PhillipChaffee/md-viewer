//! The tabs of the top row: one flat text tab for each open document,
//! named by the document's first heading, else by its file. They scroll
//! sideways where they do not all fit, and the new-tab button comes after
//! the last of them.

use std::fmt;
use std::path::Path;

use eframe::egui::{self, Rangef};

use crate::app::action::AppAction;
use crate::app::chrome::{self, MenuRow, QuietButton, BAR_HEIGHT, BUTTON_SIDE};
use crate::app::top_bar::{new_tab_span, part, pick, tabs_width};
use crate::app::MarkdownApp;
use crate::icons::Icon;
use crate::theme::Palette;

#[cfg(test)]
pub(super) mod tests;

/// Space between a tab's edge and its title.
const TAB_PADDING: f32 = 10.0;
/// Side of the square a tab keeps free for its close button, so the tabs
/// do not shift when the button appears.
const CLOSE_SIDE: f32 = 18.0;
/// Side of the cross in that square.
const CLOSE_ICON: f32 = 12.0;
/// Thickness of the line under the active tab.
const UNDERLINE: f32 = 2.0;
/// The least and the most a tab's name is given. A shorter name is
/// padded, so no tab is too small to hit; a longer one is cut short with
/// an ellipsis, so one document cannot take the row.
const NAME_WIDTH: Rangef = Rangef {
    min: 26.0,
    max: 162.0,
};
/// Diameter of the dot that stands for live reload in a tab's hover text.
const LIVE_DOT: f32 = 6.0;
const CONTEXT_MENU_WIDTH: f32 = 140.0;

/// What the user asked of the tabs during one frame.
///
/// The fields are independent because a primary click (activate) and a
/// middle click (close) can land in the same frame.
#[derive(Default)]
pub(crate) struct TabPicks {
    activate: Option<usize>,
    close: Option<usize>,
    close_others: Option<usize>,
}

/// One tab as the row draws it.
struct TabLabel<'a> {
    index: usize,
    title: &'a str,
    path: &'a Path,
    is_active: bool,
    /// Whether the document is reloaded when its file changes.
    live: bool,
}

impl MarkdownApp {
    /// The tabs, scrolled inside `room`, and the new-tab button after
    /// them. Returns how far right the two reach, or `None` where the
    /// room is too small for the button: it then holds no tab either.
    pub(crate) fn tabs_in_row(
        &mut self,
        ui: &mut egui::Ui,
        bar: egui::Rect,
        room: Rangef,
        picked: &mut Option<AppAction>,
        picks: &mut TabPicks,
    ) -> Option<f32> {
        new_tab_span(room, 0.0)?;
        // A tab that came to the front is scrolled into view once. After
        // that the tabs are the reader's to scroll.
        let front = self.front();
        let front = front.and_then(|front| Some((front, self.tabs.get(front)?.id)));
        let reveal = self.revealed_tab != front;
        self.revealed_tab = front;

        let from_left = egui::Layout::left_to_right(egui::Align::Center);
        let tabs = Rangef::new(room.min, room.min + tabs_width(room));
        let shown = ui
            .scope_builder(part(bar, tabs, from_left), |ui| {
                self.tab_strip(ui, picks, reveal)
            })
            .inner;
        let button = new_tab_span(room, shown)?;
        ui.scope_builder(part(bar, button, from_left), |ui| {
            self.new_tab_button(ui, picked);
        });
        Some(button.max)
    }

    /// The horizontally scrollable row of tabs, scrolled to the active one
    /// when `reveal` is set. Returns the width the tabs show in.
    fn tab_strip(&self, ui: &mut egui::Ui, picks: &mut TabPicks, reveal: bool) -> f32 {
        if self.tabs.is_empty() {
            return 0.0;
        }
        let palette = self.theme.palette();
        let live = self.live_reload.is_live();
        ui.spacing_mut().item_spacing.x = 0.0;

        let scrolled = egui::ScrollArea::horizontal()
            .max_width(ui.available_width())
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                // The tabs go straight into the scrolled row: a row of
                // their own inside it would be centered at its starting
                // height and then grow downward, off the bar.
                for (index, tab) in self.tabs.iter().enumerate() {
                    let label = TabLabel {
                        index,
                        title: tab.name(),
                        path: &tab.path,
                        is_active: self.front() == Some(index),
                        live,
                    };
                    let shown = tab_label(ui, &label, palette, picks);
                    if reveal && label.is_active {
                        scroll_into_strip(ui, shown.rect);
                    }
                }
                ui.add_space(pixel(ui));
            });
        scrolled.content_size.x.min(scrolled.inner_rect.width())
    }

    /// A new tab starts on the home page, which lists what to open in
    /// it. There is no empty tab: the tab is there once a document is.
    fn new_tab_button(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let new_tab = QuietButton::icon(Icon::Plus, "New tab").show(ui, self.theme.palette());
        let new_tab = self.hints.tooltip(new_tab, "New Tab", AppAction::NewTab);
        pick(&new_tab, AppAction::NewTab, picked);
    }

    /// Carry out what the tabs were asked this frame: a tab comes to the
    /// front, then tabs close. Returns whether anything was asked.
    pub(crate) fn apply_tab_picks(&mut self, picks: &TabPicks) -> bool {
        if let Some(index) = picks.activate {
            self.focus_tab(index);
        }
        if let Some(keep) = picks.close_others {
            self.request_close_other_tabs(keep);
        } else if let Some(index) = picks.close {
            self.request_close_tab(index);
        }
        picks
            .activate
            .or(picks.close)
            .or(picks.close_others)
            .is_some()
    }
}

/// One screen pixel, in points. egui sets the scrolled row on whole pixels,
/// so a tab scrolled to an end of the row can land up to half a pixel
/// past it. A pixel to spare, after the last tab and around a tab that is
/// scrolled to, keeps the tab wholly in view.
fn pixel(ui: &egui::Ui) -> f32 {
    ui.pixels_per_point().recip()
}

/// Scroll the strip the least way that brings the tab at `tab` wholly into
/// it. egui takes a widget for in view once it is in the clip rect, which
/// reaches past each end of the strip by the clip margin.
///
/// A tab that is in view already is left alone: asking egui to scroll to
/// where it is would still start an animation, and frames to run it.
fn scroll_into_strip(ui: &egui::Ui, tab: egui::Rect) {
    let margin = ui.visuals().clip_rect_margin;
    let shown = ui.clip_rect().x_range().shrink(margin);
    if tab.left() < shown.min || tab.right() > shown.max {
        ui.scroll_to_rect(tab.expand2(egui::vec2(margin + pixel(ui), 0.0)), None);
    }
}

/// The hover text of a tab: where its document is, and whether it is
/// reloaded when the file changes.
fn tab_hover(ui: &mut egui::Ui, label: &TabLabel<'_>, palette: &Palette) {
    ui.style_mut().override_font_id = Some(chrome::label_font());
    ui.label(label.path.display().to_string());
    if label.live {
        ui.horizontal(|ui| {
            let (dot, _) =
                ui.allocate_exact_size(egui::Vec2::splat(LIVE_DOT), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot.center(), LIVE_DOT / 2.0, palette.muted);
            chrome::small_label(ui, palette, "Live: reloads when the file changes");
        });
    }
}

/// Draws one tab: its name, the line under it while it is the active one,
/// the focus ring while the keyboard is on it, the close button while the
/// pointer is over the tab or the keyboard is on either, its hover text
/// and its context menu. Returns the tab's response.
fn tab_label(
    ui: &mut egui::Ui,
    label: &TabLabel<'_>,
    palette: &'static Palette,
    actions: &mut TabPicks,
) -> egui::Response {
    let mut job = egui::text::LayoutJob::simple_singleline(
        label.title.to_owned(),
        chrome::label_font(),
        egui::Color32::PLACEHOLDER,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(NAME_WIDTH.max);
    let galley = ui.painter().layout_job(job);
    let name_width = galley.size().x.max(NAME_WIDTH.min);
    let width = 2.0_f32.mul_add(TAB_PADDING, name_width) + CLOSE_SIDE;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, BAR_HEIGHT), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            label.is_active,
            label.title,
        )
    });
    // egui has no tab among its widget types: a tab is told apart from a
    // heading or a sidebar row of the same words by its role.
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Tab);
    });

    // The close button is there whether or not it is drawn, so the Tab key
    // finds it right after its tab. A button made only once its tab had the
    // focus would come too late: egui has moved on to the next tab by then.
    let close = QuietButton::icon_sized(Icon::Close, CLOSE_ICON, CloseName(label.title));
    let button = close.place(ui, close_square(rect), response.id.with("close"));

    // The pointer counts as over the tab while it is over the close button
    // too, which takes the hover for itself.
    let pointed = ui.rect_contains_pointer(rect) || response.has_focus();
    let color = chrome::ink(palette, true, label.is_active || pointed);
    let title_corner = egui::pos2(
        rect.left() + TAB_PADDING,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(title_corner, galley, color);
    if label.is_active {
        let line = egui::Rect::from_min_max(
            egui::pos2(rect.left() + TAB_PADDING, rect.bottom() - UNDERLINE),
            egui::pos2(rect.right() - CLOSE_SIDE - TAB_PADDING, rect.bottom()),
        );
        ui.painter().rect_filled(line, 0.0, palette.accent);
    }
    // As tall as the ring of the buttons beside the tabs, so it clears the
    // line under the active tab and the edge of the bar.
    let ring = rect.shrink2(egui::vec2(0.0, (BAR_HEIGHT - BUTTON_SIDE) / 2.0));
    chrome::focus_ring(ui, palette, &response, ring);
    if pointed || button.has_focus() {
        close.paint(ui, palette, &button);
    }

    if response.clicked() {
        actions.activate = Some(label.index);
    }
    // A middle click on the tab closes it, as its close button does.
    if response.middle_clicked() || button.clicked() {
        actions.close = Some(label.index);
    }

    egui::Popup::context_menu(&response)
        .style(chrome::popup_style(palette))
        .show(|ui| tab_context_menu(ui, label, palette, actions));
    response.on_hover_ui(|ui| tab_hover(ui, label, palette))
}

/// The square a tab keeps free at its right end for its close button.
fn close_square(tab: egui::Rect) -> egui::Rect {
    let center = egui::pos2(
        tab.right() - TAB_PADDING / 2.0 - CLOSE_SIDE / 2.0,
        tab.center().y,
    );
    egui::Rect::from_center_size(center, egui::Vec2::splat(CLOSE_SIDE))
}

/// What a screen reader calls the close button of the tab named: every tab
/// has one, so each says whose it is. The words are put together when they
/// are asked for, and a frame nothing listens to asks for none.
#[derive(Clone, Copy)]
struct CloseName<'a>(&'a str);

impl fmt::Display for CloseName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Close {}", self.0)
    }
}

fn tab_context_menu(
    ui: &mut egui::Ui,
    label: &TabLabel<'_>,
    palette: &Palette,
    actions: &mut TabPicks,
) {
    ui.set_width(CONTEXT_MENU_WIDTH);
    if MenuRow::new("Close").flush().show(ui, palette).clicked() {
        actions.close = Some(label.index);
    }
    if MenuRow::new("Close Others")
        .flush()
        .show(ui, palette)
        .clicked()
    {
        actions.close_others = Some(label.index);
    }
}
