//! A GIF of several frames plays, and costs repaints only while it is in
//! view.
//!
//! egui keeps each frame under the picture's address with the frame number
//! on the end (`picture.gif#1`). The renderer picks the frame that is due
//! and asks for the next repaint itself, so that a picture out of view, or
//! one of a single frame, leaves the window at rest.
//!
//! The frames are kept as textures, within a budget. Over it, pictures out
//! of view are let go and read again when they come back, and a picture
//! too large to play shows its first frame.

#![cfg(feature = "animated-images")]

use std::time::Duration;

use egui::load::TexturePoll;
use egui_commonmark_extended::animation::{self, FrameBudget};
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
    gif_of(SIZE, frames)
}

/// A GIF of `frames` frames of `size` pixels, one flat color of [`COLORS`]
/// each.
fn gif_of(size: (u32, u32), frames: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = GifEncoder::new(&mut bytes);
    encoder.set_repeat(Repeat::Infinite).unwrap();
    let delay = image::Delay::from_numer_denom_ms(DELAY_MS, 1);
    for color in &COLORS[..frames] {
        let pixels = image::RgbaImage::from_pixel(size.0, size.1, image::Rgba(*color));
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
    page: String,
    above: f32,
    events: Vec<egui::Event>,
}

impl Window {
    /// A window whose picture has `frames` frames.
    fn new(frames: usize) -> Self {
        let window = Self::on(PULSE_PAGE);
        window.ctx.include_bytes(PULSE, gif(frames));
        window
    }

    /// A window on `page`, whose pictures are still to be handed to it.
    fn on(page: &str) -> Self {
        Self {
            ctx: egui::Context::default(),
            cache: CommonMarkCache::default(),
            page: page.to_owned(),
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
        let (cache, above, page) = (&mut self.cache, self.above, &self.page);
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(above);
                CommonMarkViewer::new().show(ui, cache, page);
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
    frame_texture_of(ctx, PULSE, index)
}

/// The texture egui holds for frame `index` of the picture at `uri`.
fn frame_texture_of(
    ctx: &egui::Context,
    uri: &str,
    index: usize,
) -> Option<egui::load::SizedTexture> {
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

/// How many frames of the picture at `uri` egui holds a texture of.
fn textures_of(ctx: &egui::Context, uri: &str) -> usize {
    let frames = format!("{uri}#");
    let textures = ctx.tex_manager();
    let textures = textures.read();
    let held = textures.allocated();
    held.filter(|(_, texture)| texture.name.starts_with(&frames))
        .count()
}

/// How many bytes of decoded pictures egui's loaders hold.
fn decoded_bytes(ctx: &egui::Context) -> usize {
    let loaders = ctx.loaders();
    let loaders = loaders.image.lock();
    loaders.iter().map(|loader| loader.byte_size()).sum()
}

/// What the three frames of the test picture take as textures.
const PICTURE_BYTES: u64 = 3 * (SIZE.0 * SIZE.1 * 4) as u64;

#[test]
fn the_frames_are_kept_as_textures_and_not_as_decoded_pictures_too() {
    let mut window = Window::new(3);

    // The frame that reads the picture: the page may not draw it again.
    window.frame_at(0.1);

    assert_eq!(textures_of(&window.ctx, PULSE), 3);
    assert_eq!(decoded_bytes(&window.ctx), 0);
}

#[test]
fn a_gif_too_large_to_play_shows_its_first_frame_and_leaves_the_window_at_rest() {
    let mut window = Window::new(3);
    animation::set_frame_budget(
        &window.ctx,
        FrameBudget {
            one_picture: PICTURE_BYTES - 1,
            all_pictures: u64::MAX,
        },
    );
    window.frame_at(0.1);
    window.frame_at(0.2);

    let shown = [0.7, 1.2, 1.7].map(|seconds| window.frame_at(seconds));
    window.click(shown[2].pictures[0].1, 1.7);

    let first = frame_texture(&window.ctx, 0).unwrap();
    for shown in &shown {
        assert_eq!(shown.pictures, [(first.id, shown.pictures[0].1)]);
        assert_eq!(shown.next_repaint, Duration::MAX);
    }
    assert_eq!(shown[0].pictures[0].1.size(), egui::vec2(6.0, 4.0));
    assert_eq!(textures_of(&window.ctx, PULSE), 1);
    assert_eq!(decoded_bytes(&window.ctx), 0);
    assert_eq!(
        window.cache.take_clicked_image(),
        Some((first.id, first.size))
    );
}

const FIRST: &str = "bytes://first.gif";
const SECOND: &str = "bytes://second.gif";
/// Taller than a line of text, so the room such a picture takes on the page
/// is its own.
const TALL: (u32, u32) = (8, 60);
const TALL_BYTES: u64 = 3 * (TALL.0 * TALL.1 * 4) as u64;

/// A window on a page with the picture at [`FIRST`] at its top and the one
/// at [`SECOND`] more than a window further down, which may keep the frames
/// of one of them.
fn two_pictures_and_room_for_one() -> Window {
    let between = "a line between the pictures\n\n".repeat(30);
    let window = Window::on(&format!(
        "![first]({FIRST})\n\n{between}![second]({SECOND})"
    ));
    window.ctx.include_bytes(FIRST, gif_of(TALL, 3));
    window.ctx.include_bytes(SECOND, gif_of(TALL, 3));
    animation::set_frame_budget(
        &window.ctx,
        FrameBudget {
            one_picture: TALL_BYTES,
            all_pictures: TALL_BYTES,
        },
    );
    window
}

/// How far the page is moved up to bring the second picture into view.
const DOWN_THE_PAGE: f32 = -700.0;

#[test]
fn over_the_budget_a_picture_out_of_view_is_let_go_and_stays_so_until_it_is_in_view() {
    let mut window = two_pictures_and_room_for_one();
    window.frame_at(0.1);
    let read = window.frame_at(0.2);
    let both = (
        textures_of(&window.ctx, FIRST),
        textures_of(&window.ctx, SECOND),
    );

    let later = window.frame_at(3.0);
    let still_later = window.frame_at(3.1);

    assert_eq!(both, (3, 3), "pictures read together are both kept");
    assert_eq!(read.pictures.len(), 1, "only the first is in view");
    assert_eq!(textures_of(&window.ctx, FIRST), 3);
    assert_eq!(textures_of(&window.ctx, SECOND), 0);
    assert_eq!(decoded_bytes(&window.ctx), 0, "nothing was read again");
    assert_eq!(later.pictures.len(), 1);
    assert_eq!(still_later.pictures[0].1, read.pictures[0].1);
    assert!(still_later.next_repaint < Duration::from_secs(1));
}

#[test]
fn a_picture_that_was_let_go_plays_again_in_its_place_when_it_is_back_in_view() {
    let mut window = two_pictures_and_room_for_one();
    window.frame_at(0.1);
    window.frame_at(0.2);
    window.above = DOWN_THE_PAGE;
    let before = window.frame_at(0.3);
    window.above = 0.0;
    window.frame_at(3.0);
    assert_eq!(textures_of(&window.ctx, SECOND), 0, "let go");

    window.above = DOWN_THE_PAGE;
    let asked_for = window.frame_at(3.1);
    let read_again = (textures_of(&window.ctx, SECOND), decoded_bytes(&window.ctx));
    let back = [3.6, 4.1, 4.6].map(|seconds| window.frame_at(seconds));
    // The first is let go in a frame that had drawn it already: the frame
    // after that one holds its place.
    window.frame_at(6.2);
    let long_after = window.frame_at(6.3);

    assert!(asked_for.pictures.is_empty(), "its place is held empty");
    assert_eq!(asked_for.next_repaint, Duration::ZERO);
    assert_eq!(read_again, (3, 0), "read and taken in by one frame");
    let frames: Vec<_> = (0..3)
        .map(|index| frame_texture_of(&window.ctx, SECOND, index).unwrap().id)
        .collect();
    // At 3.6 seconds the second frame is due.
    let due = [frames[1], frames[2], frames[0]];
    for (shown, due) in back.iter().zip(due) {
        assert_eq!(shown.pictures, [(due, before.pictures[0].1)]);
        assert!(shown.next_repaint < Duration::from_secs(1));
    }
    assert_eq!(
        textures_of(&window.ctx, FIRST),
        0,
        "the first is let go in its turn"
    );
    assert_eq!(before.pictures[0].1.size(), egui::vec2(8.0, 60.0));
    assert_eq!(
        long_after.pictures[0].1, before.pictures[0].1,
        "the place of the first is held at its size"
    );
}

#[test]
fn room_is_made_for_a_picture_in_the_frame_before_it_is_read_again() {
    let mut window = two_pictures_and_room_for_one();
    window.frame_at(0.1);
    window.frame_at(3.0);
    assert_eq!(textures_of(&window.ctx, SECOND), 0, "let go");
    let kept = |window: &Window| {
        (
            textures_of(&window.ctx, FIRST),
            textures_of(&window.ctx, SECOND),
            decoded_bytes(&window.ctx),
        )
    };

    window.above = DOWN_THE_PAGE;
    let makes_room = window.frame_at(6.0);
    let room_made = kept(&window);
    let reads = window.frame_at(6.1);
    let read = kept(&window);
    let shows = window.frame_at(6.2);

    assert_eq!(room_made, (0, 0, 0), "neither picture is kept");
    assert_eq!(makes_room.next_repaint, Duration::ZERO);
    assert_eq!(read, (0, 3, 0));
    assert!(reads.pictures.is_empty());
    assert_eq!(shows.pictures.len(), 1);
}
