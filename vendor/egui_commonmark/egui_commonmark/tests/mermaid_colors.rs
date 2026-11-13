//! Mermaid diagrams in the application's colors, seen the way the GPU sees
//! them: which textures a frame uploads, frees and paints.
//!
//! Diagrams render on a background thread, so each test drives frames until
//! the texture it waits for is on screen.

#![cfg(feature = "mermaid")]

use egui::{Color32, ColorImage, Context, Shape, TextureId};
use egui_commonmark_extended::{CommonMarkCache, CommonMarkViewer, MermaidColors};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const DOCUMENT: &str = "```mermaid\nflowchart LR\n    A[Prompt] --> B[Clean pass]\n```\n";

const PLACEHOLDER: &str = "Rendering diagram\u{2026}";

fn sepia() -> MermaidColors {
    MermaidColors {
        page: Color32::from_rgb(0xf8, 0xf1, 0xe3),
        text: Color32::from_rgb(0x2b, 0x26, 0x22),
        line: Color32::from_rgb(0x5e, 0x57, 0x50),
        node_fill: Color32::from_rgb(0xf0, 0xe7, 0xd3),
        node_stroke: Color32::from_rgb(0xb5, 0xa9, 0x94),
        secondary_fill: Color32::from_rgb(0xf4, 0xec, 0xdb),
        note_fill: Color32::from_rgb(0xf3, 0xe3, 0xb3),
        accent: Color32::from_rgb(0x9a, 0x5b, 0x2e),
    }
}

fn night() -> MermaidColors {
    MermaidColors {
        page: Color32::BLACK,
        text: Color32::from_rgb(0xc8, 0xc6, 0xc1),
        line: Color32::from_rgb(0xa0, 0x9e, 0x9a),
        node_fill: Color32::from_rgb(0x1c, 0x1c, 0x1e),
        node_stroke: Color32::from_rgb(0x5a, 0x58, 0x55),
        secondary_fill: Color32::from_rgb(0x12, 0x12, 0x13),
        note_fill: Color32::from_rgb(0x2e, 0x29, 0x1a),
        accent: Color32::from_rgb(0x7a, 0xa2, 0xd6),
    }
}

/// What one frame painted.
struct Frame {
    /// Textures of the images painted, in paint order.
    images: Vec<TextureId>,
    texts: Vec<String>,
}

/// A viewer showing [`DOCUMENT`], with the textures it currently holds.
struct Viewer {
    ctx: Context,
    cache: CommonMarkCache,
    /// Every texture uploaded and not yet freed, except egui's font atlas.
    live: BTreeMap<TextureId, ColorImage>,
}

impl Viewer {
    fn new() -> Self {
        Self {
            ctx: Context::default(),
            cache: CommonMarkCache::default(),
            live: BTreeMap::new(),
        }
    }

    fn frame(&mut self, colors: Option<MermaidColors>) -> Frame {
        let cache = &mut self.cache;
        let output = self.ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                CommonMarkViewer::new()
                    .mermaid_colors(colors)
                    .show(ui, cache, DOCUMENT);
            });
        });
        for (id, delta) in output.textures_delta.set {
            let egui::ImageData::Color(image) = delta.image;
            if id != TextureId::default() && delta.pos.is_none() {
                self.live.insert(id, ColorImage::clone(&image));
            }
        }
        for id in output.textures_delta.free {
            self.live.remove(&id);
        }
        let mut frame = Frame {
            images: Vec::new(),
            texts: Vec::new(),
        };
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut frame);
        }
        frame
    }

    /// Drive frames until the diagram on screen is a texture other than
    /// `previous`, and return it.
    fn diagram(&mut self, colors: Option<MermaidColors>, previous: Option<TextureId>) -> TextureId {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let frame = self.frame(colors);
            match frame.images.first() {
                Some(id) if Some(*id) != previous => return *id,
                _ => {}
            }
            assert!(Instant::now() < deadline, "no new diagram within a minute");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn collect(shape: &Shape, frame: &mut Frame) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, frame)),
        Shape::Rect(rect) if rect.fill_texture_id() != TextureId::default() => {
            frame.images.push(rect.fill_texture_id());
        }
        Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => {
            frame.images.push(mesh.texture_id);
        }
        Shape::Text(text) => frame.texts.push(text.galley.job.text.clone()),
        _ => {}
    }
}

/// The color most of an image's fully opaque pixels have.
fn dominant_opaque_color(image: &ColorImage) -> Color32 {
    let mut counts = BTreeMap::<[u8; 4], usize>::new();
    for pixel in image.pixels.iter().filter(|pixel| pixel.a() == 255) {
        *counts.entry(pixel.to_array()).or_default() += 1;
    }
    let (color, _) = counts.into_iter().max_by_key(|(_, count)| *count).unwrap();
    Color32::from_rgb(color[0], color[1], color[2])
}

#[test]
fn a_diagram_is_drawn_in_the_colors_given() {
    let mut viewer = Viewer::new();

    let id = viewer.diagram(Some(sepia()), None);

    let image = &viewer.live[&id];
    assert_eq!(image.pixels[0], Color32::TRANSPARENT);
    assert_eq!(dominant_opaque_color(image), sepia().node_fill);
}

#[test]
fn without_colors_a_diagram_keeps_the_default_theme() {
    let mut viewer = Viewer::new();

    let id = viewer.diagram(None, None);

    let image = &viewer.live[&id];
    assert_eq!(image.pixels[0], Color32::WHITE);
    assert!(image.pixels.contains(&Color32::from_rgb(0xec, 0xec, 0xff)));
}

#[test]
fn new_colors_redraw_the_diagram_and_free_the_old_texture() {
    let mut viewer = Viewer::new();
    let in_sepia = viewer.diagram(Some(sepia()), None);

    let in_night = viewer.diagram(Some(night()), Some(in_sepia));

    assert_eq!(dominant_opaque_color(&viewer.live[&in_night]), night().node_fill);
    // The frame after the swap, egui has released the texture nothing holds.
    viewer.frame(Some(night()));
    assert_eq!(viewer.live.keys().collect::<Vec<_>>(), [&in_night]);
}

#[test]
fn the_previous_diagram_stays_on_screen_until_the_new_one_is_ready() {
    let mut viewer = Viewer::new();
    // The very first render has nothing to show yet.
    let first = viewer.frame(Some(sepia()));
    assert!(first.images.is_empty());
    assert_eq!(first.texts, [PLACEHOLDER]);
    let in_sepia = viewer.diagram(Some(sepia()), None);

    // The first frame in new colors cannot have them rendered yet.
    let switched = viewer.frame(Some(night()));

    assert_eq!(switched.images, [in_sepia]);
    assert!(switched.texts.is_empty(), "{:?}", switched.texts);
}

#[test]
fn switching_back_and_forth_never_holds_more_than_one_texture() {
    let mut viewer = Viewer::new();
    let mut current = viewer.diagram(Some(sepia()), None);

    for colors in [night(), sepia(), night(), sepia(), night()] {
        current = viewer.diagram(Some(colors), Some(current));
        viewer.frame(Some(colors));
        assert_eq!(viewer.live.keys().collect::<Vec<_>>(), [&current]);
    }
}

#[test]
fn colors_changed_again_before_a_render_finishes_end_on_the_last_ones() {
    let mut viewer = Viewer::new();
    let in_sepia = viewer.diagram(Some(sepia()), None);
    // Ask for night, then go back to a third palette before it arrives.
    viewer.frame(Some(night()));
    let mut dusk = night();
    dusk.node_fill = Color32::from_rgb(0x3a, 0x3a, 0x3c);

    let last = viewer.diagram(Some(dusk), Some(in_sepia));

    assert_eq!(dominant_opaque_color(&viewer.live[&last]), dusk.node_fill);
    viewer.frame(Some(dusk));
    assert_eq!(viewer.live.keys().collect::<Vec<_>>(), [&last]);
}
