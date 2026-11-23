//! Inline code: told from the prose around it by its face and a tint of
//! its own, with no box behind it.

use super::pages::{page, read_on, NEAR};
use super::support::Reader;

const THEMES: [&str; 4] = ["Paper", "Sepia", "Dusk", "Night"];

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

    for theme in THEMES {
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
fn inline_code_is_in_an_ink_of_its_own_that_reads_as_well_as_the_prose() {
    let mut reader = sentence();

    for theme in THEMES {
        read_on(&mut reader, theme);
        let ink = reader.ink_of("cargo test");

        assert_ne!(ink, reader.ink_of("Run "), "{theme}: the ink of the prose");
        let contrast = ink.contrast(reader.page_color());
        assert!(contrast >= 7.0, "{theme}: inline code at {contrast:.2}:1");
    }
}

/// What tells a link from inline code does not rest on their colors: it
/// is the link's underline and the code's face.
#[test]
fn inline_code_is_told_from_a_link_by_its_face_and_the_links_underline() {
    let mut reader = Reader::with_docs(&[
        (
            "page.md",
            "# Marks\n\n[illi](guide.md) and [mwwm](guide.md) are links, `ilil` and `mwmw` code.",
        ),
        ("guide.md", "# Guide\n\nThe guide itself."),
    ]);

    for theme in THEMES {
        read_on(&mut reader, theme);
        for link in ["illi", "mwwm"] {
            assert!(reader.is_underlined(link), "{theme}: the link {link}");
        }
        for code in ["ilil", "mwmw"] {
            assert!(!reader.is_underlined(code), "{theme}: the code {code}");
        }
    }
    // A link is in the face of the prose, whose letters have widths of
    // their own; the letters of code are all one width.
    let (narrow, wide) = (reader.width_of("illi"), reader.width_of("mwwm"));
    assert!(wide > narrow * 1.5, "links: {narrow} and {wide}");
    let (narrow, wide) = (reader.width_of("ilil"), reader.width_of("mwmw"));
    assert!((narrow - wide).abs() < NEAR, "code: {narrow} and {wide}");
}

/// Sepia's links are a warm brown. The code beside one is an olive, well
/// round the color wheel from it towards green, so the two differ at a
/// glance and not only by underline and face.
#[test]
fn on_sepia_inline_code_is_on_the_green_side_of_a_link() {
    /// Half the step from one named color to the next, red to yellow.
    const APART: f64 = 30.0;
    const GREEN: f64 = 120.0;
    let mut reader = Reader::with_docs(&[
        (
            "page.md",
            "# Inks\n\nA [plain link](guide.md) beside `some code`.",
        ),
        ("guide.md", "# Guide\n\nThe guide itself."),
    ]);

    read_on(&mut reader, "Sepia");
    let link = reader.ink_of("plain link").hue();
    let code = reader.ink_of("some code").hue();
    let (link, code) = (link.expect("a gray link"), code.expect("gray code"));

    assert!(
        code >= link + APART && code <= GREEN,
        "the link is at {link:.0} degrees and the code at {code:.0}"
    );
}

#[test]
fn code_in_bold_text_is_set_in_a_bolder_cut_of_the_code_face() {
    // Each piece of code starts its line, so both capitals land on the
    // same dots of the screen and differ only by their face.
    let reader = page("# Weights\n\n`Hxq` in plain text.\n\n**`Hxb` in bold text.**");

    let (plain, bold) = (reader.drawing_of("Hxq", 'H'), reader.drawing_of("Hxb", 'H'));

    assert!(!plain.is_blank() && !bold.is_blank());
    assert_ne!(plain, bold);
    let (plain, bold) = (reader.width_of("Hxq"), reader.width_of("Hxb"));
    assert!((plain - bold).abs() < NEAR, "{plain} and {bold}");
}

#[test]
fn inline_code_is_as_large_as_the_text_around_it_and_stands_on_its_line() {
    let mut reader = page("# Size\n\nHxk in prose and `Hxq` in code.");

    for preset in ["Reader", "System", "GitHub", "VS Code"] {
        if preset != "Reader" {
            reader.click("Appearance");
            reader.click("Fonts\u{2026}");
            reader.click(preset);
            reader.press("Escape");
        }
        // A capital and a small letter of code are as tall as the prose's, and
        // letters without a curve at the foot stand on the line itself.
        for letter in ['H', 'x'] {
            let prose = reader.letter_of("Hxk in prose", letter);
            let code = reader.letter_of("Hxq", letter);
            assert!(
                (prose.height - code.height).abs() <= NEAR,
                "{preset} {letter}: {} px tall in prose, {} in code",
                prose.height,
                code.height
            );
            let (prose, code) = (prose.bottom(), code.bottom());
            assert!(
                (prose - code).abs() <= NEAR,
                "{preset} {letter}: {prose} and {code}"
            );
        }
    }
}

#[test]
fn code_in_a_heading_is_tinted_code_with_no_box() {
    let reader = page("# Title\n\n## Uxv and `Uxw` here\n\nbody `Uxy` text");

    // The contents sidebar repeats the heading as one piece of text; the
    // page sets the code as a piece of its own.
    let tint = reader.ink_of("Uxy");
    assert_eq!(reader.ink_of_each("Uxw"), [tint]);
    assert_ne!(reader.ink_of("Uxv and "), tint);
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
    // A block is set a little smaller than code in a line of prose.
    assert!(
        reader.width_of("plain block") / 11.0 < reader.width_of("cargo test") / 10.0,
        "a letter of the block is narrower than one of the code in the line"
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
