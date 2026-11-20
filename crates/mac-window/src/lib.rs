//! What md-viewer asks of macOS about its own window.
//!
//! [`center_window_buttons`] and [`hand_quit_key_to_app`] do nothing on
//! every other system, so the app calls them without asking where it
//! runs. [`placement`] is plain arithmetic, the same everywhere.

#[cfg(not(target_os = "macos"))]
mod elsewhere;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(test)]
mod tests;

#[cfg(not(target_os = "macos"))]
pub use elsewhere::{center_window_buttons, hand_quit_key_to_app};
#[cfg(target_os = "macos")]
pub use mac::{center_window_buttons, hand_quit_key_to_app};

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
