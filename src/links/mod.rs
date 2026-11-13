//! Local link classification, resolution, and fragment navigation.

use std::borrow::Cow;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use egui_commonmark_extended::header_position_key;

use crate::headers::Header;
use crate::layout::line_fraction;
use crate::tab::Tab;

#[cfg(test)]
mod tests;

/// Split a clicked destination into its file part and `#fragment`, if the
/// fragment is non-empty. A trailing `#` with nothing after it is a link with
/// no anchor, not a navigation request.
pub(crate) fn split_link_fragment(link: &str) -> (&str, Option<&str>) {
    match link.split_once('#') {
        Some((path, fragment)) if !fragment.is_empty() => (path, Some(fragment)),
        _ => (link, None),
    }
}

/// GitHub-style anchor slug for a heading title: lowercased, whitespace runs
/// become hyphens, and anything that is not a letter, number, hyphen or
/// underscore is dropped. This is the convention most markdown sites use for
/// `#fragment` anchors, so `#settings--config` matches a `Settings & Config`
/// heading.
fn slugify_heading(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '\u{0}'
            }
        })
        .filter(|c| *c != '\u{0}')
        .collect()
}

/// Where a `#fragment` landed inside a document.
enum FragmentTarget<'a> {
    /// The document title (the first `h1`, which the outline excludes) — the
    /// very top of the document.
    DocumentTop,
    Header(&'a Header),
    Missing,
}

/// Resolve a `#fragment` against a document's headings using slug semantics:
/// exact slug match, first heading wins. The document title is matched before
/// the outline headers because it is the page's primary anchor.
fn find_fragment_target<'a>(
    document_title: Option<&str>,
    headers: &'a [Header],
    fragment: &str,
) -> FragmentTarget<'a> {
    let wanted = slugify_heading(fragment);
    if wanted.is_empty() {
        return FragmentTarget::Missing;
    }
    if let Some(title) = document_title {
        if slugify_heading(title) == wanted {
            return FragmentTarget::DocumentTop;
        }
    }
    headers
        .iter()
        .find(|header| slugify_heading(&header.title) == wanted)
        .map_or(FragmentTarget::Missing, FragmentTarget::Header)
}

/// Point a tab at the heading a `#fragment` names, mirroring the outline-click
/// flow: a cached heading position scrolls immediately, and a fresh document
/// (no measured height, nothing cached) starts at the top and lets the
/// post-render corrective pass snap to the recorded position once the
/// bootstrap has painted. Returns whether the fragment named a heading.
pub(crate) fn apply_fragment_navigation(tab: &mut Tab, fragment: &str) -> bool {
    match find_fragment_target(
        tab.document_title.as_deref(),
        &tab.outline_headers,
        fragment,
    ) {
        FragmentTarget::DocumentTop => {
            tab.pending_scroll_offset = Some(0.0);
            true
        }
        FragmentTarget::Header(header) => {
            let key = header_position_key(header.source_start);
            if let Some(y_pos) = tab.cache.get_header_position(&key) {
                tab.pending_scroll_offset = Some((y_pos - 50.0).max(0.0));
            } else if tab.last_content_height > 0.0 && tab.content_lines > 0 {
                let estimated_y =
                    line_fraction(header.line_number, tab.content_lines) * tab.last_content_height;
                tab.pending_scroll_offset = Some((estimated_y - 50.0).max(0.0));
                // The estimate only gets us near the target; one full paint
                // records the exact position for the corrective pass.
                tab.pending_header_click_key = Some(key);
            } else {
                tab.pending_header_click_key = Some(key);
            }
            true
        }
        FragmentTarget::Missing => false,
    }
}

/// Handle a renderer link hook without coupling path resolution to the UI.
///
/// Plain click navigates the current tab, while Ctrl/Cmd+click returns a path
/// and `#fragment` for the caller to open in a new tab. Both modes surface
/// path-resolution errors consistently.
pub(crate) fn handle_local_link_click(
    tab: &mut Tab,
    clicked_link: &str,
    open_in_new_tab: bool,
) -> (Option<(PathBuf, Option<String>)>, Option<String>) {
    let result = if open_in_new_tab {
        let (destination, fragment) = split_link_fragment(clicked_link);
        tab.resolve_link(destination)
            .map(|path| path.map(|p| (p, fragment.map(str::to_owned))))
    } else {
        tab.navigate_to_link(clicked_link)
            .map(|_| None::<(PathBuf, Option<String>)>)
    };

    match result {
        Ok(target) => (target, None),
        Err(error) => (
            None,
            Some(format!("Unable to open {clicked_link}: {error}")),
        ),
    }
}

/// Parse explicit links plus bare local Markdown file references, skipping code blocks.
pub(crate) fn parse_local_links(content: &str, document_path: &Path) -> Vec<String> {
    let mut links = HashSet::new();
    let document_dir = document_path.parent().unwrap_or_else(|| Path::new("."));
    let mut in_code_block = false;

    // Let CommonMark decide which bytes are visible text. Event::Text excludes
    // fenced/indented code and link destinations, while Event::Code preserves
    // the viewer extension that makes an inline-code filename clickable.
    for event in pulldown_cmark::Parser::new_ext(content, pulldown_cmark::Options::all()) {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) => {
                let destination = dest_url.as_ref();
                if is_local_markdown_link(destination) || destination.starts_with('#') {
                    links.insert(destination.to_owned());
                }
            }
            pulldown_cmark::Event::Code(code) => {
                register_existing_markdown_path(&code, document_dir, &mut links);
            }
            pulldown_cmark::Event::Text(text) if !in_code_block => {
                register_bare_markdown_paths(&text, document_dir, &mut links);
            }
            _ => {}
        }
    }

    let mut links: Vec<_> = links.into_iter().collect();
    links.sort();
    links
}

fn register_bare_markdown_paths(text: &str, document_dir: &Path, links: &mut HashSet<String>) {
    for token in text.split_whitespace() {
        let destination = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '`' | '*'
                    | '_'
                    | '"'
                    | '\''
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | ','
                    | '.'
                    | ';'
                    | ':'
                    | '!'
                    | '?'
                    | '，'
                    | '。'
                    | '；'
                    | '：'
                    | '！'
                    | '？'
                    | '（'
                    | '）'
                    | '【'
                    | '】'
                    | '《'
                    | '》'
            )
        });
        register_existing_markdown_path(destination, document_dir, links);
    }
}

fn register_existing_markdown_path(
    destination: &str,
    document_dir: &Path,
    links: &mut HashSet<String>,
) {
    if !is_local_markdown_link(destination) {
        return;
    }
    // A bare mention stays existence-gated, and an undecodable one is simply not
    // registered: only an *explicit* Markdown link should be able to report a
    // broken target, which is the boundary #181 drew.
    if resolve_local_link_path(destination, document_dir)
        .ok()
        .flatten()
        .is_some_and(|path| path.is_file())
    {
        links.insert(destination.to_owned());
    }
}

/// One classification of a destination's shape, used by *both* the gate that
/// decides which destinations become clickable and the resolver that turns one
/// into a path.
///
/// They used to classify separately and disagreed on four shapes, which #188
/// pinned: a query string produced a path with `?v=2` in the file name while
/// the gate rejected it; an unencoded `#` produced an extensionless path; an
/// empty destination produced the directory itself; and a destination whose
/// percent escapes are not UTF-8 passed the gate but resolved to nothing, so
/// the link rendered as internal and a click did nothing at all — no
/// navigation and no error. Sharing one classification is what makes those
/// disagreements unrepresentable rather than merely fixed.
enum DestinationShape<'a> {
    /// A leading `#`: within the current document, no path involved.
    Fragment,
    /// Nothing this viewer opens from the filesystem — a non-`file` scheme, a
    /// `file://` URI with a non-local authority, a protocol-relative URL, or a
    /// destination with nothing left to name a file with.
    External,
    /// Names a local file. `raw` keeps the source spelling with fragment and
    /// query removed, because the extension test has to run on what the author
    /// wrote; `decoded` is `None` when the percent escapes are not UTF-8.
    Local {
        raw: &'a str,
        decoded: Option<Cow<'a, str>>,
    },
}

fn classify_destination(destination: &str) -> DestinationShape<'_> {
    if destination.starts_with('#') {
        return DestinationShape::Fragment;
    }
    // A protocol-relative URL carries no scheme, so the path branch below would
    // read `//host/path` as an absolute filesystem path — the reinterpretation
    // #141 point 8 forbids.
    if destination.starts_with("//") {
        return DestinationShape::External;
    }
    if let Some(scheme) = explicit_uri_scheme(destination) {
        if !scheme.eq_ignore_ascii_case("file") {
            return DestinationShape::External;
        }
        // `to_file_path` decodes the path and rejects a non-local authority.
        let Some(path) = url::Url::parse(destination)
            .ok()
            .and_then(|url| url.to_file_path().ok())
        else {
            return DestinationShape::External;
        };
        return DestinationShape::Local {
            raw: destination,
            decoded: path.into_os_string().into_string().ok().map(Cow::Owned),
        };
    }
    // Fragment first, then query: `guide.md?v=2#s` names `guide.md`. Stripping
    // the query is what stops `?v=2` becoming part of the file name.
    let without_fragment = destination.split('#').next().unwrap_or(destination);
    let raw = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    if raw.is_empty() {
        // Nothing left to name a file with. The resolver used to answer with the
        // containing directory here, which only stayed unreachable because
        // `Tab::resolve_link` guarded a leading `#` separately.
        return DestinationShape::External;
    }
    DestinationShape::Local {
        raw,
        decoded: percent_encoding::percent_decode_str(raw).decode_utf8().ok(),
    }
}

/// Whether `path` carries one of the extensions this viewer opens as a
/// document (`md`, `markdown`, `txt`, in any letter case).
pub(crate) fn has_markdown_extension(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        let ext = ext.to_string_lossy().to_lowercase();
        ext == "md" || ext == "markdown" || ext == "txt"
    })
}

/// Check if a link destination points to a local markdown file.
///
/// The extension test runs on the source spelling, not the decoded path: an
/// author writing `bad%FF.md` plainly means a Markdown file, and it has to pass
/// this gate for a click to be able to *report* that it cannot be opened.
fn is_local_markdown_link(destination: &str) -> bool {
    match classify_destination(destination) {
        DestinationShape::Local { raw, .. } => has_markdown_extension(Path::new(raw)),
        DestinationShape::Fragment | DestinationShape::External => false,
    }
}

/// Return an RFC 3986 URI scheme when the destination explicitly contains one.
///
/// Markdown link destinations are URIs, so a colon after a valid scheme prefix
/// is not a filesystem separator. On Windows, preserve drive-letter paths such
/// as `C:\docs\guide.md` instead of classifying `C` as a URI scheme.
fn explicit_uri_scheme(destination: &str) -> Option<&str> {
    let colon = destination.find(':')?;
    let scheme = &destination[..colon];
    let mut chars = scheme.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
    {
        return None;
    }

    #[cfg(target_os = "windows")]
    if scheme.len() == 1 {
        return None;
    }

    Some(scheme)
}

/// Resolve a relative/absolute filesystem path or a `file:` URI against the
/// directory containing the Markdown document.
/// Turn a destination into a path to try.
///
/// `Ok(None)` means there is nothing to open — a fragment, or a destination
/// this viewer treats as external. `Err` means the destination names a local
/// file that cannot be turned into a path at all, which callers must surface
/// rather than swallow: before #141 that case returned `None` like the others,
/// so an internal-looking link did nothing whatsoever when clicked.
pub(crate) fn resolve_local_link_path(
    destination: &str,
    document_dir: &Path,
) -> io::Result<Option<PathBuf>> {
    match classify_destination(destination) {
        DestinationShape::Fragment | DestinationShape::External => Ok(None),
        DestinationShape::Local { raw, decoded: None } => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{raw} is not valid UTF-8 after percent-decoding"),
        )),
        DestinationShape::Local {
            decoded: Some(decoded),
            ..
        } => {
            let path = Path::new(decoded.as_ref());
            Ok(Some(if path.is_absolute() {
                path.to_path_buf()
            } else {
                document_dir.join(path)
            }))
        }
    }
}
