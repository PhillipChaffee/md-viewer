//! Where a tab's document scrolls to.

use eframe::egui::scroll_area::ScrollAreaOutput;
use egui_commonmark_extended::header_position_key;

use super::Tab;
use crate::headers::Header;
use crate::layout::{keyboard_scroll_target, line_fraction, KeyboardScrollAction};

#[cfg(test)]
mod tests;

/// Space left above a heading the document scrolls to.
pub(crate) const HEADER_SCROLL_MARGIN: f32 = 50.0;
/// How far below the top of the viewport a search match is put, as a share
/// of the viewport's height.
pub(crate) const MATCH_INSET: f32 = 0.35;
/// Space left above a search match while the height of the viewport is not
/// known.
const MATCH_MARGIN_UNMEASURED: f32 = 100.0;

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
    /// Record where the pane's scroll area was after it showed the
    /// document, and how tall the area and the document were.
    pub(crate) fn measured<R>(&mut self, shown: &ScrollAreaOutput<R>) {
        self.scroll_offset = shown.state.offset.y;
        self.last_viewport_height = shown.inner_rect.height();
        self.last_content_height = shown.content_size.y;
    }

    /// Scroll the document to `offset` the next time the pane shows it.
    pub(crate) const fn scroll_to(&mut self, offset: f32) {
        self.pending_scroll_offset = Some(offset);
    }

    /// The offset the pane is to scroll to, asked for since the pane last
    /// took one.
    pub(crate) const fn take_pending_scroll(&mut self) -> Option<f32> {
        self.pending_scroll_offset.take()
    }

    /// Scroll one keyboard `step` from where the document is.
    pub(crate) fn scroll_by_key(&mut self, step: KeyboardScrollAction) {
        self.scroll_to(keyboard_scroll_target(
            self.scroll_offset,
            self.last_viewport_height,
            self.last_content_height,
            step,
        ));
    }

    /// Schedule the scroll that shows the search match at `index`. It does
    /// nothing without such a match, or before the document is laid out.
    pub(crate) fn jump_to_match(&mut self, index: usize) {
        let Some(found) = self.search_matches.get(index) else {
            return;
        };
        if self.last_content_height <= 0.0 || self.content_lines == 0 {
            return;
        }
        // The line-ratio estimate gets the view roughly to the right area.
        // The renderer records the actual y of the active match during
        // paint, and the pane corrects the scroll next frame if this
        // estimate was off.
        let estimated_y =
            line_fraction(found.line_number, self.content_lines) * self.last_content_height;
        let margin = if self.last_viewport_height > 0.0 {
            self.last_viewport_height * MATCH_INSET
        } else {
            MATCH_MARGIN_UNMEASURED
        };
        self.scroll_to((estimated_y - margin).max(0.0));
        // Grant the next render one full-paint attempt to record an exact Y.
        // The request is consumed before that render even when the source
        // match has no visible glyph, so later frames return to clipping.
        self.corrections.search = true;
    }

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
        self.scroll_to((at.y - HEADER_SCROLL_MARGIN).max(0.0));
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
