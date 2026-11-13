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
