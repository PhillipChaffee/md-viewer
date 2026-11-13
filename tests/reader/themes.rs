//! The four reading themes.

use super::support::{Reader, Rgb};

/// Each theme and the color of its paper, in the order Ctrl+D steps
/// through them.
const THEMES: [(&str, &str); 4] = [
    ("Paper", "#FDFDFB"),
    ("Sepia", "#F8F1E3"),
    ("Dusk", "#2C2C2E"),
    ("Night", "#000000"),
];

fn article() -> Reader {
    Reader::with_docs(&[("article.md", "# Tides\n\nThe sea keeps its own hours.")])
}

#[test]
fn a_first_run_reads_on_sepia() {
    let mut reader = article();

    assert_eq!(reader.page_color(), Rgb::hex("#F8F1E3"));
    reader.click("Appearance");
    assert!(reader.is_on("Sepia"));
    assert!(!reader.is_on("Paper"));
}

#[test]
fn picking_a_theme_changes_the_paper() {
    let mut reader = article();
    reader.click("Appearance");

    for (theme, paper) in THEMES {
        reader.click(theme);

        assert_eq!(reader.page_color(), Rgb::hex(paper), "{theme}");
        let chosen: Vec<_> = THEMES
            .iter()
            .filter(|(name, _)| reader.is_on(name))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(chosen, [theme]);
    }
}

#[test]
fn ctrl_d_steps_through_the_themes_and_wraps() {
    let mut reader = article();

    for paper in ["#2C2C2E", "#000000", "#FDFDFB", "#F8F1E3"] {
        reader.press("Ctrl+D");

        assert_eq!(reader.page_color(), Rgb::hex(paper));
    }
}

/// Known bug, listed in `tests/README.md`: the app reads the modifier keys
/// once per frame, so Ctrl+D pressed and released inside one frame is seen
/// as a plain D. The real window drops such taps when a frame runs long.
/// This test states the right behavior and is expected to fail until the
/// app has it.
#[test]
#[should_panic(expected = "a shortcut tapped inside one frame was dropped")]
fn a_shortcut_tapped_inside_one_frame_still_counts() {
    let mut reader = article();

    reader.tap("Ctrl+D");

    assert!(
        reader.page_color() == Rgb::hex("#2C2C2E"),
        "a shortcut tapped inside one frame was dropped"
    );
}

#[test]
fn the_menu_steps_and_picks_themes_too() {
    let mut reader = article();

    reader.click("More");
    reader.click("Theme");
    reader.click("Night");
    assert_eq!(reader.page_color(), Rgb::hex("#000000"));

    reader.click("More");
    reader.click("Theme");
    reader.click("Next Theme");
    assert_eq!(reader.page_color(), Rgb::hex("#FDFDFB"));
}

#[test]
fn body_text_keeps_seven_to_one_contrast_on_every_theme() {
    let mut reader = article();
    reader.click("Appearance");

    for (theme, _) in THEMES {
        reader.click(theme);

        let contrast = reader.ink_of("The sea keeps").contrast(reader.page_color());
        assert!(contrast >= 7.0, "{theme}: body text at {contrast:.2}:1");
    }
}

#[test]
fn the_light_themes_are_light_and_the_dark_ones_dark() {
    let mut reader = article();
    reader.click("Appearance");
    let mut paper = Vec::new();

    for (theme, _) in THEMES {
        reader.click(theme);
        paper.push(reader.page_color().luminance());
    }

    let [paper_white, sepia, dusk, night] = paper[..] else {
        panic!("four themes");
    };
    assert!(paper_white > 0.8 && sepia > 0.8, "{paper_white} {sepia}");
    assert!(dusk < 0.1 && night < dusk, "{dusk} {night}");
}

#[test]
fn the_home_page_is_on_the_theme_too() {
    let mut reader = Reader::first_run();
    assert_eq!(reader.page_color(), Rgb::hex("#F8F1E3"));

    reader.press("Ctrl+D");

    assert_eq!(reader.page_color(), Rgb::hex("#2C2C2E"));
}

#[test]
fn the_top_bar_is_on_the_theme_too() {
    let mut reader = article();
    let bar = reader.area_of("Go forward");
    let beside_the_button = (bar.right() + 60.0, bar.top + bar.height / 2.0);
    let sepia_bar = reader.color_at(beside_the_button.0, beside_the_button.1);

    reader.press("Ctrl+D");

    let dusk_bar = reader.color_at(beside_the_button.0, beside_the_button.1);
    assert!(sepia_bar.luminance() > 0.8, "{sepia_bar:?}");
    assert!(dusk_bar.luminance() < 0.1, "{dusk_bar:?}");
}

#[test]
fn the_theme_is_remembered() {
    let mut reader = article();
    reader.press("Ctrl+D");
    reader.press("Ctrl+D");

    let mut reader = reader.restart(&["article.md"]);

    assert_eq!(reader.page_color(), Rgb::hex("#000000"));
    reader.click("Appearance");
    assert!(reader.is_on("Night"));
}
