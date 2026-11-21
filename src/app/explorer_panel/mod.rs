//! The file explorer sidebar.

use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;

use crate::app::chrome::{self, MenuRow};
use crate::app::shortcuts::hints::Hints;
use crate::app::sidebar::{self, Chevron, End, Row, Side};
use crate::app::MarkdownApp;
use crate::explorer::{FileTreeNode, SortOrder};
use crate::icons::Icon;
use crate::layout::EXPLORER_RIGHT_RESIZE_GUTTER;
use crate::theme::Palette;

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// Width of a file's menu: room for its longest row and the hint beside it.
const FILE_MENU_WIDTH: f32 = 200.0;
/// What the button at the end of a file's row does, as its tooltip says it.
const NEW_TAB_BUTTON: &str = "Open in new tab";

/// Where a file picked from the listing is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// In the tab in front, in place of what it shows.
    Here,
    /// In a tab of its own, or the tab it already has.
    NewTab,
}

/// What one frame's rows were clicked for. A later row's click replaces an
/// earlier one of its kind.
#[derive(Default)]
struct TreeClicks {
    /// The file to show, and where.
    file: Option<(PathBuf, Place)>,
    /// The folder to open or close, once the rows are drawn.
    dir_to_toggle: Option<PathBuf>,
}

/// What a pick from the explorer's menu asks for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuPick {
    ExpandAll,
    CollapseAll,
    Refresh,
    Sort(SortOrder),
}

impl MarkdownApp {
    /// Render the file explorer sidebar, then show the file its rows were
    /// clicked for. With `in_new_tab`, the command key held, the file gets
    /// a tab of its own whatever was clicked.
    pub(crate) fn render_file_explorer(&mut self, ctx: &egui::Context, in_new_tab: bool) {
        if !self.show_explorer {
            return;
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
        let (picked, width) = panel.show(ctx, palette, self.explorer_width, |ui| {
            // The header stays outside the ScrollArea.
            self.explorer_header(ui, palette);
            self.explorer_tree(ui, palette)
        });
        self.explorer_width = width;
        // Explorer matches document links: primary click replaces the active
        // document, while Ctrl/Cmd+click opens (or focuses) a separate tab.
        match picked {
            Some((path, Place::Here)) if !in_new_tab => self.navigate_active_tab_to(&path),
            Some((path, _)) => self.open_in_new_tab(path),
            None => {}
        }
    }

    /// "FILES", the folder they are in, and the menu that holds what the
    /// explorer can do to its listing.
    fn explorer_header(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        let root = self.file_explorer.root();
        let folder = root.map_or(Cow::Borrowed("No folder"), folder_name);
        let header = sidebar::Header {
            caption: "FILES",
            detail: Some(&folder),
            menu: Some("Explorer options"),
        };
        let more = sidebar::header(ui, palette, &header);

        let sort_order = self.file_explorer.sort_order();
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
    /// returns the file a row was clicked for, with where to show it.
    fn explorer_tree(
        &mut self,
        ui: &mut egui::Ui,
        palette: &'static Palette,
    ) -> Option<(PathBuf, Place)> {
        let active_tab = self.front_tab();
        let listing = Listing {
            palette,
            open_paths: &self.open_tab_paths,
            current: active_tab.map(|tab| tab.path.as_path()),
            hints: &self.hints,
        };

        let mut clicks = TreeClicks::default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("file_explorer")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for node in self.file_explorer.tree() {
                    self.render_tree_node(ui, node, 0, &listing, &mut clicks);
                }
            });

        // The rows only read the tree; the toggle is applied once they are
        // drawn.
        if let Some(dir_path) = &clicks.dir_to_toggle {
            self.file_explorer.toggle_expanded(dir_path);
            // Keep the non-recursive explorer watches in sync with the
            // newly expanded/collapsed directory.
            self.reconcile_explorer_watches();
        }
        clicks.file
    }

    /// Render a single node in the file tree (recursive). Clicks are
    /// recorded in `clicks`.
    fn render_tree_node(
        &self,
        ui: &mut egui::Ui,
        node: &FileTreeNode,
        depth: u16,
        listing: &Listing<'_>,
        clicks: &mut TreeClicks,
    ) {
        match node {
            FileTreeNode::File { path, name, .. } => {
                let row = TreeRow { path, name, depth };
                self.file_row(ui, &row, listing, clicks);
            }
            FileTreeNode::Directory {
                path,
                name,
                children,
                ..
            } => {
                let row = TreeRow { path, name, depth };
                let is_expanded = self.file_explorer.is_expanded(path);
                self.directory_row(ui, &row, is_expanded, listing.palette, clicks);
                if is_expanded {
                    for child in children.iter().flatten() {
                        self.render_tree_node(ui, child, depth + 1, listing, clicks);
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
        clicks: &mut TreeClicks,
    ) {
        let new_tab = new_tab_label(row.name);
        let shown = Row {
            text: row.name,
            indent: row.indent(),
            current: listing.current == Some(row.path),
            // A file's name lines up with the names of the folders beside it.
            chevron: Chevron::Blank,
            // The dot of a file that has a tab gives way to the button.
            end: End::Button {
                icon: Icon::NewTab,
                label: &new_tab,
                marked: listing.open_paths.contains(row.path),
            },
        };
        let drawn = sidebar::row(ui, listing.palette, &shown);
        let response = drawn.response;

        // A middle click opens a tab, as a browser's does on a link.
        let on_button = new_tab_button(drawn.button, listing);
        let place = if on_button || response.middle_clicked() {
            Some(Place::NewTab)
        } else {
            response.clicked().then_some(Place::Here)
        };
        let menu = egui::Popup::context_menu(&response)
            .style(chrome::popup_style(listing.palette))
            .show(|ui| file_context_menu(ui, row.path, listing));
        if let Some(place) = menu.and_then(|menu| menu.inner).or(place) {
            clicks.file = Some((row.path.to_path_buf(), place));
        }
        self.paint_flash(ui, row.path, response.rect);
    }

    fn directory_row(
        &self,
        ui: &mut egui::Ui,
        row: &TreeRow<'_>,
        is_expanded: bool,
        palette: &Palette,
        clicks: &mut TreeClicks,
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
            end: End::Nothing,
        };
        let response = sidebar::row(ui, palette, &shown).response;

        // The toggle is applied after the rows are drawn.
        if response.clicked() {
            clicks.dir_to_toggle = Some(row.path.to_path_buf());
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
    palette: &'static Palette,
    /// The documents open in a tab.
    open_paths: &'a HashSet<PathBuf>,
    /// The document the active tab shows.
    current: Option<&'a Path>,
    /// How the click that opens a file in a tab of its own is written.
    hints: &'a Hints,
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

/// What a screen reader calls the button at the end of the row of the file
/// `name`: every file in view has one, so each says whose it is.
fn new_tab_label(name: &str) -> String {
    format!("Open {name} in new tab")
}

/// Whether the button at the end of a file's row was clicked. Its tooltip
/// names the click that does the same on the row.
fn new_tab_button(button: Option<egui::Response>, listing: &Listing<'_>) -> bool {
    button.is_some_and(|button| {
        let clicked = button.clicked();
        listing.hints.click_tooltip(button, NEW_TAB_BUTTON);
        clicked
    })
}

/// A file's menu: the two ways to open it, then what can be copied from
/// it. Returns where the reader asked for the file to be shown.
fn file_context_menu(ui: &mut egui::Ui, path: &Path, listing: &Listing<'_>) -> Option<Place> {
    let palette = listing.palette;
    ui.set_width(FILE_MENU_WIDTH);
    let here = MenuRow::new("Open").flush().show(ui, palette).clicked();
    let new_tab = MenuRow::new("Open in New Tab").shortcut(listing.hints.command_click());
    let new_tab = new_tab.flush().show(ui, palette).clicked();
    chrome::hairline(ui, palette);

    let copy = |ui: &mut egui::Ui, what| MenuRow::new(what).flush().show(ui, palette).clicked();
    if copy(ui, "Copy Contents") {
        if let Ok(contents) = fs::read_to_string(path) {
            ui.ctx().copy_text(contents);
        }
    }
    if copy(ui, "Copy Path") {
        ui.ctx().copy_text(path.display().to_string());
    }
    if copy(ui, "Copy File URI") {
        ui.ctx().copy_text(format!("file://{}", path.display()));
    }
    if new_tab {
        Some(Place::NewTab)
    } else {
        here.then_some(Place::Here)
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
