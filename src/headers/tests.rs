use super::*;

use egui_commonmark_extended::header_position_key;

#[test]
fn heading_parser_keeps_raw_shortcode_and_source_identity() {
    let markdown = "# Doc\n\n## Pin :pushpin:\n\n## Pin :pushpin:\n";
    let parsed = parse_headers(markdown);

    assert_eq!(parsed.outline_headers.len(), 3);
    assert_eq!(parsed.outline_headers[1].title, "Pin :pushpin:");
    assert_eq!(
        parsed.outline_headers[1].source_start,
        markdown.find("## Pin").unwrap()
    );
    assert_eq!(
        parsed.outline_headers[2].source_start,
        markdown.rfind("## Pin").unwrap()
    );
    assert_eq!(
        header_position_key(parsed.outline_headers[1].source_start),
        "heading-source:7"
    );
}

#[test]
fn unknown_shortcode_heading_stays_raw() {
    let parsed = parse_headers("# Doc\n\n## Pin :not_a_gemoji:\n");
    assert_eq!(parsed.outline_headers[1].title, "Pin :not_a_gemoji:");
}

#[test]
fn heading_parser_uses_commonmark_for_formatting_links_and_setext() {
    let markdown = concat!(
        "# **Bold** and `code` [link](guide.md)\n\n",
        "Setext title\n---\n\n",
        "~~~markdown\n# Hidden heading\n~~~\n",
    );
    let parsed = parse_headers(markdown);

    assert_eq!(parsed.outline_headers.len(), 2);
    assert_eq!(parsed.outline_headers[0].title, "Bold and code link");
    assert_eq!(parsed.outline_headers[0].level, 1);
    assert_eq!(parsed.outline_headers[1].title, "Setext title");
    assert_eq!(parsed.outline_headers[1].level, 2);
    assert_eq!(
        parsed.outline_headers[1].source_start,
        markdown.find("Setext").unwrap()
    );
}

#[test]
fn heading_title_joins_math_footnote_labels_and_line_breaks_as_text() {
    let markdown = "Alpha\nbeta $x$[^note]\n=====\n\n[^note]: Footnote text.\n";
    let parsed = parse_headers(markdown);

    assert_eq!(parsed.outline_headers.len(), 1);
    assert_eq!(parsed.outline_headers[0].title, "Alpha beta x[note]");
    assert_eq!(parsed.document_title.as_deref(), Some("Alpha beta x[note]"));
}

#[test]
fn heading_without_visible_text_is_left_out_of_the_outline() {
    let parsed = parse_headers("# \n\n## ![](logo.png)\n\n## Kept\n");

    assert_eq!(parsed.outline_headers.len(), 1);
    assert_eq!(parsed.outline_headers[0].title, "Kept");
    assert_eq!(parsed.document_title, None);
}
