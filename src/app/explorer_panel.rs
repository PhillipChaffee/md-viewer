//! The file explorer sidebar.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use eframe::egui;

use crate::app::MarkdownApp;
use crate::explorer::{ExplorerAction, FileTreeNode, SortOrder};
use crate::icons::{paint_file_icon, paint_folder_icon, paint_sidebar_resize_affordance};
use crate::layout::{EXPLORER_RIGHT_RESIZE_GUTTER, SIDEBAR_MIN_WIDTH};
use crate::watcher::FLASH_DURATION_MS;

impl MarkdownApp {
    /// Render the file explorer sidebar
    /// Returns actions for files to open or close
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
                // Header with folder name - OUTSIDE ScrollArea
                if let Some(root) = &self.file_explorer.root {
                    let folder_name = root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| root.display().to_string());
                    ui.strong(&folder_name);
                } else {
                    ui.strong("No folder");
                }

                // Expand/collapse/refresh buttons - OUTSIDE ScrollArea
                ui.horizontal(|ui| {
                    let expand_btn = ui.small_button("⊞").on_hover_text("Expand all directories");
                    #[cfg(feature = "mcp")]
                    self.mcp_bridge.register_widget(
                        "Explorer: Expand All",
                        "button",
                        &expand_btn,
                        None,
                    );
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

                // Sort order dropdown
                ui.horizontal(|ui| {
                    ui.label("Sort:");
                    #[allow(unused_variables)]
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

                ui.separator();

                // Pre-load children for all expanded dirs to avoid mutation during render
                for dir in self
                    .file_explorer
                    .expanded_dirs
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    if self.file_explorer.get_children(&dir).is_none() {
                        self.file_explorer.load_children(&dir);
                    }
                }

                // Take tree to iterate without cloning, then put it back
                let tree = std::mem::take(&mut self.file_explorer.tree);
                // Clone the small set of open tab paths (avoids borrow conflict with &mut self)
                let open_paths = self.open_tab_paths.clone();

                // File tree inside ScrollArea
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .id_salt("file_explorer")
                    .show(ui, |ui| {
                        for node in &tree {
                            let node_action = self.render_tree_node(ui, node, 0, &open_paths);
                            if node_action.file_to_open.is_some() {
                                action.file_to_open = node_action.file_to_open;
                            }
                            if node_action.file_to_close.is_some() {
                                action.file_to_close = node_action.file_to_close;
                            }
                            if node_action.dir_to_toggle.is_some() {
                                action.dir_to_toggle = node_action.dir_to_toggle;
                            }
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

    /// Calculate flash intensity for a path (0.0 = no flash, 1.0 = full flash)
    fn get_flash_intensity(&self, path: &PathBuf) -> f32 {
        if self.flashing_paths.is_empty() {
            return 0.0;
        }

        if let Some(start_time) = self.flashing_paths.get(path) {
            let elapsed = start_time.elapsed().as_millis() as u64;
            if elapsed < FLASH_DURATION_MS {
                // Fade out: 1.0 -> 0.0 over the duration
                1.0 - (elapsed as f32 / FLASH_DURATION_MS as f32)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Render a single node in the file tree (recursive)
    fn render_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        node: &FileTreeNode,
        depth: usize,
        open_paths: &HashSet<PathBuf>,
    ) -> ExplorerAction {
        let mut action = ExplorerAction::default();
        let indent = depth * 16;

        match node {
            FileTreeNode::File { path, name, .. } => {
                // Calculate flash intensity for this file
                let flash_intensity = self.get_flash_intensity(path);
                let dark_mode = self.dark_mode;

                // Render file row and get its rect
                let row_response = ui.horizontal(|ui| {
                    ui.add_space(indent as f32);

                    // File icon (vector-drawn — no emoji in app chrome)
                    paint_file_icon(ui, 14.0);

                    // Highlight if file is open in a tab
                    let is_open = open_paths.contains(path);
                    let text = if is_open {
                        egui::RichText::new(name.as_str()).strong()
                    } else {
                        egui::RichText::new(name.as_str())
                    };

                    let response = ui.add(
                        egui::Button::selectable(is_open, text)
                            .wrap_mode(egui::TextWrapMode::Extend),
                    );
                    #[cfg(feature = "mcp")]
                    {
                        let state_value = if is_open { "open" } else { "" };
                        self.mcp_bridge.register_widget(
                            &format!("File: {}", name),
                            "button",
                            &response,
                            Some(state_value),
                        );
                    }

                    if response.clicked() {
                        action.file_to_open = Some(path.clone());
                    }
                    // Middle-click to close tab (only if file is open)
                    if response.middle_clicked() && is_open {
                        action.file_to_close = Some(path.clone());
                    }

                    // Context menu for file actions
                    response.context_menu(|ui| {
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
                    });
                });

                // Paint flash overlay on top using the row rect
                if flash_intensity > 0.0 {
                    let alpha = ((flash_intensity * 180.0) as u8).max(60);
                    let flash_color = if dark_mode {
                        egui::Color32::from_rgba_unmultiplied(60, 200, 60, alpha)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(80, 200, 80, alpha)
                    };
                    let rect = row_response.response.rect;
                    // Use debug_painter which draws on top of everything
                    ui.ctx().debug_painter().rect_filled(rect, 4.0, flash_color);
                }
            }
            FileTreeNode::Directory { path, name, .. } => {
                // Calculate flash intensity for this directory
                let flash_intensity = self.get_flash_intensity(path);
                let dark_mode = self.dark_mode;

                // Track if we should toggle this frame (detected in closure, applied after)
                let mut should_toggle = false;

                // Get current expansion state
                let is_expanded = self.file_explorer.is_expanded(path);

                // Render directory row
                let row_response = ui.horizontal(|ui| {
                    ui.add_space(indent as f32);

                    // Expand/collapse indicator
                    let indicator = if is_expanded { "v" } else { ">" };

                    #[cfg(feature = "mcp")]
                    let expand_btn = ui.mcp_small_button(format!("Toggle: {}", name), indicator);
                    #[cfg(not(feature = "mcp"))]
                    let expand_btn = ui.small_button(indicator);

                    if expand_btn.clicked() {
                        should_toggle = true;
                    }

                    // Vector folder icon; expanded state shows in the fill.
                    paint_folder_icon(ui, 14.0, is_expanded);

                    let response = ui.add(
                        egui::Label::new(name.as_str())
                            .extend()
                            .selectable(false)
                            .sense(egui::Sense::click()),
                    );
                    #[cfg(feature = "mcp")]
                    {
                        let state_value = if is_expanded { "expanded" } else { "collapsed" };
                        self.mcp_bridge.register_widget(
                            &format!("Directory: {}", name),
                            "button",
                            &response,
                            Some(state_value),
                        );
                    }

                    // Click directory name to toggle expansion
                    if response.clicked() {
                        should_toggle = true;
                    }
                });

                // Defer toggle to after tree is restored (avoids clone)
                if should_toggle {
                    action.dir_to_toggle = Some(path.clone());
                }

                // Paint flash overlay on top using the row rect
                if flash_intensity > 0.0 {
                    let alpha = ((flash_intensity * 180.0) as u8).max(60);
                    let flash_color = if dark_mode {
                        egui::Color32::from_rgba_unmultiplied(60, 200, 60, alpha)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(80, 200, 80, alpha)
                    };
                    let rect = row_response.response.rect;
                    // Use debug_painter which draws on top of everything
                    ui.ctx().debug_painter().rect_filled(rect, 4.0, flash_color);
                }

                // Render children if expanded (read from node directly, no clone needed)
                if self.file_explorer.is_expanded(path) {
                    if let FileTreeNode::Directory {
                        children: Some(ref child_nodes),
                        ..
                    } = node
                    {
                        for child in child_nodes {
                            let child_action =
                                self.render_tree_node(ui, child, depth + 1, open_paths);
                            if child_action.file_to_open.is_some() {
                                action.file_to_open = child_action.file_to_open;
                            }
                            if child_action.file_to_close.is_some() {
                                action.file_to_close = child_action.file_to_close;
                            }
                            if child_action.dir_to_toggle.is_some() {
                                action.dir_to_toggle = child_action.dir_to_toggle;
                            }
                        }
                    }
                }
            }
        }

        action
    }
}
