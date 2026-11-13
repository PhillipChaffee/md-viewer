use super::*;

use crate::system_fonts::tests::{
    bundled_face, Fonts, LATIN_FACE, MONO_FACE, SCRIPT_SAMPLES_FONT, SYMBOL_FACE,
};
use crate::test_support::ScratchDir;

/// What `install_regular_fonts` chose from `fonts`, with the chains it built.
struct Installed {
    fonts: Vec<InstalledFont>,
    definitions: FontDefinitions,
}

impl Installed {
    fn regular(
        fonts: &mut Fonts,
        preferred: Option<&str>,
        preset_body: Option<&'static [&'static str]>,
    ) -> Self {
        let mut definitions = FontDefinitions::default();
        let installed = install_regular_fonts(
            &mut fonts.system(),
            &mut definitions,
            None,
            preferred,
            preset_body,
        );
        Self {
            fonts: installed,
            definitions,
        }
    }

    /// Family of the primary sans, if one was installed.
    fn primary_family(&self) -> Option<&str> {
        self.fonts
            .iter()
            .find(|font| font.primary)
            .map(|font| font.selected.family.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.fonts.iter().map(|font| font.key.as_str()).collect()
    }

    fn chain(&self, family: &FontFamily) -> &[String] {
        &self.definitions.families[family]
    }

    /// Add the bold faces and return how many were loaded, with the chain
    /// strong text is drawn from.
    fn strong(mut self, fonts: &mut Fonts) -> (usize, Vec<String>) {
        let count = install_strong_font_family(
            &mut fonts.system(),
            &mut self.definitions,
            &self.fonts,
            &["DefaultLatin".to_owned()],
        );
        let strong = FontFamily::Name(STRONG_FONT_FAMILY.into());
        (count, self.definitions.families[&strong].clone())
    }
}

#[test]
fn reported_scripts_use_full_coverage_samples() {
    let sample = |script| {
        SCRIPT_FALLBACKS
            .iter()
            .find(|fallback| fallback.script == script)
            .unwrap()
            .required_glyphs
    };
    assert_eq!(sample(*b"Deva"), "नमस्तेहिन्दी");
    assert_eq!(sample(*b"Thai"), "สวัสดีภาษาไทย");
    assert!(sample(*b"Hani").contains('测'));
    assert!(sample(*b"Hani").contains('體'));
}

#[test]
fn default_latin_precedes_script_bold_without_primary_system_sans() {
    let strong = build_strong_family(
        None,
        None,
        &["DefaultLatin".into()],
        &["HanBold".into()],
        &["DefaultLatin".into(), "Han".into()],
    );
    assert_eq!(strong, ["DefaultLatin", "HanBold", "Han"]);
}

#[test]
fn primary_regular_and_bold_stay_together() {
    let strong = build_strong_family(
        Some("SystemSans"),
        Some("SystemSansBold"),
        &["DefaultLatin".into()],
        &["HanBold".into()],
        &["SystemSans".into(), "Han".into()],
    );
    assert_eq!(
        strong,
        [
            "SystemSansBold",
            "SystemSans",
            "DefaultLatin",
            "HanBold",
            "Han"
        ]
    );
}

#[test]
fn system_sans_is_the_primary_face_and_leads_the_body_chain() {
    let installed = Installed::regular(&mut Fonts::with_sans(), None, None);

    assert_eq!(installed.primary_family(), Some("Fixture Sans"));
    assert_eq!(installed.chain(&FontFamily::Proportional)[0], "SystemSans");
    // Code keeps its own face first and gets the sans only as a fallback.
    let monospace = installed.chain(&FontFamily::Monospace);
    assert_ne!(monospace[0], "SystemSans");
    assert_eq!(monospace.last().map(String::as_str), Some("SystemSans"));
}

#[test]
fn machine_without_fonts_installs_nothing() {
    let installed = Installed::regular(&mut Fonts::none(), Some("Fixture Sans"), None);

    assert!(installed.fonts.is_empty());
    assert_eq!(installed.definitions, FontDefinitions::default());
}

#[test]
fn unknown_preferred_family_falls_back_to_auto_detect() {
    let installed = Installed::regular(
        &mut Fonts::with_sans(),
        Some("Definitely Not An Installed Font Name 12345"),
        None,
    );
    assert_eq!(installed.primary_family(), Some("Fixture Sans"));
}

#[test]
fn known_preferred_family_becomes_primary() {
    let mut fonts = Fonts::with_sans();
    fonts.register_latin("Reader Sans");

    let installed = Installed::regular(&mut fonts, Some("Reader Sans"), None);

    assert_eq!(installed.primary_family(), Some("Reader Sans"));
    assert_eq!(installed.keys(), ["SystemSans"]);
}

#[test]
fn preferred_family_without_latin_letters_falls_back_to_auto_detect() {
    let mut fonts = Fonts::with_sans();
    fonts.register(SCRIPT_SAMPLES_FONT, "Fixture Scripts", FontWeight::NORMAL);

    let installed = Installed::regular(&mut fonts, Some("Fixture Scripts"), None);

    assert_eq!(installed.primary_family(), Some("Fixture Sans"));
}

#[test]
fn preset_stack_picks_its_first_installed_family_over_the_system_sans() {
    let mut fonts = Fonts::with_sans();
    fonts.register_latin("Later In Stack");
    fonts.register_latin("Earlier In Stack");
    let stack: &[&str] = &["Not Installed", "Earlier In Stack", "Later In Stack"];

    let installed = Installed::regular(&mut fonts, None, Some(stack));

    assert_eq!(installed.primary_family(), Some("Earlier In Stack"));
}

#[test]
fn preset_stack_with_nothing_installed_falls_back_to_the_system_sans() {
    let stack: &[&str] = &["Not Installed", "Nor This"];
    let installed = Installed::regular(&mut Fonts::with_sans(), None, Some(stack));
    assert_eq!(installed.primary_family(), Some("Fixture Sans"));
}

#[test]
fn picked_family_outranks_the_preset_stack() {
    let mut fonts = Fonts::with_sans();
    fonts.register_latin("Picked Sans");
    fonts.register_latin("Preset Sans");

    let installed = Installed::regular(&mut fonts, Some("Picked Sans"), Some(&["Preset Sans"]));

    assert_eq!(installed.primary_family(), Some("Picked Sans"));
}

#[test]
fn one_fallback_face_is_added_per_script_the_installed_faces_lack() {
    let installed = Installed::regular(&mut Fonts::multilingual(), None, None);

    // The fixture covers every script, so the face found for the first
    // uncovered script (Han) settles all the others too.
    assert_eq!(installed.keys(), ["SystemSans", "SystemHanFallback"]);
    let body = installed.chain(&FontFamily::Proportional);
    assert_eq!(body[0], "SystemSans");
    assert_eq!(
        body.last().map(String::as_str),
        Some("SystemHanFallback"),
        "a fallback goes behind every face already in the chain"
    );
}

#[test]
fn math_and_symbol_fallbacks_fill_what_the_sans_lacks() {
    let mut fonts = Fonts::with_sans();
    let sans = fonts.collection.family_id("Fixture Sans").unwrap();
    let math = fonts.register(&bundled_face(MONO_FACE), "Fixture Math", FontWeight::NORMAL);
    let symbols = fonts.register(
        &bundled_face(SYMBOL_FACE),
        "Fixture Symbols",
        FontWeight::NORMAL,
    );
    fonts.set_generic(GenericFamily::Math, &[math]);
    fonts.set_generic(GenericFamily::SansSerif, &[sans, symbols]);

    let installed = Installed::regular(&mut fonts, None, None);

    assert_eq!(
        installed.keys(),
        ["SystemSans", "SystemMathFallback", "SystemSymbolFallback"]
    );
}

#[test]
fn fallbacks_are_skipped_when_the_sans_already_has_the_glyphs() {
    let mut fonts = Fonts::none();
    // Hack maps → itself, so no separate math face is wanted.
    let sans = fonts.register(&bundled_face(MONO_FACE), "Arrow Sans", FontWeight::NORMAL);
    let math = fonts.register(&bundled_face(MONO_FACE), "Fixture Math", FontWeight::NORMAL);
    fonts.set_generic(GenericFamily::SansSerif, &[sans]);
    fonts.set_generic(GenericFamily::Math, &[math]);

    let installed = Installed::regular(&mut fonts, None, None);

    assert_eq!(installed.keys(), ["SystemSans"]);
}

#[test]
fn strong_text_leads_with_the_bold_face_of_the_primary_family() {
    let mut fonts = Fonts::with_sans();
    let (count, strong) = Installed::regular(&mut fonts, None, None).strong(&mut fonts);

    assert_eq!(count, 1);
    assert_eq!(
        strong[..3],
        ["SystemSansBold", "SystemSans", "DefaultLatin"]
    );
}

#[test]
fn strong_text_keeps_the_regular_face_when_the_family_has_no_bold() {
    let mut fonts = Fonts::none();
    let sans = fonts.register_latin("Regular Only");
    fonts.set_generic(GenericFamily::SansSerif, &[sans]);

    let (count, strong) = Installed::regular(&mut fonts, None, None).strong(&mut fonts);

    assert_eq!(count, 0);
    assert_eq!(strong[..2], ["SystemSans", "DefaultLatin"]);
}

/// The heaviest face a family has is not bold just for being the heaviest:
/// strong text set in a medium weight would look like the text around it.
#[test]
fn medium_face_is_not_taken_for_the_bold_face() {
    let mut fonts = Fonts::none();
    let sans = fonts.register_latin("Medium Sans");
    fonts.register(&bundled_face(MONO_FACE), "Medium Sans", FontWeight::MEDIUM);
    fonts.set_generic(GenericFamily::SansSerif, &[sans]);

    let (count, strong) = Installed::regular(&mut fonts, None, None).strong(&mut fonts);

    assert_eq!(count, 0);
    assert_eq!(strong[..2], ["SystemSans", "DefaultLatin"]);
}

#[test]
fn family_with_only_a_bold_face_is_not_loaded_twice() {
    let mut fonts = Fonts::none();
    let sans = fonts.register(&bundled_face(LATIN_FACE), "Bold Only", FontWeight::BOLD);
    fonts.set_generic(GenericFamily::SansSerif, &[sans]);

    let (count, strong) = Installed::regular(&mut fonts, None, None).strong(&mut fonts);

    assert_eq!(count, 0, "the bold face is the regular face already loaded");
    assert_eq!(strong[0], "SystemSans");
}

#[test]
fn script_fallback_bold_goes_behind_the_default_latin_faces() {
    let mut fonts = Fonts::multilingual();
    fonts.register(SCRIPT_SAMPLES_FONT, "Fixture Scripts", FontWeight::BOLD);

    let (count, strong) = Installed::regular(&mut fonts, None, None).strong(&mut fonts);

    assert_eq!(count, 2);
    assert_eq!(
        strong[..4],
        [
            "SystemSansBold",
            "SystemSans",
            "DefaultLatin",
            "SystemHanFallbackBold"
        ]
    );
}

#[test]
fn preset_code_face_goes_in_front_of_the_bundled_monospace() {
    let mut fonts = Fonts::none();
    fonts.register(&bundled_face(MONO_FACE), "Second Mono", FontWeight::NORMAL);
    let mut definitions = FontDefinitions::default();
    let bundled_first = definitions.families[&FontFamily::Monospace][0].clone();

    let installed = install_preset_mono_font(
        &mut fonts.system(),
        &mut definitions,
        &["First Mono", "Second Mono"],
    );

    assert!(installed);
    let monospace = &definitions.families[&FontFamily::Monospace];
    assert_eq!(monospace[..2], ["PresetMono".to_owned(), bundled_first]);
}

#[test]
fn missing_preset_code_faces_leave_the_monospace_chain_alone() {
    let mut fonts = Fonts::with_sans();
    let mut definitions = FontDefinitions::default();

    let installed = install_preset_mono_font(
        &mut fonts.system(),
        &mut definitions,
        &["First Mono", "Second Mono"],
    );

    assert!(!installed);
    assert_eq!(definitions, FontDefinitions::default());
}

#[test]
fn body_font_selection_skips_families_the_collection_does_not_know() {
    let mut fonts = Fonts::with_sans();
    let foreign = Fonts::none().register_latin("Elsewhere Sans");
    let known = fonts.collection.family_id("Fixture Sans").unwrap();

    let none = fonts.system().regular(&[foreign], LATIN_SAMPLE);
    let found = fonts.system().regular(&[foreign, known], LATIN_SAMPLE);

    assert!(none.is_none());
    assert_eq!(
        found.map(|font| font.family).as_deref(),
        Some("Fixture Sans")
    );
}

/// A font installed as a file, the way a desktop has them: the collection
/// that found it, the family it is listed under, and the file.
struct FontFile {
    fonts: Fonts,
    family: String,
    path: std::path::PathBuf,
}

impl FontFile {
    fn install(dir: &ScratchDir) -> Self {
        let path = dir.path().join("reader-sans.ttf");
        std::fs::write(&path, bundled_face(LATIN_FACE)).unwrap();
        let mut fonts = Fonts::none();
        fonts.collection.load_fonts_from_paths([&path]);
        let family = fonts
            .collection
            .family_names()
            .next()
            .expect("the file holds one family")
            .to_owned();
        Self {
            fonts,
            family,
            path,
        }
    }

    fn select(&mut self) -> Option<SelectedFont> {
        self.fonts.system().named(&self.family, "Aa")
    }
}

#[test]
fn face_installed_as_a_file_reports_the_file_it_was_loaded_from() {
    let dir = ScratchDir::new();
    let mut installed = FontFile::install(&dir);

    let face = installed.select().expect("the family is installed");

    assert_eq!(face.source, installed.path.display().to_string());
}

#[test]
fn face_whose_file_has_vanished_is_passed_over() {
    let dir = ScratchDir::new();
    let mut installed = FontFile::install(&dir);
    std::fs::remove_file(&installed.path).unwrap();

    assert!(installed.select().is_none());
}

#[test]
fn face_registered_from_memory_says_so_in_its_source() {
    let mut fonts = Fonts::with_sans();
    let face = fonts
        .system()
        .named("Fixture Sans", "Aa")
        .expect("the family is registered");
    assert_eq!(face.source, "<memory>");
}

/// Definitions a caller built without the standard families get the faces
/// registered, and no family chain is invented for them.
#[test]
fn definitions_without_family_chains_only_gain_the_font_data() {
    let mut fonts = Fonts::with_sans();
    let mut definitions = FontDefinitions::empty();
    definitions.families.clear();

    let installed = install_regular_fonts(&mut fonts.system(), &mut definitions, None, None, None);
    let mono_installed =
        install_preset_mono_font(&mut fonts.system(), &mut definitions, &["Fixture Sans"]);

    assert_eq!(installed.len(), 1);
    assert!(mono_installed);
    let mut faces: Vec<&str> = definitions.font_data.keys().map(String::as_str).collect();
    faces.sort_unstable();
    assert_eq!(faces, ["PresetMono", "SystemSans"]);
    assert!(definitions.families.is_empty());
}
