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
fn display_math_in_a_heading_reads_as_its_source() {
    let parsed = parse_headers("## Sum $$a+b$$ explained\n");
    assert_eq!(parsed.outline_headers[0].title, "Sum a+b explained");
}

#[test]
fn heading_without_visible_text_is_left_out_of_the_outline() {
    let parsed = parse_headers("# \n\n## ![](logo.png)\n\n## Kept\n");

    assert_eq!(parsed.outline_headers.len(), 1);
    assert_eq!(parsed.outline_headers[0].title, "Kept");
    assert_eq!(parsed.document_title, None);
}

#[test]
fn every_heading_depth_keeps_its_level_and_line() {
    let parsed = parse_headers("# a\n## b\n### c\n#### d\n##### e\n###### f\n");

    let levels: Vec<u8> = parsed.outline_headers.iter().map(|h| h.level).collect();
    let lines: Vec<usize> = parsed
        .outline_headers
        .iter()
        .map(|h| h.line_number)
        .collect();
    assert_eq!(levels, [1, 2, 3, 4, 5, 6]);
    assert_eq!(lines, [0, 1, 2, 3, 4, 5]);
}

/// The outline of a document shaped like this, by heading level:
///
/// ```text
/// 0  ## Setup
/// 1  ### Linux
/// 2  #### Arch
/// 3  ### Windows
/// 4  ## Usage
/// ```
fn outline() -> Vec<Header> {
    parse_headers("## Setup\n### Linux\n#### Arch\n### Windows\n## Usage\n").outline_headers
}

fn hidden(collapsed: &[usize]) -> Vec<bool> {
    let headers = outline();
    let collapsed: HashSet<usize> = collapsed.iter().copied().collect();
    (0..headers.len())
        .map(|index| header_is_hidden(&headers, index, &collapsed))
        .collect()
}

#[test]
fn nothing_is_hidden_while_nothing_is_collapsed() {
    assert_eq!(hidden(&[]), [false; 5]);
}

#[test]
fn collapsing_a_heading_hides_everything_nested_under_it() {
    // Setup collapsed: Linux, Arch and Windows go; Usage is a sibling.
    assert_eq!(hidden(&[0]), [false, true, true, true, false]);
    // Linux collapsed: only Arch is under it.
    assert_eq!(hidden(&[1]), [false, false, true, false, false]);
}

#[test]
fn collapsed_headings_beside_a_heading_do_not_hide_it() {
    // Arch and Windows have nothing under them, and Usage is not under
    // Windows.
    assert_eq!(hidden(&[2, 3]), [false; 5]);
}

#[test]
fn positions_outside_the_outline_are_never_hidden_or_parents() {
    let headers = outline();
    let all: HashSet<usize> = (0..headers.len()).collect();
    assert!(!header_is_hidden(&headers, headers.len(), &all));
    assert!(!header_has_children(&headers, headers.len()));
}

#[test]
fn a_heading_has_children_when_a_deeper_heading_follows_it() {
    let headers = outline();
    let parents: Vec<bool> = (0..headers.len())
        .map(|index| header_has_children(&headers, index))
        .collect();
    assert_eq!(parents, [true, true, false, false, false]);
    assert!(any_header_has_children(&headers));
}

#[test]
fn a_flat_outline_has_nothing_to_fold() {
    let flat = parse_headers("## One\n## Two\n## Three\n").outline_headers;
    assert!(!any_header_has_children(&flat));
    assert!(!any_header_has_children(&[]));
}
