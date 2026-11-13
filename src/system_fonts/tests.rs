use super::*;

use std::collections::HashSet;

use egui_commonmark_extended::STRONG_FONT_FAMILY;

#[test]
#[ignore = "requires installed system fonts"]
fn pickable_family_list_is_deduplicated_and_body_capable() {
    let names = scan_pickable_font_families();
    assert!(!names.is_empty(), "no pickable families found");

    // Sorted case-insensitively, no duplicate names.
    let mut sorted = names.clone();
    sorted.sort_by_key(|name| name.to_lowercase());
    assert_eq!(names, sorted, "pickable family list must stay sorted");

    // One canonical name per family, and every listed family must pass
    // the same body-font gate used when installing a picked family —
    // otherwise picking it silently falls back to the default again.
    let mut collection = Collection::new(CollectionOptions::default());
    let total = collection.family_names().count();
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
            select_from_families(
                &mut collection,
                &mut source_cache,
                &[id],
                FontWeight::NORMAL,
                "Aa",
                false,
            )
            .is_some(),
            "listed family {name:?} cannot serve as a body font"
        );
    }
    assert!(
        names.len() < total,
        "aliases and script-only families should be filtered ({}/{})",
        names.len(),
        total
    );
}

#[test]
#[ignore = "requires installed multilingual regular and bold fonts"]
fn installed_fonts_cover_reported_scripts() {
    let context = egui::Context::default();
    setup_fonts(
        &context,
        None,
        FontPreset::default(),
        TextSizeClass::default(),
    );
    context.begin_pass(Default::default());
    let regular = egui::FontId::proportional(16.0);
    let strong = egui::FontId::new(16.0, FontFamily::Name(STRONG_FONT_FAMILY.into()));
    let samples = [
        "中文测试繁體",
        "かなカナ",
        "한글",
        "नमस्तेहिन्दी",
        "สวัสดีภาษาไทย",
    ];
    let missing_regular: Vec<_> = samples
        .iter()
        .filter(|sample| !context.fonts_mut(|fonts| fonts.has_glyphs(&regular, sample)))
        .collect();
    let missing_strong: Vec<_> = samples
        .iter()
        .filter(|sample| !context.fonts_mut(|fonts| fonts.has_glyphs(&strong, sample)))
        .collect();
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
#[ignore = "requires installed system fonts"]
fn github_preset_resolves_its_body_stack_when_fonts_exist() {
    let mut collection = Collection::new(CollectionOptions::default());
    let mut source_cache = SourceCache::default();
    let mut definitions = FontDefinitions::default();
    let installed = install_regular_fonts(
        &mut collection,
        &mut source_cache,
        &mut definitions,
        None,
        None,
        FontPreset::Github.body_families(),
    );
    let Some(primary) = installed.iter().find(|f| f.primary) else {
        return; // no system fonts available in this environment
    };
    let stack = FontPreset::Github
        .body_families()
        .expect("github body list");
    assert!(
        stack.contains(&primary.selected.family.as_str()),
        "preset lead {:?} must come from the emulated stack",
        primary.selected.family
    );
}
