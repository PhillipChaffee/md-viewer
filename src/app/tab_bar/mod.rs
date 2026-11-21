//! The tabs of the top row: one flat text tab for each open document,
//! named by the document's first heading, else by its file. They scroll
//! sideways where they do not all fit, and the new-tab button comes after
//! the last of them.

use std::path::Path;

use eframe::egui::{self, Rangef};

use crate::app::action::AppAction;
use crate::app::chrome::{self, MenuRow, QuietButton, BAR_HEIGHT};
use crate::app::disk_notice;
use crate::app::top_bar::{new_tab_span, part, pick, tabs_width};
use crate::app::MarkdownApp;
use crate::icons::Icon;
use crate::tab::DiskChange;
use crate::theme::Palette;

#[cfg(test)]
pub(super) mod tests;

/// Space between a tab's edge and its title.
const TAB_PADDING: f32 = 10.0;
/// Side of the square a tab keeps free for its close button, so the tabs
/// do not shift when the button appears.
const CLOSE_SIDE: f32 = 18.0;
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
    /// What became of the file under the document's unsaved edits, while
    /// the notice of it waits for an answer.
    disk: Option<DiskChange>,
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
                        disk: tab.disk_notice(),
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
    if let Some(change) = label.disk {
        chrome::small_label(ui, palette, disk_notice::message(change));
    }
}

/// Draws one tab: its name, the line under it while it is the active one,
/// the close button while the pointer is over it, its hover text and its
/// context menu. Returns the tab's response.
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

    if response.clicked() {
        actions.activate = Some(label.index);
    }
    // Middle-click to close
    if response.middle_clicked() {
        actions.close = Some(label.index);
    }
    if pointed {
        tab_close_button(ui, &response, label, palette, actions);
    } else if let Some(change) = label.disk {
        tab_disk_mark(ui, &response, change, palette);
    }

    egui::Popup::context_menu(&response)
        .style(chrome::popup_style(palette))
        .show(|ui| tab_context_menu(ui, label, palette, actions));
    response.on_hover_ui(|ui| tab_hover(ui, label, palette))
}

/// The square a tab keeps free at its right end.
fn end_square(tab: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        egui::pos2(
            tab.right() - TAB_PADDING / 2.0 - CLOSE_SIDE / 2.0,
            tab.center().y,
        ),
        egui::Vec2::splat(CLOSE_SIDE),
    )
}

/// The mark of a tab whose file changed under its unsaved edits: a dot in
/// the warning color where the close button shows while the tab is pointed
/// at, so the reader sees which tab waits for an answer when another is in
/// front.
fn tab_disk_mark(ui: &egui::Ui, tab: &egui::Response, change: DiskChange, palette: &Palette) {
    let square = end_square(tab.rect);
    let mark = ui.interact(square, tab.id.with("disk"), egui::Sense::hover());
    mark.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, disk_notice::mark(change))
    });
    ui.painter()
        .circle_filled(square.center(), chrome::WARNING_DOT / 2.0, palette.warning);
}

/// The close button in the square a tab keeps free at its right end.
fn tab_close_button(
    ui: &egui::Ui,
    tab: &egui::Response,
    label: &TabLabel<'_>,
    palette: &Palette,
    actions: &mut TabPicks,
) {
    let square = end_square(tab.rect);
    let close = ui.interact(square, tab.id.with("close"), egui::Sense::click());
    close.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close tab"));

    let color = chrome::ink(palette, true, close.hovered());
    Icon::Close.paint(ui.painter(), square.shrink(3.0), color);
    if close.clicked() {
        actions.close = Some(label.index);
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
