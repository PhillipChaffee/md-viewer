//! The one answer to "is the page taking input right now?".
//!
//! Two things the app shows are modal: while either is up, nothing under
//! it is clicked, typed into or scrolled, and none of the app's own
//! shortcuts run but the few that are its own. The keyboard, the wheel
//! that zooms and files dropped on the window all ask here, so an overlay
//! that takes one of them takes all of them.
//!
//! The Colors and Fonts dialogs are not modal. They float over a page
//! that goes on taking clicks, keys and drops.

use crate::app::MarkdownApp;

#[cfg(test)]
mod tests;

/// What stands over the page and takes its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Modal {
    /// The prompt about unsaved edits, until it is answered.
    Prompt,
    /// A picture enlarged in the lightbox, until it is closed.
    Lightbox,
}

impl MarkdownApp {
    /// The overlay that has the window's input, if one does. The prompt
    /// comes first: it draws over everything and closes the lightbox when
    /// it starts to ask.
    pub(crate) const fn modal(&self) -> Option<Modal> {
        if self.closing.is_asking() {
            Some(Modal::Prompt)
        } else if self.lightbox.open.is_some() {
            Some(Modal::Lightbox)
        } else {
            None
        }
    }
}
