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

/// A document's headings as the outline sidebar shows them: which are
/// folded, and what follows from that. The rows are worked out when a
/// fold or the document changes, never per frame.
#[derive(Default)]
pub(crate) struct Outline {
    headers: Vec<Header>,
    /// The headings that are collapsed, by index.
    collapsed: HashSet<usize>,
    /// The headings no collapsed ancestor hides, in document order.
    visible: Vec<usize>,
    /// Some heading has children, so the outline has a fold column.
    has_folds: bool,
}

impl Outline {
    pub(crate) fn headers(&self) -> &[Header] {
        &self.headers
    }

    /// The headings no collapsed ancestor hides, in document order.
    pub(crate) fn visible(&self) -> &[usize] {
        &self.visible
    }

    /// Which of the visible rows stands for the heading at `index`: the
    /// heading itself, or the folded heading that hides it.
    pub(crate) fn row_standing_for(&self, index: usize) -> Option<usize> {
        self.visible
            .partition_point(|&visible| visible <= index)
            .checked_sub(1)
    }

    /// Whether some heading has children, so that there is anything to fold.
    pub(crate) const fn has_folds(&self) -> bool {
        self.has_folds
    }

    pub(crate) fn has_children(&self, index: usize) -> bool {
        header_has_children(&self.headers, index)
    }

    pub(crate) fn is_collapsed(&self, index: usize) -> bool {
        self.collapsed.contains(&index)
    }

    /// Fold the heading at `index`, or unfold it if it is folded.
    pub(crate) fn toggle(&mut self, index: usize) {
        if !self.collapsed.remove(&index) {
            self.collapsed.insert(index);
        }
        self.recompute();
    }

    pub(crate) fn expand_all(&mut self) {
        self.collapsed.clear();
        self.recompute();
    }

    /// Fold every heading that has children.
    pub(crate) fn collapse_all(&mut self) {
        let headers = &self.headers;
        self.collapsed
            .extend((0..headers.len()).filter(|&index| header_has_children(headers, index)));
        self.recompute();
    }

    /// Show `headers` in place of the ones held. The folds go unless
    /// `keep_folds` asks for them and there are as many headings as
    /// before: they are kept by position, which a heading added or
    /// removed shifts.
    pub(crate) fn replace_headers(&mut self, headers: Vec<Header>, keep_folds: bool) {
        if !keep_folds || headers.len() != self.headers.len() {
            self.collapsed.clear();
        }
        self.headers = headers;
        self.has_folds = any_header_has_children(&self.headers);
        self.recompute();
    }

    fn recompute(&mut self) {
        let Self {
            headers,
            collapsed,
            visible,
            ..
        } = self;
        visible.clear();
        visible.extend(visible_rows(
            headers.iter().map(|header| header.level),
            |index| collapsed.contains(&index),
        ));
    }
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
fn header_has_children(headers: &[Header], index: usize) -> bool {
    let Some(current) = headers.get(index) else {
        return false;
    };
    // A child has a higher level number (e.g., h3 is child of h2)
    headers
        .get(index + 1)
        .is_some_and(|next| next.level > current.level)
}

/// Check if any header in the list has children.
fn any_header_has_children(headers: &[Header]) -> bool {
    for i in 0..headers.len() {
        if header_has_children(headers, i) {
            return true;
        }
    }
    false
}
