//! What the top row is to the window. On a Mac the row is the window's
//! title bar: the close, minimize and zoom buttons sit in it, on its
//! centre line, and its empty parts move the window. Everywhere else, and
//! in full screen, the row is a row.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::BAR_HEIGHT;
use crate::app::MarkdownApp;
use crate::platform::Platform;

#[cfg(test)]
mod tests;

/// Room the window's buttons take at the left of the row, in the screen's
/// points. The buttons are the system's and keep their size when the app
/// zooms.
const WINDOW_BUTTONS_ROOM: f32 = 78.0;

/// Whose the window's title bar is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TitleBar {
    /// The system's: it draws one above the row, or the window fills the
    /// screen and shows none.
    Systems,
    /// The app's top row. `zoom` is how many of the screen's points one
    /// of the app's takes.
    Row { zoom: f32 },
}

impl TitleBar {
    /// The title bar of a window on `platform`, at the app's `zoom`.
    pub(crate) const fn of(platform: Platform, fullscreen: bool, zoom: f32) -> Self {
        match (platform, fullscreen) {
            (Platform::Mac, false) => Self::Row { zoom },
            (Platform::Mac, true) | (Platform::Other, _) => Self::Systems,
        }
    }

    /// How far from the window's left edge the row's first button
    /// starts, where the bar's own margin would start it at `bar_left`.
    pub(crate) fn inset(self, bar_left: f32) -> f32 {
        match self {
            Self::Systems => 0.0,
            Self::Row { zoom } => (WINDOW_BUTTONS_ROOM / zoom - bar_left).max(0.0),
        }
    }

    /// How tall the row is in the screen's points, where the window's
    /// buttons are to sit on its centre line.
    pub(crate) fn row_height_on_screen(self) -> Option<f32> {
        match self {
            Self::Systems => None,
            Self::Row { zoom } => Some(BAR_HEIGHT * zoom),
        }
    }

    /// What an empty part of the row takes from the pointer. It always
    /// takes the hover, so nothing under the bar is pointed at through
    /// it.
    pub(crate) fn sense(self) -> egui::Sense {
        match self {
            Self::Systems => egui::Sense::hover(),
            Self::Row { .. } => egui::Sense::click_and_drag(),
        }
    }

    /// What the reader asked of the window on an empty part of the row:
    /// a drag moves it, as a drag on any title bar does, and a double
    /// click zooms it.
    pub(crate) const fn pick(self, dragged: bool, double_clicked: bool) -> Option<AppAction> {
        match (self, double_clicked, dragged) {
            (Self::Systems, _, _) | (Self::Row { .. }, false, false) => None,
            (Self::Row { .. }, true, _) => Some(AppAction::ToggleMaximized),
            (Self::Row { .. }, false, true) => Some(AppAction::MoveWindow),
        }
    }
}

impl MarkdownApp {
    /// Whose the title bar is this frame.
    pub(crate) fn title_bar(&self, ctx: &egui::Context) -> TitleBar {
        let fullscreen = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
        TitleBar::of(self.platform, fullscreen, ctx.zoom_factor())
    }

    /// Keep the window's buttons on the centre line of the top row.
    ///
    /// The system puts them back where it wants them whenever it lays the
    /// title bar out, and does not say when. So this asks every frame,
    /// and `mac_window` moves them only when they are not in place: a
    /// frame that finds them there costs a few reads.
    pub(crate) fn center_window_buttons(&self, ctx: &egui::Context, frame: &eframe::Frame) {
        if let Some(height) = self.title_bar(ctx).row_height_on_screen() {
            mac_window::center_window_buttons(frame, f64::from(height));
        }
    }
}
