//! Which frame of a moving picture (an animated GIF or WebP) is on screen,
//! and when the next one is due.
//!
//! egui's `Image` can pick the frame itself, but it asks for the next
//! repaint whether or not the picture is in view, and for a picture of one
//! frame as well. A window showing such a document would then never come to
//! rest. So the frame is picked here, and the caller asks for the repaint
//! only for a picture it has just drawn in view.

use std::time::Duration;

use egui::{
    load::{SizeHint, SizedTexture, TexturePoll},
    FrameDurations, Id, TextureOptions,
};

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

/// The frame of the picture at `uri` that is on screen now, and how long
/// until the next one, if the picture is known to be a GIF or WebP. It is
/// known once egui's loader has read it, which the first `Image` shown for
/// `uri` brings about.
fn frame_now(ctx: &egui::Context, uri: &str) -> Option<(usize, Option<Duration>)> {
    let delays: FrameDurations = ctx.data(|data| data.get_temp(Id::new(uri)))?;
    let now = ctx.input(|input| Duration::try_from_secs_f64(input.time).ok())?;
    Some(frame_at(&delays, now))
}

/// The frame to draw now for the moving picture at `uri`. `None` for any
/// other picture, and for one that is still being read.
pub(crate) fn current_frame(ctx: &egui::Context, uri: &str) -> Option<SizedTexture> {
    let (index, _) = frame_now(ctx, uri)?;
    // The address egui's loaders keep one frame under.
    let frame = format!("{uri}#{index}");
    let load = ctx.try_load_texture(&frame, TextureOptions::default(), SizeHint::default());
    match load {
        Ok(TexturePoll::Ready { texture }) => Some(texture),
        Ok(TexturePoll::Pending { .. }) | Err(_) => None,
    }
}

/// Ask for a repaint when the moving picture at `uri` is due its next
/// frame. For a picture drawn in view only: this is what keeps the window
/// drawing.
pub(crate) fn repaint_for_next_frame(ctx: &egui::Context, uri: &str) {
    if let Some((_, Some(left))) = frame_now(ctx, uri) {
        ctx.request_repaint_after(left);
    }
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
