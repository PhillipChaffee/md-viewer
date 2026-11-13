use super::*;

fn excerpt_of(content: &str) -> Option<String> {
    DocumentSummary::of(content, None).excerpt
}

#[test]
fn the_summary_carries_the_title_the_word_count_and_the_first_paragraph() {
    let content = "# On Reading\n\nRead slowly, and twice.\n";

    let summary = DocumentSummary::of(content, Some("On Reading"));

    assert_eq!(
        summary,
        DocumentSummary {
            title: Some("On Reading".to_owned()),
            excerpt: Some("Read slowly, and twice.".to_owned()),
            word_count: Some(7),
        }
    );
}

#[test]
fn a_document_without_a_title_or_a_paragraph_has_neither() {
    let summary = DocumentSummary::of("## Only a heading\n", None);

    assert_eq!(summary.title, None);
    assert_eq!(summary.excerpt, None);
    assert_eq!(summary.word_count, Some(4));
}

#[test]
fn the_excerpt_is_plain_text_with_the_markdown_syntax_gone() {
    let content = "# Title\n\nSome *emphasis*, a [link](https://example.com), `code`,\n\
                   **strong** text and $x^2$ math.\n\nSecond paragraph.\n";

    assert_eq!(
        excerpt_of(content).as_deref(),
        Some("Some emphasis, a link, code, strong text and x^2 math.")
    );
}

#[test]
fn a_line_break_inside_the_paragraph_becomes_a_space() {
    assert_eq!(
        excerpt_of("one\ntwo  \nthree\n").as_deref(),
        Some("one two three")
    );
}

#[test]
fn front_matter_headings_and_code_are_not_the_first_paragraph() {
    let content =
        "---\ntitle: Notes\n---\n\n# Notes\n\n```rust\nfn main() {}\n```\n\nThe body starts here.\n";

    assert_eq!(
        excerpt_of(content).as_deref(),
        Some("The body starts here.")
    );
}

#[test]
fn a_paragraph_of_images_only_is_passed_over() {
    let content =
        "# Project\n\n![build](https://example.com/b.svg) ![docs](d.svg)\n\nA viewer for notes.\n";

    assert_eq!(excerpt_of(content).as_deref(), Some("A viewer for notes."));
}

#[test]
fn an_image_inside_the_paragraph_leaves_its_alt_text_out() {
    assert_eq!(
        excerpt_of("Before ![a diagram](d.png) after.\n").as_deref(),
        Some("Before after.")
    );
}

#[test]
fn quotes_lists_tables_and_footnotes_are_passed_over() {
    let content = "> An epigraph.\n\n- a list item\n\n  with a paragraph\n\n\
                   | a | b |\n|---|---|\n| 1 | 2 |\n\n[^1]: A footnote.\n\n\
                   Term\n: A definition.\n\nThe first real paragraph.[^1]\n";

    assert_eq!(
        excerpt_of(content).as_deref(),
        Some("The first real paragraph.")
    );
}

#[test]
fn a_document_of_asides_only_has_no_excerpt() {
    assert_eq!(excerpt_of("> Only a quote.\n\n- and a list\n"), None);
    assert_eq!(excerpt_of(""), None);
}

#[test]
fn a_long_paragraph_is_cut_at_a_word_and_marked() {
    let paragraph = "word ".repeat(100);

    let excerpt = excerpt_of(&paragraph).unwrap();

    // 44 words of four letters and the 43 spaces between them are 219
    // characters; a 45th word would make 224.
    assert_eq!(excerpt, format!("{}…", "word ".repeat(44).trim_end()));
    assert_eq!(excerpt.chars().count(), 220);
}

#[test]
fn a_paragraph_that_just_fits_is_kept_whole() {
    // 43 words and their spaces are 215 characters; "final" makes 220.
    let paragraph = format!("{}final", "word ".repeat(43));
    assert_eq!(paragraph.chars().count(), 220);

    assert_eq!(excerpt_of(&paragraph), Some(paragraph));
}

#[test]
fn text_without_spaces_is_cut_inside_the_run() {
    let run = "字".repeat(300);

    let excerpt = excerpt_of(&run).unwrap();

    assert_eq!(excerpt, format!("{}…", "字".repeat(220)));
}
