//! Heading extraction for the document title and the outline sidebar.

use std::collections::HashSet;

#[cfg(test)]
mod tests;

/// Represents a markdown header for the outline
#[derive(Clone)]
pub(crate) struct Header {
    pub(crate) level: u8,
    pub(crate) title: String,
    /// Byte offset of the heading's Start event. The renderer records the
    /// position under the same source-stable key, independent of formatting.
    pub(crate) source_start: usize,
    pub(crate) line_number: usize,
}

/// Result of parsing markdown headers
pub(crate) struct ParsedHeaders {
    /// Document title (first h1, if any)
    pub(crate) document_title: Option<String>,
    /// Outline headers (excludes the first h1)
    pub(crate) outline_headers: Vec<Header>,
}

/// Parse headings with the same CommonMark rules used by the renderer.
pub(crate) fn parse_headers(content: &str) -> ParsedHeaders {
    use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};

    fn level_number(level: HeadingLevel) -> u8 {
        match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        }
    }

    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(offset, _)| offset + 1))
        .collect();
    let mut all_headers = Vec::new();
    let mut current: Option<(u8, usize, usize, String)> = None;

    for (event, range) in
        pulldown_cmark::Parser::new_ext(content, pulldown_cmark::Options::all()).into_offset_iter()
    {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let line_number = line_starts
                    .partition_point(|line_start| *line_start <= range.start)
                    .saturating_sub(1);
                current = Some((level_number(level), range.start, line_number, String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, source_start, line_number, title)) = current.take() {
                    let title = title.trim().to_owned();
                    if !title.is_empty() {
                        all_headers.push(Header {
                            level,
                            title,
                            source_start,
                            line_number,
                        });
                    }
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, _, title)) = current.as_mut() {
                    title.push_str(&text);
                }
            }
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                if let Some((_, _, _, title)) = current.as_mut() {
                    title.push_str(&text);
                }
            }
            Event::FootnoteReference(label) => {
                if let Some((_, _, _, title)) = current.as_mut() {
                    title.push('[');
                    title.push_str(&label);
                    title.push(']');
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, _, title)) = current.as_mut() {
                    title.push(' ');
                }
            }
            _ => {}
        }
    }

    let document_title = all_headers
        .iter()
        .find(|h| h.level == 1)
        .map(|h| h.title.clone());
    let outline_headers = all_headers;

    ParsedHeaders {
        document_title,
        outline_headers,
    }
}

/// Check if header at `index` should be hidden because an ancestor is collapsed
pub(crate) fn header_is_hidden(
    headers: &[Header],
    index: usize,
    collapsed: &HashSet<usize>,
) -> bool {
    if index == 0 || index >= headers.len() {
        return false;
    }
    let mut search_level = headers[index].level;
    // Walk backwards to find ancestors
    for i in (0..index).rev() {
        let h = &headers[i];
        // Only consider headers with lower level than what we're searching for
        if h.level < search_level {
            // Found an ancestor
            if collapsed.contains(&i) {
                return true;
            }
            // This ancestor is not collapsed, but check its ancestors too
            // Update search_level to only look for even lower level headers
            search_level = h.level;
        }
        // Headers at same or higher level are siblings/cousins, skip them
    }
    false
}

/// Check if a header has any children (headers with higher level immediately following)
pub(crate) fn header_has_children(headers: &[Header], index: usize) -> bool {
    if index >= headers.len() {
        return false;
    }
    let current_level = headers[index].level;
    // Look at the next header
    if let Some(next) = headers.get(index + 1) {
        // A child has a higher level number (e.g., h3 is child of h2)
        next.level > current_level
    } else {
        false
    }
}

/// Check if any header in the list has children
pub(crate) fn any_header_has_children(headers: &[Header]) -> bool {
    for i in 0..headers.len() {
        if header_has_children(headers, i) {
            return true;
        }
    }
    false
}
