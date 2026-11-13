//! The Colors and Fonts dialogs, clicked the way a user clicks them.

use std::sync::mpsc::{self, Sender};

use eframe::egui::{accesskit, Color32, TextStyle};
use egui_kittest::kittest::Queryable as _;

use crate::app::ui_harness::UiHarness;
use crate::system_fonts::{FontPreset, TextSizeClass};
use crate::test_support::assert_close;

const RED: Color32 = Color32::from_rgb(200, 30, 30);
const GREEN: Color32 = Color32::from_rgb(30, 160, 60);

/// The highlight and link colors egui currently draws with.
fn drawn_colors(harness: &UiHarness) -> (Color32, Color32) {
    let visuals = harness.ui.ctx.style().visuals.clone();
    (visuals.selection.bg_fill, visuals.hyperlink_color)
}

/// Open the color picker of the `index`th row of the Colors dialog and
/// switch its blending, the one edit the picker offers as a labelled widget.
fn change_color(harness: &mut UiHarness, index: usize) {
    let well = harness.centers_with_role(accesskit::Role::ColorWell)[index];
    harness.click_at(well);
    harness.click("Additive");
}

#[test]
fn picking_a_color_overrides_the_theme_color_it_belongs_to() {
    let mut harness = UiHarness::with_three_docs();
    let (theme_highlight, theme_link) = drawn_colors(&harness);
    harness.pick("Colors");

    change_color(&mut harness, 0);

    let highlight = harness.app().highlight_color.unwrap();
    assert_ne!(highlight, theme_highlight);
    assert_eq!(harness.app().link_color, None);
    assert_eq!(drawn_colors(&harness), (highlight, theme_link));

    change_color(&mut harness, 1);

    let link = harness.app().link_color.unwrap();
    assert_ne!(link, theme_link);
    assert_eq!(harness.app().highlight_color, Some(highlight));
    assert_eq!(drawn_colors(&harness), (highlight, link));
}

#[test]
fn reset_is_offered_per_override_and_brings_the_theme_color_back() {
    let mut harness = UiHarness::with_three_docs();
    let (theme_highlight, theme_link) = drawn_colors(&harness);
    harness.pick("Colors");
    assert_eq!(harness.enabled("Reset"), [false, false]);

    harness.app_mut().highlight_color = Some(RED);
    harness.app_mut().link_color = Some(GREEN);
    harness.settle();
    assert_eq!(harness.enabled("Reset"), [true, true]);
    assert_eq!(drawn_colors(&harness), (RED, GREEN));

    let highlight_reset = harness.labelled("Reset")[0].0;
    harness.click_at(highlight_reset);
    assert_eq!(harness.app().highlight_color, None);
    assert_eq!(harness.app().link_color, Some(GREEN));
    assert_eq!(drawn_colors(&harness), (theme_highlight, GREEN));
    assert_eq!(harness.enabled("Reset"), [false, true]);

    let link_reset = harness.labelled("Reset")[1].0;
    harness.click_at(link_reset);
    assert_eq!(harness.app().link_color, None);
    assert_eq!(drawn_colors(&harness), (theme_highlight, theme_link));
}

#[test]
fn each_dialog_closes_from_its_title_bar_and_stays_closed() {
    let mut harness = UiHarness::with_three_docs();

    harness.pick("Colors");
    assert!(harness.shows("Highlight Color:"));
    harness.click("Close window");
    assert!(!harness.app().show_color_settings_dialog);
    assert!(!harness.shows("Highlight Color:"));

    harness.pick("Fonts");
    assert!(harness.shows("Text Size"));
    harness.click("Close window");
    assert!(!harness.app().show_font_dialog);
    assert!(!harness.shows("Text Size"));
}

/// Open the Fonts dialog with the installed-font scan still running. The
/// returned sender finishes the scan with whatever families a test sends.
fn fonts_dialog_while_scanning() -> (UiHarness, Sender<Vec<String>>) {
    let mut harness = UiHarness::with_three_docs();
    let (scan_tx, scan_rx) = mpsc::channel();
    harness.app_mut().pending_font_family_scan = Some(scan_rx);
    harness.app_mut().show_font_dialog = true;
    harness.frames(2);
    (harness, scan_tx)
}

/// Open the Fonts dialog offering exactly `families`.
fn fonts_dialog(families: &[&str]) -> UiHarness {
    let (mut harness, scan_tx) = fonts_dialog_while_scanning();
    let families = families.iter().map(|name| (*name).to_owned()).collect();
    scan_tx.send(families).unwrap();
    harness.settle();
    harness
}

#[test]
fn the_family_list_shows_a_note_until_the_scan_lands() {
    let (mut harness, scan_tx) = fonts_dialog_while_scanning();
    assert!(harness.shows("Scanning installed fonts"));
    assert!(harness.shows("System Default"));

    scan_tx.send(vec!["Alpha Sans".to_owned()]).unwrap();
    harness.settle();

    assert!(!harness.shows("Scanning installed fonts"));
    assert!(harness.shows("Alpha Sans"));
}

#[test]
fn a_scan_that_dies_leaves_only_the_system_default() {
    let (mut harness, scan_tx) = fonts_dialog_while_scanning();

    drop(scan_tx);
    harness.settle();

    assert!(!harness.shows("Scanning installed fonts"));
    assert!(harness.shows("System Default"));
    assert!(harness.app().available_font_families.is_empty());
}

#[test]
fn clicking_a_family_selects_it_and_system_default_clears_it() {
    let mut harness = fonts_dialog(&["Alpha Sans", "Beta Serif"]);
    assert_eq!(harness.app().fonts.family, None);
    assert!(!harness.shows("A picked family leads the body chain"));

    harness.click("Beta Serif");
    assert_eq!(harness.app().fonts.family.as_deref(), Some("Beta Serif"));
    assert!(harness.shows("A picked family leads the body chain"));

    harness.click("System Default");
    assert_eq!(harness.app().fonts.family, None);
}

#[test]
fn the_search_box_narrows_the_list_without_regard_to_case() {
    let mut harness = fonts_dialog(&["Alpha Sans", "Beta Serif", "Gamma Beta"]);
    let search_box = harness.centers_with_role(accesskit::Role::TextInput)[0];
    harness.click_at(search_box);

    harness
        .ui
        .get_by_role(accesskit::Role::TextInput)
        .type_text("BETA");
    harness.settle();

    assert!(!harness.shows("Alpha Sans"));
    assert!(harness.shows("Beta Serif"));
    assert!(
        harness.shows("System Default"),
        "the pinned row always stays"
    );

    harness.click("Gamma Beta");
    assert_eq!(harness.app().fonts.family.as_deref(), Some("Gamma Beta"));
}

#[test]
fn a_preset_click_switches_the_preset() {
    let mut harness = fonts_dialog(&[]);
    assert_eq!(harness.app().fonts.preset, FontPreset::Reader);

    harness.click_exact("System");
    assert_eq!(harness.app().fonts.preset, FontPreset::Current);
    harness.click_exact("Reader");
    assert_eq!(harness.app().fonts.preset, FontPreset::Reader);
    harness.click_exact("GitHub");
    assert_eq!(harness.app().fonts.preset, FontPreset::Github);
    harness.click_exact("VS Code");
    assert_eq!(harness.app().fonts.preset, FontPreset::Vscode);
    harness.click_exact("VS Code");
    assert_eq!(harness.app().fonts.preset, FontPreset::Vscode);
}

#[test]
fn a_text_size_click_resizes_the_body_text() {
    let mut harness = fonts_dialog(&[]);
    let body_size = |harness: &UiHarness| -> f32 {
        let style = harness.ui.ctx.style();
        style.text_styles[&TextStyle::Body].size
    };
    assert_eq!(harness.app().fonts.size, TextSizeClass::Normal);
    let normal = body_size(&harness);

    harness.click_exact(TextSizeClass::Large.label());

    assert_eq!(harness.app().fonts.size, TextSizeClass::Large);
    assert_close(
        body_size(&harness) / normal,
        TextSizeClass::Large.px() / TextSizeClass::Normal.px(),
    );
}
