//! Regular, bold, and monospace face selection with script fallbacks.

use std::collections::HashSet;

use egui::{FontData, FontDefinitions, FontFamily};
use egui_commonmark_extended::STRONG_FONT_FAMILY;
use fontique::{
    Blob, Collection, FamilyId, FontStyle, FontWeight, GenericFamily, Script, SourceCache,
    SourceKind,
};

#[cfg(test)]
mod tests;

struct ScriptFallback {
    key: &'static str,
    script: [u8; 4],
    required_glyphs: &'static str,
}

// ISO 15924 identifiers and coverage samples describe scripts, not font
// families. The platform backend remains the sole source of family choices.
const SCRIPT_FALLBACKS: &[ScriptFallback] = &[
    ScriptFallback {
        key: "SystemHanFallback",
        script: *b"Hani",
        required_glyphs: "中文测试繁體",
    },
    ScriptFallback {
        key: "SystemHiraganaFallback",
        script: *b"Hira",
        required_glyphs: "かな",
    },
    ScriptFallback {
        key: "SystemKatakanaFallback",
        script: *b"Kana",
        required_glyphs: "カナ",
    },
    ScriptFallback {
        key: "SystemHangulFallback",
        script: *b"Hang",
        required_glyphs: "한글",
    },
    ScriptFallback {
        key: "SystemArabicFallback",
        script: *b"Arab",
        required_glyphs: "اب",
    },
    ScriptFallback {
        key: "SystemHebrewFallback",
        script: *b"Hebr",
        required_glyphs: "אב",
    },
    ScriptFallback {
        key: "SystemDevanagariFallback",
        script: *b"Deva",
        required_glyphs: "नमस्तेहिन्दी",
    },
    ScriptFallback {
        key: "SystemThaiFallback",
        script: *b"Thai",
        required_glyphs: "สวัสดีภาษาไทย",
    },
];

const GENERIC_FALLBACKS: &[(GenericFamily, &str, &str)] = &[
    (GenericFamily::Math, "SystemMathFallback", "→"),
    (GenericFamily::SansSerif, "SystemSymbolFallback", "⚠"),
];

#[derive(Clone)]
pub(crate) struct SelectedFont {
    blob: Blob<u8>,
    index: u32,
    family_id: FamilyId,
    pub(crate) family: String,
    source: String,
    weight: FontWeight,
}

impl SelectedFont {
    fn identity(&self) -> (u64, u32) {
        (self.blob.id(), self.index)
    }

    fn data(&self) -> FontData {
        let mut data = FontData::from_owned(self.blob.data().to_vec());
        data.index = self.index;
        data
    }
}

pub(crate) struct InstalledFont {
    key: String,
    pub(crate) selected: SelectedFont,
    required_glyphs: &'static str,
    pub(crate) primary: bool,
}

fn font_has_glyphs(blob: &Blob<u8>, index: u32, required_glyphs: &str) -> bool {
    ttf_parser::Face::parse(blob.data(), index)
        .map(|face| {
            required_glyphs
                .chars()
                .all(|c| face.glyph_index(c).is_some())
        })
        .unwrap_or(false)
}

fn source_description(source: &SourceKind) -> String {
    match source {
        SourceKind::Memory(_) => "<memory>".to_owned(),
        SourceKind::Path(path) => path.display().to_string(),
    }
}

pub(crate) fn select_from_families(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
    family_ids: &[FamilyId],
    weight: FontWeight,
    required_glyphs: &str,
    require_true_bold: bool,
) -> Option<SelectedFont> {
    for family_id in family_ids {
        let Some(family) = collection.family(*family_id) else {
            continue;
        };
        let mut candidates: Vec<_> = family
            .fonts()
            .iter()
            .filter(|font| {
                font.style() == FontStyle::Normal
                    && (!require_true_bold || font.weight() >= FontWeight::BOLD)
            })
            .collect();
        candidates.sort_by(|left, right| {
            let left_distance = (left.weight().value() - weight.value()).abs();
            let right_distance = (right.weight().value() - weight.value()).abs();
            left_distance.total_cmp(&right_distance)
        });

        for font in candidates {
            let Some(blob) = font.load(Some(source_cache)) else {
                continue;
            };
            if !font_has_glyphs(&blob, font.index(), required_glyphs) {
                continue;
            }
            return Some(SelectedFont {
                blob,
                index: font.index(),
                family_id: *family_id,
                family: family.name().to_owned(),
                source: source_description(font.source().kind()),
                weight: font.weight(),
            });
        }
    }
    None
}

fn fallback_family_ids(
    collection: &mut Collection,
    script: [u8; 4],
    locale: Option<&str>,
) -> Vec<FamilyId> {
    let script = Script::from_bytes(script);
    match locale {
        Some(locale) => collection.fallback_families((script, locale)).collect(),
        None => collection.fallback_families(script).collect(),
    }
}

fn installed_fonts_cover(installed: &[InstalledFont], glyphs: &str) -> bool {
    installed
        .iter()
        .any(|font| font_has_glyphs(&font.selected.blob, font.selected.index, glyphs))
}

fn install_regular_font(
    definitions: &mut FontDefinitions,
    installed: &mut Vec<InstalledFont>,
    loaded_faces: &mut HashSet<(u64, u32)>,
    key: &'static str,
    selected: SelectedFont,
    required_glyphs: &'static str,
    primary: bool,
) {
    if !loaded_faces.insert(selected.identity()) {
        return;
    }
    log::info!(
        "Loaded system font {} ({}, weight {}) from {} (face index {})",
        key,
        selected.family,
        selected.weight.value(),
        selected.source,
        selected.index
    );
    definitions
        .font_data
        .insert(key.to_owned(), selected.data().into());
    if let Some(family) = definitions.families.get_mut(&FontFamily::Proportional) {
        if primary {
            family.insert(0, key.to_owned())
        } else {
            family.push(key.to_owned())
        }
    }
    if let Some(family) = definitions.families.get_mut(&FontFamily::Monospace) {
        family.push(key.to_owned());
    }
    installed.push(InstalledFont {
        key: key.to_owned(),
        selected,
        required_glyphs,
        primary,
    });
}

pub(crate) fn install_regular_fonts(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
    definitions: &mut FontDefinitions,
    locale: Option<&str>,
    preferred_family: Option<&str>,
    preset_body_families: Option<&'static [&'static str]>,
) -> Vec<InstalledFont> {
    let mut installed = Vec::new();
    let mut loaded_faces = HashSet::new();

    if let Some(name) = preferred_family {
        match collection.family_id(name) {
            Some(family_id) => {
                if let Some(selected) = select_from_families(
                    collection,
                    source_cache,
                    &[family_id],
                    FontWeight::NORMAL,
                    "Aa",
                    false,
                ) {
                    install_regular_font(
                        definitions,
                        &mut installed,
                        &mut loaded_faces,
                        "SystemSans",
                        selected,
                        "Aa",
                        true,
                    );
                } else {
                    log::warn!(
                        "Preferred font '{name}' has no usable regular face; falling back to system default."
                    );
                }
            }
            None => {
                log::warn!("Preferred font '{name}' not found; falling back to system default.");
            }
        }
    }

    // A preset's body families lead the primary sans chain when no user-picked
    // family was installed (the picker's choice outranks the preset's emulated
    // stack). Each stack name is tried in the emulated viewer's resolution
    // order; a total miss falls through to the app's own auto-detection.
    if installed.is_empty() {
        if let Some(preset_families) = preset_body_families {
            let family_ids: Vec<FamilyId> = preset_families
                .iter()
                .filter_map(|name| collection.family_id(name))
                .collect();
            if let Some(selected) = select_from_families(
                collection,
                source_cache,
                &family_ids,
                FontWeight::NORMAL,
                "Aa",
                false,
            ) {
                install_regular_font(
                    definitions,
                    &mut installed,
                    &mut loaded_faces,
                    "SystemSans",
                    selected,
                    "Aa",
                    true,
                );
            } else {
                log::warn!(
                    "Preset body families {preset_families:?} had no usable regular face; falling back to system default."
                );
            }
        }
    }

    // Auto-detect the system sans-serif only if no preferred family was
    // requested, or the preferred family couldn't be installed above.
    if installed.is_empty() {
        let families: Vec<_> = collection
            .generic_families(GenericFamily::SansSerif)
            .collect();
        if let Some(selected) = select_from_families(
            collection,
            source_cache,
            &families,
            FontWeight::NORMAL,
            "Aa",
            false,
        ) {
            install_regular_font(
                definitions,
                &mut installed,
                &mut loaded_faces,
                "SystemSans",
                selected,
                "Aa",
                true,
            );
        }
    }

    for spec in SCRIPT_FALLBACKS {
        if installed_fonts_cover(&installed, spec.required_glyphs) {
            continue;
        }
        let families = fallback_family_ids(collection, spec.script, locale);
        let Some(selected) = select_from_families(
            collection,
            source_cache,
            &families,
            FontWeight::NORMAL,
            spec.required_glyphs,
            false,
        ) else {
            continue;
        };
        install_regular_font(
            definitions,
            &mut installed,
            &mut loaded_faces,
            spec.key,
            selected,
            spec.required_glyphs,
            false,
        );
    }

    for &(generic, key, glyphs) in GENERIC_FALLBACKS {
        if installed_fonts_cover(&installed, glyphs) {
            continue;
        }
        let families: Vec<_> = collection.generic_families(generic).collect();
        let Some(selected) = select_from_families(
            collection,
            source_cache,
            &families,
            FontWeight::NORMAL,
            glyphs,
            false,
        ) else {
            continue;
        };
        install_regular_font(
            definitions,
            &mut installed,
            &mut loaded_faces,
            key,
            selected,
            glyphs,
            false,
        );
    }
    installed
}

fn push_unique(
    destination: &mut Vec<String>,
    seen: &mut HashSet<String>,
    names: impl IntoIterator<Item = String>,
) {
    for name in names {
        if seen.insert(name.clone()) {
            destination.push(name)
        }
    }
}

fn build_strong_family(
    primary_regular: Option<&str>,
    primary_bold: Option<&str>,
    default_proportional: &[String],
    fallback_bold: &[String],
    proportional: &[String],
) -> Vec<String> {
    let mut family = Vec::new();
    let mut seen = HashSet::new();
    push_unique(
        &mut family,
        &mut seen,
        primary_bold.into_iter().map(str::to_owned),
    );
    push_unique(
        &mut family,
        &mut seen,
        primary_regular.into_iter().map(str::to_owned),
    );
    push_unique(&mut family, &mut seen, default_proportional.iter().cloned());
    push_unique(&mut family, &mut seen, fallback_bold.iter().cloned());
    push_unique(&mut family, &mut seen, proportional.iter().cloned());
    family
}

pub(crate) fn install_strong_font_family(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
    definitions: &mut FontDefinitions,
    installed: &[InstalledFont],
    default_proportional: &[String],
) -> usize {
    let mut loaded_faces: HashSet<_> = installed
        .iter()
        .map(|font| font.selected.identity())
        .collect();
    let mut primary_regular = None;
    let mut primary_bold = None;
    let mut fallback_bold = Vec::new();
    let mut count = 0;

    for regular in installed {
        if regular.primary {
            primary_regular = Some(regular.key.clone())
        }
        let Some(selected) = select_from_families(
            collection,
            source_cache,
            &[regular.selected.family_id],
            FontWeight::BOLD,
            regular.required_glyphs,
            true,
        ) else {
            continue;
        };
        if !loaded_faces.insert(selected.identity()) {
            continue;
        }
        let key = format!("{}Bold", regular.key);
        log::info!(
            "Loaded strong font {} ({}, weight {}) from {} (face index {})",
            key,
            selected.family,
            selected.weight.value(),
            selected.source,
            selected.index
        );
        definitions
            .font_data
            .insert(key.clone(), selected.data().into());
        count += 1;
        if regular.primary {
            primary_bold = Some(key)
        } else {
            fallback_bold.push(key)
        }
    }

    let proportional = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let strong = build_strong_family(
        primary_regular.as_deref(),
        primary_bold.as_deref(),
        default_proportional,
        &fallback_bold,
        &proportional,
    );
    if primary_regular.is_some() && primary_bold.is_none() {
        log::warn!("No true bold face found for the primary system sans; using regular fallback.");
    }
    definitions
        .families
        .insert(FontFamily::Name(STRONG_FONT_FAMILY.into()), strong);
    count
}

/// Load fonts through platform generic-family and script/locale fallback rules.
///
/// Fontique delegates to fontconfig on Linux/FreeBSD, DirectWrite on Windows,
/// CoreText on Apple platforms, and the system configuration on Android.
///
/// `preferred_family` optionally names a specific installed family (matched
/// case-insensitively) to use as the primary proportional font instead of the
/// auto-detected system sans-serif; an unset or unresolvable preference falls
/// back to today's auto-detect behavior.
/// Lead the monospace chain with the preset's code face, keeping egui's
/// bundled monospace (and any installed fallbacks) as fallback. Returns
/// whether a preset face was installed.
pub(crate) fn install_preset_mono_font(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
    definitions: &mut FontDefinitions,
    mono_families: &'static [&'static str],
) -> bool {
    let family_ids: Vec<FamilyId> = mono_families
        .iter()
        .filter_map(|name| collection.family_id(name))
        .collect();
    let Some(selected) = select_from_families(
        collection,
        source_cache,
        &family_ids,
        FontWeight::NORMAL,
        "{}",
        false,
    ) else {
        log::warn!(
            "Preset mono families {mono_families:?} had no usable face; keeping the bundled monospace."
        );
        return false;
    };
    definitions
        .font_data
        .insert("PresetMono".to_owned(), selected.data().into());
    if let Some(family) = definitions.families.get_mut(&FontFamily::Monospace) {
        family.insert(0, "PresetMono".to_owned());
    }
    true
}
