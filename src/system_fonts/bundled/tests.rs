use super::*;

/// egui's own definitions plus a strong family, as `font_definitions` leaves
/// them before the document families go in.
fn chrome_definitions() -> FontDefinitions {
    let mut definitions = FontDefinitions::default();
    let mut bold = definitions.families[&FontFamily::Proportional].clone();
    bold.insert(0, "ChromeBold".to_owned());
    definitions
        .families
        .insert(FontFamily::Name(STRONG_FONT_FAMILY.into()), bold);
    definitions
}

fn family(definitions: &FontDefinitions, name: &str) -> Vec<String> {
    definitions.families[&FontFamily::Name(name.into())].clone()
}

/// `fallbacks` with `lead` in front.
fn led_by(lead: &str, fallbacks: &[String]) -> Vec<String> {
    let mut chain = vec![lead.to_owned()];
    chain.extend(fallbacks.iter().cloned());
    chain
}

/// Distance from a face's baseline to the top of its line, in ems.
fn ascent_em(data: &[u8], index: u32) -> f32 {
    let face = ttf_parser::Face::parse(data, index).unwrap();
    f32::from(face.ascender()) / f32::from(face.units_per_em())
}

/// A registered face, parsed.
fn face<'a>(definitions: &'a FontDefinitions, key: &str) -> ttf_parser::Face<'a> {
    let data = &definitions.font_data[key];
    ttf_parser::Face::parse(&data.font, data.index).unwrap()
}

#[test]
fn without_the_bundled_faces_the_document_is_set_in_the_chromes_own_chains() {
    let before = chrome_definitions();
    let mut definitions = before.clone();

    install_document_families(&mut definitions, false);

    let regular = &before.families[&FontFamily::Proportional];
    let bold = &before.families[&FontFamily::Name(STRONG_FONT_FAMILY.into())];
    assert_eq!(&family(&definitions, DOCUMENT_BODY_FAMILY), regular);
    assert_eq!(&family(&definitions, DOCUMENT_EMPHASIS_FAMILY), regular);
    assert_eq!(&family(&definitions, DOCUMENT_HEADING_FAMILY), bold);
    assert_eq!(&family(&definitions, STRONG_FONT_FAMILY), bold);
    let code = &before.families[&FontFamily::Monospace];
    assert_eq!(&definitions.families[&FontFamily::Monospace], code);
    assert_eq!(&family(&definitions, DOCUMENT_STRONG_CODE_FAMILY), code);
    // No face is added and none is touched.
    assert_eq!(definitions.font_data, before.font_data);
}

#[test]
fn the_bundled_serif_leads_every_document_family_and_keeps_the_fallbacks() {
    let before = chrome_definitions();
    let mut definitions = before.clone();

    install_document_families(&mut definitions, true);

    let regular = &before.families[&FontFamily::Proportional];
    let bold = &before.families[&FontFamily::Name(STRONG_FONT_FAMILY.into())];
    assert_eq!(
        family(&definitions, DOCUMENT_BODY_FAMILY),
        led_by("ReaderRegular", regular)
    );
    assert_eq!(
        family(&definitions, DOCUMENT_EMPHASIS_FAMILY),
        led_by("ReaderItalic", regular)
    );
    assert_eq!(
        family(&definitions, STRONG_FONT_FAMILY),
        led_by("ReaderSemibold", bold)
    );
    assert_eq!(
        family(&definitions, DOCUMENT_HEADING_FAMILY),
        led_by("ReaderBold", bold)
    );
    // The chrome keeps its sans: the serif is the document's alone.
    assert_eq!(&definitions.families[&FontFamily::Proportional], regular);
}

#[test]
fn the_bundled_code_face_leads_the_code_families_and_keeps_the_fallbacks() {
    let before = chrome_definitions();
    let mut definitions = before.clone();

    install_document_families(&mut definitions, true);

    let code = &before.families[&FontFamily::Monospace];
    assert_eq!(
        definitions.families[&FontFamily::Monospace],
        led_by("ReaderCode", code)
    );
    assert_eq!(
        family(&definitions, DOCUMENT_STRONG_CODE_FAMILY),
        led_by("ReaderCodeSemibold", code)
    );
}

#[test]
fn each_bundled_code_face_is_the_source_code_pro_cut_its_family_asks_for() {
    let mut definitions = chrome_definitions();
    install_document_families(&mut definitions, true);

    for (key, weight) in [("ReaderCode", 400), ("ReaderCodeSemibold", 600)] {
        let face = face(&definitions, key);
        assert_eq!(face.weight().to_number(), weight, "{key}");
        assert!(face.is_monospaced(), "{key}");
        assert!(!face.is_italic(), "{key}");
        let named_source_code = face.names().into_iter().any(|name| {
            name.name_id == ttf_parser::name_id::FAMILY
                && name
                    .to_string()
                    .is_some_and(|family| family.starts_with("Source Code Pro"))
        });
        assert!(named_source_code, "{key}");
        // What code is written in, and the lines a drawn box is made of.
        for glyph in "Aa0_{}[]<>|\\`~→─│┌┘".chars() {
            assert!(face.glyph_index(glyph).is_some(), "{key} lacks {glyph:?}");
        }
    }
}

#[test]
fn each_bundled_face_is_the_source_serif_cut_its_family_asks_for() {
    let mut definitions = chrome_definitions();
    install_document_families(&mut definitions, true);

    for (key, weight, italic) in [
        ("ReaderRegular", 400, false),
        ("ReaderItalic", 400, true),
        ("ReaderSemibold", 600, false),
        ("ReaderBold", 700, false),
    ] {
        let face = face(&definitions, key);
        assert_eq!(face.weight().to_number(), weight, "{key}");
        assert_eq!(face.is_italic(), italic, "{key}");
        let named_source_serif = face.names().into_iter().any(|name| {
            name.name_id == ttf_parser::name_id::FAMILY
                && name
                    .to_string()
                    .is_some_and(|family| family.starts_with("Source Serif 4"))
        });
        assert!(named_source_serif, "{key}");
        // Latin text, with the punctuation prose uses.
        for glyph in "Aa0é—“”…".chars() {
            assert!(face.glyph_index(glyph).is_some(), "{key} lacks {glyph:?}");
        }
    }
}

#[test]
fn code_set_smaller_than_the_serif_is_shifted_onto_its_baseline() {
    let mut definitions = chrome_definitions();
    install_document_families(&mut definitions, true);

    let code_faces = ["ReaderCode", "ReaderCodeSemibold"];
    for key in code_faces {
        let code = &definitions.font_data[key];
        let shift = code.tweak.y_offset_factor;
        assert!(shift > 0.0, "{key} moves down, got {shift}");

        // egui hangs a face from the top of its line by its ascent. With
        // the shift, code at `INLINE_CODE_SCALE` of the body size has its
        // baseline at the same height as the serif's.
        let body_size = 20.0;
        let serif_baseline = ascent_em(READER_REGULAR.bytes, 0) * body_size;
        let code_ascent = ascent_em(&code.font, code.index);
        let code_baseline = (code_ascent + shift) * body_size * INLINE_CODE_SCALE;
        assert!(
            (code_baseline - serif_baseline).abs() < 1e-3,
            "{key}: code baseline {code_baseline}, serif baseline {serif_baseline}"
        );
    }

    // Only the code faces move.
    for (other, data) in &definitions.font_data {
        if !code_faces.contains(&other.as_str()) {
            assert_eq!(
                data.tweak.y_offset_factor.to_bits(),
                0.0_f32.to_bits(),
                "{other}"
            );
        }
    }
}

#[test]
fn a_family_nobody_registered_has_no_faces() {
    let definitions = FontDefinitions::default();
    assert!(chain(&definitions, &FontFamily::Name("Nobody".into())).is_empty());
    assert!(!chain(&definitions, &FontFamily::Proportional).is_empty());
}
