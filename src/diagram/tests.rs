use super::*;
use crate::test_support::assert_close;

/// Contrast WCAG AA asks of body text.
const AA: f32 = 4.5;

/// The reader's four pages, as `(name, page, text)`.
const PAGES: [(&str, Color32, Color32); 4] = [
    ("paper", Color32::WHITE, Color32::from_rgb(0x1f, 0x1f, 0x1f)),
    (
        "sepia",
        Color32::from_rgb(0xf8, 0xf1, 0xe3),
        Color32::from_rgb(0x2b, 0x26, 0x22),
    ),
    (
        "dusk",
        Color32::from_rgb(0x2c, 0x2c, 0x2e),
        Color32::from_rgb(0xda, 0xd7, 0xd0),
    ),
    ("night", Color32::BLACK, Color32::from_rgb(0xc8, 0xc6, 0xc1)),
];

fn visuals(page: Color32, text: Color32) -> egui::Visuals {
    egui::Visuals {
        panel_fill: page,
        override_text_color: Some(text),
        ..egui::Visuals::light()
    }
}

#[test]
fn a_diagram_takes_the_page_and_text_colors_of_the_visuals() {
    for (name, page, text) in PAGES {
        let colors = diagram_colors(&visuals(page, text));
        assert_eq!(colors.page, page, "{name}");
        assert_eq!(colors.text, text, "{name}");
    }
}

#[test]
fn text_and_lines_are_legible_on_every_page() {
    for (name, page, text) in PAGES {
        let colors = diagram_colors(&visuals(page, text));
        let surfaces = [
            ("page", colors.page),
            ("node", colors.node_fill),
            ("secondary", colors.secondary_fill),
            ("note", colors.note_fill),
        ];
        for (surface, fill) in surfaces {
            let text_contrast = contrast(colors.text, fill);
            let line_contrast = contrast(colors.line, fill);
            assert!(
                text_contrast >= AA,
                "{name}: text on {surface} is {text_contrast}"
            );
            assert!(
                line_contrast >= AA,
                "{name}: lines on {surface} are {line_contrast}"
            );
        }
    }
}

#[test]
fn fills_are_quiet_and_their_borders_visible() {
    for (name, page, text) in PAGES {
        let colors = diagram_colors(&visuals(page, text));
        for fill in [colors.node_fill, colors.secondary_fill, colors.note_fill] {
            assert_ne!(fill, page, "{name}");
            assert!(contrast(fill, page) < 1.5, "{name}: {fill:?} is loud");
        }
        let border = contrast(colors.node_stroke, page);
        assert!(border >= 1.5, "{name}: borders are {border}");
        assert!(border < contrast(colors.line, page), "{name}");
    }
}

#[test]
fn page_colored_labels_are_readable_on_the_accent() {
    for (name, page, text) in PAGES {
        for link in [egui::Visuals::light(), egui::Visuals::dark()].map(|v| v.hyperlink_color) {
            let mut visuals = visuals(page, text);
            visuals.hyperlink_color = link;
            let accent = contrast(diagram_colors(&visuals).accent, page);
            assert!(accent >= MIN_ACCENT_CONTRAST, "{name}: accent is {accent}");
        }
    }
}

#[test]
fn a_readable_link_color_is_the_accent_as_is() {
    let link = Color32::from_rgb(0x8a, 0x4b, 0x1f);
    let mut visuals = visuals(PAGES[1].1, PAGES[1].2);
    visuals.hyperlink_color = link;
    assert_eq!(diagram_colors(&visuals).accent, link);
}

#[test]
fn a_link_color_close_to_the_page_is_pulled_toward_the_text() {
    for (name, page, text) in PAGES {
        // The worst case: a link color that is the page color.
        let accent = accent(page, page, text);
        let reached = contrast(accent, page);
        assert!(reached >= MIN_ACCENT_CONTRAST, "{name}: {reached}");
        assert_ne!(accent, text, "{name}: went further than needed");
    }
}

#[test]
fn without_any_contrast_to_work_with_the_accent_is_the_text_color() {
    let page = Color32::from_gray(0x80);
    let text = Color32::from_gray(0x88);
    assert_eq!(accent(page, page, text), text);
}

#[test]
fn contrast_follows_the_wcag_definition() {
    assert_close(contrast(Color32::WHITE, Color32::BLACK), 21.0);
    assert_close(contrast(Color32::BLACK, Color32::WHITE), 21.0);
    assert_close(contrast(PAGES[1].1, PAGES[1].1), 1.0);
    // #767676 is the lightest gray that passes AA on white.
    let gray = contrast(Color32::from_gray(0x76), Color32::WHITE);
    assert!((4.5..4.6).contains(&gray), "{gray}");
    assert!(contrast(Color32::from_gray(0x78), Color32::WHITE) < 4.5);
}
