//! Inline code: told from the prose around it by its face, its size and a
//! tint of its own, with no box behind it.

use super::pages::{page, read_on, NEAR};
use super::support::{Reader, Rgb};

/// Each theme and the tint its inline code is set in.
const TINTS: [(&str, &str); 4] = [
    ("Paper", "#6F4220"),
    ("Sepia", "#6F4220"),
    ("Dusk", "#E6C8A4"),
    ("Night", "#D9B48A"),
];

/// A paragraph with a piece of code in the middle of it.
fn sentence() -> Reader {
    page("# Code\n\nRun `cargo test` before you push.")
}

/// Open the find bar and look for `query`.
fn find(reader: &mut Reader, query: &str) {
    reader.press("Ctrl+F");
    reader.type_text(query);
}

#[test]
fn nothing_is_painted_behind_inline_code_on_any_theme() {
    let mut reader = sentence();

    for (theme, _) in TINTS {
        read_on(&mut reader, theme);
        let code = reader.text_area("cargo test");
        let paper = reader.page_color();

        for across in [2.0, code.width / 2.0, code.width - 2.0] {
            let behind = reader.color_at(code.left + across, code.top + code.height / 2.0);
            assert_eq!(behind, paper, "{theme}: {across} px into the code");
        }
        assert_eq!(reader.marker_of("cargo test"), None, "{theme}");
        assert!(reader.marks_in(code).is_empty(), "{theme}");
    }
    reader.assert_hides("`");
}

#[test]
fn inline_code_is_in_the_tint_of_its_theme() {
    let mut reader = sentence();

    for (theme, tint) in TINTS {
        read_on(&mut reader, theme);
        let ink = reader.ink_of("cargo test");

        assert_eq!(ink, Rgb::hex(tint), "{theme}");
        assert_ne!(ink, reader.ink_of("Run "), "{theme}: the ink of the prose");
        let contrast = ink.contrast(reader.page_color());
        assert!(contrast >= 7.0, "{theme}: inline code at {contrast:.2}:1");
    }
}

#[test]
fn inline_code_is_smaller_than_the_text_around_it_and_stands_on_its_line() {
    let reader = page("# Size\n\nHxk in prose and `Hxq` in code.");

    let (prose, code) = (
        reader.letter_of("Hxk in prose", 'H'),
        reader.letter_of("Hxq", 'H'),
    );
    assert!(
        code.height < prose.height,
        "a capital in code is {} px tall, one in prose {}",
        code.height,
        prose.height
    );
    // Letters without a curve at the foot stand on the line itself.
    for letter in ['H', 'x'] {
        let prose = reader.letter_of("Hxk in prose", letter).bottom();
        let code = reader.letter_of("Hxq", letter).bottom();
        assert!((prose - code).abs() <= NEAR, "{letter}: {prose} and {code}");
    }
}

#[test]
fn code_in_a_heading_is_tinted_code_with_no_box() {
    let reader = page("# Title\n\n## Uxv and `Uxw` here\n\nbody text");

    // The contents sidebar repeats the heading as one piece of text; the
    // page sets the code as a piece of its own.
    assert_eq!(reader.ink_of_each("Uxw"), [Rgb::hex("#6F4220")]);
    assert_ne!(reader.ink_of("Uxv and "), Rgb::hex("#6F4220"));
    let marked = reader.marked();
    assert!(marked.is_empty(), "{marked:?}");
    let code = reader.on_page("Uxw");
    let behind = reader.color_at(code.left + 2.0, code.top + code.height / 2.0);
    assert_eq!(behind, reader.page_color());
}

#[test]
fn code_in_a_link_is_part_of_the_link() {
    let mut reader = Reader::with_docs(&[
        (
            "page.md",
            "# Links\n\nA [plain link](guide.md) and [the `guide` page](guide.md).",
        ),
        ("guide.md", "# Guide\n\nThe guide itself."),
    ]);

    let link = reader.ink_of("plain link");
    assert_eq!(reader.inks_of("the guide page"), [link]);
    assert!(reader.is_underlined("the guide page"));
    assert_eq!(reader.marker_of("the guide page"), None);

    reader.click("the guide page");
    reader.assert_sees("The guide itself.");
}

#[test]
fn a_found_match_in_code_is_marked_and_stays_code() {
    let mut reader = page("# Camping\n\nPack the `tentpole` last.");
    let at_rest = (reader.ink_of("tentpole"), reader.width_of("tentpole"));

    // Find takes either case, so the find box holds other letters than
    // the page does.
    find(&mut reader, "TENTPOLE");

    let marker = reader.marker_of("tentpole");
    assert!(marker.is_some(), "the match is marked");
    assert_ne!(marker, Some(reader.page_color()));
    assert_eq!(reader.ink_of("tentpole"), at_rest.0);
    let width = reader.width_of("tentpole");
    assert!(
        (width - at_rest.1).abs() < NEAR,
        "{width} and {}",
        at_rest.1
    );
}

#[test]
fn a_block_of_code_keeps_its_panel_and_its_own_ink() {
    let reader = page("# Listing\n\nRun `cargo test` first.\n\n```\nplain block\n```\n\nafter");

    let block = reader.text_area("plain block");
    let panel = reader.color_at(block.left + 4.0, block.top - 3.0);
    assert_ne!(panel, reader.page_color(), "the block sits on a panel");
    assert_ne!(reader.ink_of("plain block"), reader.ink_of("cargo test"));
    // A block is set a little larger than code in a line of prose.
    assert!(
        reader.width_of("plain block") / 11.0 > reader.width_of("cargo test") / 10.0,
        "a letter of the block is wider than one of the code in the line"
    );
}

#[test]
fn the_lines_of_a_drawn_box_are_as_wide_as_the_letters() {
    let drawn =
        page("# Box\n\n```\n\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}\n```\n\nafter");
    let lettered = page("# Box\n\n```\n\u{2502}code\u{2502}\n```\n\nafter");

    let (drawn, lettered) = (
        drawn.width_of("\u{250c}\u{2500}"),
        lettered.width_of("\u{2502}code"),
    );

    assert!((drawn - lettered).abs() < NEAR, "{drawn} and {lettered}");
}
