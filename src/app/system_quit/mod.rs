//! The app's answer when the system, not the reader's hand on the window,
//! asks it to quit: Quit in a Mac's menu bar or Dock, and logging out.
//!
//! That request arrives outside any frame, through `mac_window`, as a
//! plain function that can see none of the app. So the app leaves word:
//! [`SystemQuit`] holds whether edits are unsaved, written at the end of
//! each frame, and [`SystemQuit::asker`] is the answer made from it. With
//! nothing unsaved the system may quit at once. With edits unsaved it may
//! not: the window is asked to close instead, which is the request the
//! close prompt already answers for the window's close button.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eframe::egui;
use mac_window::QuitAnswer;

use crate::app::MarkdownApp;

#[cfg(test)]
mod tests;

/// Whether a quit had to ask about unsaved edits when the last frame
/// ended, where a request to quit can read it.
#[derive(Debug, Default)]
pub(super) struct SystemQuit {
    unsaved: Arc<AtomicBool>,
}

impl SystemQuit {
    /// What to say to a request to quit that comes to the window of `ctx`.
    ///
    /// With edits unsaved the answer is no, and the window is brought to
    /// the front and asked to close, so that the prompt asks about them
    /// where the reader can see it.
    fn asker(&self, ctx: egui::Context) -> impl Fn() -> QuitAnswer + Send + Sync + 'static {
        let unsaved = Arc::clone(&self.unsaved);
        move || {
            if !unsaved.load(Ordering::Relaxed) {
                return QuitAnswer::Now;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            QuitAnswer::NotYet
        }
    }
}

impl MarkdownApp {
    /// The app's answer to the system when it asks to quit, for
    /// `mac_window::ask_before_quit`.
    pub(crate) fn quit_asker(&self) -> impl Fn() -> QuitAnswer + Send + Sync + 'static {
        self.system_quit.asker(self.egui_ctx.clone())
    }

    /// Leave word of whether a quit has to ask about unsaved edits, at the
    /// end of a frame: only a frame changes that. One comparison a tab,
    /// which is what the tab in front already costs for its unsaved marker.
    pub(crate) fn note_unsaved_edits(&self) {
        let unsaved = self.quit_has_to_ask();
        self.system_quit.unsaved.store(unsaved, Ordering::Relaxed);
    }
}
