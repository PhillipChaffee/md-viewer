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
    /// Where the face was loaded from, for the log.
    pub(crate) source: String,
    weight: FontWeight,
}

impl SelectedFont {
    fn identity(&self) -> (u64, u32) {
        (self.blob.id(), self.index)
    }

    /// The face as egui takes it.
    pub(crate) fn data(&self) -> FontData {
        let mut data = FontData::from_owned(self.blob.data().to_vec());
        data.index = self.index;
        data
    }

    /// Log the face as the `kind` font loaded under `key`.
    fn log_loaded(&self, kind: &str, key: &str) {
        log::info!(
            "Loaded {kind} font {key} ({}, weight {}) from {} (face index {})",
            self.family,
            self.weight.value(),
            self.source,
            self.index
        );
    }
}

pub(crate) struct InstalledFont {
    key: &'static str,
    pub(crate) selected: SelectedFont,
    required_glyphs: &'static str,
    pub(crate) primary: bool,
}

fn font_has_glyphs(blob: &Blob<u8>, index: u32, required_glyphs: &str) -> bool {
    ttf_parser::Face::parse(blob.data(), index).is_ok_and(|face| {
        required_glyphs
            .chars()
            .all(|c| face.glyph_index(c).is_some())
    })
}

fn source_description(source: &SourceKind) -> String {
    match source {
        SourceKind::Memory(_) => "<memory>".to_owned(),
        SourceKind::Path(path) => path.display().to_string(),
    }
}

/// Which face of a family a selection is after.
#[derive(Clone, Copy)]
enum Weight {
    Regular,
    /// A true bold face: one of weight 700 or more, so a medium or semibold
    /// face is never taken for it.
    Bold,
}

impl Weight {
    /// The weight the chosen face should be nearest to.
    const fn target(self) -> FontWeight {
        match self {
            Self::Regular => FontWeight::NORMAL,
            Self::Bold => FontWeight::BOLD,
        }
    }

    fn admits(self, face: FontWeight) -> bool {
        match self {
            Self::Regular => true,
            Self::Bold => face >= FontWeight::BOLD,
        }
    }
}

/// The fonts the platform lists, with the files loaded from them so far.
pub(crate) struct SystemFonts<'a> {
    pub(crate) collection: &'a mut Collection,
    pub(crate) source_cache: &'a mut SourceCache,
}

impl SystemFonts<'_> {
    /// The upright face nearest to `weight` that loads and covers
    /// `required_glyphs`, from the first of `family_ids` that has one.
    fn select(
        &mut self,
        family_ids: &[FamilyId],
        weight: Weight,
        required_glyphs: &str,
    ) -> Option<SelectedFont> {
        let target = weight.target().value();
        for family_id in family_ids {
            let Some(family) = self.collection.family(*family_id) else {
                continue;
            };
            let mut candidates: Vec<_> = family
                .fonts()
                .iter()
                .filter(|font| font.style() == FontStyle::Normal && weight.admits(font.weight()))
                .collect();
            candidates.sort_by(|left, right| {
                let left_distance = (left.weight().value() - target).abs();
                let right_distance = (right.weight().value() - target).abs();
                left_distance.total_cmp(&right_distance)
            });

            for font in candidates {
                let Some(blob) = font.load(Some(self.source_cache)) else {
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

    /// Best regular face among `family_ids` that covers `required_glyphs`.
    pub(crate) fn regular(
        &mut self,
        family_ids: &[FamilyId],
        required_glyphs: &str,
    ) -> Option<SelectedFont> {
        self.select(family_ids, Weight::Regular, required_glyphs)
    }

    /// The installed families among `names`, in the order named.
    fn family_ids(&mut self, names: &[&str]) -> Vec<FamilyId> {
        names
            .iter()
            .filter_map(|name| self.collection.family_id(name))
            .collect()
    }

    /// Regular face of the installed family called `family` that covers
    /// `required_glyphs`, wherever the platform keeps its file.
    pub(crate) fn named(&mut self, family: &str, required_glyphs: &str) -> Option<SelectedFont> {
        let family_id = self.collection.family_id(family)?;
        self.regular(&[family_id], required_glyphs)
    }
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

/// Glyphs a face must cover to serve as the primary sans, which is the
/// document body font. The font picker lists a family only when it has a
/// regular face covering these, which is what guarantees that every listed
/// name takes effect when picked.
pub(crate) const LATIN_SAMPLE: &str = "Aa";

/// Key the primary sans is installed under.
const PRIMARY_SANS_KEY: &str = "SystemSans";

/// Accumulates the regular faces chosen for one `FontDefinitions`, so every
/// selection step shares the platform collection and the faces installed so
/// far.
struct RegularFontInstaller<'fonts, 'system> {
    fonts: &'fonts mut SystemFonts<'system>,
    definitions: &'fonts mut FontDefinitions,
    installed: Vec<InstalledFont>,
}

impl RegularFontInstaller<'_, '_> {
    /// Install the best regular face from `family_ids` that covers
    /// `required_glyphs`. Returns whether any face qualified.
    ///
    /// No face is installed twice: the primary sans is installed first and
    /// once, and a fallback is only looked for when no installed face covers
    /// its glyphs, so the face found for it cannot be one of those.
    fn install_from_families(
        &mut self,
        key: &'static str,
        family_ids: &[FamilyId],
        required_glyphs: &'static str,
        primary: bool,
    ) -> bool {
        let Some(selected) = self.fonts.regular(family_ids, required_glyphs) else {
            return false;
        };
        self.install(key, selected, required_glyphs, primary);
        true
    }

    fn install(
        &mut self,
        key: &'static str,
        selected: SelectedFont,
        required_glyphs: &'static str,
        primary: bool,
    ) {
        selected.log_loaded("system", key);
        self.definitions
            .font_data
            .insert(key.to_owned(), selected.data().into());
        if let Some(family) = self.definitions.families.get_mut(&FontFamily::Proportional) {
            if primary {
                family.insert(0, key.to_owned());
            } else {
                family.push(key.to_owned());
            }
        }
        if let Some(family) = self.definitions.families.get_mut(&FontFamily::Monospace) {
            family.push(key.to_owned());
        }
        self.installed.push(InstalledFont {
            key,
            selected,
            required_glyphs,
            primary,
        });
    }

    /// Install the primary sans from the first source that yields a usable
    /// face: the user's pick, then the preset's emulated stack, then the
    /// platform's generic sans-serif.
    fn install_primary_sans(
        &mut self,
        preferred_family: Option<&str>,
        preset_body_families: Option<&'static [&'static str]>,
    ) {
        if preferred_family.is_some_and(|name| self.install_preferred_family(name)) {
            return;
        }
        if preset_body_families.is_some_and(|names| self.install_preset_body_family(names)) {
            return;
        }
        self.install_generic_sans();
    }

    fn install_preferred_family(&mut self, name: &str) -> bool {
        let Some(family_id) = self.fonts.collection.family_id(name) else {
            log::warn!("Preferred font '{name}' not found; falling back to system default.");
            return false;
        };
        let installed =
            self.install_from_families(PRIMARY_SANS_KEY, &[family_id], LATIN_SAMPLE, true);
        if !installed {
            log::warn!(
                "Preferred font '{name}' has no usable regular face; falling back to system default."
            );
        }
        installed
    }

    /// Each stack name is tried in the emulated viewer's resolution order; a
    /// total miss falls through to the app's own auto-detection.
    fn install_preset_body_family(&mut self, preset_families: &'static [&'static str]) -> bool {
        let family_ids = self.fonts.family_ids(preset_families);
        let installed =
            self.install_from_families(PRIMARY_SANS_KEY, &family_ids, LATIN_SAMPLE, true);
        if !installed {
            log::warn!(
                "Preset body families {preset_families:?} had no usable regular face; falling back to system default."
            );
        }
        installed
    }

    fn install_generic_sans(&mut self) -> bool {
        let families: Vec<_> = self
            .fonts
            .collection
            .generic_families(GenericFamily::SansSerif)
            .collect();
        self.install_from_families(PRIMARY_SANS_KEY, &families, LATIN_SAMPLE, true)
    }

    /// Add one fallback face per script the faces installed so far cannot
    /// render.
    fn install_script_fallbacks(&mut self, locale: Option<&str>) {
        for spec in SCRIPT_FALLBACKS {
            if installed_fonts_cover(&self.installed, spec.required_glyphs) {
                continue;
            }
            let families = fallback_family_ids(self.fonts.collection, spec.script, locale);
            self.install_from_families(spec.key, &families, spec.required_glyphs, false);
        }
    }

    fn install_generic_fallbacks(&mut self) {
        for &(generic, key, glyphs) in GENERIC_FALLBACKS {
            if installed_fonts_cover(&self.installed, glyphs) {
                continue;
            }
            let families: Vec<_> = self.fonts.collection.generic_families(generic).collect();
            self.install_from_families(key, &families, glyphs, false);
        }
    }
}

/// Load fonts through platform generic-family and script/locale fallback rules.
///
/// Fontique delegates to fontconfig on Linux/FreeBSD, `DirectWrite` on Windows,
/// `CoreText` on Apple platforms, and the system configuration on Android.
///
/// `preferred_family` optionally names a specific installed family (matched
/// case-insensitively) to use as the primary proportional font instead of the
/// auto-detected system sans-serif; an unset or unresolvable preference falls
/// back to today's auto-detect behavior.
///
/// A preset's body families lead the primary sans chain when no user-picked
/// family was installed (the picker's choice outranks the preset's emulated
/// stack).
pub(crate) fn install_regular_fonts(
    fonts: &mut SystemFonts<'_>,
    definitions: &mut FontDefinitions,
    locale: Option<&str>,
    preferred_family: Option<&str>,
    preset_body_families: Option<&'static [&'static str]>,
) -> Vec<InstalledFont> {
    let mut installer = RegularFontInstaller {
        fonts,
        definitions,
        installed: Vec::new(),
    };
    installer.install_primary_sans(preferred_family, preset_body_families);
    installer.install_script_fallbacks(locale);
    installer.install_generic_fallbacks();
    installer.installed
}

/// The chain strong text is drawn from: each name once, where it first
/// comes in this order.
fn build_strong_family(
    primary_regular: Option<&str>,
    primary_bold: Option<&str>,
    default_proportional: &[String],
    fallback_bold: &[String],
    proportional: &[String],
) -> Vec<String> {
    let mut seen = HashSet::new();
    primary_bold
        .into_iter()
        .chain(primary_regular)
        .chain(default_proportional.iter().map(String::as_str))
        .chain(fallback_bold.iter().map(String::as_str))
        .chain(proportional.iter().map(String::as_str))
        .filter(|name| seen.insert(*name))
        .map(str::to_owned)
        .collect()
}

pub(crate) fn install_strong_font_family(
    fonts: &mut SystemFonts<'_>,
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
            primary_regular = Some(regular.key);
        }
        let Some(selected) = fonts.select(
            &[regular.selected.family_id],
            Weight::Bold,
            regular.required_glyphs,
        ) else {
            continue;
        };
        if !loaded_faces.insert(selected.identity()) {
            continue;
        }
        let key = format!("{}Bold", regular.key);
        selected.log_loaded("strong", &key);
        definitions
            .font_data
            .insert(key.clone(), selected.data().into());
        count += 1;
        if regular.primary {
            primary_bold = Some(key);
        } else {
            fallback_bold.push(key);
        }
    }

    let proportional = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let strong = build_strong_family(
        primary_regular,
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

/// Lead the monospace chain with the preset's code face, keeping egui's
/// bundled monospace (and any installed fallbacks) as fallback. Returns
/// whether a preset face was installed.
pub(crate) fn install_preset_mono_font(
    fonts: &mut SystemFonts<'_>,
    definitions: &mut FontDefinitions,
    mono_families: &'static [&'static str],
) -> bool {
    let family_ids = fonts.family_ids(mono_families);
    let Some(selected) = fonts.regular(&family_ids, "{}") else {
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
