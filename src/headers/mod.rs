//! Heading extraction for the document title and the outline sidebar.

use std::collections::HashSet;

use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// Represents a markdown header for the outline.
#[derive(Clone)]
pub(crate) struct Header {
    pub(crate) level: u8,
    pub(crate) title: String,
    /// Byte offset of the heading's Start event. The renderer records the
    /// position under the same source-stable key, independent of formatting.
    pub(crate) source_start: usize,
    pub(crate) line_number: usize,
}

/// Result of parsing markdown headers.
pub(crate) struct ParsedHeaders {
    /// Document title (first h1, if any).
    pub(crate) document_title: Option<String>,
    /// Every heading of the document, the title's h1 included.
    pub(crate) outline_headers: Vec<Header>,
}

const fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// A heading whose Start event has been seen and whose End has not, collecting
/// the plain text of the inline events in between.
struct OpenHeading {
    level: u8,
    source_start: usize,
    line_number: usize,
    title: String,
}

impl OpenHeading {
    /// Append the text an inline event contributes to the heading's title.
    fn push_inline(&mut self, event: &Event<'_>) {
        match event {
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text) => self.title.push_str(text),
            Event::FootnoteReference(label) => {
                self.title.push('[');
                self.title.push_str(label);
                self.title.push(']');
            }
            Event::SoftBreak | Event::HardBreak => self.title.push(' '),
            _ => {}
        }
    }

    /// The finished header, or `None` for a heading with no visible text.
    fn close(self) -> Option<Header> {
        let title = self.title.trim();
        (!title.is_empty()).then(|| Header {
            level: self.level,
            title: title.to_owned(),
            source_start: self.source_start,
            line_number: self.line_number,
        })
    }
}

/// Parse headings with the same CommonMark rules used by the renderer.
pub(crate) fn parse_headers(content: &str) -> ParsedHeaders {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(offset, _)| offset + 1))
        .collect();
    let mut outline_headers = Vec::new();
    let mut current: Option<OpenHeading> = None;

    for (event, range) in
        pulldown_cmark::Parser::new_ext(content, pulldown_cmark::Options::all()).into_offset_iter()
    {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let line_number = line_starts
                    .partition_point(|line_start| *line_start <= range.start)
                    .saturating_sub(1);
                current = Some(OpenHeading {
                    level: level_number(level),
                    source_start: range.start,
                    line_number,
                    title: String::new(),
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                outline_headers.extend(current.take().and_then(OpenHeading::close));
            }
            inline => {
                if let Some(heading) = current.as_mut() {
                    heading.push_inline(&inline);
                }
            }
        }
    }

    let document_title = outline_headers
        .iter()
        .find(|h| h.level == 1)
        .map(|h| h.title.clone());

    ParsedHeaders {
        document_title,
        outline_headers,
    }
}

/// The headers that no collapsed ancestor hides, in document order.
pub(crate) fn visible_headers(headers: &[Header], collapsed: &HashSet<usize>) -> Vec<usize> {
    visible_rows(headers.iter().map(|header| header.level), |index| {
        collapsed.contains(&index)
    })
    .collect()
}

/// The headings no collapsed ancestor hides, in document order, found in
/// one pass over the `levels` of the headings. A collapsed heading hides
/// every heading after it that is deeper than it, up to the next heading
/// that is not; `is_collapsed` is asked about visible headings only.
///
/// The rows come one at a time and not as a list, so that the proof of
/// this pass runs it without a heap.
fn visible_rows(
    levels: impl IntoIterator<Item = u8>,
    is_collapsed: impl Fn(usize) -> bool,
) -> impl Iterator<Item = usize> {
    // The level of the collapsed heading whose subtree is being skipped.
    let mut hidden_below: Option<u8> = None;
    levels
        .into_iter()
        .enumerate()
        .filter_map(move |(index, level)| {
            if hidden_below.is_some_and(|fold| level > fold) {
                return None;
            }
            hidden_below = is_collapsed(index).then_some(level);
            Some(index)
        })
}

/// Check if a header has any children (headers with higher level immediately following).
pub(crate) fn header_has_children(headers: &[Header], index: usize) -> bool {
    let Some(current) = headers.get(index) else {
        return false;
    };
    // A child has a higher level number (e.g., h3 is child of h2)
    headers
        .get(index + 1)
        .is_some_and(|next| next.level > current.level)
}

/// Check if any header in the list has children.
pub(crate) fn any_header_has_children(headers: &[Header]) -> bool {
    for i in 0..headers.len() {
        if header_has_children(headers, i) {
            return true;
        }
    }
    false
}
