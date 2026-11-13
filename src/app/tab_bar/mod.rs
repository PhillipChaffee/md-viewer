//! The tab strip.

use std::time::Duration;

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::MarkdownApp;
use crate::tab::Tab;
use crate::watcher::FLASH_DURATION_MS;

#[cfg(test)]
mod tests;

/// How long a save's outcome stays beside the mode toggle.
const SAVE_FLASH: Duration = Duration::from_millis(FLASH_DURATION_MS * 3);

/// A widget recorded for the MCP bridge: name, widget type, rect, value.
#[cfg(feature = "mcp")]
type McpWidget = (String, &'static str, egui::Rect, Option<String>);

/// What the user asked of the tab strip during one frame.
///
/// The fields are independent because a primary click (activate) and a
/// middle click (close) can land in the same frame.
#[derive(Default)]
struct TabStripActions {
    activate: Option<usize>,
    close: Option<usize>,
    close_others: Option<usize>,
    #[cfg(feature = "mcp")]
    widgets: Vec<McpWidget>,
}

/// One tab as the strip draws it.
struct TabLabel<'a> {
    index: usize,
    title: &'a str,
    is_active: bool,
    /// The close button shows on the active tab and the hovered one.
    show_close: bool,
    /// "Close Others" is offered only when there are other tabs.
    has_siblings: bool,
}

impl MarkdownApp {
    /// Render the custom tab bar. Returns the index of a tab to close.
    pub(crate) fn render_tab_bar(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        let mut actions = TabStripActions::default();

        ui.horizontal(|ui| {
            self.tab_strip(ui, &mut actions);

            // Placeholder hint when no document is open (the welcome page is showing)
            if self.tabs.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("No file open").weak());
                ui.add_space(4.0);
            }

            // Opening a file is what creates a new tab; there is no empty-tab state.
            let new_tab_btn = ui
                .button("+")
                .on_hover_text("Open File in New Tab... (Ctrl+T)");

            // Collect the open-file action for MCP.
            #[cfg(feature = "mcp")]
            actions.widgets.push((
                "Open File in New Tab".to_string(),
                "button",
                new_tab_btn.rect,
                None,
            ));

            if new_tab_btn.clicked() {
                self.apply_action(ui.ctx(), AppAction::OpenFile);
            }

            // Preview|Edit segmented toggle for the active tab, right-aligned
            // in the tab bar (DSH-better-sidebar editor-header placement).
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    edit_mode_controls(ui, tab);
                });
            }
        });

        // Register all collected widgets with MCP bridge
        #[cfg(feature = "mcp")]
        for (name, widget_type, rect, value) in std::mem::take(&mut actions.widgets) {
            self.mcp_bridge
                .register_widget_rect(&name, widget_type, rect, value.as_deref());
        }

        self.apply_tab_strip_actions(&actions)
    }

    /// The horizontally scrollable row of tabs.
    fn tab_strip(&mut self, ui: &mut egui::Ui, actions: &mut TabStripActions) {
        // Collect the titles first: drawing a tab updates `self.hovered_tab`.
        let titles: Vec<String> = self.tabs.iter().map(Tab::title).collect();
        let has_siblings = titles.len() > 1;
        // Last frame's hover decides the close buttons for the whole strip.
        let hovered_tab = self.hovered_tab;

        egui::ScrollArea::horizontal()
            .max_width(ui.available_width() - 30.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, title) in titles.iter().enumerate() {
                        let is_active = index == self.active_tab;
                        let label = TabLabel {
                            index,
                            title,
                            is_active,
                            show_close: is_active || hovered_tab == Some(index),
                            has_siblings,
                        };
                        if tab_label(ui, &label, actions).hovered() {
                            self.hovered_tab = Some(index);
                        }
                        ui.separator();
                    }
                });
            });
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

/// Draws one tab: its label, close button, and context menu. Returns the
/// response of the whole tab, which the caller uses to track hover.
fn tab_label(
    ui: &mut egui::Ui,
    label: &TabLabel<'_>,
    actions: &mut TabStripActions,
) -> egui::Response {
    ui.horizontal(|ui| {
        let text = egui::RichText::new(label.title);
        let text = if label.is_active { text.strong() } else { text };

        let response = ui.selectable_label(label.is_active, text);

        // Collect tab widget data for MCP
        #[cfg(feature = "mcp")]
        actions.widgets.push((
            format!("Tab: {}", label.title),
            "tab",
            response.rect,
            Some(if label.is_active {
                "active".to_string()
            } else {
                String::new()
            }),
        ));

        if response.clicked() {
            actions.activate = Some(label.index);
        }

        // Middle-click to close
        if response.middle_clicked() {
            actions.close = Some(label.index);
        }

        tab_close_button(ui, label, actions);

        response.context_menu(|ui| tab_context_menu(ui, label, actions));
    })
    .response
}

fn tab_close_button(ui: &mut egui::Ui, label: &TabLabel<'_>, actions: &mut TabStripActions) {
    if !label.show_close {
        return;
    }
    let close_btn = ui.small_button("×");

    // Collect close button widget data for MCP
    #[cfg(feature = "mcp")]
    actions.widgets.push((
        format!("Close Tab: {}", label.title),
        "button",
        close_btn.rect,
        None,
    ));

    if close_btn.clicked() {
        actions.close = Some(label.index);
    }
}

fn tab_context_menu(ui: &mut egui::Ui, label: &TabLabel<'_>, actions: &mut TabStripActions) {
    if ui.button("Close").clicked() {
        actions.close = Some(label.index);
        ui.close();
    }
    if label.has_siblings && ui.button("Close Others").clicked() {
        actions.close_others = Some(label.index);
        ui.close();
    }
}

/// The Preview|Edit toggle, the unsaved-changes marker, and the outcome of
/// the last save. The caller lays these out right to left.
fn edit_mode_controls(ui: &mut egui::Ui, tab: &mut Tab) {
    if let Some(to_edit) = mode_toggle(ui, tab.is_editing) {
        tab.set_edit_mode(to_edit);
    }
    if tab.content != tab.saved_content {
        ui.label(
            egui::RichText::new("●")
                .small()
                .color(ui.visuals().warn_fg_color),
        )
        .on_hover_text("Unsaved changes — press Ctrl+S to save");
    }
    save_flash_label(ui, tab);
}

/// Draws the segmented mode toggle. Returns the mode the user switched to:
/// `true` for edit, `false` for preview.
fn mode_toggle(ui: &mut egui::Ui, is_editing: bool) -> Option<bool> {
    let mut switch_mode = None;
    // Borderless on purpose: the tab bar strip already carries a
    // border, so only the filled active segment provides the frame.
    egui::Frame::NONE
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.set_min_height(ui.text_style_height(&egui::TextStyle::Small));
            ui.horizontal(|ui| {
                // Added in Edit, Preview order so the
                // right-to-left layout renders [Preview|Edit].
                for (label, is_edit) in [("Edit", true), ("Preview", false)] {
                    if mode_segment(ui, label, is_editing == is_edit) {
                        switch_mode = Some(is_edit);
                    }
                }
            });
        });
    switch_mode
}

/// Draws one segment of the mode toggle. Returns whether the user clicked a
/// segment that was not already the active mode.
fn mode_segment(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let text_color = if active {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let fill = if active {
        ui.visuals().widgets.inactive.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    let response = ui.add(
        egui::Button::new(egui::RichText::new(label).small().color(text_color))
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(4)),
    );
    response.clicked() && !active
}

/// Shows the outcome of the last save until it expires, then forgets it.
fn save_flash_label(ui: &mut egui::Ui, tab: &mut Tab) {
    let Some((ok, at)) = tab.save_flash else {
        return;
    };
    if at.elapsed() >= SAVE_FLASH {
        tab.save_flash = None;
        return;
    }
    let (label, color) = if ok {
        ("Saved ✓", egui::Color32::from_rgb(90, 200, 120))
    } else {
        ("Save failed", ui.visuals().error_fg_color)
    };
    ui.label(egui::RichText::new(label).small().color(color));
}
