//! The tab strip: a row of flat text tabs, shown only while there is
//! something to choose between. A lone document's edit controls go in the
//! top bar instead, so starting an edit does not move the page.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, MenuRow, QuietButton, STRIP_HEIGHT};
use crate::app::edit_controls::edit_mode_controls;
use crate::app::mcp::{self, Mcp};
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
/// Thickness of the line under the active tab.
const UNDERLINE: f32 = 2.0;
/// Width the strip keeps for the open-file button after the tabs.
const OPEN_BUTTON_ROOM: f32 = 40.0;
const CONTEXT_MENU_WIDTH: f32 = 140.0;

/// What the user asked of the tab strip during one frame.
///
/// The fields are independent because a primary click (activate) and a
/// middle click (close) can land in the same frame.
#[derive(Default)]
struct TabStripActions {
    open_file: bool,
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

impl MarkdownApp {
    /// Whether the strip earns its row: with one document open there is
    /// nothing to choose.
    const fn shows_tab_bar(&self) -> bool {
        self.tabs.len() > 1
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
                            self.open_file_button(ui, &mut actions);
                        });
                    });
                });
            });

        self.apply_tab_strip_actions(ctx, &actions)
    }

    /// The horizontally scrollable row of tabs, scrolled to the active one
    /// when `reveal` is set.
    fn tab_strip(&self, ui: &mut egui::Ui, actions: &mut TabStripActions, reveal: bool) {
        let palette = self.theme.palette();

        egui::ScrollArea::horizontal()
            .max_width(ui.available_width() - OPEN_BUTTON_ROOM)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                // The tabs go straight into the scrolled row: a row of
                // their own inside it would be centered at its starting
                // height and then grow downward, off the strip.
                for (index, tab) in self.tabs.iter().enumerate() {
                    let label = TabLabel {
                        index,
                        title: tab.title(),
                        is_active: index == self.active_tab,
                    };
                    let shown = tab_label(ui, &label, palette, &self.mcp, actions);
                    if reveal && label.is_active {
                        scroll_into_strip(ui, shown.rect);
                    }
                }
                ui.add_space(pixel(ui));
            });
    }

    /// Opening a file is what creates a new tab; there is no empty-tab
    /// state.
    fn open_file_button(&self, ui: &mut egui::Ui, actions: &mut TabStripActions) {
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
        actions.open_file = open.clicked();
    }

    /// Applies the frame's tab strip actions: a file is opened first, then
    /// a tab comes to the front, then tabs close. Returns the tab the caller
    /// should close; "Close Others" is completed here, so it returns `None`.
    fn apply_tab_strip_actions(
        &mut self,
        ctx: &egui::Context,
        actions: &TabStripActions,
    ) -> Option<usize> {
        if actions.open_file {
            self.apply_action(ctx, AppAction::OpenFile);
        }
        if let Some(index) = actions.activate {
            self.focus_tab(index);
        }
        if let Some(keep) = actions.close_others {
            self.close_other_tabs(keep);
            return None;
        }
        actions.close
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
