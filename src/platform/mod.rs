//! The kind of machine the app runs on, and what the app asks of it
//! outside its window.

use eframe::egui::IconData;

/// The kind of machine the app runs on, where that changes what it does:
/// which key makes a command, and whose title bar the window has. It is a
/// value the code is handed, so both kinds are tested on either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Platform {
    /// macOS, where ⌘ makes a command of a key.
    Mac,
    /// Linux and Windows, where Ctrl does.
    Other,
}

impl Platform {
    /// The machine this build is for: the one place the app asks.
    pub(crate) const HOST: Self = if cfg!(target_os = "macos") {
        Self::Mac
    } else {
        Self::Other
    };
}

/// Hands the Dock a picture to show as the app's icon. The app is given
/// one, so that a test can put its own in place of the system's.
pub(crate) type ShowInDock = fn(&IconData);

/// The system's [`ShowInDock`]. On a Mac the Dock shows `icon` at once:
/// the window's icon, which the app also sets, does not reach it there.
/// Elsewhere the window's icon is the app's, and this does nothing.
pub(crate) fn show_in_dock(icon: &IconData) {
    mac_window::set_dock_icon(&icon.rgba, icon.width, icon.height);
}
