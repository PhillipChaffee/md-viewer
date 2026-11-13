use super::*;

use std::collections::HashSet;
use std::sync::Arc;

use egui::{FontId, TextStyle};
use egui_commonmark_extended::STRONG_FONT_FAMILY;
use fontique::{Blob, FontInfoOverride, FontWeight, GenericFamily, Script};

use crate::test_support::assert_close;

/// Subset of GNU Unifont holding only the script coverage samples and no
/// Latin letters. See `fixtures/README.txt` for how it was produced.
pub(crate) const SCRIPT_SAMPLES_FONT: &[u8] = include_bytes!("fixtures/script-samples.otf");

/// Light Latin text face egui ships: letters, but no arrows or symbols.
pub(crate) const LATIN_FACE: &str = "Ubuntu-Light";
/// Monospace Latin face egui ships; it also maps arrows such as →.
pub(crate) const MONO_FACE: &str = "Hack";
/// Emoji and symbol face egui ships: ⚠, but no Latin letters.
pub(crate) const SYMBOL_FACE: &str = "NotoEmoji-Regular";

/// The bytes of one of the faces egui ships, to register under whatever
/// family a test needs.
pub(crate) fn bundled_face(key: &str) -> Vec<u8> {
    FontDefinitions::default().font_data[key].font.to_vec()
}

/// Every script sample `SCRIPT_SAMPLES_FONT` covers.
pub(crate) const SCRIPT_SAMPLES: [(&[u8; 4], &str); 8] = [
    (b"Hani", "中文测试繁體"),
    (b"Hira", "かな"),
    (b"Kana", "カナ"),
    (b"Hang", "한글"),
    (b"Arab", "اب"),
    (b"Hebr", "אב"),
    (b"Deva", "नमस्तेहिन्दी"),
    (b"Thai", "สวัสดีภาษาไทย"),
];

/// A font collection holding only the faces a test registers, so nothing
/// installed on the machine can change a result.
pub(crate) struct Fonts {
    pub(crate) collection: Collection,
    pub(crate) source_cache: SourceCache,
}

impl Fonts {
    /// A machine with no fonts installed.
    pub(crate) fn none() -> Self {
        crate::test_support::evaluate_log_arguments();
        Self {
            collection: Collection::new(CollectionOptions {
                shared: false,
                system_fonts: false,
            }),
            source_cache: SourceCache::default(),
        }
    }

    /// Install `face` as the `weight` face of `family`.
    pub(crate) fn register(&mut self, face: &[u8], family: &str, weight: FontWeight) -> FamilyId {
        let registered = self.collection.register_fonts(
            Blob::new(Arc::new(face.to_vec())),
            Some(FontInfoOverride {
                family_name: Some(family),
                weight: Some(weight),
                ..Default::default()
            }),
        );
        registered[0].0
    }

    /// Install `face` under the family name and weight it carries itself.
    pub(crate) fn register_as_is(&mut self, face: &[u8]) {
        self.collection
            .register_fonts(Blob::new(Arc::new(face.to_vec())), None);
    }

    /// Install a regular Latin face as `family`.
    pub(crate) fn register_latin(&mut self, family: &str) -> FamilyId {
        self.register(&bundled_face(LATIN_FACE), family, FontWeight::NORMAL)
    }

    /// Make `families` what the platform resolves `generic` to.
    pub(crate) fn set_generic(&mut self, generic: GenericFamily, families: &[FamilyId]) {
        self.collection
            .set_generic_families(generic, families.iter().copied());
    }

    /// A desktop whose sans-serif is "Fixture Sans" (regular and bold).
    pub(crate) fn with_sans() -> Self {
        let mut fonts = Self::none();
        let sans = fonts.register_latin("Fixture Sans");
        fonts.register(&bundled_face(MONO_FACE), "Fixture Sans", FontWeight::BOLD);
        fonts.set_generic(GenericFamily::SansSerif, &[sans]);
        fonts
    }

    /// `with_sans` plus "Fixture Scripts", the fallback family for every
    /// script in `SCRIPT_SAMPLES`.
    pub(crate) fn multilingual() -> Self {
        let mut fonts = Self::with_sans();
        let scripts = fonts.register(SCRIPT_SAMPLES_FONT, "Fixture Scripts", FontWeight::NORMAL);
        for (script, _) in SCRIPT_SAMPLES {
            fonts
                .collection
                .set_fallbacks(Script::from_bytes(*script), std::iter::once(scripts));
        }
        fonts
    }

    fn definitions(&mut self, preferred: Option<&str>, preset: FontPreset) -> FontDefinitions {
        font_definitions(
            &mut self.collection,
            &mut self.source_cache,
            None,
            preferred,
            preset,
        )
    }

    fn pickable(&mut self) -> Vec<String> {
        pickable_family_names(&mut self.collection, &mut self.source_cache)
    }
}

fn chain(definitions: &FontDefinitions, family: &FontFamily) -> Vec<String> {
    definitions.families[family].clone()
}

#[test]
fn preset_labels_and_descriptions_tell_the_presets_apart() {
    let labels: HashSet<_> = FontPreset::ALL.map(FontPreset::label).into();
    let descriptions: HashSet<_> = FontPreset::ALL.map(FontPreset::description).into();
    assert_eq!(labels.len(), FontPreset::ALL.len());
    assert_eq!(descriptions.len(), FontPreset::ALL.len());
    assert_eq!(FontPreset::default().label(), "Default");
    assert!(FontPreset::Github.description().contains("github.com"));
    assert!(FontPreset::Vscode.description().contains("VS Code"));
}

/// GitHub: `.markdown-body{line-height:1.5}`, `pre{line-height:1.45}`.
/// VS Code preview: `--markdown-line-height: 1.6`, code 1.357em.
#[test]
fn preset_line_heights_follow_the_emulated_stylesheets() {
    let (github_body, github_code) = FontPreset::Github.line_heights();
    assert_close(github_body, 1.5);
    assert_close(github_code, 1.45);

    let (vscode_body, vscode_code) = FontPreset::Vscode.line_heights();
    assert_close(vscode_body, 1.6);
    assert_close(vscode_code, 1.36);

    let (own_body, own_code) = FontPreset::Current.line_heights();
    assert_close(own_body, 1.5);
    assert_close(own_code, 1.3);
}

#[test]
fn text_size_labels_name_their_own_pixel_size_and_grow_in_order() {
    let sizes = TextSizeClass::ALL.map(TextSizeClass::px);
    assert!(sizes.is_sorted_by(|smaller, larger| smaller < larger));
    for size in TextSizeClass::ALL {
        let pixels = format!("({}px)", size.px());
        assert!(size.label().ends_with(&pixels), "{}", size.label());
    }
    assert_close(TextSizeClass::default().px(), 16.0);
}

#[test]
fn applying_fonts_installs_the_faces_and_derives_heading_and_code_sizes() {
    let context = egui::Context::default();
    let setup = FontSetup {
        locale: None,
        preferred_family: None,
        preset: FontPreset::default(),
        size: TextSizeClass::Large,
    };

    apply_fonts(&context, &mut Fonts::with_sans().collection, &setup);

    let styles = context.style().text_styles.clone();
    assert_eq!(styles[&TextStyle::Body], FontId::proportional(18.0));
    assert_eq!(styles[&TextStyle::Heading], FontId::proportional(36.0));
    assert_eq!(styles[&TextStyle::Monospace], FontId::monospace(16.0));
    // Fonts take effect at the start of the next frame.
    let _ = context.run(egui::RawInput::default(), |_| {});
    let fonts = context.fonts(|fonts| fonts.definitions().clone());
    assert!(fonts.font_data.contains_key("SystemSans"));
}

#[test]
fn machine_without_fonts_keeps_egui_faces_and_gains_the_bundled_emoji() {
    let definitions = Fonts::none().definitions(None, FontPreset::default());
    let defaults = FontDefinitions::default();

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let mut expected = chain(&defaults, &family);
        expected.push("NotoEmoji".to_owned());
        assert_eq!(chain(&definitions, &family), expected);
    }
}

#[test]
fn system_sans_leads_the_body_chain_and_its_bold_face_leads_strong_text() {
    let definitions = Fonts::with_sans().definitions(None, FontPreset::default());

    assert_eq!(
        chain(&definitions, &FontFamily::Proportional)[0],
        "SystemSans"
    );
    let strong = chain(&definitions, &FontFamily::Name(STRONG_FONT_FAMILY.into()));
    assert_eq!(strong[..2], ["SystemSansBold", "SystemSans"]);
    // The default preset keeps egui's own monospace face in front.
    assert_eq!(
        chain(&definitions, &FontFamily::Monospace)[0],
        chain(&FontDefinitions::default(), &FontFamily::Monospace)[0]
    );
}

#[test]
fn preset_code_face_leads_the_monospace_chain_when_installed() {
    let mut fonts = Fonts::with_sans();
    fonts.register(
        &bundled_face(MONO_FACE),
        "Noto Sans Mono",
        FontWeight::NORMAL,
    );

    let definitions = fonts.definitions(None, FontPreset::Github);

    assert_eq!(chain(&definitions, &FontFamily::Monospace)[0], "PresetMono");
}

#[test]
fn installed_fonts_cover_reported_scripts() {
    let mut fonts = Fonts::multilingual();
    let definitions = font_definitions(
        &mut fonts.collection,
        &mut fonts.source_cache,
        Some("en-US"),
        None,
        FontPreset::default(),
    );
    let context = egui::Context::default();
    context.set_fonts(definitions);
    context.begin_pass(egui::RawInput::default());
    let regular = FontId::proportional(16.0);
    let strong = FontId::new(16.0, FontFamily::Name(STRONG_FONT_FAMILY.into()));
    let missing = |font: &FontId| -> Vec<&str> {
        SCRIPT_SAMPLES
            .iter()
            .map(|(_, sample)| *sample)
            .filter(|sample| !context.fonts_mut(|fonts| fonts.has_glyphs(font, sample)))
            .collect()
    };
    let (missing_regular, missing_strong) = (missing(&regular), missing(&strong));
    let _ = context.end_pass();

    assert!(
        missing_regular.is_empty(),
        "regular chain lacks {missing_regular:?}"
    );
    assert!(
        missing_strong.is_empty(),
        "strong chain lacks {missing_strong:?}"
    );
}

/// Control for the test above: the same desktop without script fallbacks
/// cannot draw the samples, so the fallbacks are what covers them.
#[test]
fn scripts_stay_uncovered_without_fallback_fonts() {
    let definitions = Fonts::with_sans().definitions(None, FontPreset::default());
    let context = egui::Context::default();
    context.set_fonts(definitions);
    context.begin_pass(egui::RawInput::default());
    let regular = FontId::proportional(16.0);
    let covered = context.fonts_mut(|fonts| fonts.has_glyphs(&regular, "中文"));
    let _ = context.end_pass();
    assert!(!covered);
}

#[test]
fn preset_family_lists_follow_the_emulated_css_stacks() {
    // GitHub (Primer .markdown-body) leads with its locally-installed Mona
    // Sans, asks for Segoe UI before Noto Sans, and ends the named part of
    // its body stack at Arial.
    let github_body = FontPreset::Github
        .body_families()
        .expect("github body list");
    assert_eq!(github_body[..3], ["Mona Sans VF", "Mona Sans", "Segoe UI"]);
    assert!(github_body.contains(&"Noto Sans"));
    assert_eq!(*github_body.last().expect("non-empty"), "Arial");

    // VS Code preview lists Ubuntu and Droid Sans behind the system-ui
    // resolutions.
    let vscode_body = FontPreset::Vscode
        .body_families()
        .expect("vscode body list");
    let ubuntu_pos = vscode_body
        .iter()
        .position(|f| *f == "Ubuntu")
        .expect("ubuntu in vscode stack");
    let adwaita_pos = vscode_body
        .iter()
        .position(|f| *f == "Adwaita Sans")
        .expect("adwaita sans in vscode stack");
    assert!(adwaita_pos < ubuntu_pos, "system-ui precedes Ubuntu");

    // The default preset keeps the app's own sans selection.
    assert_eq!(FontPreset::Current.body_families(), None);
    assert!(FontPreset::Current.mono_families().is_empty());
}

#[test]
fn preset_mono_lists_lead_with_linux_generic_monospace_resolutions() {
    assert_eq!(
        FontPreset::Github.mono_families()[..2],
        ["Noto Sans Mono", "DejaVu Sans Mono"]
    );
    assert_eq!(
        FontPreset::Vscode.mono_families(),
        ["Droid Sans Mono", "Noto Sans Mono", "DejaVu Sans Mono"]
    );
}

#[test]
fn font_picker_lists_body_capable_families_sorted_without_regard_to_case() {
    let mut fonts = Fonts::none();
    fonts.register_latin("Zeta Sans");
    fonts.register_latin("alpha Sans");
    fonts.register_latin("Beta Sans");
    fonts.register(SCRIPT_SAMPLES_FONT, "Aaa Scripts", FontWeight::NORMAL);
    fonts.register(
        &bundled_face(SYMBOL_FACE),
        "Aab Symbols",
        FontWeight::NORMAL,
    );

    assert_eq!(fonts.pickable(), ["alpha Sans", "Beta Sans", "Zeta Sans"]);
}

#[test]
fn only_the_canonical_spelling_of_a_family_name_stands_for_the_family() {
    let mut fonts = Fonts::none();
    let sans = fonts.register_latin("Fixture Sans");

    let canonical = canonical_family_id(&mut fonts.collection, "Fixture Sans");
    let alias = canonical_family_id(&mut fonts.collection, "fixture sans");
    let unknown = canonical_family_id(&mut fonts.collection, "Missing Sans");

    assert_eq!(canonical, Some(sans));
    assert_eq!(alias, None, "another spelling resolves to the same family");
    assert_eq!(unknown, None);
}

/// Runs against whatever fonts the machine has. Each property holds for any
/// font set, an empty one included; the fixed sets above pin the details.
#[test]
fn system_font_picker_list_is_sorted_canonical_and_body_capable() {
    let names = scan_pickable_font_families();

    assert!(names.is_sorted_by_key(|name| name.to_lowercase()));

    let mut collection = Collection::new(CollectionOptions::default());
    let mut source_cache = SourceCache::default();
    let mut seen_ids = HashSet::new();
    for name in &names {
        let id = collection
            .family_id(name)
            .unwrap_or_else(|| panic!("listed family {name:?} does not resolve"));
        assert!(
            seen_ids.insert(id),
            "duplicate family id behind {name:?} — alias leak"
        );
        assert_eq!(
            collection.family_name(id).map(str::to_owned).as_deref(),
            Some(name.as_str()),
            "{name:?} is an alias, not the family's canonical name"
        );
        assert!(
            select_body_font(&mut collection, &mut source_cache, &[id]).is_some(),
            "listed family {name:?} cannot serve as a body font"
        );
    }
}

#[test]
fn github_preset_takes_its_body_face_from_the_emulated_stack() {
    let mut fonts = Fonts::with_sans();
    fonts.register_latin("Noto Sans");

    let installed = install_regular_fonts(
        &mut fonts.collection,
        &mut fonts.source_cache,
        &mut FontDefinitions::default(),
        None,
        None,
        FontPreset::Github.body_families(),
    );

    let primary = installed
        .iter()
        .find(|font| font.primary)
        .expect("primary font installed");
    assert_eq!(primary.selected.family, "Noto Sans");
}
