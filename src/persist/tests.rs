use super::*;
use crate::test_support::assert_close;

use std::collections::HashMap;

/// In-memory stand-in for eframe's key/value storage so persistence
/// roundtrips run through the same RON encode/decode path as the app.
struct MemoryStorage(HashMap<String, String>);

impl eframe::Storage for MemoryStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn flush(&mut self) {}
}

#[test]
fn font_preset_survives_a_persistence_roundtrip() {
    let mut storage = MemoryStorage(HashMap::new());
    let app_state = PersistedState {
        font_preset: Some(FontPreset::Vscode),
        text_size_class: Some(TextSizeClass::Large),
        ..PersistedState::default()
    };
    eframe::set_value(&mut storage, APP_KEY, &app_state);

    let restored: PersistedState = eframe::get_value(&storage, APP_KEY).expect("state decodes");

    assert_eq!(restored.font_preset, Some(FontPreset::Vscode));
    assert_eq!(restored.text_size_class, Some(TextSizeClass::Large));

    // Absent field (older state files) falls back to the default preset.
    let legacy: PersistedState = eframe::get_value(
        &MemoryStorage(HashMap::from([(
            APP_KEY.to_owned(),
            "(dark_mode: Some(true))".to_owned(),
        )])),
        APP_KEY,
    )
    .expect("legacy state decodes");
    assert_eq!(legacy.font_preset, None);
    assert_eq!(legacy.font_preset.unwrap_or_default(), FontPreset::Reader);
    assert_eq!(legacy.text_size_class, None);
    assert_eq!(
        legacy.text_size_class.unwrap_or_default(),
        TextSizeClass::Normal
    );
}

#[test]
fn persisted_sidebar_widths_are_sanitized() {
    assert_close(restored_sidebar_width(Some(320.0), 200.0), 320.0);
    assert_close(restored_sidebar_width(Some(20.0), 200.0), SIDEBAR_MIN_WIDTH);
    assert_close(restored_sidebar_width(Some(f32::NAN), 200.0), 200.0);
    assert_close(restored_sidebar_width(Some(f32::INFINITY), 200.0), 200.0);
    assert_close(restored_sidebar_width(None, 200.0), 200.0);
}

#[test]
fn theme_column_width_and_line_spacing_survive_a_persistence_roundtrip() {
    let mut storage = MemoryStorage(HashMap::new());
    let state = PersistedState {
        theme: Some(ThemeId::Night),
        reading_width: Some(512.0),
        line_spacing: Some(LineSpacing::Airy),
        ..PersistedState::default()
    };
    eframe::set_value(&mut storage, APP_KEY, &state);

    let restored: PersistedState = eframe::get_value(&storage, APP_KEY).expect("state decodes");

    assert_eq!(restored.theme, Some(ThemeId::Night));
    assert_close(restored.reading_width.unwrap(), 512.0);
    assert_eq!(restored.line_spacing, Some(LineSpacing::Airy));
}

#[test]
fn the_settings_the_reader_replaced_are_read_but_never_written() {
    let legacy: PersistedState = eframe::get_value(
        &MemoryStorage(HashMap::from([(
            APP_KEY.to_owned(),
            "(dark_mode: Some(true), full_width_content: Some(true))".to_owned(),
        )])),
        APP_KEY,
    )
    .expect("legacy state decodes");
    assert_eq!(legacy.dark_mode, Some(true));
    assert_eq!(legacy.full_width_content, Some(true));
    assert_eq!(legacy.theme, None);
    assert_eq!(legacy.reading_width, None);
    assert_eq!(legacy.line_spacing, None);

    let mut storage = MemoryStorage(HashMap::new());
    eframe::set_value(&mut storage, APP_KEY, &legacy);
    let written = &storage.0[APP_KEY];
    assert!(!written.contains("dark_mode"), "{written}");
    assert!(!written.contains("full_width_content"), "{written}");
    assert!(written.contains("theme"), "{written}");
}

#[test]
fn a_color_comes_back_from_its_saved_bytes_unchanged() {
    // Half-transparent, so the bytes differ from the unmultiplied color's.
    let color = Color32::from_rgba_unmultiplied(200, 120, 40, 128);

    let bytes = color_to_persisted(color);

    assert_eq!(bytes, [color.r(), color.g(), color.b(), color.a()]);
    assert_eq!(color_from_persisted(bytes), color);
    assert_eq!(
        color_from_persisted([10, 20, 30, 255]),
        Color32::from_rgb(10, 20, 30)
    );
}

#[test]
fn a_file_that_holds_no_settings_reads_as_the_defaults() {
    let fresh = PersistedState::default();

    assert_eq!(fresh.theme(), ThemeId::default());
    assert_close(fresh.zoom(), 1.0);
    assert!(fresh.show_outline());
    assert_eq!(fresh.reading_width(), ReadingWidth::DEFAULT);
    assert_eq!(fresh.line_spacing(), LineSpacing::Comfortable);
    assert_close(fresh.math_scale(), 1.0);
    assert_eq!(fresh.highlight_color(), None);
    assert_eq!(fresh.link_color(), None);
    assert!(!fresh.smooth_text_rendering());
    assert_eq!(fresh.font_preset(), FontPreset::Reader);
    assert_eq!(fresh.text_size_class(), TextSizeClass::Normal);
    assert!(!fresh.show_explorer());
    assert_close(fresh.explorer_width(), EXPLORER_DEFAULT_WIDTH);
    assert_close(fresh.outline_width(), OUTLINE_DEFAULT_WIDTH);
}

#[test]
fn saved_settings_are_read_back_with_out_of_range_numbers_pulled_in() {
    let saved = PersistedState {
        dark_mode: Some(true),
        zoom_level: Some(9.0),
        show_outline: Some(false),
        full_width_content: Some(true),
        math_scale: Some(0.2),
        highlight_color: Some([10, 20, 30, 255]),
        smooth_text_rendering: Some(true),
        show_explorer: Some(true),
        explorer_width: Some(12.0),
        outline_width: Some(300.0),
        ..PersistedState::default()
    };

    assert_eq!(saved.theme(), ThemeId::Dusk);
    assert_close(saved.zoom(), ZOOM_MAX);
    assert!(!saved.show_outline());
    assert_eq!(saved.reading_width(), ReadingWidth::FIT_WINDOW);
    assert_close(saved.math_scale(), MATH_SCALE_MIN);
    assert_eq!(saved.highlight_color(), Some(Color32::from_rgb(10, 20, 30)));
    assert!(saved.smooth_text_rendering());
    assert!(saved.show_explorer());
    assert_close(saved.explorer_width(), SIDEBAR_MIN_WIDTH);
    assert_close(saved.outline_width(), 300.0);
}
