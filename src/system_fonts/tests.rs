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

    /// The collection as the font code takes it.
    pub(crate) fn system(&mut self) -> SystemFonts<'_> {
        SystemFonts {
            collection: &mut self.collection,
            source_cache: &mut self.source_cache,
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
        font_definitions(&mut self.system(), None, preferred, preset)
    }

    fn pickable(&mut self) -> Vec<String> {
        pickable_family_names(&mut self.system())
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
    assert_eq!(FontPreset::default().label(), "Reader");
    assert!(FontPreset::Reader.description().contains("Source Serif 4"));
    assert!(FontPreset::Github.description().contains("github.com"));
    assert!(FontPreset::Vscode.description().contains("VS Code"));
}

/// GitHub: `pre{line-height:1.45}`. VS Code preview: code 1.357em.
#[test]
fn preset_code_line_heights_follow_the_emulated_stylesheets() {
    assert_close(FontPreset::Github.code_line_height(), 1.45);
    assert_close(FontPreset::Vscode.code_line_height(), 1.36);
    assert_close(FontPreset::Current.code_line_height(), 1.3);
    assert_close(FontPreset::Reader.code_line_height(), 1.5);
}

#[test]
fn text_sizes_grow_in_order_and_are_told_apart_by_label() {
    let sizes = TextSizeClass::ALL.map(TextSizeClass::px);
    assert!(sizes.is_sorted_by(|smaller, larger| smaller < larger));
    let labels: HashSet<_> = TextSizeClass::ALL.map(TextSizeClass::label).into();
    assert_eq!(labels.len(), TextSizeClass::ALL.len());
    assert_eq!(TextSizeClass::default().label(), "Normal");
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
    let definitions = Fonts::with_sans().definitions(None, FontPreset::Current);

    assert_eq!(
        chain(&definitions, &FontFamily::Proportional)[0],
        "SystemSans"
    );
    let strong = chain(&definitions, &FontFamily::Name(STRONG_FONT_FAMILY.into()));
    assert_eq!(strong[..2], ["SystemSansBold", "SystemSans"]);
    // The system preset keeps egui's own monospace face in front.
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
        &mut fonts.system(),
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
    let mut fonts = SystemFonts {
        collection: &mut collection,
        source_cache: &mut source_cache,
    };
    let mut seen_ids = HashSet::new();
    for name in &names {
        let id = fonts
            .collection
            .family_id(name)
            .unwrap_or_else(|| panic!("listed family {name:?} does not resolve"));
        assert!(
            seen_ids.insert(id),
            "duplicate family id behind {name:?} — alias leak"
        );
        assert_eq!(
            fonts
                .collection
                .family_name(id)
                .map(str::to_owned)
                .as_deref(),
            Some(name.as_str()),
            "{name:?} is an alias, not the family's canonical name"
        );
        assert!(
            fonts.regular(&[id], LATIN_SAMPLE).is_some(),
            "listed family {name:?} cannot serve as a body font"
        );
    }
}

#[test]
fn github_preset_takes_its_body_face_from_the_emulated_stack() {
    let mut fonts = Fonts::with_sans();
    fonts.register_latin("Noto Sans");

    let installed = install_regular_fonts(
        &mut fonts.system(),
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

#[test]
fn the_reader_sets_the_document_larger_than_the_chrome_with_code_a_step_smaller() {
    let fonts = document_fonts(None, FontPreset::Reader, TextSizeClass::Normal);

    assert_close(fonts.body.size, 16.0 * 1.15);
    assert_eq!(fonts.body.family, FontFamily::Name("MarkdownBody".into()));
    assert_close(fonts.heading.size, fonts.body.size * 2.0);
    assert_eq!(
        fonts.heading.family,
        FontFamily::Name("MarkdownHeading".into())
    );
    assert_eq!(fonts.code.family, FontFamily::Monospace);
    assert_close(fonts.code.size, fonts.body.size * 0.875);
    assert_eq!(
        fonts.emphasis,
        Some(FontFamily::Name("MarkdownEmphasis".into()))
    );
    assert_eq!(fonts.inline_code_scale, Some(0.875));

    // Every size class scales the same way.
    for size in TextSizeClass::ALL {
        let fonts = document_fonts(None, FontPreset::Reader, size);
        assert_close(fonts.body.size, size.px() * 1.15);
    }
}

#[test]
fn the_other_presets_set_the_document_at_the_chromes_size_without_an_italic_face() {
    for preset in [FontPreset::Current, FontPreset::Github, FontPreset::Vscode] {
        let fonts = document_fonts(None, preset, TextSizeClass::Large);

        assert_close(fonts.body.size, 18.0);
        assert_eq!(fonts.body.family, FontFamily::Name("MarkdownBody".into()));
        assert_close(fonts.code.size, 18.0 * 0.875);
        assert_eq!(fonts.emphasis, None, "{preset:?}");
        assert_eq!(fonts.inline_code_scale, None, "{preset:?}");
    }
}

#[test]
fn a_picked_family_outranks_the_readers_serif() {
    let fonts = document_fonts(
        Some("Fixture Sans"),
        FontPreset::Reader,
        TextSizeClass::Normal,
    );
    // The serif's scale stays with the preset; its faces do not lead.
    assert_eq!(fonts.emphasis, None);
    assert_eq!(fonts.inline_code_scale, None);

    let mut system = Fonts::with_sans();
    let picked = system.definitions(Some("Fixture Sans"), FontPreset::Reader);
    let body = chain(&picked, &FontFamily::Name("MarkdownBody".into()));
    assert_eq!(body, chain(&picked, &FontFamily::Proportional));
    assert!(!picked.font_data.contains_key("ReaderRegular"));
}

#[test]
fn the_reader_registers_its_serif_for_the_document_and_leaves_the_chrome_its_sans() {
    let mut system = Fonts::with_sans();

    let reader = system.definitions(None, FontPreset::Reader);
    let own = system.definitions(None, FontPreset::Current);

    let body = chain(&reader, &FontFamily::Name("MarkdownBody".into()));
    assert_eq!(body[0], "ReaderRegular");
    assert_eq!(body[1..], chain(&reader, &FontFamily::Proportional));
    assert_eq!(
        chain(&reader, &FontFamily::Name("MarkdownHeading".into()))[0],
        "ReaderBold"
    );
    assert_eq!(
        chain(&reader, &FontFamily::Name(STRONG_FONT_FAMILY.into()))[0],
        "ReaderSemibold"
    );
    assert_eq!(
        chain(&reader, &FontFamily::Name("MarkdownEmphasis".into()))[0],
        "ReaderItalic"
    );
    // The chrome is set in the same sans whichever preset the document uses.
    assert_eq!(
        chain(&reader, &FontFamily::Proportional),
        chain(&own, &FontFamily::Proportional)
    );
    // Every preset registers the document families: egui panics on a
    // family nobody registered.
    for preset in FontPreset::ALL {
        let definitions = system.definitions(None, preset);
        for family in ["MarkdownBody", "MarkdownHeading", "MarkdownEmphasis"] {
            let faces = chain(&definitions, &FontFamily::Name(family.into()));
            assert!(!faces.is_empty(), "{preset:?} has no {family}");
        }
    }
}

#[test]
fn text_sizes_step_one_class_at_a_time_and_stop_at_the_ends() {
    let all = TextSizeClass::ALL;
    for pair in all.windows(2) {
        assert_eq!(pair[0].larger(), Some(pair[1]));
        assert_eq!(pair[1].smaller(), Some(pair[0]));
    }
    assert_eq!(all[0].smaller(), None);
    assert_eq!(all[all.len() - 1].larger(), None);
}

/// A machine where fontconfig lists no fonts at all, as in a slim container.
/// The picker scan runs in a child process because the fontconfig file is
/// chosen through the environment, which is process-wide.
#[test]
fn a_machine_with_no_fonts_still_gets_a_font_list() {
    let scratch = crate::test_support::ScratchDir::new();
    let config = scratch.path().join("fonts.conf");
    std::fs::write(&config, "<?xml version=\"1.0\"?><fontconfig></fontconfig>").unwrap();

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "system_fonts::tests::system_font_picker_list_is_sorted_canonical_and_body_capable",
        ])
        .env("FONTCONFIG_FILE", &config)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The families of the viewer `preset` emulates, body then code.
fn emulated_families(preset: FontPreset) -> (&'static [&'static str], &'static [&'static str]) {
    (
        preset.body_families().unwrap_or_default(),
        preset.mono_families(),
    )
}

#[test]
fn each_preset_names_its_viewers_families_in_the_stylesheets_order() {
    let none: &[&str] = &[];
    assert_eq!(emulated_families(FontPreset::Reader), (none, none));
    assert_eq!(emulated_families(FontPreset::Current), (none, none));
    assert_eq!(
        emulated_families(FontPreset::Github),
        (
            &[
                "Mona Sans VF",
                "Mona Sans",
                "Segoe UI",
                "Adwaita Sans",
                "Noto Sans",
                "Helvetica",
                "Arial",
            ][..],
            &[
                "Noto Sans Mono",
                "DejaVu Sans Mono",
                "SFMono-Regular",
                "SF Mono",
                "Menlo",
                "Consolas",
                "Liberation Mono",
            ][..],
        )
    );
    assert_eq!(
        emulated_families(FontPreset::Vscode),
        (
            &[
                "Segoe WPC",
                "Segoe UI",
                "Adwaita Sans",
                "Cantarell",
                "Ubuntu",
                "Droid Sans",
            ][..],
            &["Droid Sans Mono", "Noto Sans Mono", "DejaVu Sans Mono"][..],
        )
    );
}

/// The chains and faces one preset installs.
#[derive(Debug, PartialEq)]
struct InstalledChains {
    proportional: Vec<String>,
    monospace: Vec<String>,
    strong: Vec<String>,
    body: Vec<String>,
    /// The egui face whose bytes the primary sans holds.
    sans_face: &'static str,
    /// The egui face whose bytes the preset's code face holds.
    code_face: Option<&'static str>,
}

fn face_behind(definitions: &FontDefinitions, key: &str) -> Option<&'static str> {
    let data = &definitions.font_data.get(key)?.font;
    [LATIN_FACE, MONO_FACE]
        .into_iter()
        .find(|face| bundled_face(face) == data.as_ref())
}

fn installed_chains(fonts: &mut Fonts, preset: FontPreset) -> InstalledChains {
    let definitions = fonts.definitions(None, preset);
    InstalledChains {
        proportional: chain(&definitions, &FontFamily::Proportional),
        monospace: chain(&definitions, &FontFamily::Monospace),
        strong: chain(&definitions, &FontFamily::Name(STRONG_FONT_FAMILY.into())),
        body: chain(&definitions, &FontFamily::Name("MarkdownBody".into())),
        sans_face: face_behind(&definitions, "SystemSans").expect("a primary sans"),
        code_face: face_behind(&definitions, "PresetMono"),
    }
}

#[test]
fn each_preset_installs_its_own_faces_in_a_fixed_order() {
    let mut fonts = Fonts::multilingual();
    // One family from each emulated stack, told apart by the face behind it.
    fonts.register(&bundled_face(MONO_FACE), "Noto Sans", FontWeight::NORMAL);
    fonts.register(&bundled_face(MONO_FACE), "Ubuntu", FontWeight::NORMAL);
    fonts.register(
        &bundled_face(LATIN_FACE),
        "Noto Sans Mono",
        FontWeight::NORMAL,
    );
    fonts.register(
        &bundled_face(MONO_FACE),
        "Droid Sans Mono",
        FontWeight::NORMAL,
    );
    let led_by = |lead: &[&str], rest: &[&str]| -> Vec<String> {
        lead.iter().chain(rest).map(|&key| key.to_owned()).collect()
    };
    let proportional = [
        "SystemSans",
        "Ubuntu-Light",
        "NotoEmoji-Regular",
        "emoji-icon-font",
        "SystemHanFallback",
        "NotoEmoji",
    ];
    let monospace = [
        "Hack",
        "Ubuntu-Light",
        "NotoEmoji-Regular",
        "emoji-icon-font",
        "SystemSans",
        "SystemHanFallback",
        "NotoEmoji",
    ];
    // Strong text is chained before the bundled emoji face goes in.
    let strong = &proportional[..5];
    let expected = |strong_lead: &[&str], body_lead: &[&str], sans_face, code_face: Option<_>| {
        let code_lead: &[&str] = code_face.map_or(&[], |_| &["PresetMono"]);
        InstalledChains {
            proportional: led_by(&[], &proportional),
            monospace: led_by(code_lead, &monospace),
            strong: led_by(strong_lead, strong),
            body: led_by(body_lead, &proportional),
            sans_face,
            code_face,
        }
    };

    assert_eq!(
        installed_chains(&mut fonts, FontPreset::Reader),
        expected(
            &["ReaderSemibold", "SystemSansBold"],
            &["ReaderRegular"],
            LATIN_FACE,
            None
        )
    );
    assert_eq!(
        installed_chains(&mut fonts, FontPreset::Current),
        expected(&["SystemSansBold"], &[], LATIN_FACE, None)
    );
    // GitHub takes "Noto Sans" and "Noto Sans Mono"; neither has a bold face.
    assert_eq!(
        installed_chains(&mut fonts, FontPreset::Github),
        expected(&[], &[], MONO_FACE, Some(LATIN_FACE))
    );
    // VS Code takes "Ubuntu" and "Droid Sans Mono".
    assert_eq!(
        installed_chains(&mut fonts, FontPreset::Vscode),
        expected(&[], &[], MONO_FACE, Some(MONO_FACE))
    );
}
