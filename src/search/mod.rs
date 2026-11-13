//! Find-in-document state and match scanning.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// A single match in a tab's content, identified by byte range and 1-based line number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchMatch {
    pub(crate) byte_start: usize,
    pub(crate) byte_end: usize,
    pub(crate) line_number: usize,
}

/// Per-frame return from `render_search_bar`.
#[derive(Default)]
pub(crate) struct SearchBarOutcome {
    pub(crate) close_requested: bool,
    pub(crate) prev_clicked: bool,
    pub(crate) next_clicked: bool,
}

/// App-level search state — only one find bar is visible at a time.
#[derive(Default)]
pub(crate) struct SearchState {
    pub(crate) is_open: bool,
    pub(crate) query: String,
    /// The tab index and the query the tabs' matches were found for.
    /// `None` when they have to be found again whatever the two are.
    built_for: Option<(usize, String)>,
    /// Set after Ctrl+F so the text input is focused next frame.
    pub(crate) focus_requested: bool,
    /// Index into the active tab's `search_matches`.
    pub(crate) active_match_index: usize,
}

impl SearchState {
    /// Forget what the matches were found for, so that the next look finds
    /// them again. For when the tab in a place holds other text than it
    /// did: another document, a reload, an edit.
    pub(crate) fn invalidate(&mut self) {
        self.built_for = None;
    }

    /// Whether the matches are those of the tab at `tab` for the query as
    /// it stands.
    pub(crate) fn is_built_for(&self, tab: usize) -> bool {
        let built_for = self.built_for.as_ref();
        built_for.is_some_and(|(built_tab, query)| *built_tab == tab && *query == self.query)
    }

    /// Record that the matches are now those of the tab at `tab` for the
    /// query as it stands.
    pub(crate) fn mark_built_for(&mut self, tab: usize) {
        self.built_for = Some((tab, self.query.clone()));
    }
}

/// Find all occurrences of `query` in `content`, case-insensitive (ASCII only).
///
/// Byte offsets are 1:1 with `content` because `to_ascii_lowercase` does not change
/// byte length. Non-ASCII letters match literally (`É` does not match `é`).
///
/// Matches spanning a newline are excluded (a search bar should not jump to results
/// the user cannot interpret as a single line).
///
/// Matches inside markdown that's not visibly rendered are also excluded:
/// - Image alt-text `![alt](url)` — alt is hover/screen-reader only
/// - Image URL `![alt](url)` — never visible
/// - Link URL `[text](url)` — never visible (only the text part is)
///
/// Without this, cycling lands on invisible matches: the user sees no visible
/// highlight change and the scroll target points to a paragraph where the rendered
/// text doesn't contain the searched bytes.
pub(crate) fn find_matches(content: &str, query: &str) -> Vec<SearchMatch> {
    if query.is_empty() || content.is_empty() {
        return Vec::new();
    }

    let content_lc = content.to_ascii_lowercase();
    let query_lc = query.to_ascii_lowercase();
    let query_len = query_lc.len();
    // A pattern that failed to compile hides nothing: every match is offered.
    let hidden_spans = markdown_link_pattern()
        .map_or_else(Vec::new, |pattern| non_rendered_spans(pattern, content));

    let mut matches = Vec::new();
    let mut line_number = 1usize;
    let mut cursor = 0usize;

    for (byte_start, _) in content_lc.match_indices(&query_lc) {
        // Count newlines between cursor and this match's start
        line_number += content[cursor..byte_start]
            .bytes()
            .filter(|&b| b == b'\n')
            .count();
        cursor = byte_start;

        let byte_end = byte_start + query_len;
        let crosses_line_boundary = content[byte_start..byte_end].contains('\n');
        let hidden = hidden_spans
            .iter()
            .any(|span| byte_start >= span.start && byte_end <= span.end);
        if crosses_line_boundary || hidden {
            continue;
        }

        matches.push(SearchMatch {
            byte_start,
            byte_end,
            line_number,
        });
    }

    matches
}

/// `(!?)[alt-or-text](url)`: group 1 is `!` for an image and empty for a link,
/// group 2 the alt text or link text, group 3 the URL.
const MARKDOWN_LINK_PATTERN: &str = r"(!?)\[([^\]]*)\]\(([^)]*)\)";

/// The compiled `MARKDOWN_LINK_PATTERN`. `None` only if the pattern itself is
/// rejected, which is logged once.
fn markdown_link_pattern() -> Option<&'static Regex> {
    static PATTERN: LazyLock<Option<Regex>> =
        LazyLock::new(|| compile_pattern(MARKDOWN_LINK_PATTERN));
    PATTERN.as_ref()
}

/// Compile `pattern`, logging why when it is rejected.
fn compile_pattern(pattern: &str) -> Option<Regex> {
    Regex::new(pattern)
        .inspect_err(|error| log::error!("Markdown link pattern does not compile: {error}"))
        .ok()
}

/// Byte ranges of `content` that are markdown source but never shown as text:
/// every link or image URL, and the alt text of images, as `pattern` (the
/// compiled `MARKDOWN_LINK_PATTERN`) finds them.
fn non_rendered_spans(pattern: &Regex, content: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    for capture in pattern.captures_iter(content) {
        let is_image = capture.get(1).is_some_and(|bang| !bang.as_str().is_empty());
        let alt = capture.get(2).filter(|_| is_image);
        let url = capture.get(3);
        spans.extend(url.into_iter().chain(alt).map(|part| part.range()));
    }
    spans
}

/// The match index one step from `current` among `count` matches: forward
/// for a `direction` of zero or more and backward below zero, wrapping at
/// either end. `count` is at least one.
pub(crate) const fn stepped_match_index(current: usize, count: usize, direction: i32) -> usize {
    if direction >= 0 {
        (current + 1) % count
    } else if current == 0 {
        count - 1
    } else {
        current - 1
    }
}
