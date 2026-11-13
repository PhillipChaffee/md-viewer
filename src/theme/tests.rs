use std::collections::HashSet;

use super::*;
use crate::test_support::assert_close;

/// WCAG 2 relative luminance of an opaque sRGB color.
fn luminance(color: Color32) -> f32 {
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.0722_f32.mul_add(
        linear(color.b()),
        0.2126_f32.mul_add(linear(color.r()), 0.7152 * linear(color.g())),
    )
}

/// WCAG 2 contrast ratio between two opaque colors, from 1 to 21.
fn contrast(one: Color32, other: Color32) -> f32 {
    let (one, other) = (luminance(one), luminance(other));
    (one.max(other) + 0.05) / (one.min(other) + 0.05)
}

/// Syntax themes the renderer's default syntect set holds, with whether each
/// is drawn for a dark background.
const SYNTAX_THEMES: [(&str, bool); 7] = [
    ("InspiredGitHub", false),
    ("Solarized (light)", false),
    ("base16-ocean.light", false),
    ("Solarized (dark)", true),
    ("base16-eighties.dark", true),
    ("base16-mocha.dark", true),
    ("base16-ocean.dark", true),
];

#[test]
fn the_contrast_measure_matches_the_wcag_reference_points() {
    let black_on_white = contrast(Color32::BLACK, Color32::WHITE);
    assert!((black_on_white - 21.0).abs() < 1e-3, "{black_on_white}");
    let white_on_black = contrast(Color32::WHITE, Color32::BLACK);
    assert!((white_on_black - 21.0).abs() < 1e-3, "{white_on_black}");
    assert_close(contrast(rgb(0x77_77_77), rgb(0x77_77_77)), 1.0);
    // #767676 on white is the textbook lightest gray that passes AA.
    let gray_on_white = contrast(rgb(0x76_76_76), Color32::WHITE);
    assert!((4.5..4.6).contains(&gray_on_white), "{gray_on_white}");
}

#[test]
fn body_text_and_headings_reach_seven_to_one_on_every_surface_of_every_theme() {
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        for (surface_name, surface) in [
            ("page", palette.page),
            ("surface", palette.surface),
            ("raised", palette.raised),
        ] {
            for (ink_name, ink) in [("text", palette.text), ("heading", palette.heading)] {
                let ratio = contrast(ink, surface);
                assert!(
                    ratio >= 7.0,
                    "{theme:?}: {ink_name} on {surface_name} is {ratio:.2}:1"
                );
            }
        }
    }
}

#[test]
fn secondary_inks_stay_readable_on_the_page_of_every_theme() {
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        for (name, ink) in [
            ("muted", palette.muted),
            ("link", palette.link),
            ("positive", palette.positive),
            ("warning", palette.warning),
        ] {
            let ratio = contrast(ink, palette.page);
            assert!(ratio >= 4.5, "{theme:?}: {name} on page is {ratio:.2}:1");
        }
        // A control outline needs 3:1, and selected text has to stay text.
        let accent = contrast(palette.accent, palette.page);
        assert!(accent >= 3.0, "{theme:?}: accent on page is {accent:.2}:1");
        let selected = contrast(palette.text, palette.selection);
        assert!(
            selected >= 4.5,
            "{theme:?}: selected text is {selected:.2}:1"
        );
        // A hairline separates without drawing the eye.
        let rule = contrast(palette.rule, palette.page);
        assert!((1.1..2.0).contains(&rule), "{theme:?}: rule is {rule:.2}:1");
    }
}

#[test]
fn the_four_themes_are_two_light_and_two_dark_papers() {
    assert_eq!(
        ThemeId::ALL.map(|theme| theme.palette().dark),
        [false, false, true, true]
    );
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        let page_is_dark = luminance(palette.page) < 0.2;
        assert_eq!(page_is_dark, palette.dark, "{theme:?}");
        assert_ne!(palette.surface, palette.page, "{theme:?}");
        assert_ne!(palette.raised, palette.page, "{theme:?}");
    }
    let pages: HashSet<_> = ThemeId::ALL.map(|theme| theme.palette().page).into();
    assert_eq!(pages.len(), ThemeId::ALL.len());
    // Sepia is the warm paper: more red than blue.
    let sepia = ThemeId::Sepia.palette().page;
    assert!(sepia.r() > sepia.b() + 10, "{sepia:?}");
}

#[test]
fn each_theme_highlights_code_with_a_syntax_theme_made_for_its_background() {
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        let known = SYNTAX_THEMES
            .iter()
            .find(|(name, _)| *name == palette.syntax_theme);
        assert_eq!(
            known.map(|(_, dark)| *dark),
            Some(palette.dark),
            "{theme:?} uses {}",
            palette.syntax_theme
        );
    }
}

#[test]
fn themes_are_labelled_apart_and_sepia_is_the_default() {
    assert_eq!(ThemeId::default(), ThemeId::Sepia);
    assert_eq!(
        ThemeId::ALL.map(ThemeId::label),
        ["Paper", "Sepia", "Dusk", "Night"]
    );
}

#[test]
fn next_walks_the_themes_in_order_and_wraps() {
    let mut theme = ThemeId::ALL[0];
    for expected in ThemeId::ALL.into_iter().skip(1) {
        theme = theme.next();
        assert_eq!(theme, expected);
    }
    assert_eq!(theme.next(), ThemeId::ALL[0]);
}

#[test]
fn a_session_from_before_the_themes_keeps_its_light_or_dark_choice() {
    assert_eq!(ThemeId::restored(None, Some(true)), ThemeId::Dusk);
    assert_eq!(ThemeId::restored(None, Some(false)), ThemeId::Paper);
    assert_eq!(ThemeId::restored(None, None), ThemeId::Sepia);
    // A saved theme wins over the old flag, whatever the flag says.
    assert_eq!(
        ThemeId::restored(Some(ThemeId::Night), Some(false)),
        ThemeId::Night
    );
}

#[test]
fn visuals_take_every_color_from_the_palette() {
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        let built = visuals(palette, ColorOverrides::default());

        assert_eq!(built.dark_mode, palette.dark, "{theme:?}");
        assert_eq!(built.panel_fill, palette.page);
        assert_eq!(built.window_fill, palette.surface);
        assert_eq!(built.extreme_bg_color, palette.raised);
        assert_eq!(built.code_bg_color, palette.raised);
        assert_eq!(built.text_color(), palette.text);
        assert_eq!(built.weak_text_color(), palette.muted);
        assert_eq!(built.strong_text_color(), palette.heading);
        assert_eq!(built.hyperlink_color, palette.link);
        assert_eq!(built.selection.bg_fill, palette.selection);
        assert_eq!(built.warn_fg_color, palette.warning);
        assert_eq!(built.error_fg_color, palette.warning);
        // Separators and frames are hairlines.
        let hairline = built.widgets.noninteractive.bg_stroke;
        assert_eq!(hairline.color, palette.rule);
        assert_close(hairline.width, 1.0);
    }
}

#[test]
fn controls_are_bare_at_rest_and_washed_when_hovered() {
    let palette = ThemeId::Sepia.palette();
    let widgets = visuals(palette, ColorOverrides::default()).widgets;

    assert_eq!(widgets.inactive.weak_bg_fill, Color32::TRANSPARENT);
    assert_eq!(widgets.inactive.bg_stroke.color, Color32::TRANSPARENT);
    assert_eq!(widgets.inactive.fg_stroke.color, palette.text);
    assert_eq!(widgets.hovered.weak_bg_fill, palette.raised);
    assert_eq!(widgets.hovered.fg_stroke.color, palette.heading);
    assert_eq!(widgets.open.weak_bg_fill, palette.raised);
    // The scrollbar handle: the hairline at rest, the muted ink in use.
    assert_eq!(widgets.inactive.bg_fill, palette.rule);
    assert_eq!(widgets.hovered.bg_fill, palette.muted);
    assert_eq!(widgets.active.bg_fill, palette.muted);
}

#[test]
fn the_users_own_colors_win_over_the_themes() {
    let palette = ThemeId::Dusk.palette();
    let (highlight, link) = (rgb(0xC8_1E_1E), rgb(0x1E_A0_3C));

    let both = visuals(
        palette,
        ColorOverrides {
            highlight: Some(highlight),
            link: Some(link),
        },
    );
    assert_eq!(both.selection.bg_fill, highlight);
    assert_eq!(both.hyperlink_color, link);

    let link_only = visuals(
        palette,
        ColorOverrides {
            highlight: None,
            link: Some(link),
        },
    );
    assert_eq!(link_only.selection.bg_fill, palette.selection);
    assert_eq!(link_only.hyperlink_color, link);
    // Nothing else moves.
    assert_eq!(link_only.panel_fill, palette.page);
}

#[test]
fn overlay_inks_are_readable_on_the_scrim_whatever_the_page() {
    for theme in ThemeId::ALL {
        // The scrim over a page: its alpha-weighted black.
        let page = theme.palette().page;
        let under = |channel: u8| {
            let kept = u16::from(channel) * u16::from(u8::MAX - SCRIM.a()) / u16::from(u8::MAX);
            u8::try_from(kept).unwrap()
        };
        let dimmed = Color32::from_rgb(under(page.r()), under(page.g()), under(page.b()));
        for ink in [ON_SCRIM, ON_SCRIM_MUTED] {
            let ratio = contrast(ink, dimmed);
            assert!(
                ratio >= 4.5,
                "{theme:?}: {ink:?} on the scrim is {ratio:.2}:1"
            );
        }
    }
}
