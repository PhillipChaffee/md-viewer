//! The app's icon: a serif M on a tile, in two forms. Which form shows
//! follows the reading theme.

use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use eframe::egui::IconData;

use crate::persist::theme_saved_in;
use crate::theme::ThemeId;

#[cfg(test)]
mod tests;

/// The file eframe keeps the app's saved settings in, inside the folder
/// it gives the app for them.
const SETTINGS_FILE: &str = "app.ron";

/// One of the two forms of the icon. Each is a picture in `assets/icons/`
/// that carries its own margin, rounded corners and shadow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppIcon {
    /// A dark M on a warm paper tile: the icon of the light themes.
    Sepia,
    /// The same M in paper on a near-black tile: the icon of the dark
    /// themes.
    Reversed,
}

impl AppIcon {
    /// The icon that goes with `theme`.
    pub(crate) const fn for_theme(theme: ThemeId) -> Self {
        if theme.palette().dark {
            Self::Reversed
        } else {
            Self::Sepia
        }
    }

    /// The icon the window opens with: the one for the theme the last
    /// session saved under `settings_folder`, which eframe has not read
    /// yet when the window is built. Without a saved session, as on a
    /// first run, that is the default theme's.
    pub(crate) fn at_launch(settings_folder: Option<&Path>) -> Self {
        let settings = settings_folder
            .and_then(|folder| fs::read_to_string(folder.join(SETTINGS_FILE)).ok())
            .unwrap_or_default();
        Self::for_theme(theme_saved_in(&settings))
    }

    /// The picture, as the PNG file compiled into the binary.
    const fn png(self) -> &'static [u8] {
        match self {
            Self::Sepia => include_bytes!("../../assets/icons/sepia-512.png"),
            Self::Reversed => include_bytes!("../../assets/icons/reversed-512.png"),
        }
    }

    /// The picture decoded, for the window and the Dock. The PNG is
    /// decoded the first time it is asked for, and the same picture is
    /// handed out from then on. It is asked for when the window is built
    /// and when the icon changes, never per frame.
    pub(crate) fn image(self) -> Arc<IconData> {
        static SEPIA: OnceLock<Arc<IconData>> = OnceLock::new();
        static REVERSED: OnceLock<Arc<IconData>> = OnceLock::new();
        let decoded = match self {
            Self::Sepia => &SEPIA,
            Self::Reversed => &REVERSED,
        };
        let decode = || Arc::new(eframe::icon_data::from_png_bytes(self.png()).unwrap_or_default());
        Arc::clone(decoded.get_or_init(decode))
    }
}
