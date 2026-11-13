//! Where a tab's document scrolls to.

use egui_commonmark_extended::header_position_key;

use super::Tab;
use crate::headers::Header;
use crate::layout::line_fraction;

#[cfg(test)]
mod tests;

/// Space left above a heading the document scrolls to.
pub(crate) const HEADER_SCROLL_MARGIN: f32 = 50.0;
/// How far below the top of the viewport a search match is put, as a share
/// of the viewport's height.
pub(crate) const MATCH_INSET: f32 = 0.35;

/// One-shot requests to re-aim a scroll that was made from an estimate.
/// The pane takes them before it renders, so each is acted on once.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ScrollCorrections {
    /// The find bar jumped to a match. Asked for once per jump: a
    /// correction that fired on every frame the match is off screen would
    /// fight the wheel and lock the view to the match (issue #19).
    pub(crate) search: bool,
    /// Position key (see `header_position_key`) of a heading the document
    /// jumped to before the renderer had measured where the heading is.
    pub(crate) header_key: Option<String>,
}

/// How far a heading is from the top of its document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HeaderY {
    pub(crate) y: f32,
    /// The renderer has not painted the heading yet, so `y` is worked out
    /// from line counts.
    pub(crate) is_estimate: bool,
}

impl Tab {
    /// Where the renderer painted `header`, or until it has, where the
    /// heading's line number suggests it is. Before the document is laid
    /// out that is the top.
    pub(crate) fn header_y(&self, header: &Header) -> HeaderY {
        let measured = self
            .cache
            .get_header_position(&header_position_key(header.source_start));
        let estimate =
            || line_fraction(header.line_number, self.content_lines) * self.last_content_height;
        HeaderY {
            y: measured.unwrap_or_else(estimate),
            is_estimate: measured.is_none(),
        }
    }

    /// Schedule the scroll that puts the heading at `index` of the outline
    /// just below the top of the pane.
    pub(crate) fn jump_to_header(&mut self, index: usize) {
        let header = &self.outline.headers()[index];
        let (at, source_start) = (self.header_y(header), header.source_start);
        self.pending_scroll_offset = Some((at.y - HEADER_SCROLL_MARGIN).max(0.0));
        if at.is_estimate {
            // The estimate only gets near the target. Ask for one complete
            // paint, so that the corrective pass can measure the heading.
            // The renderer records the same source-stable key, so formatting
            // and duplicate display titles cannot redirect the jump.
            self.corrections.header_key = Some(header_position_key(source_start));
        }
    }

    /// The corrections asked for since the last call. They are taken even
    /// when nothing can act on them: the editor shows no headings, and a
    /// match in Markdown syntax has no glyph whose position is recorded.
    pub(crate) fn take_corrections(&mut self) -> ScrollCorrections {
        std::mem::take(&mut self.corrections)
    }
}
