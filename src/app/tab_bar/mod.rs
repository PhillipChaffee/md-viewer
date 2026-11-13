//! The tab strip: a row of flat text tabs, shown only while there is
//! something to choose between. A lone document's edit controls go in the
//! top bar instead, so starting an edit does not move the page.

use std::time::Duration;

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, MenuRow, QuietButton, STRIP_HEIGHT};
use crate::app::explorer_panel::FLASH_DURATION_MS;
use crate::app::mcp::{self, Mcp};
use crate::app::MarkdownApp;
use crate::icons::Icon;
use crate::tab::{SaveOutcome, Tab};
use crate::theme::Palette;

#[cfg(test)]
mod tests;

/// How long a save's outcome stays beside the mode toggle.
const SAVE_FLASH: Duration = Duration::from_millis(FLASH_DURATION_MS * 3);

/// Space between a tab's edge and its title.
const TAB_PADDING: f32 = 10.0;
/// Side of the square a tab keeps free for its close button, so the tabs
/// do not shift when the button appears.
const CLOSE_SIDE: f32 = 18.0;
/// Thickness of the line under the active tab.
const UNDERLINE: f32 = 2.0;
/// Width the strip keeps for the open-file button after the tabs.
const OPEN_BUTTON_ROOM: f32 = 40.0;
const CONTEXT_MENU_WIDTH: f32 = 140.0;
const UNSAVED_DOT: f32 = 7.0;

/// What the user asked of the tab strip during one frame.
///
/// The fields are independent because a primary click (activate) and a
/// middle click (close) can land in the same frame.
#[derive(Default)]
struct TabStripActions {
    activate: Option<usize>,
    close: Option<usize>,
    close_others: Option<usize>,
}

/// One tab as the strip draws it.
struct TabLabel<'a> {
    index: usize,
    title: &'a str,
    is_active: bool,
}

/// Whether `tab` has something to show beside the mode toggle even when it
/// is the only tab: an edit in progress, unsaved changes, or a save's
/// outcome.
fn has_edit_state(tab: &Tab) -> bool {
    tab.is_editing || tab.is_dirty() || tab.save_flash.is_some()
}

impl MarkdownApp {
    /// Whether the strip earns its row: with one document open there is
    /// nothing to choose.
    const fn shows_tab_bar(&self) -> bool {
        self.tabs.len() > 1
    }

    /// The edit controls of a lone document, which has no strip to hold
    /// them, laid out in the top bar from the right end of `room`. Returns
    /// their left edge when there are any.
    pub(crate) fn lone_document_edit_controls(
        &mut self,
        ui: &mut egui::Ui,
        room: egui::Rect,
    ) -> Option<f32> {
        let palette = self.theme.palette();
        let [tab] = self.tabs.as_mut_slice() else {
            return None;
        };
        if !has_edit_state(tab) {
            return None;
        }
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        let builder = egui::UiBuilder::new().max_rect(room).layout(layout);
        let controls = ui.scope_builder(builder, |ui| edit_mode_controls(ui, tab, palette));
        Some(controls.response.rect.left())
    }

    /// Render the tab strip when it has something to show. Returns the
    /// index of a tab to close.
    pub(crate) fn render_tab_bar(&mut self, ctx: &egui::Context) -> Option<usize> {
        if !self.shows_tab_bar() {
            return None;
        }
        let palette = self.theme.palette();
        let mut actions = TabStripActions::default();
        // A tab that came to the front is scrolled into the strip once.
        // After that the strip is the reader's to scroll.
        let reveal = self.revealed_tab != Some(self.active_tab);
        self.revealed_tab = Some(self.active_tab);
        egui::TopBottomPanel::top("tab_bar")
            .exact_height(STRIP_HEIGHT)
            .frame(chrome::bar_frame(palette))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            edit_mode_controls(ui, tab, palette);
                        }
                        // The tabs and the open-file button take what the
                        // edit controls leave, and so stop short of them
                        // however many tabs there are.
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            self.tab_strip(ui, &mut actions, reveal);
                            self.open_file_button(ui);
                        });
                    });
                });
            });

        self.apply_tab_strip_actions(&actions)
    }

    /// The horizontally scrollable row of tabs, scrolled to the active one
    /// when `reveal` is set.
    fn tab_strip(&self, ui: &mut egui::Ui, actions: &mut TabStripActions, reveal: bool) {
        let palette = self.theme.palette();
        let titles: Vec<String> = self.tabs.iter().map(Tab::title).collect();

        egui::ScrollArea::horizontal()
            .max_width(ui.available_width() - OPEN_BUTTON_ROOM)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                // The tabs go straight into the scrolled row: a row of
                // their own inside it would be centered at its starting
                // height and then grow downward, off the strip.
                for (index, title) in titles.iter().enumerate() {
                    let label = TabLabel {
                        index,
                        title,
                        is_active: index == self.active_tab,
                    };
                    let tab = tab_label(ui, &label, palette, &self.mcp, actions);
                    if reveal && label.is_active {
                        scroll_into_strip(ui, tab.rect);
                    }
                }
                ui.add_space(pixel(ui));
            });
    }

    /// Opening a file is what creates a new tab; there is no empty-tab
    /// state.
    fn open_file_button(&mut self, ui: &mut egui::Ui) {
        let open = QuietButton::icon(Icon::Plus, "Open file in new tab")
            .show(ui, self.theme.palette())
            .on_hover_text("Open File… (Ctrl+T)");
        mcp::widget(
            &self.mcp,
            format_args!("Open File in New Tab"),
            "button",
            &open,
            None,
        );
        if open.clicked() {
            self.apply_action(ui.ctx(), AppAction::OpenFile);
        }
    }

    /// Applies the frame's tab strip actions. Returns the tab the caller
    /// should close; "Close Others" is completed here, so it returns `None`.
    fn apply_tab_strip_actions(&mut self, actions: &TabStripActions) -> Option<usize> {
        if let Some(index) = actions.activate {
            self.active_tab = index;
            self.title_dirty = true;
        }
        if let Some(keep) = actions.close_others {
            self.close_other_tabs(keep);
            return None;
        }
        actions.close
    }

    fn close_other_tabs(&mut self, keep: usize) {
        let kept = self.tabs.remove(keep);
        self.tabs.clear();
        self.tabs.push(kept);
        self.active_tab = 0;
        self.title_dirty = true;
        self.refresh_open_tab_paths();
        self.update_watched_paths();
    }
}

/// One screen pixel, in points. egui sets the scrolled row on whole pixels,
/// so a tab scrolled to an end of the strip can land up to half a pixel
/// past it. A pixel to spare, after the last tab and around a tab that is
/// scrolled to, keeps the tab wholly in the strip.
fn pixel(ui: &egui::Ui) -> f32 {
    ui.pixels_per_point().recip()
}

/// Scroll the strip the least way that brings the tab at `tab` wholly into
/// it. egui takes a widget for in view once it is in the clip rect, which
/// reaches past each end of the strip by the clip margin.
fn scroll_into_strip(ui: &egui::Ui, tab: egui::Rect) {
    let margin = ui.visuals().clip_rect_margin + pixel(ui);
    ui.scroll_to_rect(tab.expand2(egui::vec2(margin, 0.0)), None);
}

/// Draws one tab: its title, the line under it while it is the active one,
/// the close button while the pointer is over it, and its context menu.
/// Returns the tab's response.
fn tab_label(
    ui: &mut egui::Ui,
    label: &TabLabel<'_>,
    palette: &'static Palette,
    mcp: &Mcp,
    actions: &mut TabStripActions,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.title.to_owned(),
        chrome::label_font(),
        egui::Color32::PLACEHOLDER,
    );
    let width = 2.0_f32.mul_add(TAB_PADDING, galley.size().x) + CLOSE_SIDE;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, STRIP_HEIGHT), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            label.is_active,
            label.title,
        )
    });

    let state = if label.is_active { "active" } else { "" };
    mcp::rect(
        mcp,
        format_args!("Tab: {}", label.title),
        "tab",
        rect,
        Some(&state),
    );

    // The pointer counts as over the tab while it is over the close button
    // too, which takes the hover for itself.
    let pointed = ui.rect_contains_pointer(rect) || response.has_focus();
    let color = if label.is_active || pointed {
        palette.text
    } else {
        palette.muted
    };
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
        tab_close_button(ui, &response, label, palette, mcp, actions);
    }

    egui::Popup::context_menu(&response)
        .style(chrome::popup_style(palette))
        .show(|ui| tab_context_menu(ui, label, palette, actions));
    response
}

/// The close button in the square a tab keeps free at its right end.
fn tab_close_button(
    ui: &egui::Ui,
    tab: &egui::Response,
    label: &TabLabel<'_>,
    palette: &Palette,
    mcp: &Mcp,
    actions: &mut TabStripActions,
) {
    let square = egui::Rect::from_center_size(
        egui::pos2(
            tab.rect.right() - TAB_PADDING / 2.0 - CLOSE_SIDE / 2.0,
            tab.rect.center().y,
        ),
        egui::Vec2::splat(CLOSE_SIDE),
    );
    let close = ui.interact(square, tab.id.with("close"), egui::Sense::click());
    close.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close tab"));

    mcp::rect(
        mcp,
        format_args!("Close Tab: {}", label.title),
        "button",
        square,
        None,
    );

    let color = if close.hovered() {
        palette.text
    } else {
        palette.muted
    };
    Icon::Close.paint(ui.painter(), square.shrink(3.0), color);
    if close.clicked() {
        actions.close = Some(label.index);
    }
}

fn tab_context_menu(
    ui: &mut egui::Ui,
    label: &TabLabel<'_>,
    palette: &Palette,
    actions: &mut TabStripActions,
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

/// The Preview|Edit toggle, the unsaved-changes marker, and the outcome of
/// the last save. The caller lays these out right to left.
fn edit_mode_controls(ui: &mut egui::Ui, tab: &mut Tab, palette: &Palette) {
    ui.spacing_mut().item_spacing.x = 8.0;
    let modes = [(false, "Preview"), (true, "Edit")];
    if let Some(to_edit) = chrome::segmented(ui, palette, &modes, tab.is_editing, false) {
        tab.set_edit_mode(to_edit);
    }
    if tab.is_dirty() {
        let (dot, response) =
            ui.allocate_exact_size(egui::Vec2::splat(UNSAVED_DOT), egui::Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Unsaved changes")
        });
        ui.painter()
            .circle_filled(dot.center(), UNSAVED_DOT / 2.0, palette.warning);
        response.on_hover_text("Unsaved changes: press Ctrl+S to save");
    }
    save_flash_label(ui, tab, palette);
}

/// Shows the outcome of the last save until it expires, then forgets it.
fn save_flash_label(ui: &mut egui::Ui, tab: &mut Tab, palette: &Palette) {
    let Some((outcome, at)) = tab.save_flash else {
        return;
    };
    // The strip may be on screen only for this outcome, so it has to be
    // drawn again once the outcome has had its time.
    let left = SAVE_FLASH.saturating_sub(at.elapsed());
    ui.ctx().request_repaint_after(left);
    if left.is_zero() {
        tab.save_flash = None;
        return;
    }
    let (label, color) = match outcome {
        SaveOutcome::Saved => ("Saved", palette.positive),
        SaveOutcome::Failed => ("Save failed", palette.warning),
    };
    ui.label(
        egui::RichText::new(label)
            .font(chrome::small_font())
            .color(color),
    );
}
