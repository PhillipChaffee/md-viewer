//! Which heading the reader is in, for the outline to mark.

use eframe::egui;
use egui_commonmark_extended::header_position_key;

use super::HEADER_SCROLL_MARGIN;
use crate::layout::line_fraction;
use crate::tab::Tab;

#[cfg(test)]
mod tests;

/// How far below the top of the pane the reading line sits. A heading at or
/// above the line has been reached. A click in the outline puts its heading
/// [`HEADER_SCROLL_MARGIN`] below the top, so the line is a little lower
/// than that and the clicked heading counts as reached.
const READING_LINE: f32 = HEADER_SCROLL_MARGIN + 12.0;
/// How close to the end of the document the view counts as at the end.
const END_SLACK: f32 = 2.0;

/// The part of a document that is on screen, in px from the document's top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct View {
    pub(super) top: f32,
    pub(super) height: f32,
    pub(super) content_height: f32,
}

impl View {
    /// The reader has scrolled, and as far as the document goes.
    fn is_at_the_end(self) -> bool {
        self.top > 0.0 && self.top + self.height >= self.content_height - END_SLACK
    }
}

/// The heading the reader is in: the last of `count` headings that the
/// reading line has reached, where `position` gives a heading's distance
/// from the top of the document and grows with its index.
///
/// Above the first heading the reader is in none, unless that heading is on
/// screen: at the top of a document its title is what is being read. At
/// the end of the document the reader is in the last heading, which may sit
/// too low ever to reach the line.
pub(super) fn heading_in_view(
    count: usize,
    position: impl Fn(usize) -> f32,
    view: View,
) -> Option<usize> {
    let last = count.checked_sub(1)?;
    if view.content_height <= 0.0 {
        // Nothing is laid out yet, so no position means anything.
        return None;
    }
    if view.is_at_the_end() {
        return Some(last);
    }
    let reading_line = view.top + READING_LINE;
    let (mut reached, mut end) = (0, count);
    while reached < end {
        let middle = reached + (end - reached) / 2;
        if position(middle) <= reading_line {
            reached = middle + 1;
        } else {
            end = middle;
        }
    }
    reached
        .checked_sub(1)
        .or_else(|| (position(0) < view.top + view.height).then_some(0))
}

/// What the current heading was last worked out from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sighting {
    tab: egui::Id,
    content_version: u64,
    view: View,
}

/// The heading the reader is in, remembered so that it is worked out again
/// only when the document or the view of it has changed.
#[derive(Debug, Default)]
pub(crate) struct HeadingSpy {
    seen: Option<Sighting>,
    current: Option<usize>,
}

impl HeadingSpy {
    /// The index of the heading of `tab` the reader is in, and whether that
    /// is another heading than at the last look.
    pub(super) fn look(&mut self, tab: &Tab) -> (Option<usize>, bool) {
        let sighting = Sighting {
            tab: tab.id,
            content_version: tab.content_version,
            view: View {
                top: tab.scroll_offset,
                height: tab.last_viewport_height,
                content_height: tab.last_content_height,
            },
        };
        if self.seen == Some(sighting) {
            return (self.current, false);
        }
        self.seen = Some(sighting);

        let headers = &tab.outline_headers;
        // Where the renderer painted the heading, or until it has, where
        // the heading's line number suggests it is.
        let position = |index: usize| {
            let header = &headers[index];
            let measured = tab
                .cache
                .get_header_position(&header_position_key(header.source_start));
            measured.unwrap_or_else(|| {
                line_fraction(header.line_number, tab.content_lines) * tab.last_content_height
            })
        };
        let current = heading_in_view(headers.len(), position, sighting.view);
        let moved = current != self.current;
        self.current = current;
        (current, moved)
    }
}
