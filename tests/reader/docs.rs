//! Documents the tests read, and launches several of them share.

use std::fmt::Write as _;

use super::support::Reader;

/// One of everything the renderer draws.
pub(crate) const KITCHEN_SINK: &str = include_str!("../fixtures/kitchen-sink.md");

/// A document far taller than the window: `sections` headings named
/// `Section 1` and up, each over a paragraph that names it.
pub(crate) fn long_document(sections: usize) -> String {
    let mut text = String::from("# Long read\n\n");
    for number in 1..=sections {
        let _ = write!(
            text,
            "## Section {number}\n\nParagraph {number} mentions the lighthouse once.\n\n"
        );
    }
    text
}

/// Three short documents that link to each other, opened on `a.md`.
pub(crate) fn three_linked() -> Reader {
    Reader::with_docs(&[
        (
            "a.md",
            "# Alpha\n\nGo [to beta](b.md) or [to gamma](c.md) or [nowhere](missing.md).",
        ),
        (
            "b.md",
            "# Beta\n\nGo [to alpha](a.md) or [on to gamma](c.md).",
        ),
        ("c.md", "# Gamma\n\nThe end of the line."),
    ])
}

/// The kitchen sink, with the guide it links to beside it.
pub(crate) fn kitchen_sink() -> Reader {
    Reader::with_docs(&[
        ("sink.md", KITCHEN_SINK),
        ("guide.md", include_str!("../fixtures/guide.md")),
    ])
}
