use super::*;

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
    assert_eq!(legacy.font_preset.unwrap_or_default(), FontPreset::Current);
    assert_eq!(legacy.text_size_class, None);
    assert_eq!(
        legacy.text_size_class.unwrap_or_default(),
        TextSizeClass::Normal
    );
}

#[test]
fn persisted_sidebar_widths_are_sanitized() {
    assert_eq!(restored_sidebar_width(Some(320.0), 200.0), 320.0);
    assert_eq!(restored_sidebar_width(Some(20.0), 200.0), SIDEBAR_MIN_WIDTH);
    assert_eq!(restored_sidebar_width(Some(f32::NAN), 200.0), 200.0);
    assert_eq!(restored_sidebar_width(Some(f32::INFINITY), 200.0), 200.0);
    assert_eq!(restored_sidebar_width(None, 200.0), 200.0);
}
