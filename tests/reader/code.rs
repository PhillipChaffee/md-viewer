//! Code blocks: their colors, their surface, and the button that copies
//! them.

use super::pages::{page, read_on, NEAR, THEMES};
use super::support::{Reader, Rgb};

const RUST: &str = "fn answer() -> u32 {\n    let text = \"forty-two\"; // a comment\n    42\n}";

/// A page with one block of `code` fenced as `language`.
fn listing(language: &str, code: &str) -> Reader {
    page(&format!(
        "# Listing\n\nbefore\n\n```{language}\n{code}\n```\n\nafter"
    ))
}

/// The color of the block a code text sits on, sampled just above its
/// first line.
fn surface(reader: &Reader, code: &str) -> Rgb {
    let text = reader.text_area(code);
    reader.color_at(text.left + 4.0, text.top - 3.0)
}

#[test]
fn code_in_a_named_language_is_set_in_several_colors() {
    let reader = listing("rust", RUST);

    let inks = reader.inks_of("fn answer()");

    assert!(inks.len() >= 3, "keywords, strings, comments: {inks:?}");
}

#[test]
fn a_block_without_a_language_is_set_in_one_color() {
    let reader = listing("", RUST);

    let inks = reader.inks_of("fn answer()");

    assert_eq!(inks.len(), 1, "{inks:?}");
}

#[test]
fn a_language_nobody_knows_is_shown_as_plain_code() {
    let reader = listing("no-such-language", RUST);

    reader.assert_sees(RUST);
    assert_eq!(reader.inks_of("fn answer()").len(), 1);
    reader.assert_hides("no-such-language");
}

#[test]
fn common_languages_are_all_highlighted() {
    let samples = [
        ("python", "def area(r):\n    return 3.14 * r ** 2  # circle"),
        (
            "javascript",
            "const greet = (name) => `hi ${name}`; // arrow",
        ),
        (
            "json",
            "{\"name\": \"md-viewer\", \"stars\": 42, \"ok\": true}",
        ),
        ("bash", "for f in *.md; do echo \"$f\"; done # loop"),
        ("toml", "[package]\nname = \"viewer\"\nedition = 2021"),
        (
            "html",
            "<a href=\"x.html\" class=\"link\">text</a><!-- note -->",
        ),
        ("c", "int main(void) { return 0; } /* done */"),
        ("go", "func main() { fmt.Println(\"hi\") } // go"),
        ("yaml", "name: viewer\ncount: 3\nlist:\n  - one"),
        ("sql", "SELECT name FROM users WHERE id = 7; -- one row"),
    ];

    for (language, code) in samples {
        let reader = listing(language, code);
        let inks = reader.inks_of(code);
        assert!(inks.len() >= 2, "{language} is set in {inks:?}");
    }
}

#[test]
fn every_theme_has_a_color_scheme_of_its_own_for_code() {
    let mut reader = listing("rust", RUST);
    let mut schemes: Vec<Vec<Rgb>> = Vec::new();

    for theme in THEMES {
        read_on(&mut reader, theme);
        let inks = reader.inks_of("fn answer()");
        assert!(!schemes.contains(&inks), "{theme} repeats a scheme");
        schemes.push(inks);
    }
}

#[test]
fn every_color_of_the_code_can_be_read_on_its_block() {
    let mut reader = listing("rust", RUST);

    for theme in THEMES {
        read_on(&mut reader, theme);
        let block = surface(&reader, RUST);
        for ink in reader.inks_of("fn answer()") {
            let contrast = ink.contrast(block);
            assert!(
                contrast >= 2.0,
                "{theme}: {ink:?} on {block:?} is {contrast:.2}:1"
            );
        }
    }
}

#[test]
fn the_block_is_darker_on_dark_themes_and_lighter_on_light_ones() {
    let mut reader = listing("rust", RUST);
    let mut blocks = Vec::new();

    for theme in THEMES {
        read_on(&mut reader, theme);
        let block = surface(&reader, RUST);
        assert_ne!(block, reader.page_color(), "{theme}");
        blocks.push(block.luminance());
    }

    assert!(blocks[0] > 0.6 && blocks[1] > 0.6, "{blocks:?}");
    assert!(blocks[2] < 0.2 && blocks[3] < 0.2, "{blocks:?}");
}

#[test]
fn the_block_is_as_wide_as_the_column_however_short_the_code() {
    let reader = listing("rust", "let x = 1;");
    let code = reader.text_area("let x = 1;");
    let column = reader.column();
    let middle = code.top + code.height / 2.0;

    let block = reader.color_at(code.left + 2.0, middle);

    assert_eq!(reader.color_at(column.right() - 6.0, middle), block);
    assert_eq!(reader.color_at(column.left + 6.0, middle), block);
    assert_ne!(reader.color_at(column.right() + 30.0, middle), block);
}

#[test]
fn the_copy_button_puts_the_code_on_the_clipboard() {
    let mut reader = listing("rust", RUST);
    assert_eq!(reader.clipboard(), None);

    reader.click("\u{1f5d0}");

    assert_eq!(reader.clipboard().map(str::trim_end), Some(RUST));
}

#[test]
fn each_block_has_a_copy_button_of_its_own() {
    let mut reader = page("# Two\n\n```\nfirst block\n```\n\n```\nsecond block\n```");

    reader.click_nth("\u{1f5d0}", 1);

    assert_eq!(reader.clipboard().map(str::trim_end), Some("second block"));
}

#[test]
fn a_long_line_of_code_stays_inside_the_column() {
    let long = "let value = compute(); ".repeat(12);
    let reader = listing("rust", long.trim_end());
    let column = reader.column();
    let after = reader.text_area("after");

    let block = surface(&reader, long.trim_end());

    assert_ne!(
        reader.color_at(column.right() - 6.0, after.top - 40.0),
        reader.page_color()
    );
    assert_eq!(
        reader.color_at(column.right() + 30.0, after.top - 40.0),
        reader.page_color()
    );
    assert_ne!(block, reader.page_color());
}

#[test]
fn code_keeps_its_line_height_whatever_the_line_spacing() {
    let mut reader = listing("rust", RUST);
    let mut heights = Vec::new();

    reader.click("Appearance");
    for spacing in ["Compact", "Comfortable", "Airy"] {
        reader.click(spacing);
        heights.push(reader.text_area(RUST).height);
    }

    assert!((heights[0] - heights[1]).abs() < NEAR, "{heights:?}");
    assert!((heights[1] - heights[2]).abs() < NEAR, "{heights:?}");
}

#[test]
fn tilde_fences_and_indented_blocks_are_code_too() {
    let reader = page("# Fences\n\n~~~\ntilde fenced\n~~~\n\nprose\n\n    indented code\n\nafter");

    for code in ["tilde fenced", "indented code"] {
        let text = reader.text_area(code);
        let behind = reader.color_at(text.left + 2.0, text.top + text.height / 2.0);
        assert_ne!(behind, reader.page_color(), "{code} has no block");
    }
    reader.assert_hides("~~~");
}

#[test]
fn diagram_source_in_a_plain_block_stays_text() {
    let mut reader = page("# Source\n\nbefore\n\n```text\nflowchart LR\n    A --> B\n```\n\nafter");
    reader.idle_for(std::time::Duration::from_millis(300));

    reader.assert_sees("flowchart LR");
    assert!(reader.pictures().is_empty(), "{:?}", reader.pictures());
}

#[test]
fn markup_inside_a_code_block_is_shown_as_typed() {
    let reader = listing("", "# not a heading\n**not bold** and [not a link](x.md)");

    reader.assert_sees("# not a heading\n**not bold** and [not a link](x.md)");
    assert_eq!(reader.count("not a heading"), 0, "no such heading anywhere");
}
