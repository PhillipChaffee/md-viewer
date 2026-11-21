//! What md-viewer asks of macOS about its own window.
//!
//! [`center_window_buttons`], [`hand_quit_key_to_app`],
//! [`ask_before_quit`] and [`set_dock_icon`] reach nothing on every other
//! system, so the app calls them without asking where it runs.
//! [`placement`] is plain arithmetic, and [`quit_answer`] and
//! [`pixel_rows`] plain values, the same everywhere.

use std::sync::OnceLock;

#[cfg(not(target_os = "macos"))]
mod elsewhere;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(test)]
mod tests;

#[cfg(not(target_os = "macos"))]
use elsewhere::answer_quit_requests;
#[cfg(not(target_os = "macos"))]
pub use elsewhere::{center_window_buttons, hand_quit_key_to_app, set_dock_icon};
#[cfg(target_os = "macos")]
use mac::answer_quit_requests;
#[cfg(target_os = "macos")]
pub use mac::{center_window_buttons, hand_quit_key_to_app, set_dock_icon};

/// What the app says when the system asks whether it may quit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuitAnswer {
    /// Quit: nothing would be lost.
    Now,
    /// Do not quit: the app has something to ask its reader first, and
    /// closes on its own once it has the answers.
    NotYet,
}

/// The app's answer to a request to quit, once it has given one to ask.
type Asker = Box<dyn Fn() -> QuitAnswer + Send + Sync>;

/// The answer [`ask_before_quit`] was given. A static, because macOS asks
/// through a plain function that carries nothing with it.
static ASKER: OnceLock<Asker> = OnceLock::new();

/// Have macOS ask `asker` before it quits the app.
///
/// That covers Quit in the menu bar and in the Dock, and logging out,
/// restarting and shutting down. Without this macOS quits at once, and
/// the window is never asked to close.
///
/// `asker` runs on the main thread, inside the request. When it says
/// [`QuitAnswer::NotYet`] the request is refused, which also stops a
/// log-out; it is then for the app to ask its reader and to close.
///
/// The first `asker` given is the one kept. On any system but macOS it is
/// kept and never asked.
pub fn ask_before_quit(asker: impl Fn() -> QuitAnswer + Send + Sync + 'static) {
    if ASKER.set(Box::new(asker)).is_ok() {
        answer_quit_requests();
    }
}

/// What the app answers a request to quit with at this moment:
/// [`QuitAnswer::Now`] until [`ask_before_quit`] has been given an asker,
/// and from then on what that asker says.
#[must_use]
pub fn quit_answer() -> QuitAnswer {
    ASKER.get().map_or(QuitAnswer::Now, |asker| asker())
}

/// How far apart two lengths are before they count as different: macOS
/// rounds what it is given to whole pixels.
const SLACK: f64 = 0.5;

/// Where the title bar and the buttons in it go, in the screen's points.
///
/// Heights are measured up from the bottom edge of what holds them, as
/// macOS measures them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// The bottom of the title bar, above the bottom of the window.
    pub bar_bottom: f64,
    /// How tall the title bar is.
    pub bar_height: f64,
    /// The bottom of a window button, above the bottom of the title bar.
    pub button_bottom: f64,
}

/// The title bar that makes a row at the top of a window the home of the
/// window's buttons.
///
/// The bar is as tall as the row of `row_height` and ends at the top of
/// the window `window_height` tall. Buttons `button_height` tall sit on
/// its centre line. A row lower than a button gets a bar as tall as the
/// button, so the button stays whole.
#[must_use]
pub fn placement(window_height: f64, row_height: f64, button_height: f64) -> Placement {
    let bar_height = row_height.max(button_height);
    Placement {
        bar_bottom: window_height - bar_height,
        bar_height,
        button_bottom: (bar_height - button_height) / 2.0,
    }
}

/// Whether `length` is not yet `wanted`, so that it has to be set.
#[must_use]
pub fn is_off(length: f64, wanted: f64) -> bool {
    (length - wanted).abs() > SLACK
}

/// The bytes a picture is made of, four to a pixel (red, green, blue and
/// alpha), row after row from the top, in the integers macOS takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRows {
    /// Pixels across.
    pub width: isize,
    /// Pixels down.
    pub height: isize,
    /// Bytes in one row: four for each pixel across.
    pub bytes_per_row: isize,
    /// Bytes in the whole picture.
    pub bytes: usize,
}

impl PixelRows {
    /// Whether a bitmap whose rows are `bytes_per_row` long and which
    /// holds `bytes_per_plane` bytes in all is laid out as these rows are,
    /// so that the picture can be copied into it byte for byte.
    #[must_use]
    pub fn fit(self, bytes_per_row: isize, bytes_per_plane: isize) -> bool {
        bytes_per_row == self.bytes_per_row && usize::try_from(bytes_per_plane) == Ok(self.bytes)
    }
}

/// The rows of `rgba` read as a picture `width` pixels across and
/// `height` down.
///
/// `None` when it is not one: when it holds more or fewer bytes than four
/// for each pixel, when it has no pixels, or when its sizes do not fit
/// the integers macOS takes.
#[must_use]
pub fn pixel_rows(rgba: &[u8], width: u32, height: u32) -> Option<PixelRows> {
    let row = usize::try_from(width).ok()?.checked_mul(4)?;
    let bytes = row.checked_mul(usize::try_from(height).ok()?)?;
    if bytes == 0 || bytes != rgba.len() {
        return None;
    }
    Some(PixelRows {
        width: isize::try_from(width).ok()?,
        height: isize::try_from(height).ok()?,
        bytes_per_row: isize::try_from(row).ok()?,
        bytes,
    })
}
