use super::*;

use fontique::CollectionOptions;

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
#[ignore = "requires installed system fonts"]
fn unknown_preferred_family_falls_back_to_auto_detect() {
    let mut collection = Collection::new(CollectionOptions::default());
    let mut source_cache = SourceCache::default();
    let mut definitions = FontDefinitions::default();
    let installed = install_regular_fonts(
        &mut collection,
        &mut source_cache,
        &mut definitions,
        None,
        Some("Definitely Not An Installed Font Name 12345"),
        None,
    );
    assert!(
        installed.iter().any(|f| f.primary),
        "auto-detect fallback should still install a primary font"
    );
}

#[test]
#[ignore = "requires installed system fonts"]
fn known_preferred_family_becomes_primary() {
    let mut collection = Collection::new(CollectionOptions::default());
    let mut source_cache = SourceCache::default();

    // Discover whatever the system's default sans-serif family is, then
    // request it explicitly by name and confirm it round-trips as
    // primary. Avoids hardcoding a font name that may not exist on every
    // machine/CI image.
    let mut baseline_definitions = FontDefinitions::default();
    let baseline = install_regular_fonts(
        &mut collection,
        &mut source_cache,
        &mut baseline_definitions,
        None,
        None,
        None,
    );
    let Some(baseline_primary) = baseline.iter().find(|f| f.primary) else {
        return; // no system fonts available in this environment
    };
    let family_name = baseline_primary.selected.family.clone();

    let mut definitions = FontDefinitions::default();
    let installed = install_regular_fonts(
        &mut collection,
        &mut source_cache,
        &mut definitions,
        None,
        Some(&family_name),
        None,
    );
    let primary = installed
        .iter()
        .find(|f| f.primary)
        .expect("primary font installed");
    assert_eq!(primary.selected.family, family_name);
}
