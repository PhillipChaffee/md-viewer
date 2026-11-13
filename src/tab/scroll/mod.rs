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
}
