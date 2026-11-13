//! A diagram may leave the document column: it is shown at its natural
//! size, centered on the column, in the room the caller gives it, and
//! scaled down only when even that is too narrow.
//!
//! Diagrams render on a background thread, so each test drives frames until
//! the diagram is on screen.

#![cfg(feature = "mermaid")]

use egui::{Context, Rect, Shape, TextStyle, TextureId};
use egui_commonmark_extended::{BlockRhythm, BlockStyle, CommonMarkCache, CommonMarkViewer};
use std::time::{Duration, Instant};

const EM: f32 = 18.0;
const COLUMN: f32 = 300.0;
const PANE: egui::Vec2 = egui::vec2(1000.0, 600.0);

/// Seven boxes in a row: far wider than [`COLUMN`], and wider than [`PANE`].
const WIDE: &str = "Lead paragraph.\n\n```mermaid\nflowchart LR\n    A[Prompt text] --> B[Clean pass]\n    \
    B --> C[Perturbed pass]\n    C --> D[Compare logits]\n    D --> E[Candidate neurons]\n    \
    E --> F[Intervention sweep]\n    F --> G[Final report]\n```\n\nAfter paragraph.\n";

/// Two boxes: narrower than [`COLUMN`].
const SMALL: &str = "Lead paragraph.\n\n```mermaid\nflowchart LR\n    A[A] --> B[B]\n```\n\nAfter paragraph.\n";

/// The wide diagram as the content of a list item.
const IN_A_LIST: &str = "Lead paragraph.\n\n- An item.\n\n  ```mermaid\n  flowchart LR\n      A[Prompt text] --> B[Clean pass]\n      \
    B --> C[Perturbed pass]\n      C --> D[Compare logits]\n      D --> E[Candidate neurons]\n  ```\n\nAfter paragraph.\n";

const RHYTHM: BlockRhythm = BlockRhythm {
    paragraph: 1.0,
    block: 1.5,
    heading_above: 2.0,
    heading_below: 0.5,
    tight_item: 0.25,
    display_math: 0.5,
};

fn style(room: Option<f32>) -> BlockStyle {
    BlockStyle {
        rhythm: Some(RHYTHM),
        center_content: true,
        figure_max_width: room,
        ..Default::default()
    }
}

/// Where one frame painted the diagram and the paragraphs around it.
#[derive(Debug, Clone, Copy)]
struct Seen {
    diagram: Rect,
    lead: Rect,
    after: Rect,
    pane: Rect,
}

fn find(shape: &Shape, diagram: &mut Option<Rect>, texts: &mut Vec<(String, Rect)>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| find(shape, diagram, texts)),
        Shape::Rect(rect) if rect.fill_texture_id() != TextureId::default() => {
            *diagram = Some(rect.rect);
        }
        Shape::Text(text) => texts.push((
            text.galley.job.text.clone(),
            text.galley.rect.translate(text.pos.to_vec2()),
        )),
        _ => {}
    }
}

/// A viewer of one document in a pane of [`PANE`], its column centered.
struct Viewer {
    ctx: Context,
    cache: CommonMarkCache,
    markdown: &'static str,
    style: BlockStyle,
    scrollable: bool,
}

impl Viewer {
    fn new(markdown: &'static str, room: Option<f32>) -> Self {
        let ctx = Context::default();
        ctx.style_mut(|style| {
            style
                .text_styles
                .insert(TextStyle::Body, egui::FontId::proportional(EM));
        });
        Self {
            ctx,
            cache: CommonMarkCache::default(),
            markdown,
            style: style(room),
            scrollable: false,
        }
    }

    /// One frame; `None` while the diagram is still being rendered.
    /// `full` asks a scrolling viewer to paint the whole document.
    fn frame(&mut self, full: bool) -> Option<Seen> {
        let mut pane = Rect::NOTHING;
        // The wide diagram's texture is larger than the 2048 px a context
        // without a graphics backend assumes.
        self.ctx.begin_pass(egui::RawInput {
            max_texture_side: Some(8192),
            ..Default::default()
        });
        egui::CentralPanel::default().show(&self.ctx, |ui| {
            pane = Rect::from_min_size(ui.max_rect().min, PANE);
            ui.scope_builder(egui::UiBuilder::new().max_rect(pane), |ui| {
                let viewer = CommonMarkViewer::new()
                    .default_width(Some(COLUMN as usize))
                    .line_height(1.6)
                    .block_style(self.style.clone())
                    .force_full_render(full);
                if self.scrollable {
                    viewer.show_scrollable("figure", ui, &mut self.cache, self.markdown);
                } else {
                    viewer.show(ui, &mut self.cache, self.markdown);
                }
            });
        });
        let (mut diagram, mut texts) = (None, Vec::new());
        for clipped in self.ctx.end_pass().shapes {
            find(&clipped.shape, &mut diagram, &mut texts);
        }
        let text = |marker: &str| {
            texts
                .iter()
                .find(|(text, _)| text.contains(marker))
                .map(|(_, rect)| *rect)
        };
        Some(Seen {
            diagram: diagram?,
            lead: text("Lead paragraph")?,
            after: text("After paragraph")?,
            pane,
        })
    }

    /// Drive frames until the diagram is on screen and the layout settled.
    fn settled(&mut self) -> Seen {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if self.frame(false).is_some() {
                // The frame the diagram arrived in marks the layout changed;
                // the next ones lay everything out around it.
                let _ = self.frame(false);
                return self.frame(false).expect("the diagram stays on screen");
            }
            assert!(Instant::now() < deadline, "no diagram within two minutes");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[track_caller]
fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "{what}: {actual} is not {expected}"
    );
}

fn column_axis(seen: &Seen) -> f32 {
    seen.lead.left() + COLUMN / 2.0
}

#[test]
fn a_wide_diagram_is_shown_as_wide_as_its_room_centered_on_the_column() {
    let inline = Viewer::new(WIDE, None).settled();
    let roomy = Viewer::new(WIDE, Some(800.0)).settled();
    let natural = Viewer::new(WIDE, Some(5000.0)).settled();

    // Without room it is scaled into the column, at the column's left edge.
    assert_close(inline.diagram.width(), COLUMN, "scaled into the column");
    assert_close(inline.diagram.left(), inline.lead.left(), "inline in the column");

    // With room it takes the room, centered on the column's axis.
    assert_close(roomy.diagram.width(), 800.0, "as wide as its room");
    assert_close(roomy.diagram.center().x, column_axis(&roomy), "on the column axis");
    let shape = |seen: &Seen| seen.diagram.width() / seen.diagram.height();
    assert!((shape(&roomy) - shape(&inline)).abs() < 0.02 * shape(&inline));

    // Never above its natural size, however much room there is.
    assert!(natural.diagram.width() > 1000.0 && natural.diagram.width() < 5000.0);
    // Wider than the pane, it starts at the pane's left edge instead of
    // being cut off on the left.
    assert_close(natural.diagram.left(), natural.pane.left(), "kept in the pane");

    // The text below starts one block gap under the diagram as shown.
    for seen in [inline, roomy, natural] {
        assert_close(
            seen.after.top() - seen.diagram.bottom(),
            1.5 * EM,
            "the row is as tall as the diagram",
        );
        assert_close(seen.after.left(), seen.lead.left(), "text stays in the column");
    }
}

#[test]
fn a_small_diagram_keeps_its_size_and_is_centered_on_the_column() {
    let inline = Viewer::new(SMALL, None).settled();
    let roomy = Viewer::new(SMALL, Some(800.0)).settled();

    assert!(inline.diagram.width() < COLUMN);
    assert_close(roomy.diagram.width(), inline.diagram.width(), "natural size");
    assert_close(roomy.diagram.height(), inline.diagram.height(), "natural size");
    assert_close(roomy.diagram.center().x, column_axis(&roomy), "on the column axis");
    assert!(roomy.diagram.left() > roomy.lead.left() + 20.0, "not at the left edge");
}

#[test]
fn a_viewport_slice_shows_a_wide_diagram_where_the_full_pass_measured_it() {
    let mut viewer = Viewer::new(WIDE, Some(800.0));
    viewer.scrollable = true;
    let _ = viewer.settled();

    let full = viewer.frame(true).expect("a full paint shows the diagram");
    let slice = viewer.frame(false).expect("the slice shows the diagram");

    assert_eq!(slice.diagram, full.diagram);
    assert_eq!(slice.after, full.after);
    assert_close(slice.diagram.width(), 800.0, "as wide as its room");
    assert_close(slice.diagram.center().x, column_axis(&slice), "on the column axis");
}

#[test]
fn a_diagram_inside_a_list_item_stays_in_the_column() {
    let seen = Viewer::new(IN_A_LIST, Some(800.0)).settled();

    assert!(seen.diagram.left() > seen.lead.left(), "indented with its item");
    assert!(
        seen.diagram.right() <= seen.lead.left() + COLUMN + 1.0,
        "scaled into the column: {:?}",
        seen.diagram
    );
}
