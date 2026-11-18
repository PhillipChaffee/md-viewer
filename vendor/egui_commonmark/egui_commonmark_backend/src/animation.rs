//! Which frame of a moving picture (an animated GIF or WebP) is on screen,
//! when the next one is due, and how many frames are kept.
//!
//! egui's `Image` can pick the frame itself, but it asks for the next
//! repaint whether or not the picture is in view, and for a picture of one
//! frame as well. A window showing such a document would then never come to
//! rest. So the frame is picked here, and the caller asks for the repaint
//! only for a picture it has just drawn in view.
//!
//! egui's loaders decode every frame of a picture when they read it. They
//! keep the decoded frames, and a texture for each frame shown, for as long
//! as the app runs. Here the frames become textures once, the decoded
//! frames are let go, and the textures are counted against a
//! [`FrameBudget`].

use std::{collections::HashMap, time::Duration};

use egui::{
    load::{SizeHint, SizedTexture, TexturePoll},
    FrameDurations, Id, TextureOptions, Vec2,
};

/// The most the frames of one picture may take, in bytes. A picture that
/// needs more shows its first frame and does not move. 256 MiB is 139
/// frames of 800 by 600 pixels.
pub const ONE_PICTURE_BYTES: u64 = 256 * 1024 * 1024;

/// The most the frames of all pictures may take together, in bytes, before
/// pictures out of view are let go.
pub const ALL_PICTURES_BYTES: u64 = 512 * 1024 * 1024;

/// A picture is not let go until it has been out of view for this long, in
/// seconds: one that sits at the edge of the view goes in and out of it.
const KEPT_AFTER_VIEW_SECONDS: f64 = 2.0;

/// How much memory the frames of moving pictures may take. A frame takes
/// four bytes a pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameBudget {
    /// A picture whose frames take more than this shows its first frame
    /// and does not move.
    pub one_picture: u64,
    /// When the frames kept take more than this, pictures out of view are
    /// let go, the one seen longest ago first. They are read again when
    /// they come back into view. A picture in view is never let go, so
    /// pictures on screen together can take more than this.
    pub all_pictures: u64,
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self {
            one_picture: ONE_PICTURE_BYTES,
            all_pictures: ALL_PICTURES_BYTES,
        }
    }
}

/// Which frames of a picture are kept as textures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kept {
    /// All of them: the picture plays.
    Every,
    /// The first: the picture is too large to play.
    First,
    /// None: the picture was let go, and is read again when it is in view.
    Nothing,
}

/// A GIF or WebP that egui's loader has read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Picture {
    /// The size of each frame, in pixels.
    size: Vec2,
    /// How many frames the file has.
    frames: usize,
    kept: Kept,
    /// When the picture was last read or in view, on the app's clock.
    last_shown: f64,
}

impl Picture {
    fn kept_frames(&self) -> usize {
        match self.kept {
            Kept::Every => self.frames,
            Kept::First => 1,
            Kept::Nothing => 0,
        }
    }

    fn kept_bytes(&self) -> u64 {
        // The sides of a texture are whole numbers of pixels.
        let frame = self.size.x as u64 * self.size.y as u64 * 4;
        frame.saturating_mul(self.kept_frames() as u64)
    }
}

/// Every moving picture the loaders have read, by its address.
#[derive(Clone, Default)]
struct Pictures {
    budget: FrameBudget,
    known: HashMap<String, Picture>,
}

impl Pictures {
    /// Take in the picture at `uri`, just read. Returns the pictures let go
    /// to make room for it, each with the number of frames it kept.
    fn admit(&mut self, uri: &str, size: Vec2, frames: usize, now: f64) -> Vec<(String, usize)> {
        let picture = self.read_now(size, frames, now);
        self.known.insert(uri.to_owned(), picture);
        self.let_go_down_to(self.budget.all_pictures, now)
    }

    /// A picture of `frames` frames of `size` read at `now`, with as many
    /// of its frames kept as one picture may keep.
    fn read_now(&self, size: Vec2, frames: usize, now: f64) -> Picture {
        let mut picture = Picture {
            size,
            frames,
            kept: Kept::Every,
            last_shown: now,
        };
        if picture.kept_bytes() > self.budget.one_picture {
            picture.kept = Kept::First;
        }
        picture
    }

    /// Make room for the picture at `uri`, which was let go and is about to
    /// be read again. Returns the pictures let go for it, as [`Self::admit`]
    /// does. Room made before the picture is read is room the two never
    /// take together.
    fn make_room_for(&mut self, uri: &str, now: f64) -> Vec<(String, usize)> {
        let Some(coming) = self.known.get(uri) else {
            return Vec::new();
        };
        let room = self.read_now(coming.size, coming.frames, now).kept_bytes();
        self.let_go_down_to(self.budget.all_pictures.saturating_sub(room), now)
    }

    /// Let go of pictures out of view, the one seen longest ago first,
    /// until the frames kept take no more than `bytes`.
    fn let_go_down_to(&mut self, bytes: u64, now: f64) -> Vec<(String, usize)> {
        let mut let_go = Vec::new();
        loop {
            let kept: u64 = self.known.values().map(Picture::kept_bytes).sum();
            if kept <= bytes {
                break;
            }
            let out_of_view = self.known.iter_mut().filter(|(_, picture)| {
                picture.kept != Kept::Nothing && picture.last_shown + KEPT_AFTER_VIEW_SECONDS <= now
            });
            let longest_ago = out_of_view.min_by(|a, b| a.1.last_shown.total_cmp(&b.1.last_shown));
            let Some((uri, picture)) = longest_ago else {
                break;
            };
            let_go.push((uri.clone(), picture.kept_frames()));
            picture.kept = Kept::Nothing;
        }
        let_go
    }
}

fn pictures<R>(ctx: &egui::Context, read: impl FnOnce(&mut Pictures) -> R) -> R {
    let id = Id::new("egui_commonmark moving pictures");
    ctx.data_mut(|data| read(data.get_temp_mut_or_default(id)))
}

/// Set how much memory the frames of moving pictures may take in `ctx`. It
/// holds for pictures read from now on.
pub fn set_frame_budget(ctx: &egui::Context, budget: FrameBudget) {
    pictures(ctx, |pictures| pictures.budget = budget);
}

/// Browsers show a frame that asks for a hundredth of a second or less for
/// a tenth of a second, and pictures are made with that in mind.
const SHORTEST_DELAY_MS: u128 = 10;
const DELAY_IN_ITS_PLACE_MS: u128 = 100;

fn shown_for_ms(delay: &Duration) -> u128 {
    let delay = delay.as_millis();
    if delay <= SHORTEST_DELAY_MS {
        DELAY_IN_ITS_PLACE_MS
    } else {
        delay
    }
}

/// The frame on screen `now` into a loop of frames shown for `delays`, and
/// how long until the next one. A picture of one frame never has a next.
pub(crate) fn frame_at(delays: &FrameDurations, now: Duration) -> (usize, Option<Duration>) {
    if delays.all().len() < 2 {
        return (0, None);
    }
    let period: u128 = delays.all().map(shown_for_ms).sum();
    let position = now.as_millis() % period;
    let mut end = 0;
    for (index, delay) in delays.all().enumerate() {
        end += shown_for_ms(delay);
        if position < end {
            let left = u64::try_from(end - position).unwrap_or(u64::MAX);
            return (index, Some(Duration::from_millis(left)));
        }
    }
    (0, None)
}

fn delays(ctx: &egui::Context, uri: &str) -> Option<FrameDurations> {
    ctx.data(|data| data.get_temp(Id::new(uri)))
}

fn now(ctx: &egui::Context) -> f64 {
    ctx.input(|input| input.time)
}

/// The address egui's loaders keep frame `index` of the picture at `uri`
/// under.
fn frame_uri(uri: &str, index: usize) -> String {
    format!("{uri}#{index}")
}

/// The texture of frame `index` of the picture at `uri`. It is made now if
/// there is none yet and the loader holds the decoded picture.
fn frame_texture(ctx: &egui::Context, uri: &str, index: usize) -> Option<SizedTexture> {
    let load = ctx.try_load_texture(
        &frame_uri(uri, index),
        TextureOptions::default(),
        SizeHint::default(),
    );
    match load {
        Ok(TexturePoll::Ready { texture }) => Some(texture),
        Ok(TexturePoll::Pending { .. }) | Err(_) => None,
    }
}

/// Free the textures of the first `frames` frames of the picture at `uri`.
fn forget_frames(ctx: &egui::Context, uri: &str, frames: usize) {
    for index in 0..frames {
        ctx.forget_image(&frame_uri(uri, index));
    }
}

/// Take in the picture at `uri`, which egui's loader has just decoded into
/// `frames` frames: make a texture of each frame that is kept, and have
/// the loader let go of what it decoded. The file's own bytes stay with the
/// loader that read them: they are small, and a picture from the web is
/// not fetched again when it comes back.
fn take_in(ctx: &egui::Context, uri: &str, frames: usize) -> Option<Picture> {
    let first = frame_texture(ctx, uri, 0)?;
    let now = now(ctx);
    let let_go = pictures(ctx, |pictures| pictures.admit(uri, first.size, frames, now));
    for (uri, frames) in let_go {
        forget_frames(ctx, &uri, frames);
    }
    // The picture itself is not among those let go: it was read just now.
    let picture = pictures(ctx, |pictures| pictures.known.get(uri).cloned())?;
    for index in 1..picture.kept_frames() {
        frame_texture(ctx, uri, index);
    }
    for loader in ctx.loaders().image.lock().iter() {
        loader.forget(uri);
    }
    Some(picture)
}

/// What is known of the picture at `uri`, which is taken in now if egui's
/// loader has read it and it was not taken in yet. `None` for a picture
/// that does not move, and for one the loader has not read.
///
/// The frame that has the loader read a picture calls this once the loader
/// is done. The page may not draw the picture in the next frame, and until
/// the picture is taken in the loader keeps all it decoded.
pub(crate) fn taken_in(ctx: &egui::Context, uri: &str) -> Option<Picture> {
    let known = pictures(ctx, |pictures| pictures.known.get(uri).cloned());
    known.or_else(|| take_in(ctx, uri, delays(ctx, uri)?.all().len()))
}

/// How the picture at `uri` is drawn this frame.
pub(crate) enum Drawn {
    /// As egui's `Image` draws the address: it is no moving picture, or it
    /// is still being read.
    ByAddress,
    /// As this frame of it. `plays` if other frames follow it.
    Frame { texture: SizedTexture, plays: bool },
    /// Not at all: its frames, of this size, were let go. Its place is
    /// held, and [`bring_back`] reads it again.
    LetGo(Vec2),
}

/// How to draw the picture at `uri` now. A GIF or WebP is known once egui's
/// loader has read it, which the first `Image` shown for `uri` brings
/// about.
pub(crate) fn to_draw(ctx: &egui::Context, uri: &str) -> Drawn {
    let (Some(picture), Some(delays)) = (taken_in(ctx, uri), delays(ctx, uri)) else {
        return Drawn::ByAddress;
    };
    let index = match picture.kept {
        Kept::Nothing => return Drawn::LetGo(picture.size),
        Kept::First => 0,
        Kept::Every => {
            Duration::try_from_secs_f64(now(ctx)).map_or(0, |now| frame_at(&delays, now).0)
        }
    };
    let plays = picture.kept == Kept::Every && picture.frames > 1;
    match frame_texture(ctx, uri, index) {
        Some(texture) => Drawn::Frame { texture, plays },
        None => Drawn::ByAddress,
    }
}

/// The picture at `uri` was drawn in view. It is kept while it is, and if
/// it plays, a repaint is asked for when its next frame is due: this is
/// what keeps the window drawing. Pictures out of view are let go here if
/// the frames kept are over the budget, which they can be after pictures
/// that were read together have left the view.
pub(crate) fn in_view(ctx: &egui::Context, uri: &str) {
    let now = now(ctx);
    let (kept, let_go) = pictures(ctx, |pictures| {
        let kept = pictures.known.get_mut(uri).map(|picture| {
            picture.last_shown = now;
            picture.kept
        });
        (
            kept,
            pictures.let_go_down_to(pictures.budget.all_pictures, now),
        )
    });
    for (uri, frames) in let_go {
        forget_frames(ctx, &uri, frames);
    }
    if kept == Some(Kept::First) {
        return;
    }
    let (Some(delays), Ok(now)) = (delays(ctx, uri), Duration::try_from_secs_f64(now)) else {
        return;
    };
    if let (_, Some(left)) = frame_at(&delays, now) {
        ctx.request_repaint_after(left);
    }
}

/// The place of the picture at `uri`, whose frames were let go, is in view
/// again: have egui's loader read it once more. It is taken in as soon as
/// the loader is done, and the frame after that draws it.
///
/// If other pictures have to be let go for it, that is all this frame
/// does. Their textures are freed when the frame ends, and the loader reads
/// the picture in the next.
pub(crate) fn bring_back(ctx: &egui::Context, uri: &str) {
    let now = now(ctx);
    let let_go = pictures(ctx, |pictures| pictures.make_room_for(uri, now));
    if !let_go.is_empty() {
        for (uri, frames) in let_go {
            forget_frames(ctx, &uri, frames);
        }
        ctx.request_repaint();
        return;
    }
    let load = ctx.try_load_texture(
        &frame_uri(uri, 0),
        TextureOptions::default(),
        SizeHint::default(),
    );
    if matches!(load, Ok(TexturePoll::Pending { .. })) {
        return;
    }
    pictures(ctx, |pictures| pictures.known.remove(uri));
    taken_in(ctx, uri);
    ctx.request_repaint();
}

/// The frame to show now in an enlarged view of the moving picture at
/// `uri`. Asks for a repaint when the next frame is due, so call it only
/// in a frame that shows the picture. `None` once the picture's frames
/// were let go.
pub fn enlarged_frame(ctx: &egui::Context, uri: &str) -> Option<SizedTexture> {
    let Drawn::Frame { texture, .. } = to_draw(ctx, uri) else {
        return None;
    };
    in_view(ctx, uri);
    Some(texture)
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
