//! The file explorer sidebar.

use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;

use crate::app::mcp;
use crate::app::sidebar::{self, Chevron, Row, Side};
use crate::app::MarkdownApp;
use crate::explorer::{ExplorerAction, FileTreeNode, SortOrder};
use crate::layout::EXPLORER_RIGHT_RESIZE_GUTTER;
use crate::theme::Palette;

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// What a pick from the explorer's menu asks for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuPick {
    ExpandAll,
    CollapseAll,
    Refresh,
    Sort(SortOrder),
}

impl MarkdownApp {
    /// Render the file explorer sidebar.
    /// Returns actions for files to open or close.
    pub(crate) fn render_file_explorer(&mut self, ctx: &egui::Context) -> ExplorerAction {
        let mut action = ExplorerAction::default();

        if !self.show_explorer {
            return action;
        }

        let palette = self.theme.palette();
        let panel = sidebar::Panel {
            side: Side::Left,
            id: "file_explorer",
            margin: egui::Margin {
                left: 8,
                right: EXPLORER_RIGHT_RESIZE_GUTTER,
                top: 8,
                bottom: 8,
            },
        };
        ((), self.explorer_width) = panel.show(ctx, palette, self.explorer_width, |ui| {
            // The header stays outside the ScrollArea.
            self.explorer_header(ui, palette);
            self.explorer_tree(ui, palette, &mut action);
        });

        action
    }

    /// "FILES", the folder they are in, and the menu that holds what the
    /// explorer can do to its listing.
    fn explorer_header(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        let root = self.file_explorer.root.as_deref();
        let folder = root.map_or(Cow::Borrowed("No folder"), folder_name);
        let header = sidebar::Header {
            caption: "FILES",
            detail: Some(&folder),
            menu: Some("Explorer options"),
        };
        let more = sidebar::header(ui, palette, &header);
        if let Some(more) = &more {
            mcp::widget(
                &self.mcp,
                format_args!("Explorer: Options"),
                "button",
                more,
                None,
            );
        }

        let sort_order = self.file_explorer.sort_order;
        let picked = more
            .and_then(|more| egui::Popup::menu(&more).show(|ui| explorer_menu(ui, sort_order)))
            .and_then(|menu| menu.inner);
        match picked {
            Some(MenuPick::ExpandAll) => {
                self.file_explorer.expand_all();
                self.reconcile_explorer_watches();
            }
            Some(MenuPick::CollapseAll) => {
                self.file_explorer.collapse_all();
                self.reconcile_explorer_watches();
            }
            Some(MenuPick::Refresh) => self.file_explorer.refresh(),
            Some(MenuPick::Sort(order)) => self.file_explorer.set_sort_order(order),
            None => {}
        }
    }

    /// The scrollable file tree. Applies a directory toggle itself and
    /// records it, with any file click, in `action`.
    fn explorer_tree(&mut self, ui: &mut egui::Ui, palette: &Palette, action: &mut ExplorerAction) {
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

        let active_tab = self.tabs.get(self.active_tab);
        let listing = Listing {
            palette,
            open_paths: &self.open_tab_paths,
            current: active_tab.map(|tab| tab.path.as_path()),
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("file_explorer")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for node in &self.file_explorer.tree {
                    self.render_tree_node(ui, node, 0, &listing, action);
                }
            });

        // The rows only read the tree; the toggle is applied once they are
        // drawn.
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
        listing: &Listing<'_>,
        action: &mut ExplorerAction,
    ) {
        match node {
            FileTreeNode::File { path, name, .. } => {
                let row = TreeRow { path, name, depth };
                self.file_row(ui, &row, listing, action);
            }
            FileTreeNode::Directory {
                path,
                name,
                children,
                ..
            } => {
                let row = TreeRow { path, name, depth };
                let is_expanded = self.file_explorer.is_expanded(path);
                self.directory_row(ui, &row, is_expanded, listing.palette, action);
                if is_expanded {
                    for child in children.iter().flatten() {
                        self.render_tree_node(ui, child, depth + 1, listing, action);
                    }
                }
            }
        }
    }

    fn file_row(
        &self,
        ui: &mut egui::Ui,
        row: &TreeRow<'_>,
        listing: &Listing<'_>,
        action: &mut ExplorerAction,
    ) {
        let is_open = listing.open_paths.contains(row.path);
        let shown = Row {
            text: row.name,
            indent: row.indent(),
            current: listing.current == Some(row.path),
            // A file's name lines up with the names of the folders beside it.
            chevron: Chevron::Blank,
        };
        let response = sidebar::row(ui, listing.palette, &shown).response;
        let state_value = if is_open { "open" } else { "" };
        mcp::widget(
            &self.mcp,
            format_args!("File: {}", row.name),
            "button",
            &response,
            Some(state_value),
        );

        if response.clicked() {
            action.file_to_open = Some(row.path.to_path_buf());
        }
        // Middle-click to close tab (only if file is open)
        if response.middle_clicked() && is_open {
            action.file_to_close = Some(row.path.to_path_buf());
        }
        response.context_menu(|ui| file_context_menu(ui, row.path));
        self.paint_flash(ui, row.path, response.rect);
    }

    fn directory_row(
        &self,
        ui: &mut egui::Ui,
        row: &TreeRow<'_>,
        is_expanded: bool,
        palette: &Palette,
        action: &mut ExplorerAction,
    ) {
        let shown = Row {
            text: row.name,
            indent: row.indent(),
            current: false,
            chevron: if is_expanded {
                Chevron::Open
            } else {
                Chevron::Closed
            },
        };
        let response = sidebar::row(ui, palette, &shown).response;
        let state_value = if is_expanded { "expanded" } else { "collapsed" };
        mcp::widget(
            &self.mcp,
            format_args!("Directory: {}", row.name),
            "button",
            &response,
            Some(state_value),
        );

        // The toggle is applied after the rows are drawn.
        if response.clicked() {
            action.dir_to_toggle = Some(row.path.to_path_buf());
        }
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

/// What every row of one frame's listing is drawn with.
struct Listing<'a> {
    palette: &'a Palette,
    /// The documents open in a tab.
    open_paths: &'a HashSet<PathBuf>,
    /// The document the active tab shows.
    current: Option<&'a Path>,
}

/// One explorer row: the node it shows and how deep it sits in the tree.
struct TreeRow<'a> {
    path: &'a Path,
    name: &'a str,
    depth: u16,
}

impl TreeRow<'_> {
    fn indent(&self) -> f32 {
        f32::from(self.depth) * sidebar::INDENT
    }
}

/// The folder's own name, or the whole path for a root such as `/`.
fn folder_name(root: &Path) -> Cow<'_, str> {
    root.file_name()
        .map_or_else(|| root.to_string_lossy(), |name| name.to_string_lossy())
}

/// The explorer's menu: what it can do to its listing, and the order the
/// listing is in. Returns what was picked this frame.
fn explorer_menu(ui: &mut egui::Ui, sort_order: SortOrder) -> Option<MenuPick> {
    let actions = [
        ("Expand All Folders", MenuPick::ExpandAll),
        ("Collapse All Folders", MenuPick::CollapseAll),
        ("Refresh", MenuPick::Refresh),
    ];
    sidebar::menu_text(ui);
    let mut picked = None;
    for (text, pick) in actions {
        if ui.button(text).clicked() {
            picked = Some(pick);
        }
    }
    ui.separator();
    for order in SortOrder::ALL {
        if ui.radio(order == sort_order, order.label()).clicked() {
            picked = Some(MenuPick::Sort(order));
        }
    }
    if picked.is_some() {
        ui.close();
    }
    picked
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

/// How long a changed file's highlight lasts, in milliseconds.
pub(super) const FLASH_DURATION_MS: u64 = 600;

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
