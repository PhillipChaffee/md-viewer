//! Whether the outline shows when the reader has not said: it does while
//! the window has room for it beside the text.

use crate::layout::{ReadingWidth, OUTLINE_DEFAULT_WIDTH, READING_COLUMN_MARGIN};

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// Who decides whether the outline shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutlineRule {
    /// The reader toggled it, and it stays as they left it.
    Reader,
    /// Nobody has chosen. The outline shows while the window has room for
    /// it, and `had_room` is what the last frame found, so that the outline
    /// changes when the room does and not on every frame.
    Room { had_room: bool },
}

impl OutlineRule {
    /// The rule a session starts under: a session that saved whether the
    /// outline shows keeps it that way. Any other starts with the outline
    /// shown, as a window with room has it, until the first frame knows
    /// how wide the window is.
    pub(crate) const fn restored(saved: Option<bool>) -> Self {
        match saved {
            Some(_) => Self::Reader,
            None => Self::Room { had_room: true },
        }
    }

    /// What a session saves: whether the outline shows, where that was the
    /// reader's choice.
    pub(crate) const fn saved(self, shown: bool) -> Option<bool> {
        match self {
            Self::Reader => Some(shown),
            Self::Room { .. } => None,
        }
    }

    /// Take in whether the window has room this frame. Returns whether the
    /// outline should now show, when that has changed.
    pub(super) fn follow(&mut self, has_room: bool) -> Option<bool> {
        let Self::Room { had_room } = self else {
            return None;
        };
        let changed = *had_room != has_room;
        *had_room = has_room;
        changed.then_some(has_room)
    }
}

/// Whether a window `window_width` px wide holds the outline without
/// squeezing the text: beside the explorer, where that is open, and the
/// outline at its default width, the document pane still fits the column
/// the reader set and its margins. A column set to fit the window is
/// measured at the default width, since it has no width of its own.
pub(super) fn has_room(
    window_width: f32,
    explorer_width: Option<f32>,
    reading_width: ReadingWidth,
) -> bool {
    let column = reading_width.fixed().get();
    let pane = window_width - explorer_width.unwrap_or(0.0) - OUTLINE_DEFAULT_WIDTH;
    pane >= 2.0_f32.mul_add(READING_COLUMN_MARGIN, column)
}
