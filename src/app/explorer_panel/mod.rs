//! The file explorer sidebar.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;
#[cfg(feature = "mcp")]
use egui_mcp_bridge::McpUiExt;

use crate::app::MarkdownApp;
use crate::explorer::{ExplorerAction, FileTreeNode, SortOrder};
use crate::icons::{paint_file_icon, paint_folder_icon, paint_sidebar_resize_affordance};
use crate::layout::{EXPLORER_RIGHT_RESIZE_GUTTER, SIDEBAR_MIN_WIDTH};
use crate::watcher::FLASH_DURATION_MS;

#[cfg(test)]
mod tests;

impl MarkdownApp {
    /// Render the file explorer sidebar.
    /// Returns actions for files to open or close.
    pub(crate) fn render_file_explorer(&mut self, ctx: &egui::Context) -> ExplorerAction {
        let mut action = ExplorerAction::default();

        if !self.show_explorer {
            return action;
        }

        let panel = egui::SidePanel::left("file_explorer")
            .resizable(true)
            .default_width(self.explorer_width)
            .width_range(SIDEBAR_MIN_WIDTH..=f32::INFINITY)
            .frame(
                egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                    left: 8,
                    right: EXPLORER_RIGHT_RESIZE_GUTTER,
                    top: 8,
                    bottom: 8,
                }),
            )
            .show(ctx, |ui| {
                // The header and controls stay outside the ScrollArea.
                self.explorer_header(ui);
                self.explorer_toolbar(ui);
                self.explorer_sort_picker(ui);
                ui.separator();
                self.explorer_tree(ui, &mut action);
            });
        self.explorer_width = panel.response.rect.width();
        // Left panel: the draggable edge is its right border.
        paint_sidebar_resize_affordance(
            ctx,
            panel.response.rect.right(),
            panel.response.rect.y_range(),
            egui::Id::new("file_explorer"),
        );

        action
    }

    fn explorer_header(&self, ui: &mut egui::Ui) {
        let title = self
            .file_explorer
            .root
            .as_deref()
            .map_or_else(|| "No folder".to_owned(), folder_name);
        ui.strong(title);
    }

    /// The expand-all, collapse-all, and refresh buttons.
    fn explorer_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let expand_btn = ui.small_button("⊞").on_hover_text("Expand all directories");
            #[cfg(feature = "mcp")]
            self.mcp_bridge
                .register_widget("Explorer: Expand All", "button", &expand_btn, None);
            if expand_btn.clicked() {
                self.file_explorer.expand_all();
                self.reconcile_explorer_watches();
            }

            let collapse_btn = ui
                .small_button("⊟")
                .on_hover_text("Collapse all directories");
            #[cfg(feature = "mcp")]
            self.mcp_bridge.register_widget(
                "Explorer: Collapse All",
                "button",
                &collapse_btn,
                None,
            );
            if collapse_btn.clicked() {
                self.file_explorer.collapse_all();
                self.reconcile_explorer_watches();
            }

            if ui.small_button("↻").on_hover_text("Refresh").clicked() {
                self.file_explorer.refresh();
            }
        });
    }

    fn explorer_sort_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Sort:");
            #[cfg(feature = "mcp")]
            let current_label = self.file_explorer.sort_order.label();
            let combo_response = egui::ComboBox::from_id_salt("explorer_sort")
                .selected_text(self.file_explorer.sort_order.label())
                .show_ui(ui, |ui| {
                    for order in [
                        SortOrder::NameAsc,
                        SortOrder::NameDesc,
                        SortOrder::DateAsc,
                        SortOrder::DateDesc,
                    ] {
                        let is_selected = self.file_explorer.sort_order == order;
                        if ui.selectable_label(is_selected, order.label()).clicked() {
                            self.file_explorer.set_sort_order(order);
                        }
                    }
                });
            #[cfg(feature = "mcp")]
            self.mcp_bridge.register_widget(
                "Explorer: Sort Order",
                "combobox",
                &combo_response.response,
                Some(current_label),
            );
            #[cfg(not(feature = "mcp"))]
            let _ = combo_response;
        });
    }

    /// The scrollable file tree. Applies a directory toggle itself and
    /// records it, with any file click, in `action`.
    fn explorer_tree(&mut self, ui: &mut egui::Ui, action: &mut ExplorerAction) {
        // Pre-load children for all expanded dirs to avoid mutation during render
        let unloaded: Vec<PathBuf> = self
            .file_explorer
            .expanded_dirs
            .iter()
            .filter(|dir| self.file_explorer.get_children(dir).is_none())
            .cloned()
            .collect();
        for dir in &unloaded {
            self.file_explorer.load_children(dir);
        }

        // Take tree to iterate without cloning, then put it back
        let tree = std::mem::take(&mut self.file_explorer.tree);
        // Clone the small set of open tab paths (avoids borrow conflict with &mut self)
        let open_paths = self.open_tab_paths.clone();

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt("file_explorer")
            .show(ui, |ui| {
                for node in &tree {
                    self.render_tree_node(ui, node, 0, &open_paths, action);
                }
            });

        // Put tree back and apply deferred toggle
        self.file_explorer.tree = tree;
        if let Some(ref dir_path) = action.dir_to_toggle {
            self.file_explorer.toggle_expanded(dir_path);
            // Keep the non-recursive explorer watches in sync with the
            // newly expanded/collapsed directory.
            self.reconcile_explorer_watches();
        }
    }

    /// Render a single node in the file tree (recursive). Clicks are
    /// recorded in `action`; a later row's click replaces an earlier one.
    fn render_tree_node(
        &self,
        ui: &mut egui::Ui,
        node: &FileTreeNode,
        depth: u16,
        open_paths: &HashSet<PathBuf>,
        action: &mut ExplorerAction,
    ) {
        match node {
            FileTreeNode::File { path, name, .. } => {
                let row = TreeRow { path, name, depth };
                self.file_row(ui, &row, open_paths.contains(path), action);
            }
            FileTreeNode::Directory {
                path,
                name,
                children,
                ..
            } => {
                let row = TreeRow { path, name, depth };
                let is_expanded = self.file_explorer.is_expanded(path);
                self.directory_row(ui, &row, is_expanded, action);
                if is_expanded {
                    for child in children.iter().flatten() {
                        self.render_tree_node(ui, child, depth + 1, open_paths, action);
                    }
                }
            }
        }
    }

    fn file_row(
        &self,
        ui: &mut egui::Ui,
        row: &TreeRow<'_>,
        is_open: bool,
        action: &mut ExplorerAction,
    ) {
        let response = ui
            .horizontal(|ui| {
                ui.add_space(row.indent());

                // File icon (vector-drawn — no emoji in app chrome)
                paint_file_icon(ui, 14.0);

                // Highlight if file is open in a tab
                let text = if is_open {
                    egui::RichText::new(row.name).strong()
                } else {
                    egui::RichText::new(row.name)
                };

                let response = ui.add(
                    egui::Button::selectable(is_open, text).wrap_mode(egui::TextWrapMode::Extend),
                );
                #[cfg(feature = "mcp")]
                {
                    let state_value = if is_open { "open" } else { "" };
                    self.mcp_bridge.register_widget(
                        &format!("File: {}", row.name),
                        "button",
                        &response,
                        Some(state_value),
                    );
                }

                if response.clicked() {
                    action.file_to_open = Some(row.path.to_path_buf());
                }
                // Middle-click to close tab (only if file is open)
                if response.middle_clicked() && is_open {
                    action.file_to_close = Some(row.path.to_path_buf());
                }

                response.context_menu(|ui| file_context_menu(ui, row.path));
            })
            .response;
        self.paint_flash(ui, row.path, response.rect);
    }

    fn directory_row(
        &self,
        ui: &mut egui::Ui,
        row: &TreeRow<'_>,
        is_expanded: bool,
        action: &mut ExplorerAction,
    ) {
        let response = ui
            .horizontal(|ui| {
                ui.add_space(row.indent());

                // Expand/collapse indicator
                let indicator = if is_expanded { "v" } else { ">" };

                #[cfg(feature = "mcp")]
                let expand_btn = ui.mcp_small_button(format!("Toggle: {}", row.name), indicator);
                #[cfg(not(feature = "mcp"))]
                let expand_btn = ui.small_button(indicator);

                // Vector folder icon; expanded state shows in the fill.
                paint_folder_icon(ui, 14.0, is_expanded);

                let response = ui.add(
                    egui::Label::new(row.name)
                        .extend()
                        .selectable(false)
                        .sense(egui::Sense::click()),
                );
                #[cfg(feature = "mcp")]
                {
                    let state_value = if is_expanded { "expanded" } else { "collapsed" };
                    self.mcp_bridge.register_widget(
                        &format!("Directory: {}", row.name),
                        "button",
                        &response,
                        Some(state_value),
                    );
                }

                // The toggle is applied after the tree is put back (avoids a clone).
                if expand_btn.clicked() || response.clicked() {
                    action.dir_to_toggle = Some(row.path.to_path_buf());
                }
            })
            .response;
        self.paint_flash(ui, row.path, response.rect);
    }

    /// Paints the change highlight over a row whose path changed on disk
    /// within the flash duration.
    fn paint_flash(&self, ui: &egui::Ui, path: &Path, row: egui::Rect) {
        let Some(alpha) = self
            .flashing_paths
            .get(path)
            .and_then(|changed_at| flash_alpha(changed_at.elapsed()))
        else {
            return;
        };
        let [r, g, b, _] = self.theme.palette().positive.to_array();
        let flash_color = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
        // Use debug_painter which draws on top of everything
        ui.ctx().debug_painter().rect_filled(row, 4.0, flash_color);
    }
}

/// One explorer row: the node it shows and how deep it sits in the tree.
struct TreeRow<'a> {
    path: &'a Path,
    name: &'a str,
    depth: u16,
}

impl TreeRow<'_> {
    fn indent(&self) -> f32 {
        f32::from(self.depth) * 16.0
    }
}

/// The folder's own name, or the whole path for a root such as `/`.
fn folder_name(root: &Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn file_context_menu(ui: &mut egui::Ui, path: &Path) {
    if ui.button("Copy Contents").clicked() {
        if let Ok(contents) = fs::read_to_string(path) {
            ui.ctx().copy_text(contents);
        }
        ui.close();
    }
    if ui.button("Copy Path").clicked() {
        ui.ctx().copy_text(path.display().to_string());
        ui.close();
    }
    if ui.button("Copy File URI").clicked() {
        ui.ctx().copy_text(format!("file://{}", path.display()));
        ui.close();
    }
}

/// Opacity of the change highlight `elapsed` after a file changed. It fades
/// from 180 to a floor of 60 that stays visible until the flash ends, after
/// which there is none.
fn flash_alpha(elapsed: Duration) -> Option<u8> {
    let total = u128::from(FLASH_DURATION_MS);
    let remaining = total
        .checked_sub(elapsed.as_millis())
        .filter(|remaining| *remaining > 0)?;
    let alpha = u8::try_from(remaining * 180 / total).unwrap_or(u8::MAX);
    Some(alpha.max(60))
}
