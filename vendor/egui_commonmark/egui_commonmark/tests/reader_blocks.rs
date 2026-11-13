//! The opt-in block styling a reader view asks for: a rhythm of gaps
//! between blocks, typeset tables, list markers that hang in a slot, and
//! inline code on chips. Every test reads what a frame painted.

use egui::{Color32, Context, Pos2, Rect, Shape, Stroke, TextStyle};
use egui_commonmark_extended::{
    BlockRhythm, BlockStyle, CommonMarkCache, CommonMarkViewer, ListStyle, TableStyle,
};

/// Body font size of every test, so a gap of one em is 18 px.
const EM: f32 = 18.0;
const COLUMN: f32 = 680.0;
const MARKER: Color32 = Color32::from_rgb(120, 110, 100);
const RULE: Color32 = Color32::from_rgb(200, 190, 180);
const HEADER_RULE: Color32 = Color32::from_rgb(150, 140, 130);

const RHYTHM: BlockRhythm = BlockRhythm {
    paragraph: 1.0,
    block: 1.5,
    heading_above: 2.0,
    heading_below: 0.5,
    tight_item: 0.25,
    display_math: 0.5,
};

fn rhythm() -> BlockStyle {
    BlockStyle {
        rhythm: Some(RHYTHM),
        ..Default::default()
    }
}

fn list_style() -> ListStyle {
    ListStyle {
        marker: MARKER,
        bullet_radius: 0.125,
        indent: 1.5,
    }
}

fn table_style() -> TableStyle {
    TableStyle {
        rule: Stroke::new(1.0_f32, RULE),
        header_rule: Stroke::new(1.0_f32, HEADER_RULE),
        cell_padding: 10.0,
        fill_width: true,
    }
}

#[derive(Debug, Clone)]
struct Text {
    text: String,
    /// The galley's bounds. On a wrapping row the galley starts at the left
    /// edge of its ui, wherever on the row its text starts.
    rect: Rect,
    /// Where the first line of the text itself starts.
    left: f32,
    clip: Rect,
    color: Color32,
    background: Color32,
}

#[derive(Debug, Clone)]
struct Filled {
    rect: Rect,
    fill: Color32,
    stroke: Stroke,
    radius: f32,
}

/// What one frame painted.
#[derive(Debug, Clone)]
struct Frame {
    texts: Vec<Text>,
    lines: Vec<([Pos2; 2], Stroke)>,
    circles: Vec<(Pos2, f32, Color32, Stroke)>,
    rects: Vec<Filled>,
    /// Open paths: the check mark of a task box.
    paths: usize,
    /// Bounds of everything the renderer laid out.
    extent: Rect,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            lines: Vec::new(),
            circles: Vec::new(),
            rects: Vec::new(),
            paths: 0,
            extent: Rect::NOTHING,
        }
    }
}

impl Frame {
    fn collect(&mut self, shape: &Shape, clip: Rect) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| self.collect(shape, clip)),
            Shape::Text(text) => {
                let format = text.galley.job.sections.first().map(|section| &section.format);
                self.texts.push(Text {
                    text: text.galley.job.text.clone(),
                    rect: text.galley.rect.translate(text.pos.to_vec2()),
                    left: text.pos.x
                        + text
                            .galley
                            .rows
                            .first()
                            .map_or(0.0, |row| row.rect_without_leading_space().left()),
                    clip,
                    color: format.map_or(Color32::PLACEHOLDER, |format| format.color),
                    background: format.map_or(Color32::TRANSPARENT, |format| format.background),
                });
            }
            Shape::LineSegment { points, stroke } if stroke.width > 0.0 => {
                self.lines.push((*points, *stroke));
            }
            Shape::Circle(circle) => {
                self.circles
                    .push((circle.center, circle.radius, circle.fill, circle.stroke));
            }
            // Not the panel's own background.
            Shape::Rect(rect) if rect.rect.width() < 5000.0 => self.rects.push(Filled {
                rect: rect.rect,
                fill: rect.fill,
                stroke: rect.stroke,
                radius: f32::from(rect.corner_radius.nw),
            }),
            Shape::Path(path) if !path.closed => self.paths += 1,
            _ => {}
        }
    }

    /// The painted text that contains `marker`.
    fn text(&self, marker: &str) -> &Text {
        let mut found = self.texts.iter().filter(|text| text.text.contains(marker));
        let text = found
            .next()
            .unwrap_or_else(|| panic!("no text {marker:?} in {:#?}", self.texts));
        assert!(found.next().is_none(), "{marker:?} was painted twice");
        text
    }

    /// Space between the bottom of `upper` and the top of `lower`.
    fn gap(&self, upper: &str, lower: &str) -> f32 {
        self.text(lower).rect.top() - self.text(upper).rect.bottom()
    }

    /// Horizontal lines, top to bottom.
    fn rules(&self) -> Vec<([Pos2; 2], Stroke)> {
        let mut rules: Vec<_> = self
            .lines
            .iter()
            .filter(|(points, _)| points[0].y == points[1].y)
            .copied()
            .collect();
        rules.sort_by(|a, b| a.0[0].y.total_cmp(&b.0[0].y));
        rules
    }

    fn vertical_lines(&self) -> usize {
        self.lines
            .iter()
            .filter(|(points, _)| points[0].x == points[1].x)
            .count()
    }
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 0.6
}

#[track_caller]
fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(close(actual, expected), "{what}: {actual} is not {expected}");
}

fn context() -> Context {
    let ctx = Context::default();
    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(TextStyle::Body, egui::FontId::proportional(EM));
    });
    ctx
}

fn viewer<'a>(style: &BlockStyle) -> CommonMarkViewer<'a> {
    CommonMarkViewer::new()
        .default_width(Some(COLUMN as usize))
        .table_max_width(Some(COLUMN as usize))
        .line_height(1.6)
        .indentation_spaces(2)
        .block_style(style.clone())
}

/// Render `markdown` in a column of [`COLUMN`] px and return the settled
/// frame.
fn show_in(ctx: &Context, markdown: &str, style: &BlockStyle) -> Frame {
    let mut cache = CommonMarkCache::default();
    let mut frame = Frame::default();
    for _ in 0..3 {
        ctx.begin_pass(egui::RawInput::default());
        let mut extent = Rect::NOTHING;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(COLUMN);
            extent = viewer(style).show(ui, &mut cache, markdown).response.rect;
        });
        frame = Frame {
            extent,
            ..Frame::default()
        };
        for clipped in ctx.end_pass().shapes {
            frame.collect(&clipped.shape, clipped.clip_rect);
        }
    }
    frame
}

fn show(markdown: &str, style: &BlockStyle) -> Frame {
    show_in(&context(), markdown, style)
}

// --- rhythm ---------------------------------------------------------------

#[test]
fn blocks_are_spaced_by_what_they_are() {
    let frame = show(
        "# Title\n\nPara one.\n\nPara two.\n\n## Heading two\n\nPara three.\n\n> Quoted.\n\nPara four.\n",
        &rhythm(),
    );

    assert_close(frame.gap("Title", "Para one"), 0.5 * EM, "below a heading");
    assert_close(frame.gap("Para one", "Para two"), EM, "between paragraphs");
    assert_close(frame.gap("Para two", "Heading two"), 2.0 * EM, "above a heading");
    assert_close(frame.gap("Heading two", "Para three"), 0.5 * EM, "below a heading");
    assert_close(frame.gap("Para three", "Quoted"), 1.5 * EM, "above a quote");
    assert_close(frame.gap("Quoted", "Para four"), 1.5 * EM, "below a quote");
}

#[test]
fn without_a_rhythm_blocks_are_a_blank_line_apart() {
    let ctx = context();
    let frame = show_in(&ctx, "Para one.\n\nPara two.\n", &BlockStyle::default());

    let body = TextStyle::Body.resolve(&ctx.style());
    let blank_line = ctx.fonts_mut(|fonts| fonts.row_height(&body));
    assert_close(frame.gap("Para one", "Para two"), blank_line, "the built-in gap");
    assert!(!close(blank_line, EM), "the control must differ from the rhythm");
}

#[test]
fn a_gap_is_the_larger_wish_of_two_neighbors_not_their_sum() {
    let frame = show("> Quoted.\n\n## Heading\n\n## Next heading\n\nText.\n", &rhythm());

    assert_close(frame.gap("Quoted", "Heading"), 2.0 * EM, "a heading after a quote");
    assert_close(
        frame.gap("Heading", "Next heading"),
        0.5 * EM,
        "stacked headings",
    );
}

#[test]
fn a_quote_inside_a_quote_is_a_quote_of_its_own() {
    let frame = show("> Outer.\n>\n> > Inner.\n>\n> Tail.\n\nAfter.\n", &rhythm());
    let [outer, inner, tail, after] =
        ["Outer", "Inner", "Tail", "After"].map(|marker| frame.text(marker));

    assert!(outer.left > after.left, "a quote steps in");
    assert_close(
        inner.left - outer.left,
        outer.left - after.left,
        "the inner quote steps in as far again",
    );
    assert_eq!(frame.vertical_lines(), 2, "each quote has its bar");
    assert_close(frame.gap("Outer", "Inner"), 1.5 * EM, "above the inner quote");
    assert_close(frame.gap("Inner", "Tail"), 1.5 * EM, "below the inner quote");
    assert_close(tail.left, outer.left, "back in the outer quote");
    assert_eq!(tail.color, outer.color, "and still quoted");
    assert_ne!(tail.color, after.color, "the control: prose is not weak");
}

#[test]
fn the_last_block_leaves_no_gap_below_it() {
    let frame = show("Para one.\n\nLast para.\n", &rhythm());

    assert_close(
        frame.extent.bottom(),
        frame.text("Last para").rect.bottom(),
        "the document ends with its last line",
    );
}

#[test]
fn a_thematic_break_is_spaced_as_a_block() {
    let frame = show("Before.\n\n---\n\nAfter.\n", &rhythm());

    let rule_height = frame.gap("Before", "After") - 2.0 * 1.5 * EM;
    assert!(
        (0.0..=8.0).contains(&rule_height),
        "a block gap on each side of the rule, and the rule: {rule_height}"
    );
}

#[test]
fn tight_list_items_sit_closer_than_loose_ones() {
    let frame = show(
        "Before.\n\n- tight a\n- tight b\n\nBetween.\n\n- loose a\n\n- loose b\n\nAfter.\n",
        &rhythm(),
    );

    assert_close(frame.gap("Before", "tight a"), EM, "above a list");
    assert_close(frame.gap("tight a", "tight b"), 0.25 * EM, "tight items");
    assert_close(frame.gap("tight b", "Between"), EM, "below a list");
    assert_close(frame.gap("loose a", "loose b"), EM, "loose items");
    assert_close(frame.gap("loose b", "After"), EM, "below a loose list");
}

#[test]
fn a_nested_list_follows_its_parent_item_as_closely_as_its_own_items() {
    let frame = show(
        "- parent\n  - child one\n  - child two\n- next item\n",
        &rhythm(),
    );

    assert_close(frame.gap("parent", "child one"), 0.25 * EM, "into the nested list");
    assert_close(frame.gap("child one", "child two"), 0.25 * EM, "nested items");
    assert_close(frame.gap("child two", "next item"), 0.25 * EM, "out of it");
}

#[test]
fn paragraphs_of_one_list_item_are_set_apart() {
    let frame = show("- first para\n\n  second para\n\n- other item\n", &rhythm());

    assert_close(frame.gap("first para", "second para"), EM, "inside the item");
    assert_close(
        frame.text("second para").left,
        frame.text("first para").left,
        "both paragraphs start at the item's text",
    );
    assert_close(frame.gap("second para", "other item"), EM, "to the next item");
}

#[test]
fn a_block_after_the_text_of_a_tight_item_starts_below_it() {
    let frame = show(
        "- item text\n  ```\n  code line\n  ```\n  trailing text\n- next item\n",
        &rhythm(),
    );

    let mut bottom = f32::NEG_INFINITY;
    for marker in ["item text", "code line", "trailing text", "next item"] {
        let rect = frame.text(marker).rect;
        assert!(rect.top() >= bottom, "{marker} overlaps what is above it");
        bottom = rect.bottom();
    }
}

#[test]
fn every_kind_of_block_keeps_its_place_in_the_rhythm() {
    let markdown = "Lead.\n\nFirst[^note].\n\n[^note]: Footnote text.\n\n<div>html block</div>\n\n\
        Hard  \nbreak.\n\n> - quoted item\n\n| h |\n|---|\n| cell |\n\n1. numbered\n\nEnd.\n";
    let frame = show(markdown, &rhythm());

    let mut bottom = f32::NEG_INFINITY;
    for marker in [
        "Lead", "First", "Footnote text", "html block", "Hard", "break.", "quoted item", "cell",
        "numbered", "End",
    ] {
        let rect = frame.text(marker).rect;
        assert!(
            rect.top() >= bottom - 0.6,
            "{marker} overlaps what is above it"
        );
        bottom = rect.bottom();
    }
}

#[test]
fn a_footnote_starts_with_its_label_and_runs_on_from_it() {
    let frame = show(
        "Claim[^why].\n\n[^why]: The note that backs the claim.\n",
        &styled_list(),
    );

    let label = frame.text("why.");
    let note = frame.text("The note");
    assert_eq!(label.color, MARKER);
    assert_close(label.left, frame.extent.left(), "the label starts the line");
    assert!(note.left > label.rect.right(), "the note follows its label");
    assert!(
        note.rect.y_range().contains(label.rect.center().y),
        "on the same line"
    );
    assert_close(frame.gap("Claim", "The note"), EM, "a paragraph below the text");
}

// Without the `math` feature `$$` is not parsed as a formula at all.
#[cfg(feature = "math")]
#[test]
fn text_after_a_display_formula_starts_at_the_margin() {
    let frame = show(
        "Text before the formula\n$$\na^2 + b^2\n$$\nand text after it.\n",
        &rhythm(),
    );

    let (before, after) = (frame.text("Text before"), frame.text("and text after"));
    assert!(after.rect.top() > before.rect.bottom(), "the formula has its own lines");
    assert_close(after.left, before.left, "no stray space indents the line");
}

/// A long document of every block the rhythm spaces, with a unique word
/// per block.
fn long_document() -> String {
    (0..40)
        .map(|n| {
            format!(
                "## head{n}\n\npara{n} with enough words to be a line of running text.\n\n\
                 - tight{n}a\n- tight{n}b\n  - nested{n}\n\n- [x] task{n}\n\n\
                 > quote{n}\n\n| name{n} | value{n} |\n|---|--:|\n| cell{n} | {n}.5 |\n\n\
                 ```\ncode{n}\n```\n\n---\n\n1. loose{n}a\n\n2. loose{n}b\n\n"
            )
        })
        .collect()
}

/// One frame of the long document in a scrolling pane, with the scroll
/// offset the frame ended at and the height of the whole document.
fn scrolled_frame(
    ctx: &Context,
    cache: &mut CommonMarkCache,
    markdown: &str,
    offset: Option<f32>,
    full: bool,
) -> (Frame, f32, f32) {
    let style = BlockStyle {
        rhythm: Some(RHYTHM),
        list: Some(list_style()),
        table: Some(table_style()),
        center_content: true,
        inline_code_radius: Some(4.0),
        ..Default::default()
    };
    let mut frame = Frame::default();
    let (mut at, mut height) = (0.0, 0.0);
    ctx.begin_pass(egui::RawInput::default());
    egui::CentralPanel::default().show(ctx, |ui| {
        // A pane wider than the column, as a window is.
        let pane = Rect::from_min_size(ui.max_rect().min, egui::vec2(900.0, 500.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(pane), |ui| {
            let out = viewer(&style)
                .pending_scroll_offset(offset)
                .force_full_render(full)
                .show_scrollable("reader_blocks", ui, cache, markdown);
            (at, height) = (out.state.offset.y, out.content_size.y);
        });
    });
    for clipped in ctx.end_pass().shapes {
        frame.collect(&clipped.shape, clipped.clip_rect);
    }
    frame
        .texts
        .retain(|text| text.rect.intersects(text.clip) && !text.text.trim().is_empty());
    (frame, at, height)
}

#[test]
fn a_viewport_slice_paints_every_block_where_the_full_pass_measured_it() {
    let markdown = long_document();
    let ctx = context();
    let mut cache = CommonMarkCache::default();
    let (_, _, height) = scrolled_frame(&ctx, &mut cache, &markdown, None, false);
    assert!(height > 8000.0, "the document must be many viewports long");

    // Offsets that do not divide a section: over the run the slice starts
    // at every kind of block the document has.
    for step in 1..=60_u16 {
        let offset = (f32::from(step) * 217.0).min(height - 600.0);
        // A full paint at this offset, then the slice the next frame shows.
        let (full, full_at, _) = scrolled_frame(&ctx, &mut cache, &markdown, Some(offset), true);
        let (slice, slice_at, slice_height) = scrolled_frame(&ctx, &mut cache, &markdown, None, false);
        assert_close(slice_at, full_at, "both frames show the same offset");
        assert!(
            (slice_height - height).abs() <= 2.0,
            "the slice at {offset} changed the document height from {height} to {slice_height}"
        );
        assert!(slice.texts.len() >= 8, "the slice at {offset} is nearly empty");

        for text in &slice.texts {
            let measured = full
                .texts
                .iter()
                .find(|candidate| candidate.text == text.text)
                .unwrap_or_else(|| panic!("{:?} is only in the slice at {offset}", text.text));
            assert!(
                close(text.rect.top(), measured.rect.top())
                    && close(text.rect.left(), measured.rect.left()),
                "{:?} at offset {offset}: slice {:?}, full pass {:?}",
                text.text,
                text.rect.min,
                measured.rect.min,
            );
        }
    }
}

// --- tables ---------------------------------------------------------------

const TABLE: &str = "| Name | Count | Share |\n|:--|--:|:-:|\n| alpha | 5 | 10% |\n\
    | a longer name | 1200 | a wide share |\n| gamma | 30 | 2% |\n";

fn styled_table(style: TableStyle) -> BlockStyle {
    BlockStyle {
        table: Some(style),
        ..Default::default()
    }
}

#[test]
fn a_styled_table_is_ruled_between_rows_and_has_no_box_stripes_or_grid() {
    let frame = show(TABLE, &styled_table(table_style()));

    assert_eq!(frame.vertical_lines(), 0, "no grid lines at rest");
    assert!(
        frame
            .rects
            .iter()
            .all(|rect| rect.fill == Color32::TRANSPARENT && rect.stroke.width == 0.0),
        "no box and no stripes: {:#?}",
        frame.rects
    );
    let rules = frame.rules();
    let strokes: Vec<Color32> = rules.iter().map(|(_, stroke)| stroke.color).collect();
    assert_eq!(
        strokes,
        [HEADER_RULE, RULE, RULE],
        "a rule under the header and between rows, none under the last row"
    );
    // Every rule lies between two rows, spans the table and sits on a
    // pixel center, where a one pixel line is drawn at full strength.
    let rows = ["Name", "alpha", "a longer name", "gamma"];
    for (index, (points, _)) in rules.iter().enumerate() {
        let y = points[0].y;
        assert!(frame.text(rows[index]).rect.bottom() < y);
        assert!(y < frame.text(rows[index + 1]).rect.top());
        assert!(
            (y.fract() - 0.5).abs() < 0.01,
            "the rule at {y} is not on a pixel center"
        );
        assert_close(points[0].x, frame.text("Name").left, "rule start");
        assert!(points[1].x - points[0].x > COLUMN - 4.0, "the rule spans the table");
    }
}

/// One frame of [`TABLE`] in a styled table with the pointer doing
/// `events`.
fn table_frame(ctx: &Context, cache: &mut CommonMarkCache, events: Vec<egui::Event>) -> Frame {
    ctx.begin_pass(egui::RawInput {
        events,
        ..Default::default()
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.set_width(COLUMN);
        viewer(&styled_table(table_style())).show(ui, cache, TABLE);
    });
    let mut frame = Frame::default();
    for clipped in ctx.end_pass().shapes {
        frame.collect(&clipped.shape, clipped.clip_rect);
    }
    frame
}

#[test]
fn a_column_separator_shows_under_the_pointer_and_resizes_the_columns_beside_it() {
    let ctx = context();
    let mut cache = CommonMarkCache::default();
    let settled = (0..3)
        .map(|_| table_frame(&ctx, &mut cache, Vec::new()))
        .last()
        .unwrap();
    assert_eq!(settled.vertical_lines(), 0);
    let row = settled.text("alpha").rect.center().y;
    let name_column = settled.text("alpha").clip;
    let count_column = settled.text("1200").clip;
    let table_width = |frame: &Frame| {
        let (points, _) = frame.rules()[0];
        points[1].x - points[0].x
    };

    // Sweep the pointer along a row until a separator answers.
    let separator = |frame: &Frame| {
        frame
            .lines
            .iter()
            .find(|(points, _)| points[0].x == points[1].x)
            .copied()
    };
    let mut hovered = None;
    let mut x = name_column.left() + 20.0;
    while hovered.is_none() && x < name_column.right() + 100.0 {
        x += 2.0;
        table_frame(&ctx, &mut cache, vec![egui::Event::PointerMoved(egui::pos2(x, row))]);
        hovered = separator(&table_frame(&ctx, &mut cache, Vec::new()));
    }
    let (line, stroke) = hovered.expect("no separator under the pointer at the column's end");
    assert_eq!(stroke, Stroke::new(1.0_f32, HEADER_RULE));
    // Within the reach of a resize handle, between the two columns.
    assert!((line[0].x - x).abs() <= 10.0, "line at {}, pointer at {x}", line[0].x);
    assert!(name_column.right() <= line[0].x && line[0].x <= count_column.left());
    assert!(line[0].y <= settled.text("Name").rect.top());
    assert!(line[1].y >= settled.text("gamma").rect.bottom());

    // Hold it and pull right: the column on its left grows by what the one
    // on its right gives up, and the table stays as wide as it was.
    let button = |pressed| egui::Event::PointerButton {
        pos: egui::pos2(x, row),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    table_frame(&ctx, &mut cache, vec![button(true)]);
    table_frame(
        &ctx,
        &mut cache,
        vec![egui::Event::PointerMoved(egui::pos2(x + 40.0, row))],
    );
    let dragged = table_frame(&ctx, &mut cache, Vec::new());
    let (held, held_stroke) = separator(&dragged).expect("the held separator stays visible");
    assert!(held_stroke.width > stroke.width, "a held separator is drawn stronger");
    assert_close(held[0].x - line[0].x, 40.0, "the separator follows the pointer");
    assert_close(
        dragged.text("alpha").clip.width() - name_column.width(),
        40.0,
        "the column on the left is wider",
    );
    // All but the pixel a filled table keeps spare comes from the right.
    let given = count_column.width() - dragged.text("1200").clip.width();
    assert!((39.0..=40.6).contains(&given), "the column on the right gave {given}");
    assert!(
        (table_width(&dragged) - table_width(&settled)).abs() <= 1.6,
        "the table keeps its width"
    );

    // Let go and move away: the separator is gone, the widths stay, and
    // the cache learns that rows may have rewrapped.
    let layout_while_dragging = cache.layout_revision();
    table_frame(&ctx, &mut cache, vec![button(false)]);
    assert_ne!(cache.layout_revision(), layout_while_dragging);
    table_frame(
        &ctx,
        &mut cache,
        vec![egui::Event::PointerMoved(egui::pos2(x + 40.0, row + 400.0))],
    );
    let released = table_frame(&ctx, &mut cache, Vec::new());
    assert_eq!(released.vertical_lines(), 0);
    assert_close(
        released.text("alpha").clip.width() - name_column.width(),
        40.0,
        "the new width is kept",
    );
}

#[test]
fn the_built_in_table_keeps_its_box_stripes_and_grid() {
    let frame = show(TABLE, &BlockStyle::default());

    assert!(frame.vertical_lines() > 0);
    assert!(frame.rules().is_empty());
    assert!(frame.rects.iter().any(|rect| rect.stroke.width > 0.0));
}

#[test]
fn a_styled_table_aligns_its_columns_as_the_markdown_asks() {
    let frame = show(TABLE, &styled_table(table_style()));
    let rect = |marker: &str| frame.text(marker).rect;
    let left = |marker: &str| frame.text(marker).left;

    // `:--` left
    assert_close(left("alpha"), left("a longer name"), "left column");
    // `--:` right, header included
    for cell in ["Count", "5", "30"] {
        assert!(
            (rect(cell).right() - rect("1200").right()).abs() <= 1.0,
            "{cell} ends at {} and 1200 at {}",
            rect(cell).right(),
            rect("1200").right()
        );
    }
    assert!(left("5") > left("1200") + 10.0);
    // `:-:` centered
    let center = |marker: &str| (left(marker) + rect(marker).right()) / 2.0;
    for cell in ["Share", "10%", "2%"] {
        assert!(
            (center(cell) - center("a wide share")).abs() <= 1.0,
            "{cell} is not centered on its column"
        );
    }
}

#[test]
fn the_built_in_table_sets_every_column_left() {
    let frame = show(TABLE, &BlockStyle::default());

    assert_close(
        frame.text("5").left,
        frame.text("1200").left,
        "the right-aligned column of the markdown",
    );
}

#[test]
fn text_and_code_in_a_cell_are_aligned_together_and_a_link_is_left_alone() {
    let table = "| Notes |\n|--:|\n| short |\n| plain and `code` together |\n\
        | **strong** and *leaning* |\n| see [the link](https://example.com) here |\n";
    let frame = show(table, &styled_table(table_style()));
    let table_left = frame.rules()[0].0[0].x;
    let right = |marker: &str| frame.text(marker).rect.right();

    assert!(frame.text("short").left > table_left + 100.0, "plain text moves");
    // Text, code and styled text move as one line and end where the
    // column ends.
    for last_label in ["together", "leaning"] {
        assert!(
            (right(last_label) - right("short")).abs() <= 3.0,
            "{last_label} ends at {}, the column at {}",
            right(last_label),
            right("short")
        );
    }
    assert!(frame.text("plain and").left > table_left + 100.0);
    // A link is not measured, so its cell stays where it is laid out.
    assert_close(frame.text("see").left, table_left, "a cell with a link stays");
}

#[test]
fn a_compact_table_fills_its_bound_only_when_asked() {
    let filled = show(TABLE, &styled_table(table_style()));
    let hugging = show(
        TABLE,
        &styled_table(TableStyle {
            fill_width: false,
            ..table_style()
        }),
    );
    let width = |frame: &Frame| {
        let (points, _) = frame.rules()[0];
        points[1].x - points[0].x
    };

    assert!(width(&filled) > COLUMN - 4.0 && width(&filled) <= COLUMN);
    assert!(width(&hugging) < COLUMN - 100.0, "{}", width(&hugging));
    // Filling widens the columns in proportion: the last one moves right.
    assert_close(
        filled.text("gamma").left,
        hugging.text("gamma").left,
        "the first column starts where it did",
    );
    assert!(filled.text("30").rect.right() > hugging.text("30").rect.right() + 50.0);
}

#[test]
fn cell_padding_is_added_above_and_below_every_row() {
    let pitch = |padding: f32| {
        let frame = show(
            TABLE,
            &styled_table(TableStyle {
                cell_padding: padding,
                ..table_style()
            }),
        );
        for text in &frame.texts {
            assert!(
                text.clip.contains_rect(text.rect),
                "{:?} is clipped by its row",
                text.text
            );
        }
        // The text sits the padding below the rule above its row, with as
        // much room again under it.
        let rules = frame.rules();
        let above = frame.text("alpha").rect.top() - rules[0].0[0].y;
        let below = rules[1].0[0].y - frame.text("alpha").rect.bottom();
        assert!((above - padding).abs() <= 3.0, "{above} above for {padding}");
        assert!((below - above).abs() <= 4.0, "{above} above, {below} below");
        (
            frame.text("gamma").rect.top() - frame.text("alpha").rect.top(),
            frame.extent.height(),
        )
    };

    let (bare, bare_height) = pitch(0.0);
    let (padded, padded_height) = pitch(10.0);
    assert_close(padded - bare, 2.0 * 2.0 * 10.0, "two rows down, padded twice each");
    assert_close(padded_height - bare_height, 4.0 * 2.0 * 10.0, "four rows");
}

#[test]
fn the_header_of_a_styled_table_is_set_strong() {
    let ctx = context();
    let frame = show_in(&ctx, TABLE, &styled_table(table_style()));
    let strong = ctx.style().visuals.strong_text_color();

    assert_eq!(frame.text("Name").color, strong);
    assert_ne!(frame.text("alpha").color, strong);
    assert_ne!(
        show_in(&ctx, TABLE, &BlockStyle::default()).text("Name").color,
        strong,
        "the built-in table leaves its header as it is"
    );
}

#[test]
fn a_row_is_as_tall_as_its_strong_text_needs() {
    // Stand in a much wider face for the strong one, as an application
    // that registers a bold family does.
    let ctx = context();
    let mut fonts = egui::FontDefinitions::default();
    let monospace = fonts.families[&egui::FontFamily::Monospace].clone();
    fonts.families.insert(
        egui::FontFamily::Name(egui_commonmark_extended::STRONG_FONT_FAMILY.into()),
        monospace,
    );
    ctx.set_fonts(fonts);
    let table = "| Step | Result |\n|---|---|\n\
        | **Subsequent strong words that need the room to wrap** | **off screen above** |\n\
        | plain | plain result |\n";
    let mut cache = CommonMarkCache::default();
    let mut frame = Frame::default();
    for _ in 0..3 {
        ctx.begin_pass(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.set_width(360.0);
            CommonMarkViewer::new()
                .default_width(Some(360))
                .table_max_width(Some(360))
                .use_strong_font_family(true)
                .block_style(styled_table(table_style()))
                .show(ui, &mut cache, table);
        });
        frame = Frame::default();
        for clipped in ctx.end_pass().shapes {
            frame.collect(&clipped.shape, clipped.clip_rect);
        }
    }

    for text in &frame.texts {
        assert!(
            text.rect.bottom() <= text.clip.bottom() + 0.6,
            "{:?} is cut off at the bottom of its row: text {:?}, clip {:?}",
            text.text,
            text.rect,
            text.clip
        );
    }
}

#[test]
fn an_html_table_takes_the_table_style_too() {
    let html = "<table><tr><th>Key</th><th>Value</th></tr>\
        <tr><td>one</td><td>1</td></tr><tr><td>two</td><td>2</td></tr></table>\n";
    let frame = show(html, &styled_table(table_style()));

    assert_eq!(frame.vertical_lines(), 0);
    let rules = frame.rules();
    let strokes: Vec<Color32> = rules.iter().map(|(_, stroke)| stroke.color).collect();
    assert_eq!(strokes, [HEADER_RULE, RULE]);
    // Filled to its bound, flush with it on the left, its header strong.
    let (rule, _) = rules[0];
    assert!(rule[1].x - rule[0].x > COLUMN - 4.0, "{rule:?}");
    assert_close(frame.text("one").left, rule[0].x, "the first column is flush");
    assert_eq!(
        frame.text("Key").color,
        context().style().visuals.strong_text_color()
    );
    for text in &frame.texts {
        assert!(text.clip.contains_rect(text.rect), "{:?} is clipped", text.text);
    }
}

#[test]
fn a_frontmatter_block_loses_its_box_and_stripes_with_the_table_style() {
    let markdown = "---\ntitle: Reader view\nauthor: Someone\n---\n\nBody.\n";
    let frontmatter = |style: &BlockStyle| {
        let ctx = context();
        let mut cache = CommonMarkCache::default();
        let mut frame = Frame::default();
        for _ in 0..3 {
            ctx.begin_pass(egui::RawInput::default());
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.set_width(COLUMN);
                viewer(style)
                    .render_frontmatter(true)
                    .show(ui, &mut cache, markdown);
            });
            frame = Frame::default();
            for clipped in ctx.end_pass().shapes {
                frame.collect(&clipped.shape, clipped.clip_rect);
            }
        }
        frame
    };

    let boxed = frontmatter(&BlockStyle::default());
    let plain = frontmatter(&styled_table(table_style()));

    assert!(boxed.rects.iter().any(|rect| rect.stroke.width > 0.0));
    assert!(
        plain
            .rects
            .iter()
            .all(|rect| rect.fill == Color32::TRANSPARENT && rect.stroke.width == 0.0),
        "{:#?}",
        plain.rects
    );
    assert!(boxed.rects.iter().any(|rect| rect.fill != Color32::TRANSPARENT));
    // Still a key and a value per line.
    assert!(plain.text("author").rect.top() > plain.text("title").rect.top());
    assert!(plain.text("Someone").left > plain.text("author").rect.right());
}

// --- lists ----------------------------------------------------------------

fn styled_list() -> BlockStyle {
    BlockStyle {
        list: Some(list_style()),
        rhythm: Some(RHYTHM),
        ..Default::default()
    }
}

#[test]
fn a_task_item_shows_its_box_in_place_of_a_bullet() {
    let frame = show("- [x] done\n- [ ] open\n- plain\n", &styled_list());

    assert_eq!(frame.circles.len(), 1, "only the plain item has a bullet");
    let boxes: Vec<&Filled> = frame
        .rects
        .iter()
        .filter(|rect| rect.stroke.color == MARKER)
        .collect();
    assert_eq!(boxes.len(), 2);
    assert_eq!(frame.paths, 1, "one check mark, in the checked box");
    for (task, marker) in boxes.iter().zip(["done", "open"]) {
        let text = frame.text(marker).rect;
        assert!(
            task.rect.right() < frame.text(marker).left,
            "the box is left of its text"
        );
        assert!(task.rect.left() >= frame.extent.left());
        assert!(
            text.y_range().contains(task.rect.center().y),
            "the box sits on its item's line"
        );
        assert!(task.radius > 0.0 && task.rect.width() == task.rect.height());
    }
    // All three items start their text at the same place.
    assert_close(
        frame.text("done").left,
        frame.text("plain").left,
        "task and plain items line up",
    );
}

#[test]
fn the_built_in_list_draws_a_bullet_and_a_check_box_for_a_task_item() {
    let frame = show("- [x] done\n- [ ] open\n- plain\n", &BlockStyle::default());

    assert_eq!(frame.circles.len(), 3);
    assert!(frame.text("done").left > frame.text("plain").left + 10.0);
}

#[test]
fn a_numbered_task_item_keeps_its_number() {
    let frame = show("1. [ ] numbered task\n", &styled_list());

    assert_eq!(frame.text("1.").color, MARKER);
    assert!(frame.circles.is_empty());
}

#[test]
fn markers_hang_in_the_list_color_a_fixed_gap_left_of_the_text() {
    let frame = show("- bullet item\n\n1. first\n2. second\n", &styled_list());

    let (center, radius, fill, _) = frame.circles[0];
    let text = frame.text("bullet item").rect;
    let text_left = frame.text("bullet item").left;
    assert_eq!(fill, MARKER);
    assert_close(radius, 0.125 * EM, "bullet radius");
    assert_close(text_left - frame.extent.left(), 1.5 * EM, "the slot is the indent");
    let gap = text_left - (center.x + radius);
    assert!(gap > 0.3 * EM && gap < 0.6 * EM, "bullet to text: {gap}");
    assert!(text.y_range().contains(center.y));

    for (number, item) in [("1.", "first"), ("2.", "second")] {
        let marker = frame.text(number);
        assert_eq!(marker.color, MARKER);
        assert_close(
            frame.text(item).left - marker.rect.right(),
            gap,
            "numbers hang the same gap from their text as bullets",
        );
    }
}

#[test]
fn every_level_of_a_nested_list_steps_in_by_one_slot() {
    let frame = show(
        "- one\n  - two\n    - three\n      - four\n",
        &styled_list(),
    );
    let left = |marker: &str| frame.text(marker).left;

    for (outer, inner) in [("one", "two"), ("two", "three"), ("three", "four")] {
        assert_close(left(inner) - left(outer), 1.5 * EM, "one slot per level");
    }
    // Deeper levels are drawn hollow, in the list color.
    assert_eq!(frame.circles.len(), 4);
    assert!(frame.circles[1..]
        .iter()
        .all(|(_, _, fill, stroke)| *fill == Color32::TRANSPARENT && stroke.color == MARKER));
}

// --- inline code ----------------------------------------------------------

fn chips() -> BlockStyle {
    BlockStyle {
        inline_code_radius: Some(4.0),
        ..Default::default()
    }
}

#[test]
fn inline_code_sits_on_a_rounded_chip_and_no_line_moves() {
    let markdown = "Some text with `inline code` in the middle of a line.\n";
    let ctx = context();
    let plain = show_in(&ctx, markdown, &BlockStyle::default());
    let chipped = show_in(&ctx, markdown, &chips());
    let fill = ctx.style().visuals.code_bg_color;

    // The same labels at the same places.
    assert_eq!(plain.texts.len(), chipped.texts.len());
    for (before, after) in plain.texts.iter().zip(&chipped.texts) {
        assert_eq!((before.text.as_str(), before.rect), (after.text.as_str(), after.rect));
    }
    // The code's own square background makes way for the chip.
    assert_eq!(plain.text("inline code").background, fill);
    let code = chipped.text("inline code");
    assert_eq!(code.background, Color32::TRANSPARENT);
    let chip = chipped
        .rects
        .iter()
        .find(|rect| rect.fill == fill)
        .expect("a chip in the code background color");
    assert_close(chip.radius, 4.0, "chip radius");
    assert!(chip.rect.left() < code.left && chip.rect.right() > code.rect.right());
    assert!(chip.rect.left() > code.left - 4.0, "the chip hugs the code");
    assert!(
        chip.rect.height() < code.rect.height() && chip.rect.height() > EM,
        "shorter than the line, taller than the letters: {}",
        chip.rect.height()
    );
    assert!(code.rect.y_range().contains(chip.rect.center().y));
}

#[test]
fn code_that_wraps_gets_a_chip_on_every_line() {
    let ctx = context();
    let markdown = "Lead `fifty characters of code that will not fit a line` tail.\n";
    let mut cache = CommonMarkCache::default();
    let mut frame = Frame::default();
    for _ in 0..3 {
        ctx.begin_pass(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.set_width(260.0);
            CommonMarkViewer::new()
                .default_width(Some(260))
                .block_style(chips())
                .show(ui, &mut cache, markdown);
        });
        frame = Frame::default();
        for clipped in ctx.end_pass().shapes {
            frame.collect(&clipped.shape, clipped.clip_rect);
        }
    }
    let fill = ctx.style().visuals.code_bg_color;
    let chips: Vec<&Filled> = frame.rects.iter().filter(|rect| rect.fill == fill).collect();

    assert!(chips.len() >= 2, "one chip per line of code: {chips:#?}");
    assert!(chips
        .windows(2)
        .all(|pair| pair[0].rect.bottom() <= pair[1].rect.top() + 0.6));
    let code = frame.text("fifty characters").rect;
    assert!(chips.iter().all(|chip| code.expand(4.0).contains_rect(chip.rect)));
}

#[test]
fn code_in_a_table_cell_gets_a_chip_and_a_found_match_keeps_its_highlight() {
    let ctx = context();
    let fill = ctx.style().visuals.code_bg_color;
    let table = "| a header |\n|---|\n| `code` |\n";
    let frame = show_in(&ctx, table, &chips());
    let cell_chips: Vec<&Filled> = frame.rects.iter().filter(|rect| rect.fill == fill).collect();
    assert_eq!(cell_chips.len(), 1, "{cell_chips:#?}");

    // A search match in inline code is painted in the match color instead.
    let markdown = "Find `needle` here.\n";
    let mut cache = CommonMarkCache::default();
    cache.set_search_ranges(vec![6..12]);
    ctx.begin_pass(egui::RawInput::default());
    egui::CentralPanel::default().show(&ctx, |ui| {
        viewer(&chips()).show(ui, &mut cache, markdown);
    });
    let mut frame = Frame::default();
    for clipped in ctx.end_pass().shapes {
        frame.collect(&clipped.shape, clipped.clip_rect);
    }
    assert!(frame.rects.iter().all(|rect| rect.fill != fill));
    assert_ne!(frame.text("needle").background, Color32::TRANSPARENT);
}
