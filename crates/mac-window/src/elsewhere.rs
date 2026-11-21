//! Every system but macOS, where the window keeps the title bar and the
//! menu its desktop gives it.

use raw_window_handle::HasWindowHandle;

/// Does nothing: only on macOS do the window's buttons share a row with
/// the app.
pub const fn center_window_buttons(_window: &impl HasWindowHandle, _row_height: f64) {}

/// Does nothing: only on macOS does a menu bar take the quit key.
pub const fn hand_quit_key_to_app() {}

/// Does nothing: only on macOS does the app show an icon of its own
/// outside its window, in the Dock. Elsewhere the window's icon is the
/// app's, and the window is asked to change it.
pub const fn set_dock_icon(_rgba: &[u8], _width: u32, _height: u32) {}

/// Does nothing: only macOS asks an app whether it may quit.
pub(crate) const fn answer_quit_requests() {}
