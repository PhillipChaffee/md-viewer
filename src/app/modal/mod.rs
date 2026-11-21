//! Which overlay has the window's key bindings, wheel zoom and dropped
//! files.
//!
//! Two things the app shows are modal: the prompt about unsaved edits and
//! a picture enlarged in the lightbox. While either is up, the shortcuts,
//! the wheel or pinch that zooms the document and files dropped on the
//! window all ask here, so those three cannot disagree.
//!
//! Clicks, typing and the wheel that scrolls do not ask here. Each
//! overlay keeps those from the page itself: the prompt through egui's
//! modal layer, the lightbox through a sink that covers the window and by
//! taking the wheel before egui reads it. A third overlay has to do the
//! same for itself.
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
    /// comes first: it closes the lightbox when it starts to ask, so the
    /// two are not up together.
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
