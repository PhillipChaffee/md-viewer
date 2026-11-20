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
//!
//! A picture is decoded on a thread of its own (see [`crate::decode`]), and
//! only once the page asks for it with [`read`], which a page does for a
//! picture near the view. Until its frames are here its place is held.

use std::{collections::HashMap, time::Duration};

use egui::{
    load::{SizeHint, SizedTexture, TextureLoadResult, TexturePoll},
    FrameDurations, Id, TextureOptions, Vec2,
};

#[cfg(feature = "animated-images")]
#[doc(hidden)]
pub use crate::decode::{decode_by_hand, decode_held};
pub use crate::picture::NEAR_IN_WINDOW_HEIGHTS;

/// The most the frames of one picture may take, in bytes. A picture that
/// needs more shows its first frame and does not move. 256 MiB is 139
/// frames of 800 by 600 pixels.
pub const ONE_PICTURE_BYTES: u64 = 256 * 1024 * 1024;

/// The most the frames of all pictures may take together, in bytes, before
/// pictures out of view are let go.
pub const ALL_PICTURES_BYTES: u64 = 512 * 1024 * 1024;

/// The most of a picture's frames that are made textures in one frame of
/// the window, in bytes: eight frames of 800 by 600 pixels.
pub const EACH_FRAME_BYTES: u64 = 16 * 1024 * 1024;

/// A picture is not let go until it has been away from the view for this
/// long, in seconds: one that sits at the edge goes in and out.
const KEPT_AFTER_VIEW_SECONDS: f64 = 2.0;

/// How much memory the frames of moving pictures may take. A frame takes
/// four bytes a pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameBudget {
    /// A picture whose frames take more than this shows its first frame
    /// and does not move.
    pub one_picture: u64,
    /// When the frames kept take more than this, pictures away from the
    /// view are let go, the one seen longest ago first. They are read
    /// again when they come back near it. A picture in or near the view is
    /// never let go, so such pictures can take more than this together.
    ///
    /// A picture that is being read counts as the most one picture may
    /// take, or as what it took if it was let go before, so that the room
    /// for it is made before its frames arrive.
    pub all_pictures: u64,
    /// The most of a picture that was decoded that is made textures in one
    /// frame of the window, though never less than one frame of it. A
    /// texture is filled on the thread that draws, so a large picture
    /// arrives over several frames of the window, and plays once it is
    /// all there.
    pub each_frame: u64,
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self {
            one_picture: ONE_PICTURE_BYTES,
            all_pictures: ALL_PICTURES_BYTES,
            each_frame: EACH_FRAME_BYTES,
        }
    }
}

/// Which frames of a picture are kept as textures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kept {
    /// All of them.
    Every,
    /// None: the picture was let go, and is read again when it is near the
    /// view.
    Nothing,
}

/// A GIF or WebP that egui's loader has read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Picture {
    /// The size of each frame, in pixels.
    size: Vec2,
    /// How many frames were decoded: one, for a picture too large to play.
    frames: usize,
    kept: Kept,
    /// When the picture was last read or near the view, on the app's clock.
    last_shown: f64,
}

impl Picture {
    fn kept_frames(&self) -> usize {
        match self.kept {
            Kept::Every => self.frames,
            Kept::Nothing => 0,
        }
    }

    /// What the frames take when all of them are kept.
    fn bytes(&self) -> u64 {
        // The sides of a texture are whole numbers of pixels.
        let frame = self.size.x as u64 * self.size.y as u64 * 4;
        frame.saturating_mul(self.frames as u64)
    }

    fn kept_bytes(&self) -> u64 {
        match self.kept {
            Kept::Every => self.bytes(),
            Kept::Nothing => 0,
        }
    }
}

/// Every moving picture the loaders have read, by its address.
#[derive(Clone, Default)]
struct Pictures {
    budget: FrameBudget,
    known: HashMap<String, Picture>,
    /// The pictures that are being read, each with the room kept for it.
    coming: HashMap<String, u64>,
    /// The pictures that are decoded and not yet all textures, each with
    /// the number of its frames that are, and the pass of the window that
    /// made the last of them.
    arriving: HashMap<String, (usize, u64)>,
}

impl Pictures {
    /// Take in the picture at `uri`, just read. Returns the pictures let go
    /// to make room for it, each with the number of frames it kept.
    fn admit(&mut self, uri: &str, size: Vec2, frames: usize, now: f64) -> Vec<(String, usize)> {
        self.coming.remove(uri);
        self.arriving.remove(uri);
        let picture = Picture {
            size,
            frames,
            kept: Kept::Every,
            last_shown: now,
        };
        self.known.insert(uri.to_owned(), picture);
        self.let_go_down_to(self.budget.all_pictures, now)
    }

    /// The picture at `uri` is being read. Keep room for it from now on,
    /// and return the pictures let go to make that room, as [`Self::admit`]
    /// does. Room made before the frames arrive is room the two never take
    /// together, and two pictures read at once cannot pass the budget
    /// between them.
    fn expect(&mut self, uri: &str, now: f64) -> Vec<(String, usize)> {
        if self.coming.contains_key(uri) {
            return Vec::new();
        }
        let before = self.known.get(uri).map(Picture::bytes);
        let room = before.unwrap_or(self.budget.one_picture);
        self.coming.insert(uri.to_owned(), room);
        self.let_go_down_to(self.budget.all_pictures, now)
    }

    /// Let go of pictures away from the view, the one seen longest ago
    /// first, until the frames kept and the room for those being read take
    /// no more than `bytes`.
    fn let_go_down_to(&mut self, bytes: u64, now: f64) -> Vec<(String, usize)> {
        let mut let_go = Vec::new();
        loop {
            let kept: u64 = self.known.values().map(Picture::kept_bytes).sum();
            let coming: u64 = self.coming.values().sum();
            if kept.saturating_add(coming) <= bytes {
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

/// The most the frames of one picture may take in `ctx`, in bytes.
#[cfg(feature = "animated-images")]
pub(crate) fn most_for_one_picture(ctx: &egui::Context) -> u64 {
    pictures(ctx, |pictures| pictures.budget.one_picture)
}

/// Whether egui takes the picture at `uri` for one that may move, which it
/// tells by the end of the address. Such a picture is decoded by
/// [`crate::decode`] and drawn frame by frame from here, whether it turns
/// out to have one frame or many.
pub(crate) fn may_move(uri: &str) -> bool {
    uri.ends_with(".gif") || uri.ends_with(".webp")
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

/// The texture of frame `index` of the picture at `uri`, as egui's loaders
/// have it. It is made now if there is none yet and the loader holds the
/// decoded picture.
fn load_frame(ctx: &egui::Context, uri: &str, index: usize) -> TextureLoadResult {
    ctx.try_load_texture(
        &frame_uri(uri, index),
        TextureOptions::default(),
        SizeHint::default(),
    )
}

fn frame_texture(ctx: &egui::Context, uri: &str, index: usize) -> Option<SizedTexture> {
    match load_frame(ctx, uri, index) {
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

fn let_go_of(ctx: &egui::Context, pictures: Vec<(String, usize)>) {
    for (uri, frames) in pictures {
        forget_frames(ctx, &uri, frames);
    }
}

/// Take in the picture at `uri`, which the loader has decoded and whose
/// first frame is `first`: make a texture of every frame, and have the
/// loader let go of what it decoded. The file's own bytes stay with the
/// loader that read them: they are small, and a picture from the web is
/// not fetched again when it comes back.
///
/// One call makes textures of no more than [`FrameBudget::each_frame`]
/// allows, and asks for another frame of the window if frames are left.
/// Returns whether the picture is all here.
fn take_in(ctx: &egui::Context, uri: &str, first: SizedTexture) -> bool {
    // A picture egui took for one that may move and that is no GIF or WebP
    // after all has no delays, and one frame.
    let frames = delays(ctx, uri).map_or(1, |delays| delays.all().len().max(1));
    let one_frame = (first.size.x as u64 * first.size.y as u64 * 4).max(1);
    let pass = ctx.cumulative_pass_nr();
    let (arriving, each_frame) = pictures(ctx, |pictures| {
        let arriving = pictures.arriving.get(uri).copied();
        (arriving, pictures.budget.each_frame)
    });
    let here = match arriving {
        // This frame of the window has made its share of textures.
        Some((_, last)) if last == pass => return false,
        Some((here, _)) => here,
        None => 1,
    };
    let more = usize::try_from(each_frame / one_frame).unwrap_or(usize::MAX);
    let here_after = here.saturating_add(more.max(1)).min(frames);
    for index in here..here_after {
        frame_texture(ctx, uri, index);
    }
    if here_after < frames {
        let arriving = (here_after, pass);
        pictures(ctx, |pictures| {
            pictures.arriving.insert(uri.to_owned(), arriving)
        });
        ctx.request_repaint();
        return false;
    }
    let now = now(ctx);
    let let_go = pictures(ctx, |pictures| pictures.admit(uri, first.size, frames, now));
    let_go_of(ctx, let_go);
    for loader in ctx.loaders().image.lock().iter() {
        loader.forget(uri);
    }
    true
}

/// Have the picture at `uri` read, if its frames are not here and it is not
/// being read yet, and tell how far that has come. Call it only for a
/// picture for which [`may_move`] holds.
///
/// The picture is decoded on another thread, which asks for a repaint when
/// it is done. The call that finds it decoded takes it in, and
/// [`to_draw`] gives its frames from then on. Room for the frames is made
/// by the call that starts the reading.
pub(crate) fn read(ctx: &egui::Context, uri: &str) -> TextureLoadResult {
    let loaded = load_frame(ctx, uri, 0);
    let now = now(ctx);
    let is_kept = pictures(ctx, |pictures| {
        let kept = pictures.known.get(uri).map(|picture| picture.kept);
        kept == Some(Kept::Every)
    });
    match &loaded {
        Ok(TexturePoll::Ready { .. }) if is_kept => {}
        // A picture that is not all here yet is looked at again by
        // `take_in_decoded` for as long as room is kept for it.
        Ok(TexturePoll::Ready { texture }) if take_in(ctx, uri, *texture) => {}
        Ok(TexturePoll::Ready { .. } | TexturePoll::Pending { .. }) => {
            let let_go = pictures(ctx, |pictures| pictures.expect(uri, now));
            let_go_of(ctx, let_go);
        }
        Err(_) => {
            pictures(ctx, |pictures| pictures.coming.remove(uri));
        }
    }
    loaded
}

/// Take in every picture that was being read and is decoded now, wherever
/// on its page it is: until it is taken in, the loader keeps all that was
/// decoded of it.
pub(crate) fn take_in_decoded(ctx: &egui::Context) {
    let coming: Vec<String> = pictures(ctx, |pictures| pictures.coming.keys().cloned().collect());
    for uri in coming {
        let _ = read(ctx, &uri);
    }
}

/// How the picture at `uri` is drawn this frame.
pub(crate) enum Drawn {
    /// As egui draws any picture: it is none that may move.
    Still,
    /// As this frame of it. `plays` if other frames follow it.
    Frame { texture: SizedTexture, plays: bool },
    /// Not at all: its frames are not here. They were never read, are
    /// being read, or were let go. Its place is held, at the size of its
    /// frames if that is known, and [`read`] brings the frames.
    Away(Option<Vec2>),
}

/// How to draw the picture at `uri` now.
pub(crate) fn to_draw(ctx: &egui::Context, uri: &str) -> Drawn {
    if !may_move(uri) {
        return Drawn::Still;
    }
    let Some(picture) = pictures(ctx, |pictures| pictures.known.get(uri).cloned()) else {
        return Drawn::Away(None);
    };
    if picture.kept == Kept::Nothing {
        return Drawn::Away(Some(picture.size));
    }
    let due = delays(ctx, uri)
        .zip(Duration::try_from_secs_f64(now(ctx)).ok())
        .map_or(0, |(delays, now)| frame_at(&delays, now).0);
    match frame_texture(ctx, uri, due) {
        Some(texture) => Drawn::Frame {
            texture,
            plays: picture.frames > 1,
        },
        None => Drawn::Away(Some(picture.size)),
    }
}

/// The picture at `uri` is near the view: it is not let go while it is.
/// Pictures away from the view are let go here if the frames kept are over
/// the budget, which they can be after pictures that were read together
/// have left the view.
pub(crate) fn near_view(ctx: &egui::Context, uri: &str) {
    let now = now(ctx);
    let let_go = pictures(ctx, |pictures| {
        if let Some(picture) = pictures.known.get_mut(uri) {
            picture.last_shown = now;
        }
        pictures.let_go_down_to(pictures.budget.all_pictures, now)
    });
    let_go_of(ctx, let_go);
}

/// The picture at `uri` was drawn in view. If it plays, a repaint is asked
/// for when its next frame is due: this is what keeps the window drawing.
pub(crate) fn in_view(ctx: &egui::Context, uri: &str) {
    near_view(ctx, uri);
    let now = Duration::try_from_secs_f64(now(ctx));
    let (Some(delays), Ok(now)) = (delays(ctx, uri), now) else {
        return;
    };
    if let (_, Some(left)) = frame_at(&delays, now) {
        ctx.request_repaint_after(left);
    }
}

/// The frame to show now in an enlarged view of the moving picture at
/// `uri`. Asks for a repaint when the next frame is due, so call it only
/// in a frame that shows the picture. `None` while the picture's frames
/// are not here.
pub fn enlarged_frame(ctx: &egui::Context, uri: &str) -> Option<SizedTexture> {
    let Drawn::Frame { texture, .. } = to_draw(ctx, uri) else {
        return None;
    };
    in_view(ctx, uri);
    Some(texture)
}

/// Let go of everything egui keeps of the picture at `uri`, moving or not:
/// the file's bytes, what was decoded from them, and the textures. For a
/// picture no open document shows. It is read again if one does. A picture
/// that is being decoded is dropped when the thread gets to the end of it.
pub fn forget(ctx: &egui::Context, uri: &str) {
    pictures(ctx, |pictures| {
        pictures.known.remove(uri);
        pictures.coming.remove(uri);
        pictures.arriving.remove(uri);
    });
    let delays: Option<FrameDurations> = ctx.data_mut(|data| data.remove_temp(Id::new(uri)));
    forget_frames(ctx, uri, delays.map_or(0, |delays| delays.all().len()));
    ctx.forget_image(uri);
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
