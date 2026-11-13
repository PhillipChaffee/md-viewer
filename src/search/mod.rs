//! Find-in-document state and match scanning.

use regex::Regex;

#[cfg(test)]
mod tests;

/// A single match in a tab's content, identified by byte range and 1-based line number
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchMatch {
    pub(crate) byte_start: usize,
    pub(crate) byte_end: usize,
    pub(crate) line_number: usize,
}

/// Per-frame return from `render_search_bar`
#[derive(Default)]
pub(crate) struct SearchBarOutcome {
    pub(crate) close_requested: bool,
    pub(crate) prev_clicked: bool,
    pub(crate) next_clicked: bool,
}

/// App-level search state — only one find bar is visible at a time
#[derive(Default)]
pub(crate) struct SearchState {
    pub(crate) is_open: bool,
    pub(crate) query: String,
    /// Shadow copy of `query` used to detect changes across frames
    pub(crate) last_query: String,
    /// Tab index the cached matches were built for; `None` forces rebuild
    pub(crate) last_tab: Option<usize>,
    /// Set after Ctrl+F so the text input is focused next frame
    pub(crate) focus_requested: bool,
    /// Index into the active tab's `search_matches`
    pub(crate) active_match_index: usize,
}

/// Find all occurrences of `query` in `content`, case-insensitive (ASCII only).
///
/// Byte offsets are 1:1 with `content` because `to_ascii_lowercase` does not change
/// byte length. Non-ASCII letters match literally (`É` does not match `é`) — see the
/// devlog future-improvements list for the case-toggle and full Unicode case folding.
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

    // Identify byte ranges of non-renderable markdown parts so we can skip matches
    // inside them. Pattern: `(!?)[alt-or-text](url)`.
    // - Group 1 = `!` for images, empty for links
    // - Group 2 = alt (image) or text (link); exclude only if image (group 1 = `!`)
    // - Group 3 = url; always exclude (URLs are never visible inline)
    let skip_spans: Vec<std::ops::Range<usize>> = {
        static MD_LINK_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let re = MD_LINK_RE.get_or_init(|| {
            Regex::new(r"(!?)\[([^\]]*)\]\(([^)]*)\)").expect("static regex must compile")
        });
        let mut spans = Vec::new();
        for cap in re.captures_iter(content) {
            let is_image = cap.get(1).map(|m| !m.as_str().is_empty()).unwrap_or(false);
            // Always exclude URL (group 3)
            if let Some(url) = cap.get(3) {
                spans.push(url.start()..url.end());
            }
            // For images, also exclude alt (group 2)
            if is_image {
                if let Some(alt) = cap.get(2) {
                    spans.push(alt.start()..alt.end());
                }
            }
        }
        spans
    };

    let in_skip = |start: usize, end: usize| -> bool {
        skip_spans.iter().any(|s| start >= s.start && end <= s.end)
    };

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
        if content[byte_start..byte_end].contains('\n') {
            continue; // Skip matches that cross line boundaries
        }
        if in_skip(byte_start, byte_end) {
            continue; // Skip matches inside image alt-text or URL portions of links/images
        }

        matches.push(SearchMatch {
            byte_start,
            byte_end,
            line_number,
        });
    }

    matches
}

pub(crate) fn take_search_correction_request(pending: &mut bool) -> bool {
    std::mem::take(pending)
}
