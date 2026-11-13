use super::*;

use crate::app::tests::Harness;
use crate::tab::SaveOutcome;
use crate::test_support::assert_close;

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
    let smooth_text = harness.app.smooth_text_rendering;
    assert!(!harness.app.reading_width.fits_window());

    harness.apply(AppAction::ToggleFullWidth);
    assert!(harness.app.reading_width.fits_window());
    harness.apply(AppAction::ToggleFullWidth);
    assert_eq!(harness.app.reading_width, ReadingWidth::DEFAULT);
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
fn a_save_that_cannot_be_written_is_reported_and_flagged() {
    let mut harness = Harness::with_docs(&[("a.md", "original")]);
    harness.app.tabs[0].set_edit_mode(true);
    harness.app.tabs[0].content = "edited".to_owned();
    std::fs::remove_dir_all(&harness.dir).unwrap();

    harness.apply(AppAction::Save);

    assert!(matches!(
        harness.app.tabs[0].save_flash,
        Some((SaveOutcome::Failed, _))
    ));
    let message = harness.app.error_message.clone().unwrap();
    assert!(
        message.contains("a.md"),
        "the message names the file: {message}"
    );
}

#[test]
fn unsaved_edits_are_saved_from_the_preview_as_from_the_editor() {
    let mut harness = Harness::with_docs(&[("a.md", "original")]);
    let on_disk = |harness: &Harness| std::fs::read_to_string(harness.path("a.md")).unwrap();

    harness.apply(AppAction::Save);
    assert!(
        harness.app.tabs[0].save_flash.is_none(),
        "there was nothing to save"
    );

    harness.app.tabs[0].set_edit_mode(true);
    harness.app.tabs[0].content = "edited".to_owned();
    harness.app.tabs[0].set_edit_mode(false);
    harness.apply(AppAction::Save);

    assert_eq!(on_disk(&harness), "edited");
    assert!(!harness.app.tabs[0].is_dirty());
    assert!(matches!(
        harness.app.tabs[0].save_flash,
        Some((SaveOutcome::Saved, _))
    ));
}

#[test]
fn document_actions_without_a_document_do_nothing() {
    let mut harness = Harness::with_three_docs();
    harness.apply(AppAction::CloseTab);
    assert!(harness.app.tabs.is_empty());

    harness.apply(AppAction::Save);
    harness.apply(AppAction::Scroll(KeyboardScrollAction::PageDown));
    harness.apply(AppAction::CloseTab);

    assert!(harness.app.tabs.is_empty());
    assert!(harness.app.error_message.is_none());
}

#[test]
fn lightbox_zoom_actions_do_nothing_without_a_lightbox() {
    let mut harness = Harness::with_three_docs();

    harness.apply(AppAction::ZoomLightboxBy(1.25));
    harness.apply(AppAction::ResetLightboxZoom);

    assert!(harness.app.lightbox.is_none());
    assert!((harness.app.zoom_level - 1.0).abs() < 1e-6);
}

#[test]
fn theme_actions_pick_a_theme_or_step_to_the_next() {
    let mut harness = Harness::with_three_docs();

    harness.apply(AppAction::SetTheme(ThemeId::Night));
    assert_eq!(harness.app.theme, ThemeId::Night);
    harness.apply(AppAction::NextTheme);
    assert_eq!(harness.app.theme, ThemeId::Paper);
    harness.apply(AppAction::NextTheme);
    assert_eq!(harness.app.theme, ThemeId::Sepia);
}

#[test]
fn a_column_width_is_set_inside_its_range() {
    let mut harness = Harness::with_three_docs();

    harness.apply(AppAction::SetReadingWidth(512.0));
    assert_close(harness.app.reading_width.get(), 512.0);
    harness.apply(AppAction::SetReadingWidth(1.0));
    assert_close(harness.app.reading_width.get(), ReadingWidth::MIN);
    harness.apply(AppAction::SetReadingWidth(ReadingWidth::MAX));
    assert!(harness.app.reading_width.fits_window());
    harness.apply(AppAction::SetReadingWidth(f32::NAN));
    assert_eq!(harness.app.reading_width, ReadingWidth::DEFAULT);
}

#[test]
fn a_width_step_before_any_frame_starts_from_the_stored_width() {
    let mut harness = Harness::with_three_docs();

    // No frame has measured the pane yet.
    harness.apply(AppAction::StepReadingWidth(ReadingWidth::STEP));
    assert_close(harness.app.reading_width.get(), 720.0);
    harness.apply(AppAction::StepReadingWidth(-2.0 * ReadingWidth::STEP));
    assert_close(harness.app.reading_width.get(), 640.0);
}

#[test]
fn a_new_line_spacing_lays_every_open_document_out_again() {
    let mut harness = Harness::with_three_docs();
    let b = harness.path("b.md");
    harness.app.open_in_new_tab(b);
    let revisions = |harness: &Harness| -> Vec<u64> {
        let tabs = harness.app.tabs.iter();
        tabs.map(|tab| tab.cache.layout_revision()).collect()
    };
    let before = revisions(&harness);
    assert_eq!(before.len(), 2);

    harness.apply(AppAction::SetLineSpacing(LineSpacing::Airy));

    assert_eq!(harness.app.line_spacing, LineSpacing::Airy);
    for (after, before) in revisions(&harness).into_iter().zip(before) {
        assert!(after > before, "{before} -> {after}");
    }
}

#[test]
fn a_new_column_width_lays_documents_out_again_and_the_same_width_does_not() {
    let mut harness = Harness::with_three_docs();
    let revision = |harness: &Harness| harness.app.tabs[0].cache.layout_revision();

    let before = revision(&harness);
    harness.apply(AppAction::SetReadingWidth(512.0));
    let resized = revision(&harness);
    assert!(resized > before);

    harness.apply(AppAction::SetReadingWidth(512.0));
    assert_eq!(revision(&harness), resized, "nothing changed");

    harness.apply(AppAction::StepReadingWidth(ReadingWidth::STEP));
    let stepped = revision(&harness);
    assert!(stepped > resized);
    harness.apply(AppAction::ToggleFullWidth);
    assert!(revision(&harness) > stepped);
}

#[test]
fn the_edit_action_switches_the_active_tab_between_source_and_preview() {
    let mut harness = Harness::with_three_docs();
    let b = harness.path("b.md");
    harness.app.open_in_new_tab(b);
    assert_eq!(harness.app.active_tab, 1);

    harness.apply(AppAction::ToggleEditMode);
    assert!(harness.app.tabs[1].is_editing);
    assert!(!harness.app.tabs[0].is_editing, "only the tab in front");

    harness.apply(AppAction::ToggleEditMode);
    assert!(!harness.app.tabs[1].is_editing);
}

#[test]
fn the_edit_action_does_nothing_without_a_document() {
    let mut harness = Harness::with_docs(&[]);

    harness.apply(AppAction::ToggleEditMode);

    assert!(harness.app.tabs.is_empty());
    assert!(harness.app.error_message.is_none());
}

#[test]
fn a_text_size_action_sets_the_size_class_the_fonts_follow() {
    let mut harness = Harness::with_three_docs();
    harness.idle();
    let body = |harness: &Harness| harness.ctx.style().text_styles[&egui::TextStyle::Body].size;
    let normal = body(&harness);

    harness.apply(AppAction::SetTextSize(TextSizeClass::Huge));
    assert_eq!(harness.app.text_size_class, TextSizeClass::Huge);
    harness.idle();
    harness.idle();

    assert!(body(&harness) > normal, "the next frame sets text larger");
}
