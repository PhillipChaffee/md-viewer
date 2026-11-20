//! A GIF of several frames plays, and costs repaints only while it is in
//! view.
//!
//! egui keeps each frame under the picture's address with the frame number
//! on the end (`picture.gif#1`). The renderer picks the frame that is due
//! and asks for the next repaint itself, so that a picture out of view, or
//! one of a single frame, leaves the window at rest.
//!
//! The frames are kept as textures, within a budget. Over it, pictures
//! away from the view are let go and read again when they come back, and a
//! picture too large to play shows its first frame.
//!
//! A picture is read only once its place is near the view, and it is
//! decoded on a thread of its own. Here the window decodes by hand
//! instead (`Window::decode`), at the point where a test has the thread be
//! done: a thread is done whenever it is done.

#![cfg(feature = "animated-images")]

use std::time::Duration;

use egui::load::TexturePoll;
use egui_commonmark_extended::animation::{self, FrameBudget};
use egui_commonmark_extended::{ClickedImage, CommonMarkCache, CommonMarkViewer};
use image::codecs::gif::{GifEncoder, Repeat};

const WINDOW: egui::Vec2 = egui::vec2(400.0, 300.0);
const SIZE: (u32, u32) = (6, 4);
const COLORS: [[u8; 4]; 3] = [[204, 51, 51, 255], [51, 170, 85, 255], [51, 102, 204, 255]];
/// Each frame is on for half a second.
const DELAY_MS: u32 = 500;

/// Where the window finds the picture. egui hands over bytes it was given
/// under such an address in the frame that asks for them. A file is read
/// on a thread of its own, which asks for a repaint whenever it gets to
/// it: no frame count tells a test that the request is behind it.
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

fn painted_text(shape: &egui::Shape, found: &mut Vec<(String, f32)>) {
    match shape {
        egui::Shape::Text(text) => found.push((text.galley.text().to_owned(), text.pos.y)),
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| painted_text(shape, found)),
        _ => {}
    }
}

/// What one frame of the window showed.
struct Shown {
    /// The pictures painted inside the window.
    pictures: Vec<(egui::TextureId, egui::Rect)>,
    /// The lines of text painted, each with where its top is.
    text: Vec<(String, f32)>,
    /// How far down a page that scrolls the view is, and where the top of
    /// the view is in the window.
    scrolled: f32,
    pane_top: f32,
    /// How long until the window asked to be drawn again.
    next_repaint: Duration,
    /// The frame an enlarged view of the picture would show, if the window
    /// has one.
    enlarged: Option<egui::TextureId>,
}

/// A window on a page whose one picture is the GIF at [`PULSE`]. The page
/// starts `above` points down the window.
struct Window {
    ctx: egui::Context,
    cache: CommonMarkCache,
    page: String,
    above: f32,
    events: Vec<egui::Event>,
    /// Whether an enlarged view of the picture at [`PULSE`] is open.
    enlarged: bool,
    /// Whether the page scrolls inside the window, as a document does in
    /// an app, instead of starting `above` points down it.
    page_scrolls: bool,
    /// How far down such a page the next frame scrolls.
    scroll_to: Option<f32>,
    /// How far down such a page the last frame was.
    scrolled: f32,
}

impl Shown {
    /// Where the top of the line that reads `line` is, if it was painted.
    fn find(&self, line: &str) -> Option<f32> {
        let found = self.text.iter().find(|(text, _)| text.trim() == line);
        found.map(|(_, top)| *top)
    }

    #[track_caller]
    fn top_of(&self, line: &str) -> f32 {
        let top = self.find(line);
        top.unwrap_or_else(|| panic!("{line:?} is not among {:?}", self.text))
    }
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
        let ctx = egui::Context::default();
        animation::decode_by_hand(&ctx);
        Self {
            ctx,
            cache: CommonMarkCache::default(),
            page: page.to_owned(),
            above: 0.0,
            events: Vec::new(),
            enlarged: false,
            page_scrolls: false,
            scroll_to: None,
            scrolled: 0.0,
        }
    }

    /// Decode the pictures the window has asked for, as the thread that
    /// decodes would by now, and return how many were kept. The next frame
    /// finds them decoded.
    fn decode(&self) -> usize {
        animation::decode_held(&self.ctx)
    }

    /// The frames in which a picture near the view is read: one asks for
    /// it, the thread decodes it, and the next takes it in and draws it.
    fn read_at(&mut self, seconds: f64) -> Shown {
        self.frame_at(seconds);
        self.decode();
        self.frame_at(seconds)
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
        // As an app does, the window scrolls on by as much as the page is
        // about to grow above the view.
        let grown = cache.take_growth_above_the_view();
        let keep_in_view = grown.map(|grown| self.scrolled + grown);
        let scroll_to = self.scroll_to.take().or(keep_in_view);
        let page_scrolls = self.page_scrolls;
        let (mut scrolled, mut pane_top) = (0.0, 0.0);
        let mut enlarged = None;
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if page_scrolls {
                    let pane = CommonMarkViewer::new()
                        .pending_scroll_offset(scroll_to)
                        .show_scrollable("page", ui, cache, page);
                    scrolled = pane.state.offset.y;
                    pane_top = pane.inner_rect.top();
                } else {
                    ui.add_space(above);
                    CommonMarkViewer::new().show(ui, cache, page);
                }
            });
            if self.enlarged {
                enlarged = animation::enlarged_frame(ctx, PULSE).map(|frame| frame.id);
            }
        });
        let (mut pictures, mut text) = (Vec::new(), Vec::new());
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, WINDOW);
        for clipped in &output.shapes {
            if clipped.clip_rect.intersects(window) {
                painted_pictures(&clipped.shape, &mut pictures);
                painted_text(&clipped.shape, &mut text);
            }
        }
        pictures.retain(|(_, rect)| rect.intersects(window));
        self.scrolled = scrolled;
        Shown {
            pictures,
            text,
            scrolled,
            pane_top,
            next_repaint: output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
            enlarged,
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
    window.read_at(0.1);

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

    let arrived = window.read_at(0.1);
    let later = window.frame_at(0.75);

    let step = Duration::from_millis(u64::from(DELAY_MS));
    assert!(
        arrived.next_repaint <= step,
        "the frame that takes the picture in starts it moving: {:?}",
        arrived.next_repaint
    );
    assert!(later.next_repaint <= step / 2, "{:?}", later.next_repaint);
    assert!(later.next_repaint > step / 4, "{:?}", later.next_repaint);
}

#[test]
fn a_gif_out_of_view_asks_for_no_repaint() {
    let mut window = Window::new(3);
    window.read_at(0.1);

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
    window.read_at(0.1);

    window.frame_at(0.2);
    let shown = window.frame_at(0.3);

    assert_eq!(shown.pictures.len(), 1);
    assert_eq!(shown.next_repaint, Duration::MAX);
}

#[test]
fn a_click_on_a_gif_hands_over_the_frame_that_was_clicked() {
    let mut window = Window::new(3);
    window.read_at(0.1);
    let on_screen = window.frame_at(0.6).pictures[0];

    window.click(on_screen.1, 0.6);

    let second = frame_texture(&window.ctx, 1).unwrap();
    assert_eq!(on_screen.0, second.id);
    assert_eq!(
        window.cache.take_clicked_image(),
        Some(ClickedImage {
            texture: second.id,
            size: second.size,
            moving: Some(PULSE.to_owned()),
        })
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
    window.frame_at(0.1);
    window.decode();
    let decoded = decoded_bytes(&window.ctx);

    // The frame that finds the picture decoded: the page may not draw it
    // again.
    window.frame_at(0.2);

    assert_eq!(decoded as u64, PICTURE_BYTES, "the thread left its frames");
    assert_eq!(textures_of(&window.ctx, PULSE), 3);
    assert_eq!(decoded_bytes(&window.ctx), 0);
}

#[test]
fn a_picture_that_is_decoded_in_view_has_a_spinner_in_a_place_of_its_size() {
    let mut window = Window::on(&format!("before\n\n![tall]({FIRST})\n\nafter"));
    window.ctx.include_bytes(FIRST, gif_of(TALL, 3));

    let asked = window.frame_at(0.1);
    let waiting = window.frame_at(0.2);
    let kept = window.decode();
    let arrived = window.frame_at(0.3);
    let playing = window.frame_at(0.4);

    assert!(waiting.pictures.is_empty(), "nothing is drawn of it yet");
    assert_eq!(waiting.next_repaint, Duration::ZERO, "the spinner turns");
    assert_eq!(kept, 1);
    assert_eq!(arrived.pictures.len(), 1);
    assert_eq!(arrived.pictures[0].1.size(), egui::vec2(8.0, 60.0));
    let gap = waiting.top_of("after") - waiting.top_of("before");
    assert!(
        gap > 60.0,
        "the place was held at the picture's height: {gap}"
    );
    for shown in [&asked, &arrived, &playing] {
        assert_eq!(shown.top_of("after"), waiting.top_of("after"));
    }
    assert!(playing.next_repaint < Duration::from_secs(1));
}

#[test]
fn a_decoded_picture_becomes_textures_over_several_frames_of_the_window_and_then_plays() {
    let mut window = Window::on(&format!("before\n\n![tall]({FIRST})\n\nafter"));
    window.ctx.include_bytes(FIRST, gif_of(TALL, 3));
    animation::set_frame_budget(
        &window.ctx,
        FrameBudget {
            each_frame: TALL_BYTES / 3,
            ..FrameBudget::default()
        },
    );
    let waiting = window.frame_at(0.1);
    window.decode();

    let arriving = window.frame_at(0.2);
    let some = (textures_of(&window.ctx, FIRST), decoded_bytes(&window.ctx));
    let arrived = window.frame_at(0.3);
    let all = (textures_of(&window.ctx, FIRST), decoded_bytes(&window.ctx));
    let playing = window.frame_at(0.4);

    assert_eq!(some, (2, TALL_BYTES as usize), "one more frame of it");
    assert!(arriving.pictures.is_empty(), "its place is still held");
    assert_eq!(arriving.next_repaint, Duration::ZERO, "the rest follows");
    assert_eq!(all, (3, 0));
    assert_eq!(arrived.pictures.len(), 1);
    assert!(playing.next_repaint < Duration::from_secs(1));
    for shown in [&arriving, &arrived, &playing] {
        assert_eq!(shown.top_of("after"), waiting.top_of("after"));
    }
}

#[test]
fn an_enlarged_view_is_shown_the_frame_that_is_due_and_wakes_the_window_for_the_next() {
    let mut window = Window::new(3);
    window.read_at(0.1);
    window.above = WINDOW.y + 50.0;
    window.frame_at(0.2);

    window.enlarged = true;
    let enlarged: Vec<_> = [0.3, 0.6, 1.1, 1.6]
        .into_iter()
        .map(|seconds| window.frame_at(seconds))
        .collect();
    window.enlarged = false;
    window.frame_at(1.7);
    let closed = window.frame_at(1.8);

    let frames: Vec<_> = (0..3)
        .map(|index| frame_texture(&window.ctx, index).unwrap().id)
        .collect();
    let shown: Vec<_> = enlarged.iter().map(|shown| shown.enlarged).collect();
    let due = [frames[0], frames[1], frames[2], frames[0]].map(Some);
    assert_eq!(shown, due);
    let step = Duration::from_millis(u64::from(DELAY_MS));
    for shown in &enlarged {
        assert!(shown.next_repaint <= step, "{:?}", shown.next_repaint);
        assert!(
            shown.pictures.is_empty(),
            "the page's picture is out of view"
        );
    }
    assert_eq!(closed.next_repaint, Duration::MAX);
}

#[test]
fn a_gif_too_large_to_play_shows_its_first_frame_and_leaves_the_window_at_rest() {
    let mut window = Window::new(3);
    animation::set_frame_budget(
        &window.ctx,
        FrameBudget {
            one_picture: PICTURE_BYTES - 1,
            all_pictures: u64::MAX,
            ..FrameBudget::default()
        },
    );
    window.frame_at(0.1);
    window.decode();
    let decoded = decoded_bytes(&window.ctx);
    window.frame_at(0.2);

    let shown = [0.7, 1.2, 1.7].map(|seconds| window.frame_at(seconds));
    window.click(shown[2].pictures[0].1, 1.7);

    assert_eq!(
        decoded as u64,
        PICTURE_BYTES / 3,
        "the thread kept the one frame that is shown"
    );
    let first = frame_texture(&window.ctx, 0).unwrap();
    for shown in &shown {
        assert_eq!(shown.pictures, [(first.id, shown.pictures[0].1)]);
        assert_eq!(shown.next_repaint, Duration::MAX);
    }
    assert_eq!(shown[0].pictures[0].1.size(), egui::vec2(6.0, 4.0));
    assert_eq!(textures_of(&window.ctx, PULSE), 1);
    assert_eq!(decoded_bytes(&window.ctx), 0);
    let clicked = window.cache.take_clicked_image().unwrap();
    assert_eq!((clicked.texture, clicked.moving), (first.id, None));
}

const FIRST: &str = "bytes://first.gif";
const SECOND: &str = "bytes://second.gif";
/// Taller than a line of text, so the room such a picture takes on the page
/// is its own.
const TALL: (u32, u32) = (8, 60);
const TALL_BYTES: u64 = 3 * (TALL.0 * TALL.1 * 4) as u64;

/// A window on a page with the picture at [`FIRST`] at its top and the one
/// at [`SECOND`] more than three windows further down, which may keep the
/// frames of one of them.
fn two_pictures_and_room_for_one() -> Window {
    let between = "a line between the pictures\n\n".repeat(40);
    let window = Window::on(&format!(
        "![first]({FIRST})\n\n{between}![second]({SECOND})\n\nthe end"
    ));
    window.ctx.include_bytes(FIRST, gif_of(TALL, 3));
    window.ctx.include_bytes(SECOND, gif_of(TALL, 3));
    animation::set_frame_budget(
        &window.ctx,
        FrameBudget {
            one_picture: TALL_BYTES,
            all_pictures: TALL_BYTES,
            ..FrameBudget::default()
        },
    );
    window
}

/// How far the page is moved up to bring the second picture into view.
const DOWN_THE_PAGE: f32 = -1200.0;

/// How far down its page the top of the second picture is.
fn second_picture_at() -> f32 {
    let mut window = two_pictures_and_room_for_one();
    window.above = DOWN_THE_PAGE;
    let shown = window.read_at(0.1);
    shown.pictures[0].1.top() - DOWN_THE_PAGE
}

fn kept(window: &Window) -> (usize, usize) {
    (
        textures_of(&window.ctx, FIRST),
        textures_of(&window.ctx, SECOND),
    )
}

#[test]
fn a_picture_far_from_the_view_is_not_read_and_its_place_is_held_at_its_size() {
    let mut window = two_pictures_and_room_for_one();

    window.frame_at(0.1);
    let asked_for = window.decode();
    let at_the_top = window.frame_at(0.2);
    window.frame_at(0.3);
    let read_later = window.decode();
    let only_the_first = kept(&window);
    let none_pending = !window.ctx.has_pending_images();
    window.above = DOWN_THE_PAGE;
    let held = window.frame_at(0.4);
    window.decode();
    let arrived = window.frame_at(0.5);

    assert_eq!(asked_for, 1, "only the picture in view was asked for");
    assert_eq!(read_later, 0, "the other was not asked for afterwards");
    assert_eq!(only_the_first, (3, 0));
    assert!(none_pending);
    assert_eq!(at_the_top.pictures.len(), 1);
    assert!(held.pictures.is_empty(), "its place is held empty");
    assert_eq!(arrived.pictures.len(), 1);
    let second = arrived.pictures[0].1;
    assert_eq!(second.size(), egui::vec2(8.0, 60.0));
    // Nothing on the page moves when the picture arrives: the line after
    // it was a picture's height and a paragraph's gap below the line
    // before it all along.
    assert_eq!(arrived.top_of("the end"), held.top_of("the end"));
    assert!(held.top_of("the end") - second.bottom() < 20.0);
    assert!(held.top_of("the end") > second.bottom());
}

#[test]
fn a_picture_is_read_before_it_is_in_view_once_it_is_within_a_window_of_it() {
    let second_at = second_picture_at();
    let mut window = two_pictures_and_room_for_one();
    window.read_at(0.1);

    // The top of the second picture is half a window below the window.
    window.above = WINDOW.y * 1.5 - second_at;
    let near = window.frame_at(0.3);
    let asked_for = window.decode();
    let ready = window.frame_at(0.4);
    // And two windows below it.
    let mut far = two_pictures_and_room_for_one();
    far.read_at(0.1);
    far.above = WINDOW.y * 3.0 - second_at;
    far.frame_at(0.3);

    assert!(animation::NEAR_IN_WINDOW_HEIGHTS >= 0.5);
    assert!(near.pictures.is_empty() && ready.pictures.is_empty());
    assert_eq!(
        near.next_repaint,
        Duration::MAX,
        "nothing asks for a frame while a picture out of view is decoded"
    );
    assert_eq!(asked_for, 1);
    assert_eq!(textures_of(&window.ctx, SECOND), 3, "ready to be shown");
    assert_eq!(far.decode(), 0, "too far away to be read");
}

#[test]
fn over_the_budget_a_picture_away_from_the_view_is_let_go_and_stays_so_until_it_is_near() {
    let mut window = two_pictures_and_room_for_one();
    window.read_at(0.1);
    window.above = DOWN_THE_PAGE;
    let read = window.read_at(0.2);
    let both = kept(&window);

    let later = window.frame_at(3.0);
    let still_later = window.frame_at(3.1);

    assert_eq!(both, (3, 3), "one seen a moment ago is kept");
    assert_eq!(read.pictures.len(), 1, "only the second is in view");
    assert_eq!(kept(&window), (0, 3));
    assert_eq!(window.decode(), 0, "nothing was read again");
    assert_eq!(decoded_bytes(&window.ctx), 0);
    assert_eq!(later.pictures.len(), 1);
    assert_eq!(still_later.pictures[0].1, read.pictures[0].1);
    assert!(still_later.next_repaint < Duration::from_secs(1));
}

#[test]
fn a_picture_that_was_let_go_plays_again_in_its_place_when_it_is_back_in_view() {
    let mut window = two_pictures_and_room_for_one();
    window.read_at(0.1);
    window.above = DOWN_THE_PAGE;
    let before = window.read_at(0.3);
    window.above = 0.0;
    window.frame_at(3.0);
    assert_eq!(textures_of(&window.ctx, SECOND), 0, "let go");

    window.above = DOWN_THE_PAGE;
    let asked_for = window.frame_at(3.1);
    window.decode();
    let decoded = decoded_bytes(&window.ctx);
    let back = [3.6, 4.1, 4.6].map(|seconds| window.frame_at(seconds));
    window.frame_at(6.2);
    let long_after = window.frame_at(6.3);

    assert!(asked_for.pictures.is_empty(), "its place is held");
    assert_eq!(asked_for.next_repaint, Duration::ZERO, "a spinner turns");
    assert_eq!(decoded as u64, TALL_BYTES, "decoded off the frame");
    assert_eq!(textures_of(&window.ctx, SECOND), 3);
    assert_eq!(decoded_bytes(&window.ctx), 0);
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
fn room_is_made_for_a_picture_when_it_starts_to_be_read_and_not_when_it_arrives() {
    let mut window = two_pictures_and_room_for_one();
    window.read_at(0.1);
    window.above = DOWN_THE_PAGE;
    window.read_at(0.2);
    window.above = 0.0;
    window.frame_at(3.0);
    assert_eq!(kept(&window), (3, 0), "the second was let go");

    window.above = DOWN_THE_PAGE;
    let asks = window.frame_at(6.0);
    let room_made = (kept(&window), decoded_bytes(&window.ctx));
    let decoded = window.decode();
    let shows = window.frame_at(6.1);

    assert_eq!(room_made, ((0, 0), 0), "before anything was decoded");
    assert!(asks.pictures.is_empty());
    assert_eq!(decoded, 1);
    assert_eq!(kept(&window), (0, 3));
    assert_eq!(shows.pictures.len(), 1);
}

#[test]
fn a_picture_forgotten_while_it_is_decoded_is_dropped_when_the_thread_is_done() {
    let mut window = Window::new(3);
    window.frame_at(0.1);
    assert!(window.ctx.has_pending_images(), "it is being decoded");

    animation::forget(&window.ctx, PULSE);
    let kept = window.decode();
    let after = window.frame_at(0.2);

    assert_eq!(kept, 0, "the result was dropped");
    assert_eq!(decoded_bytes(&window.ctx), 0);
    assert_eq!(textures_of(&window.ctx, PULSE), 0);
    assert!(after.pictures.is_empty());
}

#[test]
fn a_picture_asked_for_again_while_it_is_decoded_is_kept_once() {
    let mut window = Window::new(3);
    window.frame_at(0.1);
    animation::forget(&window.ctx, PULSE);
    // egui lets go of bytes it was handed, where it would read a file again.
    window.ctx.include_bytes(PULSE, gif(3));
    window.frame_at(0.2);

    let kept = window.decode();
    let shown = window.frame_at(0.3);

    assert_eq!(kept, 1, "of the two requests the later one is kept");
    assert_eq!(shown.pictures.len(), 1);
    assert_eq!(textures_of(&window.ctx, PULSE), 3);
    assert_eq!(decoded_bytes(&window.ctx), 0);
}

#[test]
fn a_picture_that_is_forgotten_gives_back_what_was_kept_of_it_and_is_read_again_when_shown() {
    let mut window = Window::new(3);
    window.read_at(0.1);
    window.frame_at(0.2);

    animation::forget(&window.ctx, PULSE);
    let kept = (textures_of(&window.ctx, PULSE), decoded_bytes(&window.ctx));
    // egui lets go of bytes it was handed, where it would read a file again.
    window.ctx.include_bytes(PULSE, gif(3));
    window.read_at(0.3);
    let again = window.frame_at(0.6);

    assert_eq!(kept, (0, 0));
    let second = frame_texture(&window.ctx, 1).unwrap();
    assert_eq!(again.pictures.len(), 1);
    assert_eq!(again.pictures[0].0, second.id);
    assert_eq!(textures_of(&window.ctx, PULSE), 3);
}

/// A page that scrolls inside the window, with the picture at [`SECOND`]
/// far down it and lines of text before and after.
fn a_long_page(picture: &str) -> Window {
    let lines = |name: &str| {
        let lines = (0..40).map(|line| format!("{name} {line}\n\n"));
        lines.collect::<String>()
    };
    let mut window = Window::on(&format!(
        "{}![far]({picture})\n\n{}",
        lines("above"),
        lines("below")
    ));
    window.page_scrolls = true;
    window
}

#[test]
fn scrolling_towards_a_picture_has_it_read_while_it_is_still_out_of_view() {
    let mut window = a_long_page(SECOND);
    window.ctx.include_bytes(SECOND, gif_of(TALL, 3));
    window.frame_at(0.1);
    window.frame_at(0.2);
    assert_eq!(window.decode(), 0, "far below the view it is not read");

    let (mut asked_at, mut seen_at) = (None, None);
    for step in 1..200 {
        let offset = step as f32 * 20.0;
        window.scroll_to = Some(offset);
        let shown = window.frame_at(0.2 + f64::from(step) * 0.01);
        if window.decode() == 1 {
            asked_at = Some(offset);
        }
        if !shown.pictures.is_empty() {
            seen_at = Some(offset);
            break;
        }
    }

    let (asked_at, seen_at) = (asked_at.unwrap(), seen_at.unwrap());
    assert!(
        seen_at - asked_at > WINDOW.y / 2.0,
        "asked for at {asked_at}, in view at {seen_at}"
    );
}

/// Bytes that are a while in coming, as those of a picture on the web
/// are: nothing tells the size of the picture before they are here.
#[derive(Default)]
struct Slow {
    arrived: std::sync::atomic::AtomicBool,
}

const SLOW: &str = "slow://late.gif";

impl egui::load::BytesLoader for Slow {
    fn id(&self) -> &str {
        "moving_picture::Slow"
    }

    fn load(&self, _: &egui::Context, uri: &str) -> egui::load::BytesLoadResult {
        if uri != SLOW {
            return Err(egui::load::LoadError::NotSupported);
        }
        if !self.arrived.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(egui::load::BytesPoll::Pending { size: None });
        }
        Ok(egui::load::BytesPoll::Ready {
            size: None,
            bytes: gif_of(TALL, 3).into(),
            mime: None,
        })
    }

    fn forget(&self, _: &str) {}

    fn forget_all(&self) {}

    fn byte_size(&self) -> usize {
        0
    }
}

#[test]
fn a_picture_above_the_view_whose_size_is_known_late_does_not_move_what_is_in_view() {
    let mut window = a_long_page(SLOW);
    let slow = std::sync::Arc::new(Slow::default());
    window.ctx.add_bytes_loader(slow.clone());
    window.frame_at(0.1);
    // Scroll until the lines on either side of the picture are in view,
    // and then until the edge of the view cuts the place held for the
    // picture, which is a small one and lies between them.
    let mut rough = window.frame_at(0.2);
    while rough.find("above 39").is_none() || rough.find("below 0").is_none() {
        assert!(rough.scrolled < 3000.0, "the picture was never reached");
        window.scroll_to = Some(rough.scrolled + 50.0);
        window.frame_at(0.2);
        rough = window.frame_at(0.2);
    }
    let between = (rough.top_of("above 39") + rough.top_of("below 0")) / 2.0;
    window.scroll_to = Some(rough.scrolled + between - rough.pane_top);
    window.frame_at(0.4);
    let before = window.frame_at(0.5);
    assert!(before.pictures.is_empty());
    assert!(before.find("above 39").is_none(), "it is above the view");

    slow.arrived
        .store(true, std::sync::atomic::Ordering::Relaxed);
    window.frame_at(0.6);
    let decoded = window.decode();
    let after: Vec<_> = (0..4)
        .map(|frame| window.frame_at(0.7 + f64::from(frame) * 0.01))
        .collect();

    assert_eq!(decoded, 1);
    let settled = after.last().unwrap();
    assert_eq!(settled.pictures.len(), 1, "the picture is drawn");
    assert_eq!(settled.pictures[0].1.size(), egui::vec2(8.0, 60.0));
    for shown in &after {
        assert_eq!(
            shown.top_of("below 0"),
            before.top_of("below 0"),
            "the line under the reader's eyes stayed where it was"
        );
    }
    assert!(
        settled.scrolled > before.scrolled + 30.0,
        "{}",
        settled.scrolled
    );
}
