use super::*;

use crate::app::tests::Harness;

#[test]
fn a_history_action_moves_one_step_and_stops_at_either_end() {
    let mut harness = Harness::with_three_docs();
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));
    harness.app.navigate_active_tab_to(&b);

    harness.apply(AppAction::NavigateForward);
    assert_eq!(harness.active_path(), b, "nothing lies ahead of the newest");
    harness.apply(AppAction::NavigateBack);
    assert_eq!(harness.active_path(), a);
    harness.apply(AppAction::NavigateBack);
    assert_eq!(harness.active_path(), a, "nothing lies behind the oldest");
    harness.apply(AppAction::NavigateForward);
    assert_eq!(harness.active_path(), b);
    assert!(harness.app.error_message.is_none());
}

#[test]
fn menu_only_actions_change_their_own_setting() {
    let mut harness = Harness::with_three_docs();
    let (full_width, smooth_text) = (
        harness.app.full_width_content,
        harness.app.smooth_text_rendering,
    );

    harness.apply(AppAction::ToggleFullWidth);
    assert_eq!(harness.app.full_width_content, !full_width);
    assert_eq!(harness.app.smooth_text_rendering, smooth_text);
    harness.apply(AppAction::ToggleSmoothText);
    assert_eq!(harness.app.smooth_text_rendering, !smooth_text);

    harness.apply(AppAction::SetFormulaScale(1.5));
    assert!((harness.app.math_scale - 1.5).abs() < 1e-6);

    assert!(!harness.app.show_color_settings_dialog);
    harness.apply(AppAction::OpenColorSettings);
    assert!(harness.app.show_color_settings_dialog);
    assert!(!harness.app.show_font_dialog);
    harness.apply(AppAction::OpenFontSettings);
    assert!(harness.app.show_font_dialog);
}

#[test]
fn document_zoom_stays_within_its_bounds() {
    let mut harness = Harness::with_three_docs();

    for _ in 0..20 {
        harness.apply(AppAction::ZoomBy(1.25));
    }
    assert!((harness.app.zoom_level - 3.0).abs() < 1e-6);
    for _ in 0..40 {
        harness.apply(AppAction::ZoomBy(0.8));
    }
    assert!((harness.app.zoom_level - 0.5).abs() < 1e-6);
    harness.apply(AppAction::ResetZoom);
    assert!((harness.app.zoom_level - 1.0).abs() < 1e-6);
}

#[test]
fn lightbox_zoom_actions_do_nothing_without_a_lightbox() {
    let mut harness = Harness::with_three_docs();

    harness.apply(AppAction::ZoomLightboxBy(1.25));
    harness.apply(AppAction::ResetLightboxZoom);

    assert!(harness.app.lightbox.is_none());
    assert!((harness.app.zoom_level - 1.0).abs() < 1e-6);
}
