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

/// That outline with the headings at `collapsed` folded.
fn folded(collapsed: &[usize]) -> Outline {
    let mut folded = Outline::default();
    folded.replace_headers(outline(), false);
    for &index in collapsed {
        folded.toggle(index);
    }
    folded
}

fn hidden(collapsed: &[usize]) -> Vec<bool> {
    let folded = folded(collapsed);
    (0..folded.headers().len())
        .map(|index| !folded.visible().contains(&index))
        .collect()
}

/// Whether a collapsed ancestor hides the heading at `index`, by the rule
/// itself: a heading's parent is the nearest heading before it with a
/// lower level, and a heading is hidden when its parent is collapsed or
/// hidden.
fn hidden_by_an_ancestor(levels: &[u8], index: usize, collapsed: &[bool]) -> bool {
    let parent = (0..index)
        .rev()
        .find(|&before| levels[before] < levels[index]);
    parent
        .is_some_and(|parent| collapsed[parent] || hidden_by_an_ancestor(levels, parent, collapsed))
}

/// Every outline of `len` headings whose levels are drawn from `choices`.
fn outlines_of(len: u32, choices: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    (0..choices.len().pow(len)).map(move |code| {
        (0..len)
            .map(|place| choices[code / choices.len().pow(place) % choices.len()])
            .collect()
    })
}

#[test]
fn the_one_pass_fold_agrees_with_the_ancestor_rule_on_every_small_outline() {
    // Level 5 after level 3 skips a level, as `###` straight to `#####`.
    let choices = [1, 2, 3, 5];
    let mut hidden_rows = 0;
    for len in 0..=6 {
        for levels in outlines_of(len, &choices) {
            for folds in 0..1_usize << len {
                let collapsed: Vec<bool> = (0..len).map(|place| folds >> place & 1 == 1).collect();
                let expected: Vec<usize> = (0..levels.len())
                    .filter(|&index| !hidden_by_an_ancestor(&levels, index, &collapsed))
                    .collect();

                let visible: Vec<usize> =
                    visible_rows(levels.iter().copied(), |index| collapsed[index]).collect();

                assert_eq!(
                    visible, expected,
                    "levels {levels:?}, collapsed {collapsed:?}"
                );
                hidden_rows += levels.len() - visible.len();
            }
        }
    }
    assert!(hidden_rows > 0, "no generated outline folded anything away");
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
    // A fold recorded for a position past the end hides nothing and is
    // never a row.
    let all: Vec<usize> = (0..=headers.len()).collect();
    assert_eq!(folded(&all).visible(), [0, 4]);
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

#[test]
fn folding_a_heading_twice_shows_what_is_under_it_again() {
    let mut outline = folded(&[0]);
    assert!(outline.is_collapsed(0));
    assert_eq!(outline.visible(), [0, 4]);

    outline.toggle(0);

    assert!(!outline.is_collapsed(0));
    assert_eq!(outline.visible(), [0, 1, 2, 3, 4]);
}

#[test]
fn a_hidden_heading_is_stood_for_by_the_folded_heading_above_it() {
    // Linux is folded, so the rows are Setup, Linux, Windows, Usage.
    let outline = folded(&[1]);
    let rows: Vec<Option<usize>> = (0..5)
        .map(|index| outline.row_standing_for(index))
        .collect();
    assert_eq!(rows, [Some(0), Some(1), Some(1), Some(2), Some(3)]);

    assert_eq!(Outline::default().row_standing_for(0), None);
}

#[test]
fn collapse_all_folds_the_headings_with_children_and_expand_all_none() {
    let mut outline = folded(&[]);

    outline.collapse_all();
    let collapsed: Vec<bool> = (0..5).map(|index| outline.is_collapsed(index)).collect();
    assert_eq!(collapsed, [true, true, false, false, false]);
    assert_eq!(outline.visible(), [0, 4]);

    outline.expand_all();
    assert!((0..5).all(|index| !outline.is_collapsed(index)));
    assert_eq!(outline.visible(), [0, 1, 2, 3, 4]);
}

#[test]
fn an_outline_knows_which_headings_have_children_and_whether_any_does() {
    let mut outline = folded(&[]);
    assert!(outline.has_folds());
    assert!(outline.has_children(1));
    assert!(!outline.has_children(2));

    let flat = parse_headers("## One\n## Two\n").outline_headers;
    outline.replace_headers(flat, true);
    assert!(!outline.has_folds());
    assert!(!Outline::default().has_folds());
}

#[test]
fn new_headings_keep_the_folds_only_when_asked_and_as_many_as_before() {
    let renamed =
        || parse_headers("## Install\n### Linux\n#### Arch\n### Windows\n## Use\n").outline_headers;

    let mut outline = folded(&[1]);
    outline.replace_headers(renamed(), true);
    assert!(outline.is_collapsed(1), "the same headings, reworded");
    assert_eq!(outline.visible(), [0, 1, 3, 4]);
    assert_eq!(outline.headers()[0].title, "Install");

    outline.replace_headers(renamed(), false);
    assert!(!outline.is_collapsed(1), "another document starts unfolded");
    assert_eq!(outline.visible(), [0, 1, 2, 3, 4]);

    let mut outline = folded(&[1]);
    let shorter = parse_headers("## Setup\n### Linux\n#### Arch\n").outline_headers;
    outline.replace_headers(shorter, true);
    assert!(
        !outline.is_collapsed(1),
        "a heading removed shifts the rest"
    );
    assert_eq!(outline.visible(), [0, 1, 2]);
}
