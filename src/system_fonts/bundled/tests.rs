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
    let code = &before.families[&FontFamily::Monospace];
    assert_eq!(&definitions.families[&FontFamily::Monospace], code);
    // Inline code leads with an aligned copy of the monospace face so code
    // blocks keep their untouched chain while inline code matches the body.
    let aligned_code = led_by(DOCUMENT_INLINE_CODE_KEY, &code[1..]);
    assert_eq!(family(&definitions, DOCUMENT_CODE_FAMILY), aligned_code);
    assert_eq!(
        family(&definitions, DOCUMENT_STRONG_CODE_FAMILY),
        aligned_code
    );
    // Only the aligned inline-code copy is added to font_data.
    assert_eq!(definitions.font_data.len(), before.font_data.len() + 1);
    assert!(definitions.font_data.contains_key(DOCUMENT_INLINE_CODE_KEY));
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
fn inline_code_is_scaled_to_the_body_x_height_and_shifted_onto_its_baseline() {
    for bundled in [true, false] {
        let mut definitions = chrome_definitions();
        install_document_families(&mut definitions, bundled);

        let body_key = &family(&definitions, DOCUMENT_BODY_FAMILY)[0];
        let body = &definitions.font_data[body_key];
        let body_metrics = face_metrics(body).expect("body face metrics");

        let code_keys: &[&str] = if bundled {
            &["ReaderInlineCode", "ReaderCodeSemibold"]
        } else {
            &[DOCUMENT_INLINE_CODE_KEY]
        };
        for key in code_keys {
            let code = &definitions.font_data[*key];
            let code_metrics = face_metrics(code).expect("code face metrics");

            // egui hangs a face from the top of its line by `ascent * size *
            // scale` and shifts it by `y_offset_factor * size * scale`.
            let body_size = 20.0;
            let body_baseline = (body_metrics.ascent_em + body.tweak.y_offset_factor)
                * body.tweak.scale
                * body_size;
            let code_baseline = (code_metrics.ascent_em + code.tweak.y_offset_factor)
                * code.tweak.scale
                * body_size;
            assert!(
                (code_baseline - body_baseline).abs() < 1e-3,
                "bundled={bundled} {key}: code baseline {code_baseline}, body baseline {body_baseline}"
            );

            // Lowercase x-height matches the body face's x-height.
            let body_x = body_metrics.x_height_em.unwrap() * body.tweak.scale * body_size;
            let code_x = code_metrics.x_height_em.unwrap() * code.tweak.scale * body_size;
            assert!(
                (code_x - body_x).abs() < 1e-3,
                "bundled={bundled} {key}: code x-height {code_x}, body x-height {body_x}"
            );
        }
    }
}

#[test]
fn the_letters_of_a_code_block_sit_in_the_middle_of_their_line() {
    let mut definitions = chrome_definitions();
    install_document_families(&mut definitions, true);
    let code = &definitions.font_data["ReaderCode"];
    let face = ttf_parser::Face::parse(&code.font, code.index).unwrap();
    let em = f32::from(face.units_per_em());
    let own_line = f32::from(face.ascender() - face.descender()) / em;
    let line = FontPreset::Reader.code_line_height() * own_line;

    let shift = code.tweak.y_offset_factor;
    let room_below = line - shift - own_line;

    assert!(shift > 0.0, "the letters move down, got {shift}");
    assert!(
        (shift - room_below).abs() < 1e-3,
        "{shift} above, {room_below} below"
    );
}

#[test]
fn only_the_code_faces_are_shifted() {
    let mut definitions = chrome_definitions();
    install_document_families(&mut definitions, true);
    let code_faces = ["ReaderCode", "ReaderInlineCode", "ReaderCodeSemibold"];

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
