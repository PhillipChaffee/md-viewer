//! A GIF of several frames plays, and costs repaints only while it is in
//! view.
//!
//! egui keeps each frame under the picture's address with the frame number
//! on the end (`picture.gif#1`). The renderer picks the frame that is due
//! and asks for the next repaint itself, so that a picture out of view, or
//! one of a single frame, leaves the window at rest.

#![cfg(feature = "animated-images")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::load::TexturePoll;
use egui_commonmark_extended::{CommonMarkCache, CommonMarkViewer};
use image::codecs::gif::{GifEncoder, Repeat};

const WINDOW: egui::Vec2 = egui::vec2(400.0, 300.0);
const SIZE: (u32, u32) = (6, 4);
const COLORS: [[u8; 4]; 3] = [[204, 51, 51, 255], [51, 170, 85, 255], [51, 102, 204, 255]];
/// Each frame is on for half a second.
const DELAY_MS: u32 = 500;

/// A folder of this test's own, removed when the test is done.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("moving-picture-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// Write a GIF of `frames` frames, one flat color of [`COLORS`] each,
    /// and give its address.
    fn gif(&self, name: &str, frames: usize) -> String {
        let path = self.0.join(name);
        write_gif(&path, frames);
        format!("file://{}", path.display())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_gif(path: &Path, frames: usize) {
    let mut encoder = GifEncoder::new(std::fs::File::create(path).unwrap());
    encoder.set_repeat(Repeat::Infinite).unwrap();
    let delay = image::Delay::from_numer_denom_ms(DELAY_MS, 1);
    for color in &COLORS[..frames] {
        let pixels = image::RgbaImage::from_pixel(SIZE.0, SIZE.1, image::Rgba(*color));
        encoder
            .encode_frame(image::Frame::from_parts(pixels, 0, 0, delay))
            .unwrap();
    }
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

/// A window on `markdown`, which starts `above` points down the page.
struct Window<'a> {
    ctx: egui::Context,
    cache: CommonMarkCache,
    markdown: &'a str,
    above: f32,
    events: Vec<egui::Event>,
}

impl<'a> Window<'a> {
    fn new(markdown: &'a str) -> Self {
        Self {
            ctx: egui::Context::default(),
            cache: CommonMarkCache::default(),
            markdown,
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
        let (cache, markdown, above) = (&mut self.cache, self.markdown, self.above);
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(above);
                CommonMarkViewer::new().show(ui, cache, markdown);
            });
        });
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

    /// Frames at `seconds` until the picture at `uri` has been read, which
    /// happens on another thread. Gives the first frame that shows it.
    #[track_caller]
    fn frame_once_loaded(&mut self, uri: &str, seconds: f64) -> Shown {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let shown = self.frame_at(seconds);
            if frame_texture(&self.ctx, uri, 0).is_some() {
                return shown;
            }
            assert!(Instant::now() < deadline, "waited too long for {uri}");
            std::thread::sleep(Duration::from_millis(5));
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

/// The texture egui holds for frame `index` of the picture at `uri`.
fn frame_texture(ctx: &egui::Context, uri: &str, index: usize) -> Option<egui::load::SizedTexture> {
    let load = ctx.try_load_texture(
        &format!("{uri}#{index}"),
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
    let scratch = Scratch::new("turn");
    let uri = scratch.gif("pulse.gif", 3);
    let markdown = format!("![pulse]({uri})");
    let mut window = Window::new(&markdown);
    window.frame_once_loaded(&uri, 0.1);

    let shown: Vec<_> = [0.1, 0.6, 1.1, 1.6, 2.1]
        .into_iter()
        .map(|seconds| window.frame_at(seconds).pictures)
        .collect();

    let frames: Vec<_> = (0..3)
        .map(|index| frame_texture(&window.ctx, &uri, index).unwrap())
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
    let scratch = Scratch::new("due");
    let uri = scratch.gif("pulse.gif", 3);
    let markdown = format!("![pulse]({uri})");
    let mut window = Window::new(&markdown);

    let loaded = window.frame_once_loaded(&uri, 0.1);
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
    let scratch = Scratch::new("away");
    let uri = scratch.gif("pulse.gif", 3);
    let markdown = format!("![pulse]({uri})");
    let mut window = Window::new(&markdown);
    window.frame_once_loaded(&uri, 0.1);

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
    let scratch = Scratch::new("still");
    let uri = scratch.gif("still.gif", 1);
    let markdown = format!("![still]({uri})");
    let mut window = Window::new(&markdown);
    window.frame_once_loaded(&uri, 0.1);

    window.frame_at(0.2);
    let shown = window.frame_at(0.3);

    assert_eq!(shown.pictures.len(), 1);
    assert_eq!(shown.next_repaint, Duration::MAX);
}

#[test]
fn a_click_on_a_gif_hands_over_the_frame_that_was_clicked() {
    let scratch = Scratch::new("click");
    let uri = scratch.gif("pulse.gif", 3);
    let markdown = format!("![pulse]({uri})");
    let mut window = Window::new(&markdown);
    window.frame_once_loaded(&uri, 0.1);
    let on_screen = window.frame_at(0.6).pictures[0];

    window.click(on_screen.1, 0.6);

    let second = frame_texture(&window.ctx, &uri, 1).unwrap();
    assert_eq!(on_screen.0, second.id);
    assert_eq!(
        window.cache.take_clicked_image(),
        Some((second.id, second.size))
    );
}
