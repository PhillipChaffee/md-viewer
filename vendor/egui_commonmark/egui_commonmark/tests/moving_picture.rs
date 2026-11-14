//! A GIF of several frames plays, and costs repaints only while it is in
//! view.
//!
//! egui keeps each frame under the picture's address with the frame number
//! on the end (`picture.gif#1`). The renderer picks the frame that is due
//! and asks for the next repaint itself, so that a picture out of view, or
//! one of a single frame, leaves the window at rest.

#![cfg(feature = "animated-images")]

use std::time::Duration;

use egui::load::TexturePoll;
use egui_commonmark_extended::{CommonMarkCache, CommonMarkViewer};
use image::codecs::gif::{GifEncoder, Repeat};

const WINDOW: egui::Vec2 = egui::vec2(400.0, 300.0);
const SIZE: (u32, u32) = (6, 4);
const COLORS: [[u8; 4]; 3] = [[204, 51, 51, 255], [51, 170, 85, 255], [51, 102, 204, 255]];
/// Each frame is on for half a second.
const DELAY_MS: u32 = 500;

/// Where the window finds the picture. egui hands over bytes it was given
/// under such an address in the frame that asks for them, and the GIF is
/// decoded in that frame too. A file is read on a thread of its own, which
/// asks for a repaint whenever it gets to it: no frame count tells a test
/// that the request is behind it.
const PULSE: &str = "bytes://pulse.gif";
const PULSE_PAGE: &str = "![pulse](bytes://pulse.gif)";

/// A GIF of `frames` frames, one flat color of [`COLORS`] each.
fn gif(frames: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = GifEncoder::new(&mut bytes);
    encoder.set_repeat(Repeat::Infinite).unwrap();
    let delay = image::Delay::from_numer_denom_ms(DELAY_MS, 1);
    for color in &COLORS[..frames] {
        let pixels = image::RgbaImage::from_pixel(SIZE.0, SIZE.1, image::Rgba(*color));
        encoder
            .encode_frame(image::Frame::from_parts(pixels, 0, 0, delay))
            .unwrap();
    }
    drop(encoder);
    bytes
}

fn painted_pictures(shape: &egui::Shape, found: &mut Vec<(egui::TextureId, egui::Rect)>) {
    match shape {
        egui::Shape::Rect(rect) => {
            if let Some(brush) = &rect.brush {
                found.push((brush.fill_texture_id, rect.rect));
            }
        }
        egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
            found.push((mesh.texture_id, mesh.calc_bounds()));
        }
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .for_each(|shape| painted_pictures(shape, found)),
        _ => {}
    }
}

/// What one frame of the window showed.
struct Shown {
    /// The pictures painted inside the window.
    pictures: Vec<(egui::TextureId, egui::Rect)>,
    /// How long until the window asked to be drawn again.
    next_repaint: Duration,
}

/// A window on a page whose one picture is the GIF at [`PULSE`]. The page
/// starts `above` points down the window.
struct Window {
    ctx: egui::Context,
    cache: CommonMarkCache,
    above: f32,
    events: Vec<egui::Event>,
}

impl Window {
    /// A window whose picture has `frames` frames.
    fn new(frames: usize) -> Self {
        let ctx = egui::Context::default();
        ctx.include_bytes(PULSE, gif(frames));
        Self {
            ctx,
            cache: CommonMarkCache::default(),
            above: 0.0,
            events: Vec::new(),
        }
    }

    /// One frame at `seconds` on the app's clock.
    fn frame_at(&mut self, seconds: f64) -> Shown {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, WINDOW)),
            time: Some(seconds),
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let (cache, above) = (&mut self.cache, self.above);
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(above);
                CommonMarkViewer::new().show(ui, cache, PULSE_PAGE);
            });
        });
        assert!(
            !self.ctx.has_pending_images(),
            "the picture is being read on another thread"
        );
        let mut pictures = Vec::new();
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, WINDOW);
        for clipped in &output.shapes {
            if clipped.clip_rect.intersects(window) {
                painted_pictures(&clipped.shape, &mut pictures);
            }
        }
        pictures.retain(|(_, rect)| rect.intersects(window));
        Shown {
            pictures,
            next_repaint: output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        }
    }

    /// Click the middle of `rect` over the next two frames.
    fn click(&mut self, rect: egui::Rect, seconds: f64) {
        let pos = rect.center();
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.events = vec![egui::Event::PointerMoved(pos), button(true)];
        self.frame_at(seconds);
        self.events = vec![button(false)];
        self.frame_at(seconds);
    }
}

/// The texture egui holds for frame `index` of the picture.
fn frame_texture(ctx: &egui::Context, index: usize) -> Option<egui::load::SizedTexture> {
    let load = ctx.try_load_texture(
        &format!("{PULSE}#{index}"),
        egui::TextureOptions::default(),
        Default::default(),
    );
    match load {
        Ok(TexturePoll::Ready { texture }) => Some(texture),
        _ => None,
    }
}

#[test]
fn each_frame_of_a_gif_is_shown_in_turn_in_one_place() {
    let mut window = Window::new(3);
    window.frame_at(0.1);

    let shown: Vec<_> = [0.1, 0.6, 1.1, 1.6, 2.1]
        .into_iter()
        .map(|seconds| window.frame_at(seconds).pictures)
        .collect();

    let frames: Vec<_> = (0..3)
        .map(|index| frame_texture(&window.ctx, index).unwrap())
        .collect();
    for (at, pictures) in shown.iter().enumerate() {
        assert_eq!(pictures.len(), 1, "one picture is painted");
        assert_eq!(pictures[0].0, frames[at % 3].id, "at step {at}");
        assert_eq!(pictures[0].1, shown[0][0].1, "at step {at}");
    }
    let size = egui::vec2(6.0, 4.0);
    assert!(frames.iter().all(|frame| frame.size == size), "{frames:?}");
    assert_eq!(shown[0][0].1.size(), size);
}

#[test]
fn a_gif_in_view_asks_for_a_repaint_when_its_next_frame_is_due() {
    let mut window = Window::new(3);

    let loaded = window.frame_at(0.1);
    let later = window.frame_at(0.75);

    let step = Duration::from_millis(u64::from(DELAY_MS));
    assert!(
        loaded.next_repaint <= step,
        "the frame that reads the picture starts it moving: {:?}",
        loaded.next_repaint
    );
    assert!(later.next_repaint <= step / 2, "{:?}", later.next_repaint);
    assert!(later.next_repaint > step / 4, "{:?}", later.next_repaint);
}

#[test]
fn a_gif_out_of_view_asks_for_no_repaint() {
    let mut window = Window::new(3);
    window.frame_at(0.1);

    window.above = WINDOW.y + 50.0;
    window.frame_at(0.2);
    let away = window.frame_at(0.3);
    window.above = 0.0;
    window.frame_at(0.4);
    let back = window.frame_at(0.5);

    assert!(away.pictures.is_empty(), "{:?}", away.pictures);
    assert_eq!(away.next_repaint, Duration::MAX);
    assert_eq!(back.pictures.len(), 1);
    assert!(back.next_repaint < Duration::from_secs(1));
}

#[test]
fn a_gif_of_one_frame_asks_for_no_repaint() {
    let mut window = Window::new(1);
    window.frame_at(0.1);

    window.frame_at(0.2);
    let shown = window.frame_at(0.3);

    assert_eq!(shown.pictures.len(), 1);
    assert_eq!(shown.next_repaint, Duration::MAX);
}

#[test]
fn a_click_on_a_gif_hands_over_the_frame_that_was_clicked() {
    let mut window = Window::new(3);
    window.frame_at(0.1);
    let on_screen = window.frame_at(0.6).pictures[0];

    window.click(on_screen.1, 0.6);

    let second = frame_texture(&window.ctx, 1).unwrap();
    assert_eq!(on_screen.0, second.id);
    assert_eq!(
        window.cache.take_clicked_image(),
        Some((second.id, second.size))
    );
}
