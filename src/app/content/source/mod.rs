//! The text field of edit mode.
//!
//! It is egui's `TextEdit`, with one difference: characters drawn as one,
//! as an emoji written as several is, are one character to Backspace and
//! Delete. What is drawn as one is asked of the laid-out text, which is
//! also what the arrow keys and a click go by, so the keys and the picture
//! cannot disagree.

use std::any::TypeId;
use std::cell::RefCell;
use std::ops::Range;
use std::sync::Arc;

use eframe::egui;
use egui::text::{CCursor, LayoutJob};
use egui::{Galley, TextBuffer};

#[cfg(test)]
mod tests;

/// The source as `TextEdit` edits it: a `String`, except that the character
/// before or after the cursor is all the characters drawn as one with it.
struct Source<'text> {
    text: &'text mut String,
    /// The text as it is laid out now. `TextEdit` lays it out before it
    /// reads the keyboard and again after each edit.
    laid_out: &'text RefCell<Option<Arc<Galley>>>,
}

impl Source<'_> {
    /// The characters drawn as one with the character at `index`.
    fn drawn_as_one(&self, index: usize) -> Range<usize> {
        let laid_out = self.laid_out.borrow();
        laid_out
            .as_ref()
            .map_or(index..index + 1, |galley| galley.cluster_at(index))
    }

    fn delete(&mut self, chars: Range<usize>) -> CCursor {
        self.delete_selected_ccursor_range([CCursor::new(chars.start), CCursor::new(chars.end)])
    }
}

impl TextBuffer for Source<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.text
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        self.text.insert_text(text, char_index)
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        self.text.delete_char_range(char_range);
    }

    fn replace_with(&mut self, text: &str) {
        self.text.replace_with(text);
    }

    fn delete_previous_char(&mut self, ccursor: CCursor) -> CCursor {
        let before = ccursor.index.checked_sub(1);
        before.map_or(ccursor, |before| self.delete(self.drawn_as_one(before)))
    }

    fn delete_next_char(&mut self, ccursor: CCursor) -> CCursor {
        self.delete(self.drawn_as_one(ccursor.index))
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Source<'static>>()
    }
}

/// A multiline text field on `text`, set in `font` and `width` wide. Returns
/// whether the text was edited this frame.
pub(super) fn edit(ui: &mut egui::Ui, text: &mut String, font: &egui::FontId, width: f32) -> bool {
    let laid_out = RefCell::new(None);
    // As `TextEdit` lays text out itself, in the color it paints with.
    let mut lay_out = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let text = text.as_str().to_owned();
        let job = LayoutJob::simple(text, font.clone(), egui::Color32::PLACEHOLDER, wrap_width);
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        laid_out.replace(Some(Arc::clone(&galley)));
        galley
    };
    let mut source = Source {
        text,
        laid_out: &laid_out,
    };
    egui::TextEdit::multiline(&mut source)
        .id_salt(ui.id().with("edit"))
        .font(font.clone())
        .desired_width(width)
        .desired_rows(24)
        .frame(false)
        .layouter(&mut lay_out)
        .show(ui)
        .response
        .changed()
}
