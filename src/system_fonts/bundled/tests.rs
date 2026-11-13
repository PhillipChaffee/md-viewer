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
    let before = chrome_definitions();
    let mut definitions = before.clone();
    install_document_families(&mut definitions, true);

    let key = &definitions.families[&FontFamily::Monospace][0];
    let code = &definitions.font_data[key];
    let shift = code.tweak.y_offset_factor;
    assert!(shift > 0.0, "the code face moves down, got {shift}");

    // egui hangs a face from the top of its line by its ascent. With the
    // shift, code at `CODE_SCALE` of the body size has its baseline at the
    // same height as the serif's.
    let body_size = 20.0;
    let serif_baseline = ascent_em(READER_REGULAR.1, 0).unwrap() * body_size;
    let code_ascent = ascent_em(&code.font, code.index).unwrap();
    let code_baseline = (code_ascent + shift) * body_size * CODE_SCALE;
    assert!(
        (code_baseline - serif_baseline).abs() < 1e-3,
        "code baseline {code_baseline}, serif baseline {serif_baseline}"
    );

    // Only the code face moves.
    for (other, data) in &definitions.font_data {
        if other != key {
            assert_eq!(
                data.tweak.y_offset_factor.to_bits(),
                before
                    .font_data
                    .get(other)
                    .map_or(0.0_f32, |data| data.tweak.y_offset_factor)
                    .to_bits(),
                "{other}"
            );
        }
    }
}

#[test]
fn an_ascent_is_read_in_ems_and_only_from_a_real_face() {
    let ascent = ascent_em(READER_REGULAR.1, 0).unwrap();
    assert!((0.8..1.2).contains(&ascent), "{ascent}");
    assert_eq!(ascent_em(b"not a font", 0), None);
}

#[test]
fn no_shift_is_worked_out_without_a_readable_code_face() {
    // No monospace family at all.
    let mut definitions = chrome_definitions();
    definitions.families.remove(&FontFamily::Monospace);
    assert_eq!(code_baseline_shift(&definitions), None);

    // A monospace family with no face in it.
    let mut definitions = chrome_definitions();
    definitions
        .families
        .insert(FontFamily::Monospace, Vec::new());
    assert_eq!(code_baseline_shift(&definitions), None);

    // A face the chain names but nobody registered.
    let mut definitions = chrome_definitions();
    definitions
        .families
        .insert(FontFamily::Monospace, vec!["Missing".to_owned()]);
    assert_eq!(code_baseline_shift(&definitions), None);

    // A registered face that is not a font.
    let mut definitions = chrome_definitions();
    definitions.font_data.insert(
        "Broken".to_owned(),
        FontData::from_static(b"not a font").into(),
    );
    definitions
        .families
        .insert(FontFamily::Monospace, vec!["Broken".to_owned()]);
    assert_eq!(code_baseline_shift(&definitions), None);

    // The document families are registered all the same.
    install_document_families(&mut definitions, true);
    assert_eq!(
        family(&definitions, DOCUMENT_BODY_FAMILY)[0],
        "ReaderRegular"
    );
    assert_eq!(
        definitions.font_data["Broken"]
            .tweak
            .y_offset_factor
            .to_bits(),
        0.0_f32.to_bits()
    );
}

#[test]
fn a_family_nobody_registered_has_no_faces() {
    let definitions = FontDefinitions::default();
    assert!(chain(&definitions, &FontFamily::Name("Nobody".into())).is_empty());
    assert!(!chain(&definitions, &FontFamily::Proportional).is_empty());
}
