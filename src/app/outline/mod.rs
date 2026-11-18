//! The outline sidebar.

use eframe::egui;

use crate::app::sidebar::{self, Chevron, Row, Side};
use crate::app::MarkdownApp;
use crate::headers::{Header, Outline};
use crate::tab::Tab;
use crate::theme::Palette;

pub(crate) use room::OutlineRule;
pub(crate) use spy::HeadingSpy;

mod room;
mod spy;
#[cfg(test)]
mod tests;

/// How far the fold control reaches past its chevron on each side.
const FOLD_REACH: f32 = 3.0;

/// What a click in the outline asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutlineClick {
    /// Fold or unfold the header at this index.
    ToggleFold(usize),
    ExpandAll,
    CollapseAll,
    /// Scroll the document to the header at this index.
    JumpTo(usize),
}

/// State shared by the outline's widgets while one frame is drawn.
struct OutlineUi<'a> {
    palette: &'a Palette,
    /// A pressed pointer may be resizing the panel, so clicks are ignored.
    is_dragging: bool,
    /// The heading the reader is in, or the nearest one above it that no
    /// folded heading hides.
    current: Option<usize>,
    /// The row of that heading, when the reader has just moved to it, so
    /// that the list scrolls to show it.
    follow_row: Option<usize>,
    click: Option<OutlineClick>,
}

/// The fold column of the row for the heading at `index`. There is none
/// where no heading in the document has children; otherwise a heading
/// without children keeps the column blank so that the titles align.
fn chevron_of(outline: &Outline, index: usize) -> Chevron {
    if !outline.has_folds() {
        Chevron::None
    } else if !outline.has_children(index) {
        Chevron::Blank
    } else if outline.is_collapsed(index) {
        Chevron::Closed
    } else {
        Chevron::Open
    }
}

/// What a click on a row's fold control does, for a screen reader. `None`
/// for a row with nothing to fold.
const fn fold_verb(chevron: Chevron) -> Option<&'static str> {
    match chevron {
        Chevron::Closed => Some("Expand"),
        Chevron::Open => Some("Collapse"),
        Chevron::None | Chevron::Blank => None,
    }
}

/// One outline entry as a row draws it.
struct OutlineRow<'a> {
    index: usize,
    header: &'a Header,
    chevron: Chevron,
}

impl MarkdownApp {
    /// Show or hide the outline, as the reader's own choice from now on.
    pub(crate) const fn toggle_outline(&mut self) {
        self.show_outline = !self.show_outline;
        self.outline_rule = OutlineRule::Reader;
    }

    /// Until the reader has chosen, show the outline while the window has
    /// room for it beside the text.
    fn follow_room(&mut self, ctx: &egui::Context) {
        let explorer = self.show_explorer.then_some(self.explorer_width);
        let window = ctx.content_rect().width();
        let has_room = room::has_room(window, explorer, self.reading_width);
        if let Some(shown) = self.outline_rule.follow(has_room) {
            self.show_outline = shown;
        }
    }

    /// Render the outline sidebar (right panel).
    /// Rendered at top level for proper layout space allocation.
    pub(crate) fn render_outline(&mut self, ctx: &egui::Context) {
        self.follow_room(ctx);
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        if !self.show_outline || tab.outline.headers().is_empty() {
            return;
        }

        let palette = self.theme.palette();
        let (heading, follow) = self.heading_spy.look(tab);
        let current_row = heading.and_then(|heading| tab.outline.row_standing_for(heading));
        let mut outline = OutlineUi {
            palette,
            is_dragging: ctx.input(|i| i.pointer.any_down()),
            current: current_row.map(|row| tab.outline.visible()[row]),
            follow_row: current_row.filter(|_| follow),
            click: None,
        };

        let panel = sidebar::Panel {
            side: Side::Right,
            id: "outline",
            margin: egui::Margin {
                left: 8,
                right: 8,
                top: 8,
                bottom: 8,
            },
        };
        ((), self.outline_width) = panel.show(ctx, palette, self.outline_width, |ui| {
            outline_header(ui, tab, &mut outline);
            outline_rows(ui, tab, &mut outline);
        });

        if let Some(click) = outline.click {
            apply_click(tab, click);
        }
    }
}

fn apply_click(tab: &mut Tab, click: OutlineClick) {
    match click {
        OutlineClick::ToggleFold(index) => tab.outline.toggle(index),
        OutlineClick::ExpandAll => tab.outline.expand_all(),
        OutlineClick::CollapseAll => tab.outline.collapse_all(),
        OutlineClick::JumpTo(index) => tab.jump_to_header(index),
    }
}

/// "CONTENTS", and where some header has children, the menu that folds and
/// unfolds all of them.
fn outline_header(ui: &mut egui::Ui, tab: &Tab, outline: &mut OutlineUi<'_>) {
    let can_fold = tab.outline.has_folds();
    let header = sidebar::Header {
        caption: "CONTENTS",
        detail: None,
        menu: can_fold.then_some("Outline options"),
    };
    let Some(more) = sidebar::header(ui, outline.palette, &header) else {
        return;
    };
    let picked = egui::Popup::menu(&more).show(outline_menu);
    if let Some(click) = picked.and_then(|menu| menu.inner) {
        outline.click = Some(click);
    }
}

/// The outline's menu. Returns what was picked this frame.
fn outline_menu(ui: &mut egui::Ui) -> Option<OutlineClick> {
    let items = [
        ("Expand All", OutlineClick::ExpandAll),
        ("Collapse All", OutlineClick::CollapseAll),
    ];
    sidebar::menu_text(ui);
    let mut picked = None;
    for (text, click) in items {
        if ui.button(text).clicked() {
            picked = Some(click);
        }
    }
    if picked.is_some() {
        ui.close();
    }
    picked
}

/// Distance from the top of the list to the top of its row `row`.
#[expect(
    clippy::cast_precision_loss,
    reason = "a document has far fewer headings than f32 counts exactly, and the result only aims a scroll"
)]
fn row_top(row: usize) -> f32 {
    row as f32 * sidebar::ROW_HEIGHT
}

/// The scrollable list of headers that no collapsed ancestor hides.
fn outline_rows(ui: &mut egui::Ui, tab: &Tab, outline: &mut OutlineUi<'_>) {
    let visible_indices = tab.outline.visible();

    // Rows touch, so that the wash under the pointer never falls between
    // two of them; show_rows adds this spacing to the row height.
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("outline")
        .show_rows(
            ui,
            sidebar::ROW_HEIGHT,
            visible_indices.len(),
            |ui, row_range| {
                if let Some(row) = outline.follow_row {
                    // The list starts `row_range.start` rows above this
                    // ui, which holds only the rows on screen.
                    let top = ui.max_rect().top() + row_top(row) - row_top(row_range.start);
                    let rows = top..=top + sidebar::ROW_HEIGHT;
                    let rect = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), rows);
                    ui.scroll_to_rect(rect, None);
                }
                for &index in &visible_indices[row_range] {
                    let row = OutlineRow {
                        index,
                        header: &tab.outline.headers()[index],
                        chevron: chevron_of(&tab.outline, index),
                    };
                    outline_row(ui, &row, outline);
                }
            },
        );
}

fn outline_row(ui: &mut egui::Ui, row: &OutlineRow<'_>, outline: &mut OutlineUi<'_>) {
    let shown = Row {
        text: &row.header.title,
        // Indent based on header level (h2 = 0, h3 = 1 indent, etc.)
        indent: f32::from(row.header.level.saturating_sub(2)) * sidebar::INDENT,
        current: outline.current == Some(row.index),
        chevron: row.chevron,
    };
    let laid_out = sidebar::row(ui, outline.palette, &shown);

    if !outline.is_dragging && laid_out.response.clicked() {
        outline.click = Some(OutlineClick::JumpTo(row.index));
    }
    fold_control(ui, row, laid_out.chevron, outline);
}

/// Makes the chevron of a row that has children a control of its own, laid
/// over the row: a click on it folds, a click beside it jumps.
fn fold_control(
    ui: &egui::Ui,
    row: &OutlineRow<'_>,
    chevron: egui::Rect,
    outline: &mut OutlineUi<'_>,
) {
    let Some(verb) = fold_verb(row.chevron) else {
        return;
    };
    let rect = chevron.expand2(egui::vec2(FOLD_REACH, 0.0));
    let response = ui.interact(
        rect,
        ui.id().with(("fold", row.index)),
        egui::Sense::click(),
    );
    // The chevron is painted, not a widget with text, so it names itself
    // for screen readers.
    response.widget_info(|| {
        let name = format!("{verb} {}", row.header.title);
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
    });

    if !outline.is_dragging && response.clicked() {
        outline.click = Some(OutlineClick::ToggleFold(row.index));
    }
}
