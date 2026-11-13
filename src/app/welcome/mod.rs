//! The home page, shown when no tab is open: the wordmark, the two ways to
//! open something, and the recently read documents.

use std::path::{Path, PathBuf};

use eframe::egui;
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{FontFamily, FontId, TextStyle};

use crate::app::action::AppAction;
use crate::app::sidebar;
use crate::app::MarkdownApp;
use crate::layout::{centered_column_left, ReadingWidth};
use crate::recent::now_epoch_secs;
use crate::theme::Palette;

pub(crate) use list::HomeList;
use list::HomeRow;

mod list;
#[cfg(test)]
mod tests;

/// Space between the top of the pane and the wordmark.
const TOP_SPACE: f32 = 56.0;
const WORDMARK: &str = "md-viewer";
const WORDMARK_SIZE: f32 = 30.0;
/// Space between the wordmark's line and what follows it.
const HEADER_GAP: f32 = 40.0;
/// Least space between the wordmark and the buttons on its line. With
/// less, the buttons go on a line of their own.
const WORDMARK_GAP: f32 = 24.0;
const BUTTON_TEXT_SIZE: f32 = 14.0;
/// Space between a quiet button's text and its edge.
const BUTTON_PADDING: egui::Vec2 = egui::vec2(10.0, 6.0);
const BUTTON_GAP: f32 = 4.0;
const CAPTION_GAP: f32 = 10.0;
/// Space above a row's title and below its last line.
const ROW_PADDING: f32 = 18.0;
const META_SIZE: f32 = 13.0;
const TOOLTIP_SIZE: f32 = 12.0;
/// Space between a row's title and its meta line.
const META_GAP: f32 = 4.0;
/// Space between a row's meta line and its excerpt.
const EXCERPT_GAP: f32 = 8.0;
const EXCERPT_LINE_HEIGHT: f32 = 1.45;
/// A row's title and excerpt, as multiples of the document's body size.
const TITLE_SCALE: f32 = 1.15;
const EXCERPT_SCALE: f32 = 0.88;
/// How far the wash under the pointer reaches past the column on each
/// side, so the text keeps its place and the wash has room around it.
const WASH_REACH: f32 = 14.0;
const WASH_RADIUS: f32 = 8.0;
const BOTTOM_SPACE: f32 = 48.0;

/// What the user picked on the home page.
#[derive(Debug, PartialEq, Eq)]
enum WelcomeAction {
    OpenFile,
    OpenFolder,
    OpenRecent(PathBuf),
    ToggleShowAll,
}

/// What the page is set in.
struct HomeStyle<'a> {
    palette: &'a Palette,
    /// The document's body face.
    serif: FontFamily,
    /// The document's heading face.
    display: FontFamily,
    /// The document's body size, which the list's text follows.
    body_size: f32,
    /// The text column the reader set for documents; the page keeps to it.
    column: ReadingWidth,
}

impl MarkdownApp {
    /// Render the home page shown when no document is open (issue #28).
    pub(crate) fn render_welcome(&mut self, ui: &mut egui::Ui) {
        let list = HomeList::current(
            &mut self.home_list,
            &self.recent_files,
            self.welcome_show_all,
            now_epoch_secs(),
            Path::exists,
        );
        let body = TextStyle::Body.resolve(&self.document_look.style);
        let heading = TextStyle::Heading.resolve(&self.document_look.style);
        let style = HomeStyle {
            palette: self.theme.palette(),
            serif: body.family,
            display: heading.family,
            body_size: body.size,
            column: self.reading_width,
        };

        // Deferred actions (run after the borrow on the list is released).
        match home_page(ui, list, &style) {
            Some(WelcomeAction::OpenFile) => self.apply_action(ui.ctx(), AppAction::OpenFile),
            Some(WelcomeAction::OpenFolder) => self.apply_action(ui.ctx(), AppAction::OpenFolder),
            Some(WelcomeAction::OpenRecent(path)) => self.open_in_new_tab(path),
            Some(WelcomeAction::ToggleShowAll) => {
                self.welcome_show_all = !self.welcome_show_all;
                self.home_list = None;
            }
            None => {}
        }
    }
}

/// Draws the whole page. Returns what the user picked this frame.
fn home_page(ui: &mut egui::Ui, list: &HomeList, style: &HomeStyle<'_>) -> Option<WelcomeAction> {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // The page keeps to the column a document would have here, so
            // the two line up when one replaces the other.
            let pane = ui.max_rect();
            let width = style.column.fixed().resolve(pane.width());
            let left = centered_column_left(pane.left(), pane.right(), width);
            let rect = egui::Rect::from_min_size(
                egui::pos2(left, pane.top() + TOP_SPACE),
                egui::vec2(width, 0.0),
            );
            let layout = egui::Layout::top_down(egui::Align::Min);
            let builder = egui::UiBuilder::new().max_rect(rect).layout(layout);
            ui.scope_builder(builder, |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let from_header = header(ui, style);
                ui.add_space(HEADER_GAP);
                let from_list = if list.rows.is_empty() {
                    empty_state(ui, style);
                    None
                } else {
                    recent_list(ui, list, style)
                };
                ui.add_space(BOTTOM_SPACE);
                from_header.or(from_list)
            })
            .inner
        })
        .inner
}

/// Text on one line in the chrome's face.
fn sans(text: &str, size: f32, color: egui::Color32) -> LayoutJob {
    LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(FontId::proportional(size), color),
    )
}

/// Wrapping that keeps text to `rows` lines of `width` px and marks what
/// it cut with an ellipsis.
fn wrapped(width: f32, rows: usize) -> TextWrapping {
    TextWrapping {
        max_width: width,
        max_rows: rows,
        ..Default::default()
    }
}

/// Distance from the top of `galley` to the baseline of its first line.
/// A galley without text stands at its top.
fn baseline(galley: &egui::Galley) -> f32 {
    let line = galley.rows.first();
    let glyph = line.and_then(|line| Some(line.pos.y + line.glyphs.first()?.pos.y));
    glyph.unwrap_or(0.0)
}

/// The wordmark, with the two open buttons at the right end of its line
/// where the column has room for them and under it where it has not.
fn header(ui: &mut egui::Ui, style: &HomeStyle<'_>) -> Option<WelcomeAction> {
    let palette = style.palette;
    let font = FontId::new(WORDMARK_SIZE, style.display.clone());
    let wordmark = ui
        .painter()
        .layout_no_wrap(WORDMARK.to_owned(), font, palette.heading);
    let buttons = [
        ("Open File…", WelcomeAction::OpenFile),
        ("Open Folder…", WelcomeAction::OpenFolder),
    ];
    let labels = buttons.each_ref().map(|(text, _)| {
        ui.painter()
            .layout_job(sans(text, BUTTON_TEXT_SIZE, palette.muted))
    });
    let buttons_width = labels
        .iter()
        .map(|label| 2.0_f32.mul_add(BUTTON_PADDING.x, label.size().x))
        .sum::<f32>()
        + BUTTON_GAP;
    let column = ui.available_width();
    let on_one_line = wordmark.size().x + WORDMARK_GAP + buttons_width <= column;

    let (line, response) =
        ui.allocate_exact_size(egui::vec2(column, wordmark.size().y), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, WORDMARK));
    let wordmark_baseline = baseline(&wordmark);
    ui.painter().galley(line.min, wordmark, palette.heading);

    let button_height = 2.0_f32.mul_add(BUTTON_PADDING.y, labels[0].size().y);
    let (mut x, y) = if on_one_line {
        // On the wordmark's line, the labels standing on its baseline.
        let label_top = line.top() + wordmark_baseline - baseline(&labels[0]);
        let x = line.right() - buttons_width + BUTTON_PADDING.x;
        (x, label_top - BUTTON_PADDING.y)
    } else {
        ui.add_space(12.0);
        let row = ui.allocate_space(egui::vec2(column, button_height)).1;
        // The first label lines up with the wordmark, not its padding.
        (row.left() - BUTTON_PADDING.x, row.top())
    };
    let mut action = None;
    for ((text, pick), label) in buttons.into_iter().zip(labels) {
        let size = label.size() + 2.0 * BUTTON_PADDING;
        let rect = egui::Rect::from_min_size(egui::pos2(x, y), size);
        if quiet_button(ui, palette, rect, text).clicked() {
            action = Some(pick);
        }
        x += size.x + BUTTON_GAP;
    }
    action
}

/// A button that is only its text until the pointer is on it: muted at
/// rest, inked on the raised wash when lit.
fn quiet_button(ui: &egui::Ui, palette: &Palette, rect: egui::Rect, text: &str) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(text), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    let ink = if response.hovered() {
        ui.painter()
            .rect_filled(rect, sidebar::RADIUS, palette.raised);
        palette.text
    } else {
        palette.muted
    };
    let label = ui.painter().layout_job(sans(text, BUTTON_TEXT_SIZE, ink));
    ui.painter().galley(rect.min + BUTTON_PADDING, label, ink);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A hairline across the column.
fn hairline(ui: &mut egui::Ui, palette: &Palette) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0_f32, palette.rule),
    );
}

/// Text that is read and not clicked, at the top left of what is left of
/// the column.
fn text_block(ui: &mut egui::Ui, job: LayoutJob) {
    let galley = ui.painter().layout_job(job);
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, galley.text()));
    ui.painter()
        .galley(rect.min, galley, egui::Color32::PLACEHOLDER);
}

/// What the page says when nothing has been read yet.
fn empty_state(ui: &mut egui::Ui, style: &HomeStyle<'_>) {
    let palette = style.palette;
    hairline(ui, palette);
    ui.add_space(ROW_PADDING + 6.0);
    let width = ui.available_width();
    let mut lead = LayoutJob::single_section(
        "Nothing read yet.".to_owned(),
        TextFormat::simple(
            FontId::new(style.body_size * TITLE_SCALE, style.serif.clone()),
            palette.heading,
        ),
    );
    lead.wrap = wrapped(width, 2);
    text_block(ui, lead);
    ui.add_space(EXCERPT_GAP);
    let size = style.body_size * EXCERPT_SCALE;
    let mut body = LayoutJob::single_section(
        "Open a Markdown file, or a folder of them, and it opens here in a clean \
         page made for reading. What you read is kept on this page, so you can \
         pick it up again."
            .to_owned(),
        TextFormat {
            font_id: FontId::new(size, style.serif.clone()),
            color: palette.text,
            line_height: Some(size * EXCERPT_LINE_HEIGHT),
            ..Default::default()
        },
    );
    body.wrap = wrapped(width, 6);
    text_block(ui, body);
    ui.add_space(ROW_PADDING);
    let mut hint = sans(
        "Ctrl+O opens a file. You can also drop one anywhere on this window.",
        META_SIZE,
        palette.muted,
    );
    hint.wrap = wrapped(width, 3);
    text_block(ui, hint);
}

/// The recently read documents, one row each, and the control that shows
/// the rest of them.
fn recent_list(ui: &mut egui::Ui, list: &HomeList, style: &HomeStyle<'_>) -> Option<WelcomeAction> {
    let palette = style.palette;
    let mut action = None;
    text_block(ui, sidebar::caption("RECENTLY READ", palette.muted));
    ui.add_space(CAPTION_GAP);
    for row in &list.rows {
        hairline(ui, palette);
        if recent_row(ui, row, style) {
            action = Some(WelcomeAction::OpenRecent(row.path.clone()));
        }
    }
    if list.has_more {
        hairline(ui, palette);
        ui.add_space(CAPTION_GAP);
        let text = if list.show_all {
            "Show less"
        } else {
            "Show more…"
        };
        let label = ui
            .painter()
            .layout_job(sans(text, BUTTON_TEXT_SIZE, palette.muted));
        let size = label.size() + 2.0 * BUTTON_PADDING;
        let (slot, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        // The label lines up with the titles above it, not its padding.
        let rect = slot.translate(egui::vec2(-BUTTON_PADDING.x, 0.0));
        if quiet_button(ui, palette, rect, text).clicked() {
            action = Some(WelcomeAction::ToggleShowAll);
        }
    }
    action
}

/// The three texts of a row, laid out for a column `width` px wide.
struct RowText {
    title: std::sync::Arc<egui::Galley>,
    meta: std::sync::Arc<egui::Galley>,
    excerpt: Option<std::sync::Arc<egui::Galley>>,
}

impl RowText {
    fn lay_out(ui: &egui::Ui, row: &HomeRow, style: &HomeStyle<'_>, width: f32) -> Self {
        let palette = style.palette;
        // A file that is gone is set in the muted ink throughout.
        let (title_ink, excerpt_ink) = if row.exists {
            (palette.heading, palette.text)
        } else {
            (palette.muted, palette.muted)
        };
        let title_font = FontId::new(style.body_size * TITLE_SCALE, style.serif.clone());
        let mut title =
            LayoutJob::single_section(row.title.clone(), TextFormat::simple(title_font, title_ink));
        title.wrap = wrapped(width, 2);
        let mut meta = sans(&row.meta, META_SIZE, palette.muted);
        meta.wrap = wrapped(width, 1);
        let excerpt = row.excerpt.as_ref().map(|text| {
            let size = style.body_size * EXCERPT_SCALE;
            let format = TextFormat {
                font_id: FontId::new(size, style.serif.clone()),
                color: excerpt_ink,
                line_height: Some(size * EXCERPT_LINE_HEIGHT),
                ..Default::default()
            };
            let mut job = LayoutJob::single_section(text.clone(), format);
            job.wrap = wrapped(width, 2);
            ui.painter().layout_job(job)
        });
        Self {
            title: ui.painter().layout_job(title),
            meta: ui.painter().layout_job(meta),
            excerpt,
        }
    }

    fn height(&self) -> f32 {
        let excerpt = self.excerpt.as_ref();
        let excerpt = excerpt.map_or(0.0, |excerpt| EXCERPT_GAP + excerpt.size().y);
        2.0_f32.mul_add(ROW_PADDING, self.title.size().y) + META_GAP + self.meta.size().y + excerpt
    }
}

/// Draws one recently read document: its title, its meta line and the
/// opening of its text. Returns whether the user clicked it; a file that
/// no longer exists takes no click.
fn recent_row(ui: &mut egui::Ui, row: &HomeRow, style: &HomeStyle<'_>) -> bool {
    let width = ui.available_width();
    let text = RowText::lay_out(ui, row, style, width);
    let sense = if row.exists {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, text.height()), sense);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, row.exists, &row.title)
    });
    if ui.is_rect_visible(rect) {
        let palette = style.palette;
        if row.exists && response.hovered() {
            // Clear of the hairlines above and below.
            let wash = rect.expand2(egui::vec2(WASH_REACH, -1.0));
            ui.painter().rect_filled(wash, WASH_RADIUS, palette.raised);
        }
        let mut at = rect.min + egui::vec2(0.0, ROW_PADDING);
        let title_height = text.title.size().y;
        ui.painter().galley(at, text.title, palette.heading);
        at.y += title_height + META_GAP;
        let meta_height = text.meta.size().y;
        ui.painter().galley(at, text.meta, palette.muted);
        if let Some(excerpt) = text.excerpt {
            at.y += meta_height + EXCERPT_GAP;
            ui.painter().galley(at, excerpt, palette.text);
        }
    }
    let cursor = if row.exists {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    };
    let response = response.on_hover_cursor(cursor);
    // Where the file is, for a reader who rests the pointer on the row.
    let hovered = response.on_hover_ui(|ui| {
        let path = egui::RichText::new(row.path.to_string_lossy());
        ui.label(path.size(TOOLTIP_SIZE).color(style.palette.muted));
    });
    hovered.clicked()
}
