use crate::typography::TypographyConfig;
use egui::{self, NumExt, RichText, Sense, TextStyle, Ui, Vec2, epaint};

/// Add extra vertical spacing based on typography configuration.
/// Returns the amount of space added.
#[inline]
pub fn typography_spacing(ui: &mut Ui, spacing: f32) {
    if spacing > 0.0 {
        ui.add_space(spacing);
    }
}

/// Add paragraph spacing based on typography configuration.
#[inline]
pub fn paragraph_end_spacing(ui: &mut Ui, typography: &TypographyConfig) {
    let font_size = ui.text_style_height(&TextStyle::Body);
    let spacing = typography.resolve_paragraph_spacing(font_size);
    typography_spacing(ui, spacing);
}

/// Add heading spacing above based on typography configuration.
#[inline]
pub fn heading_start_spacing(ui: &mut Ui, typography: &TypographyConfig) {
    let font_size = ui.text_style_height(&TextStyle::Body);
    let spacing = typography.resolve_heading_above(font_size);
    typography_spacing(ui, spacing);
}

/// Add heading spacing below based on typography configuration.
#[inline]
pub fn heading_end_spacing(ui: &mut Ui, typography: &TypographyConfig) {
    let font_size = ui.text_style_height(&TextStyle::Body);
    let spacing = typography.resolve_heading_below(font_size);
    typography_spacing(ui, spacing);
}

#[inline]
pub fn rule(ui: &mut Ui, end_line: bool) {
    rule_styled(ui, end_line, false);
}

/// A thematic break. `short` draws a centered hairline a fifth of the
/// column wide instead of a separator across the whole column.
pub fn rule_styled(ui: &mut Ui, end_line: bool, short: bool) {
    if short {
        let width = ui.available_size_before_wrap().x;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 6.0), Sense::hover());
        let half = (width * 0.1).clamp(24.0, 60.0).min(width / 2.0);
        ui.painter().hline(
            (rect.center().x - half)..=(rect.center().x + half),
            rect.center().y,
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    } else {
        ui.add(egui::Separator::default().horizontal());
    }
    // This does not add a new line, but instead ends the separator
    if end_line {
        newline(ui);
    }
}

#[inline]
pub fn soft_break(ui: &mut Ui) {
    ui.label(" ");
}

#[inline]
pub fn newline(ui: &mut Ui) {
    ui.label("\n");
}

/// Whether nothing has been placed on the current row of a wrapping row
/// layout yet.
pub fn row_is_fresh(ui: &Ui) -> bool {
    ui.cursor().left() <= ui.max_rect().left() + 0.5
}

/// Start an empty row `gap` below everything laid out in `ui` so far.
///
/// This is how a [`crate::typography::BlockRhythm`] spaces blocks: the gap
/// is measured from the content's bottom edge, so it does not depend on
/// what the row that ended happened to contain. The new row has no height
/// of its own; whatever is placed on it gives it one. It follows that
/// calling this twice in a row moves nothing the second time round.
pub fn start_row_below(ui: &mut Ui, gap: f32) {
    let top = ui.min_rect().bottom() + gap;
    // A rect that misses the cursor's row starts a new row at its own
    // height; an empty one always misses.
    ui.advance_cursor_after_rect(egui::Rect::from_min_size(
        egui::pos2(ui.max_rect().left(), top),
        Vec2::ZERO,
    ));
    ui.set_row_height(0.0);
}

/// The body font size, which [`crate::typography::BlockRhythm`] and
/// [`crate::typography::ListStyle`] measure in.
pub fn body_font_size(ui: &Ui) -> f32 {
    TextStyle::Body.resolve(ui.style()).size
}

/// Indent what follows on the row by `count` spaces of body text.
///
/// The built-in indent is a label of spaces, which gives the row the
/// height of a line of text. `flat` moves the cursor instead and leaves the
/// row's height to what is placed on it; see [`reserve_list_marker_styled`].
pub fn indent_by_spaces(ui: &mut Ui, count: usize, flat: bool) {
    if flat {
        ui.add_space(width_body_space(ui) * count as f32);
    } else {
        ui.label(" ".repeat(count));
    }
}

/// One reserved list-marker slot. `List::start_item` reserves these before any
/// item text exists; the slot is painted later, once the item's first line is
/// about to be laid out (see [`paint_list_marker`]).
#[derive(Debug, Clone)]
pub struct ListMarkerSlot {
    /// Horizontal centre for a bullet dot.
    pub centre_x: f32,
    /// Right edge an ordered marker's text aligns to.
    pub right_x: f32,
    /// The body style's natural painted height; the dot radius is `raw / 6`.
    pub raw: f32,
    /// The height the marker's box reserved. Doubles as the line-height hint
    /// when a marker is painted without a text format.
    pub box_height: f32,
    /// How the marker is drawn; `None` is the built-in look.
    pub style: Option<crate::typography::ListStyle>,
}

/// Which glyph or shape paints into a reserved [`ListMarkerSlot`].
#[derive(Debug, Clone)]
pub enum ListMarkerKind {
    Bullet,
    BulletHollow,
    Number(String),
    /// The check box of a task list item, checked or not. Only painted in
    /// a slot with a [`crate::typography::ListStyle`].
    Task(bool),
}

/// Reserve the horizontal slot for a list marker without painting anything.
///
/// The reserved box is as tall as the item text's line box so the row grows to
/// the same height either way; the vertical position is decided at paint time.
pub fn reserve_list_marker(ui: &mut Ui, row_height: f32) -> ListMarkerSlot {
    reserve_list_marker_styled(ui, row_height, None, false)
}

/// [`reserve_list_marker`] with the slot sized as `style` says; `None` is
/// the built-in slot, four spaces wide.
///
/// `flat` reserves the slot's width only. A slot with height makes the row
/// taller than the block the item text is laid out in, and that block is
/// then placed at the row's bottom, below where the row starts; a flat slot
/// leaves the item text at the top of its row, where a
/// [`crate::typography::BlockRhythm`] measures gaps from.
pub fn reserve_list_marker_styled(
    ui: &mut Ui,
    row_height: f32,
    style: Option<crate::typography::ListStyle>,
    flat: bool,
) -> ListMarkerSlot {
    let raw = height_body(ui);
    let height = row_height.max(raw);
    let width = match style {
        Some(style) => (style.indent * body_font_size(ui)).round(),
        None => width_body_space(ui) * 4.0,
    };
    let allocated = if flat { 0.0 } else { height };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, allocated), Sense::hover());
    ListMarkerSlot {
        centre_x: rect.center().x,
        right_x: rect.right(),
        raw,
        box_height: height,
        style,
    }
}

/// Paint a reserved marker so it lands on the item text's first line.
///
/// `format` is the text format the item text will be laid out with — taken from
/// the label's job immediately before `ui.label` runs. `None` falls back to the
/// body style with the marker's own reserved line box.
///
/// The marker's centre is the text's lowercase optical centre: the baseline of
/// a reference `x` glyph (laid out under the same conditions egui will lay out
/// the item text) minus half the glyph's x-height. That derivation has to live
/// at paint time: egui anchors a wrapping label's galley at the cursor's top
/// edge and gives the first row the cursor's current height as a minimum
/// (`Label::layout_in_ui`), so every widget sharing the marker's line — task
/// checkboxes, inline images — moves the text, and the marker must move with it.
pub fn paint_list_marker(
    ui: &Ui,
    slot: &ListMarkerSlot,
    kind: &ListMarkerKind,
    format: Option<&egui::TextFormat>,
) {
    let (baseline_y, x_height) = first_line_baseline(ui, slot, format);
    let optical_centre_y = baseline_y - x_height / 2.0;
    if let Some(style) = slot.style {
        paint_styled_list_marker(ui, slot, kind, format, style, baseline_y, optical_centre_y);
        return;
    }
    let color = ui.visuals().strong_text_color();
    match kind {
        // Without a list style a task item keeps its bullet and its check
        // box widget; nothing asks for this marker.
        ListMarkerKind::Task(_) => {}
        ListMarkerKind::Bullet => {
            ui.painter().circle_filled(
                egui::pos2(slot.centre_x, optical_centre_y),
                slot.raw / 6.0,
                color,
            );
        }
        ListMarkerKind::BulletHollow => {
            ui.painter().circle(
                egui::pos2(slot.centre_x, optical_centre_y),
                slot.raw / 6.0,
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(0.6_f32, color),
            );
        }
        ListMarkerKind::Number(number) => {
            // Ordered markers are glyphs, so they read as aligned when their
            // baseline matches the item text's baseline; anchoring their box
            // centre on the lowercase optical centre would sink them below it.
            let numbered = format!("{number}.");
            let font = format
                .map(|f| f.font_id.clone())
                .unwrap_or_else(|| TextStyle::Body.resolve(ui.style()));
            let galley = ui.painter().layout_no_wrap(numbered, font, color);
            let Some(row) = galley.rows.first() else {
                return;
            };
            let Some(glyph) = row.glyphs.first() else {
                return;
            };
            let baseline_offset = row.pos.y + glyph.pos.y;
            let pos = egui::pos2(slot.right_x - galley.size().x, baseline_y - baseline_offset);
            ui.painter().add(epaint::TextShape::new(pos, galley, color));
        }
    }
}

/// Distance from the right edge of a styled marker slot, where the item
/// text starts, to the marker, in multiples of the body font size.
const STYLED_MARKER_GAP: f32 = 0.45;
/// Side of a task box in multiples of the body font size.
const TASK_BOX_SIDE: f32 = 0.74;

/// [`paint_list_marker`] for a slot with a [`crate::typography::ListStyle`]:
/// every marker hangs a fixed gap left of the item text, in the style's
/// color.
fn paint_styled_list_marker(
    ui: &Ui,
    slot: &ListMarkerSlot,
    kind: &ListMarkerKind,
    format: Option<&egui::TextFormat>,
    style: crate::typography::ListStyle,
    baseline_y: f32,
    optical_centre_y: f32,
) {
    let em = body_font_size(ui);
    let marker_right = slot.right_x - STYLED_MARKER_GAP * em;
    let radius = style.bullet_radius * em;
    match kind {
        ListMarkerKind::Bullet => {
            ui.painter().circle_filled(
                egui::pos2(marker_right - radius, optical_centre_y),
                radius,
                style.marker,
            );
        }
        ListMarkerKind::BulletHollow => {
            ui.painter().circle_stroke(
                egui::pos2(marker_right - radius, optical_centre_y),
                radius,
                egui::Stroke::new(1.0_f32, style.marker),
            );
        }
        ListMarkerKind::Number(number) => {
            let font = format
                .map(|f| f.font_id.clone())
                .unwrap_or_else(|| TextStyle::Body.resolve(ui.style()));
            let galley = ui
                .painter()
                .layout_no_wrap(format!("{number}."), font, style.marker);
            let Some(baseline_offset) = galley
                .rows
                .first()
                .and_then(|row| Some(row.pos.y + row.glyphs.first()?.pos.y))
            else {
                return;
            };
            let pos = egui::pos2(
                marker_right - galley.size().x,
                baseline_y - baseline_offset,
            );
            ui.painter()
                .add(epaint::TextShape::new(pos, galley, style.marker));
        }
        ListMarkerKind::Task(checked) => {
            let side = (TASK_BOX_SIDE * em).round();
            // Centered on the capitals: a box centered on the lowercase
            // letters hangs below the baseline.
            let centre = egui::pos2(marker_right - side / 2.0, baseline_y - 0.33 * em);
            task_box(ui, egui::Rect::from_center_size(centre, Vec2::splat(side)), *checked, style.marker);
        }
    }
}

/// A read-only task box: an outlined square, with a check mark when
/// `checked`.
pub fn task_box(ui: &Ui, rect: egui::Rect, checked: bool, color: egui::Color32) {
    let rect = egui::emath::GuiRounding::round_to_pixels(rect, ui.pixels_per_point());
    let radius = (rect.width() * 0.2).round();
    ui.painter().rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.25_f32, color),
        egui::StrokeKind::Inside,
    );
    if checked {
        let at = |x: f32, y: f32| rect.min + egui::vec2(x, y) * rect.size();
        ui.painter().add(egui::Shape::line(
            vec![at(0.25, 0.52), at(0.43, 0.70), at(0.76, 0.30)],
            egui::Stroke::new(1.6_f32, color),
        ));
    }
}

/// Where the item text's first line will paint its baseline, and how tall a
/// lowercase `x` is under that format, both in ui coordinates.
///
/// The reference galley reproduces what `Label::layout_in_ui` does to the job
/// it lays out on a wrapping horizontal row: `first_row_min_height` from the
/// cursor's current height, `valign` from the layout, and the format's own
/// line height. The row box that drives glyph placement is
/// `max(cursor height, line height)` — so this must run after every widget
/// sharing the marker's line has been allocated, right before the text's own
/// label.
fn first_line_baseline(
    ui: &Ui,
    slot: &ListMarkerSlot,
    format: Option<&egui::TextFormat>,
) -> (f32, f32) {
    let cursor_top = ui.cursor().top();
    let mut format = format.cloned().unwrap_or_else(|| egui::TextFormat {
        font_id: TextStyle::Body.resolve(ui.style()),
        line_height: Some(slot.box_height),
        ..egui::TextFormat::default()
    });
    format.valign = ui.text_valign();
    let job = egui::text::LayoutJob {
        text: "x".to_owned(),
        sections: vec![egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: 0..1,
            format,
        }],
        first_row_min_height: ui.cursor().height(),
        halign: egui::Align::LEFT,
        justify: false,
        wrap: egui::text::TextWrapping::default(),
        ..egui::text::LayoutJob::default()
    };
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let row = galley.rows.first().expect("galleys are never empty");
    let glyph = row
        .glyphs
        .first()
        .expect("a one-character job lays out one glyph");
    let baseline_offset = row.pos.y + glyph.pos.y;
    (cursor_top + baseline_offset, glyph.uv_rect.size.y)
}

/// Reserve and immediately paint a bullet marker.
///
/// The code generated by `egui_commonmark_macros` has no render state to defer
/// a paint with, so it cannot wait for the item text's layout. Positioning from
/// the live cursor is still exact whenever the item text directly follows the
/// marker on the same line.
pub fn bullet_point(ui: &mut Ui, row_height: f32) {
    let slot = reserve_list_marker(ui, row_height);
    paint_list_marker(ui, &slot, &ListMarkerKind::Bullet, None);
}

pub fn bullet_point_hollow(ui: &mut Ui, row_height: f32) {
    let slot = reserve_list_marker(ui, row_height);
    paint_list_marker(ui, &slot, &ListMarkerKind::BulletHollow, None);
}

pub fn number_point(ui: &mut Ui, number: &str, row_height: f32) {
    let slot = reserve_list_marker(ui, row_height);
    paint_list_marker(ui, &slot, &ListMarkerKind::Number(number.to_owned()), None);
}

/// How far the ink of body text reaches above and below its baseline, in
/// multiples of the font size: a chip this tall holds any line of text.
const TEXT_BAND_ABOVE: f32 = 0.80;
const TEXT_BAND_BELOW: f32 = 0.27;
/// How far a chip reaches past the text it holds, on every side.
const CHIP_MARGIN: f32 = 2.0;

/// A label of inline code on a rounded chip instead of on the square,
/// line-high background egui paints behind code.
///
/// The label is the one `ui.label` would add: the chip is painted behind
/// it from a galley laid out the way the label lays out its own, so no
/// line moves. `wrap` asks for [`egui::Label::wrap`], as a table cell does.
/// Where the label would not flow as wrapped text in a row, the built-in
/// background is kept.
pub fn inline_code_label(ui: &mut Ui, code: RichText, radius: f32, wrap: bool) -> egui::Response {
    let label = |code: RichText| {
        if wrap {
            egui::Label::new(code).wrap()
        } else {
            egui::Label::new(code)
        }
    };
    let layout = *ui.layout();
    let available_width = ui.available_width();
    let flows = (wrap || ui.wrap_mode() == egui::TextWrapMode::Wrap)
        && layout.main_dir() == egui::Direction::LeftToRight
        && layout.main_wrap()
        && available_width.is_finite();
    if !flows {
        return ui.add(label(code));
    }

    let fill = ui.visuals().code_bg_color;
    ui.visuals_mut().code_bg_color = egui::Color32::TRANSPARENT;

    // What `Label::layout_in_ui` does on a wrapping row, step for step.
    let mut job = egui::text::LayoutJob::default();
    code.clone().append_to(
        &mut job,
        ui.style(),
        egui::FontSelection::Default,
        ui.text_valign(),
    );
    let line_height = job.sections.first().and_then(|section| section.format.line_height);
    let code_font = job.sections.first().map(|section| section.format.font_id.clone());
    job.wrap.max_width = available_width;
    job.first_row_min_height = ui.cursor().height();
    job.halign = egui::Align::Min;
    job.justify = false;
    if let Some(first_section) = job.sections.first_mut() {
        first_section.leading_space = available_width - ui.available_size_before_wrap().x;
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let origin = egui::vec2(ui.max_rect().left(), ui.cursor().top());

    // The chip is set on the baseline of the text around the code, which a
    // code face with a different ascent does not share.
    let body_font = TextStyle::Body.resolve(ui.style());
    let to_text_baseline = code_font.map_or(0.0, |code_font| {
        baseline_in_line(ui, &body_font, line_height) - baseline_in_line(ui, &code_font, line_height)
    });
    let above = TEXT_BAND_ABOVE * body_font.size + CHIP_MARGIN;
    let below = TEXT_BAND_BELOW * body_font.size + CHIP_MARGIN;
    for (index, row) in galley.rows.iter().enumerate() {
        let Some(glyph) = row.glyphs.first() else {
            continue;
        };
        let extent = if index == 0 {
            row.rect_without_leading_space()
        } else {
            row.rect()
        };
        let baseline = row.pos.y + glyph.pos.y + to_text_baseline;
        // Never taller than the line: the chips of code that wraps must
        // not run into each other.
        let chip = egui::Rect::from_x_y_ranges(
            extent.left() - CHIP_MARGIN..=extent.right() + CHIP_MARGIN,
            (baseline - above).max(extent.top())..=(baseline + below).min(extent.bottom()),
        )
        .translate(origin);
        if ui.is_rect_visible(chip) {
            ui.painter().rect_filled(chip, radius, fill);
        }
    }

    let response = ui.add(label(code));
    ui.visuals_mut().code_bg_color = fill;
    response
}

/// Distance from the top of a line of `line_height` to the baseline of text
/// set in `font` on it.
fn baseline_in_line(ui: &Ui, font: &egui::FontId, line_height: Option<f32>) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        "x",
        0.0,
        egui::TextFormat {
            font_id: font.clone(),
            line_height,
            ..Default::default()
        },
    );
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    galley
        .rows
        .first()
        .and_then(|row| Some(row.pos.y + row.glyphs.first()?.pos.y))
        .unwrap_or(font.size)
}

#[inline]
pub fn footnote_start(ui: &mut Ui, note: &str) {
    ui.label(RichText::new(note).raised().strong().small());
}

pub fn footnote(ui: &mut Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width_body_space(ui) * 4.0, height_body(ui)),
        Sense::hover(),
    );
    ui.painter().text(
        rect.right_top(),
        egui::Align2::RIGHT_TOP,
        format!("{text}."),
        TextStyle::Small.resolve(ui.style()),
        ui.visuals().strong_text_color(),
    );
}

/// The label of a footnote definition as the start of its line: small, in
/// `color`, with the note's text following on the same row.
pub fn footnote_label(ui: &mut Ui, text: &str, color: egui::Color32) {
    ui.label(RichText::new(format!("{text}.")).small().color(color));
    ui.add_space(0.4 * body_font_size(ui));
}

fn height_body(ui: &Ui) -> f32 {
    ui.text_style_height(&TextStyle::Body)
}

fn width_body_space(ui: &Ui) -> f32 {
    let id = TextStyle::Body.resolve(ui.style());
    ui.fonts_mut(|f| f.glyph_width(&id, ' '))
}

/// Enhanced/specialized version of egui's code blocks. This one features copy button and borders.
/// Uses selectable Label instead of TextEdit to allow text selection across code block boundaries.
pub fn code_block(
    ui: &mut Ui,
    text: &str,
    layout_job: egui::text::LayoutJob,
    max_width: f32,
    id: egui::Id,
) {
    code_block_styled(ui, text, layout_job, max_width, id, None);
}

/// [`code_block`] with the box drawn as `style` says; `None` is the
/// built-in box.
pub fn code_block_styled(
    ui: &mut Ui,
    text: &str,
    layout_job: egui::text::LayoutJob,
    _max_width: f32,
    id: egui::Id,
    style: Option<crate::typography::CodeBlockStyle>,
) {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let mut layout_job = layout_job;
    if style.is_some() {
        crate::typography::CodeBlockStyle::trim_final_newline(&mut layout_job);
    }

    // Reserve space for background drawing
    let where_to_put_background = ui.painter().add(egui::Shape::Noop);

    // Use a Frame with horizontal scroll for wide code blocks.
    // All code blocks use the same width with horizontal scrolling
    // for content that exceeds it.
    let frame_response = egui::Frame::new()
        .inner_margin(egui::Margin::same(style.map_or(8, |style| style.padding)))
        .show(ui, |ui| {
            // Force all code blocks to fill the available width, but never
            // wider than what is visible: the CentralPanel's clip spans the
            // whole window (egui clips only side panels to their own rect),
            // so an unclamped fill used to paint over the right sidebar.
            let visible_width = (ui.clip_rect().right() - ui.max_rect().left()).max(0.0);
            ui.set_min_width(ui.available_width().min(visible_width));
            egui::ScrollArea::horizontal()
                .id_salt(id)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(layout_job)
                            .selectable(true)
                            .wrap_mode(egui::TextWrapMode::Extend),
                    )
                })
                .inner
        });

    let frame_rect = frame_response.response.rect;

    // Draw background color + frame
    ui.painter().set(
        where_to_put_background,
        match style {
            Some(style) => epaint::RectShape::new(
                frame_rect,
                egui::CornerRadius::same(style.corner_radius),
                style.fill,
                style.stroke,
                egui::StrokeKind::Outside,
            ),
            None => epaint::RectShape::new(
                frame_rect,
                ui.style().noninteractive().corner_radius,
                ui.visuals().extreme_bg_color,
                ui.visuals().widgets.noninteractive.bg_stroke,
                egui::StrokeKind::Outside,
            ),
        },
    );

    // Copy icon
    let spacing = &ui.style().spacing;
    let position = egui::pos2(
        frame_rect.right_top().x - spacing.icon_width * 0.5 - spacing.button_padding.x,
        frame_rect.right_top().y + spacing.button_padding.y * 2.0,
    );

    // Check if we should show ✔ instead of 🗐 if the text was copied and the mouse is hovered
    let persistent_id = ui.make_persistent_id(frame_response.response.id);
    let copied_icon = ui.memory_mut(|m| *m.data.get_temp_mut_or_default::<bool>(persistent_id));

    let copy_button = ui
        .put(
            egui::Rect {
                min: position,
                max: position,
            },
            egui::Button::new(if copied_icon { "✔" } else { "🗐" })
                .small()
                .frame(false)
                .fill(egui::Color32::TRANSPARENT),
        )
        .on_hover_cursor(
            ui.visuals()
                .interact_cursor
                .unwrap_or(egui::CursorIcon::Default),
        );

    // Update icon state in persistent memory
    if copied_icon && !copy_button.hovered() {
        ui.memory_mut(|m| *m.data.get_temp_mut_or_default(persistent_id) = false);
    }
    if !copied_icon && copy_button.clicked() {
        ui.memory_mut(|m| *m.data.get_temp_mut_or_default(persistent_id) = true);
    }

    // Copy full code block text when button clicked
    if copy_button.clicked() {
        ui.ctx().copy_text(text.to_owned());
    }
}

// Stripped down version of egui's Checkbox. The only difference is that this
// creates a noninteractive checkbox. ui.add_enabled could have been used instead,
// but it makes the checkbox too grey.
pub struct ImmutableCheckbox<'a> {
    checked: &'a mut bool,
    /// Outline and check mark color; `None` is the widget outline color.
    marker: Option<egui::Color32>,
}

impl<'a> ImmutableCheckbox<'a> {
    pub fn without_text(checked: &'a mut bool) -> Self {
        ImmutableCheckbox {
            checked,
            marker: None,
        }
    }

    /// Draw the outline and the check mark in `marker` instead of the
    /// widget outline color.
    pub fn marker(mut self, marker: Option<egui::Color32>) -> Self {
        self.marker = marker;
        self
    }
}

impl egui::Widget for ImmutableCheckbox<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let ImmutableCheckbox { checked, marker } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;

        let mut desired_size = egui::vec2(icon_width, 0.0);
        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().visuals.noninteractive();
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape::new(
                big_icon_rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.bg_fill,
                marker.map_or(visuals.bg_stroke, |color| egui::Stroke::new(1.0_f32, color)),
                egui::StrokeKind::Inside,
            ));

            if *checked {
                // Check mark:
                ui.painter().add(egui::Shape::line(
                    vec![
                        egui::pos2(small_icon_rect.left(), small_icon_rect.center().y),
                        egui::pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                        egui::pos2(small_icon_rect.right(), small_icon_rect.top()),
                    ],
                    marker.map_or(visuals.fg_stroke, |color| egui::Stroke::new(1.5_f32, color)),
                ));
            }
        }

        response
    }
}

pub fn blockquote(ui: &mut Ui, accent: egui::Color32, add_contents: impl FnOnce(&mut Ui)) {
    let start = ui.painter().add(egui::Shape::Noop);
    let response = egui::Frame::new()
        // offset the frame so that we can use the space for the horizontal line and other stuff
        // By not using a separator we have better control
        .outer_margin(egui::Margin {
            left: 10,
            ..Default::default()
        })
        .show(ui, add_contents)
        .response;

    // FIXME: Add some rounding

    ui.painter().set(
        start,
        egui::epaint::Shape::line_segment(
            [
                egui::pos2(response.rect.left_top().x, response.rect.left_top().y + 5.0),
                egui::pos2(
                    response.rect.left_bottom().x,
                    response.rect.left_bottom().y - 5.0,
                ),
            ],
            egui::Stroke::new(3.0_f32, accent),
        ),
    );
}
