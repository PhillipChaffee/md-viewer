//! The appearance popover, used in the running app: each control changes
//! the document at once and shows the setting in force.

use eframe::egui::{self, Color32, Key, Modifiers, Shape};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::app::ui_harness::UiHarness;
use crate::system_fonts::TextSizeClass;
use crate::test_support::{assert_close, painted_shapes};

/// The app on a document with the explorer and the outline put away, so
/// the document pane is as wide as the window, and the popover open.
fn popover() -> UiHarness {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().show_explorer = false;
    harness.app_mut().show_outline = false;
    harness.settle();
    harness.click_exact("Appearance");
    harness
}

/// Whether the popover is on screen.
fn is_open(harness: &UiHarness) -> bool {
    harness.shows("Line spacing")
}

/// The circles the last frame painted: center, radius, fill, outline.
fn circles(harness: &UiHarness) -> Vec<egui::epaint::CircleShape> {
    painted_shapes(harness.ui.output())
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Circle(circle) => Some(circle),
            _ => None,
        })
        .collect()
}

/// Click the width slider `fraction` of the way along it.
fn click_slider(harness: &mut UiHarness, fraction: f32) {
    let rect = harness.ui.get_by_label("Text column width").rect();
    let scale = harness.ui.ctx.pixels_per_point();
    let x = egui::lerp(rect.left()..=rect.right(), fraction) / scale;
    harness.click_at(egui::pos2(x, rect.center().y / scale));
}

#[test]
fn the_appearance_button_opens_the_popover_under_itself_and_closes_it_again() {
    let mut harness = UiHarness::with_three_docs();
    assert!(!is_open(&harness));

    harness.click_exact("Appearance");
    assert!(is_open(&harness));
    let button = harness.labelled("Appearance")[0].0;
    for theme in ThemeId::ALL {
        let swatch = harness.labelled(theme.label())[0].0;
        assert!(swatch.y > button.y, "{theme:?} is below the bar");
        assert!(swatch.x < 1200.0, "{theme:?} is inside the window");
    }

    harness.click_exact("Appearance");
    assert!(!is_open(&harness));
}

#[test]
fn the_popover_closes_on_escape_and_on_a_click_elsewhere() {
    let mut harness = popover();
    harness.press(Modifiers::NONE, Key::Escape);
    assert!(!is_open(&harness));

    harness.click_exact("Appearance");
    assert!(is_open(&harness));
    harness.click_at(egui::pos2(300.0, 500.0));
    assert!(!is_open(&harness));
}

#[test]
fn a_swatch_switches_the_theme_at_once_and_leaves_the_popover_open() {
    let mut harness = popover();
    assert!(harness.is_on("Sepia"));

    for theme in [
        ThemeId::Dusk,
        ThemeId::Night,
        ThemeId::Paper,
        ThemeId::Sepia,
    ] {
        harness.click_exact(theme.label());

        assert_eq!(harness.app().theme, theme);
        assert!(is_open(&harness));
        for other in ThemeId::ALL {
            assert_eq!(harness.is_on(other.label()), other == theme, "{other:?}");
        }
        let window = harness.ui.ctx.style().visuals.panel_fill;
        assert_eq!(window, theme.palette().page, "the page follows at once");
    }
}

#[test]
fn each_swatch_is_a_disc_of_its_theme_and_the_one_in_use_is_ringed() {
    let mut harness = popover();
    harness.click_exact("Dusk");
    harness.hover_at(egui::pos2(300.0, 500.0));
    let accent = ThemeId::Dusk.palette().accent;

    let painted = circles(&harness);
    let disc = |theme: ThemeId| {
        let page = theme.palette().page;
        let found = painted.iter().find(|circle| circle.fill == page);
        *found.unwrap_or_else(|| panic!("no disc for {theme:?}"))
    };
    for theme in ThemeId::ALL {
        assert_close(disc(theme).radius, SWATCH_DIAMETER / 2.0);
    }
    let rings: Vec<_> = painted
        .iter()
        .filter(|circle| circle.stroke.color == accent && circle.fill == Color32::TRANSPARENT)
        .collect();
    let [ring] = rings[..] else {
        panic!("one ring: {rings:?}");
    };
    assert_eq!(ring.center, disc(ThemeId::Dusk).center);
    assert!(ring.radius > disc(ThemeId::Dusk).radius);
    assert_close(ring.stroke.width, 2.0);
}

#[test]
fn each_swatch_writes_aa_in_the_ink_of_its_theme() {
    let harness = popover();

    let inks: Vec<Color32> = painted_shapes(harness.ui.output())
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == "Aa" && text.pos.y > 40.0 => {
                Some(text.fallback_color)
            }
            _ => None,
        })
        .collect();
    assert_eq!(inks, ThemeId::ALL.map(|theme| theme.palette().text));
}

#[test]
fn the_two_letters_step_the_text_size_and_name_the_size_in_force() {
    let mut harness = popover();
    assert_eq!(harness.app().fonts.size, TextSizeClass::Normal);
    assert!(harness.shows("Normal"));

    harness.click_exact("Larger text");
    assert_eq!(harness.app().fonts.size, TextSizeClass::Large);
    assert!(harness.shows("Large"));
    assert!(is_open(&harness));

    harness.click_exact("Smaller text");
    harness.click_exact("Smaller text");
    assert_eq!(harness.app().fonts.size, TextSizeClass::Small);
    assert!(harness.shows("Small"));
}

#[test]
fn the_letters_go_dim_at_the_ends_of_the_scale() {
    let mut harness = popover();
    harness.app_mut().fonts.size = TextSizeClass::Small;
    harness.settle();
    assert_eq!(harness.enabled("Smaller text"), [false]);
    assert_eq!(harness.enabled("Larger text"), [true]);

    harness.click_exact("Smaller text");
    assert_eq!(harness.app().fonts.size, TextSizeClass::Small);

    harness.app_mut().fonts.size = TextSizeClass::Huge;
    harness.settle();
    assert_eq!(harness.enabled("Smaller text"), [true]);
    assert_eq!(harness.enabled("Larger text"), [false]);
    harness.click_exact("Larger text");
    assert_eq!(harness.app().fonts.size, TextSizeClass::Huge);
}

#[test]
fn the_slider_sets_the_column_anywhere_between_the_narrowest_and_the_window() {
    let mut harness = popover();
    let pane = harness.app().document_pane_width;
    let widest = ReadingWidth::FIT_WINDOW.resolve(pane);
    assert!(harness.shows("680 px"));

    click_slider(&mut harness, 0.0);
    assert_close(harness.app().reading_width.get(), ReadingWidth::MIN);
    assert!(harness.shows("280 px"));

    click_slider(&mut harness, 0.5);
    let middle = harness.app().reading_width.get();
    let halfway = egui::lerp(ReadingWidth::MIN..=widest, 0.5);
    assert!((middle - halfway).abs() < 12.0, "{middle} near {halfway}");
    assert!(harness.shows(&format!("{middle:.0} px")));

    click_slider(&mut harness, 1.0);
    assert_close(harness.app().reading_width.get(), widest);
    assert!(is_open(&harness));
}

#[test]
fn fit_window_and_reset_set_the_two_widths_worth_a_button() {
    let mut harness = popover();

    harness.click_exact("Fit window");
    assert!(harness.app().reading_width.fits_window());
    assert!(harness.shows("Fits the window"));
    assert!(!harness.shows(" px"));

    harness.click_exact("Reset width");
    assert_eq!(harness.app().reading_width, ReadingWidth::DEFAULT);
    assert!(harness.shows("680 px"));
    assert!(is_open(&harness));
}

#[test]
fn the_slider_rests_at_its_end_while_the_column_fits_the_window() {
    let mut harness = popover();
    harness.click_exact("Fit window");

    let slider = harness.ui.get_by_label("Text column width");
    let value = slider.accesskit_node().numeric_value().unwrap();
    let widest = f64::from(widest_column(harness.app().document_pane_width));
    assert!((value - widest).abs() < 0.5, "{value} at {widest}");
}

#[test]
fn the_three_steps_set_the_line_spacing_and_show_the_one_in_force() {
    let mut harness = popover();
    assert!(harness.is_on("Comfortable"));

    for spacing in [LineSpacing::Airy, LineSpacing::Compact] {
        harness.click_exact(spacing.label());

        assert_eq!(harness.app().line_spacing, spacing);
        for other in LineSpacing::ALL {
            assert_eq!(harness.is_on(other.label()), other == spacing);
        }
        assert!(is_open(&harness));
    }
}

#[test]
fn fonts_opens_the_font_dialog_and_puts_the_popover_away() {
    let mut harness = popover();
    assert!(!harness.app().font_picker.open);

    harness.click("Fonts…");

    assert!(harness.app().font_picker.open);
    assert!(!is_open(&harness));
}

#[test]
fn the_popover_is_reached_and_used_from_the_keyboard() {
    let mut harness = UiHarness::with_three_docs();
    harness.ui.get_by_label("Appearance").focus();
    harness.settle();

    harness.press(Modifiers::NONE, Key::Enter);
    assert!(is_open(&harness));

    // The first stop after the button is the first swatch.
    harness.press(Modifiers::NONE, Key::Tab);
    harness.press(Modifiers::NONE, Key::Enter);
    assert_eq!(harness.app().theme, ThemeId::Paper);
    harness.press(Modifiers::NONE, Key::Tab);
    harness.press(Modifiers::NONE, Key::Tab);
    harness.press(Modifiers::NONE, Key::Enter);
    assert_eq!(harness.app().theme, ThemeId::Dusk);
}

#[test]
fn the_slider_reaches_the_column_that_fits_the_pane() {
    assert_close(
        widest_column(1000.0),
        ReadingWidth::FIT_WINDOW.resolve(1000.0),
    );
}

#[test]
fn the_slider_keeps_a_range_in_a_pane_narrower_than_the_narrowest_column() {
    assert_close(widest_column(100.0), ReadingWidth::MIN + ReadingWidth::STEP);
}
