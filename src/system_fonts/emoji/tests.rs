use super::*;

use std::borrow::Cow;

use egui::{Color32, FontFamily};
use egui_commonmark_extended::STRONG_FONT_FAMILY;
use fontique::FontWeight;

use crate::system_fonts::bundled::{
    DOCUMENT_BODY_FAMILY, DOCUMENT_EMPHASIS_FAMILY, DOCUMENT_HEADING_FAMILY,
};
use crate::system_fonts::tests::{bundled_face, Fonts, LATIN_FACE};
use crate::system_fonts::{font_definitions, FontConfig, FontPreset};

/// Subset of Noto Color Emoji holding only the glyphs these tests name, so
/// they run the same on a machine with no color emoji font installed. See
/// `fixtures/README.txt` for how it was produced.
const COLOR_EMOJI_FIXTURE: &[u8] = include_bytes!("fixtures/color-emoji-subset.ttf");

/// 🟢 🔴 🔵 ⚪ ✅, the emoji the project's own documents use.
const DOC_EMOJI: [char; 5] = [
    '\u{1F7E2}',
    '\u{1F534}',
    '\u{1F535}',
    '\u{26AA}',
    '\u{2705}',
];

/// 🟢 🔴 ✅, the doc emoji drawn as filled shapes; ⚪ is a thin outline with
/// too little ink for a blank-glyph check to tell apart from nothing.
const SOLID_DOC_EMOJI: [char; 3] = ['\u{1F7E2}', '\u{1F534}', '\u{2705}'];

/// ▶ ⚠ ✔, symbols the UI draws that Noto Color Emoji also maps.
const UI_SYMBOLS_IN_COLOR_FONT: [char; 3] = ['\u{25B6}', '\u{26A0}', '\u{2714}'];

/// The emoji faces `setup_fonts` adds on a machine with `fonts` installed.
fn emoji_definitions(mut fonts: Fonts) -> FontDefinitions {
    let mut definitions = FontDefinitions::default();
    install_emoji_fonts(&mut fonts.system(), &mut definitions);
    definitions
}

/// What `setup_fonts` installs on a machine whose color emoji font is the
/// fixture, installed under the family name it carries: Noto Color Emoji.
fn definitions_with_color_font() -> FontDefinitions {
    let mut fonts = Fonts::none();
    fonts.register_as_is(COLOR_EMOJI_FIXTURE);
    emoji_definitions(fonts)
}

/// What `setup_fonts` installs on a machine with no color emoji font.
fn definitions_without_color_font() -> FontDefinitions {
    emoji_definitions(Fonts::none())
}

/// `definitions` narrowed to the face `key` as the whole proportional chain,
/// to see what that one face supplies.
fn only_face(mut definitions: FontDefinitions, key: &str) -> FontDefinitions {
    definitions.font_data.retain(|name, _| name == key);
    definitions
        .families
        .insert(FontFamily::Proportional, vec![key.to_owned()]);
    definitions
}

fn context_with_fonts(definitions: FontDefinitions) -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_fonts(definitions);
    // Fonts aren't built until the first Context::run() pass.
    let _ = ctx.run(egui::RawInput::default(), |_ctx| {});
    ctx
}

fn font_id() -> egui::FontId {
    egui::FontId::proportional(16.0)
}

fn has_glyph(ctx: &egui::Context, ch: char) -> bool {
    ctx.fonts_mut(|fonts| fonts.has_glyph(&font_id(), ch))
}

/// A glyph as it landed in the font atlas.
struct RasterizedGlyph {
    pixels: Vec<Color32>,
    /// Whether the glyph took epaint's color-glyph path.
    colored: bool,
}

impl RasterizedGlyph {
    fn ink_pixels(&self) -> usize {
        self.pixels.iter().filter(|px| px.a() > 16).count()
    }

    /// Brightest `channel` value among pixels where it clearly dominates
    /// `counterpart`, or 0 when no pixel is tinted that way.
    fn peak(&self, channel: fn(&Color32) -> u8, counterpart: fn(&Color32) -> u8) -> u8 {
        self.pixels
            .iter()
            .filter(|px| px.a() > 32)
            .map(|px| (channel(px), counterpart(px)))
            .filter(|&(value, other)| value > 48 && u16::from(value) > u16::from(other) + 16)
            .map(|(value, _)| value)
            .max()
            .unwrap_or(0)
    }
}

fn rasterize(ctx: &egui::Context, ch: char) -> RasterizedGlyph {
    rasterize_in(ctx, font_id(), ch)
}

/// `ch` as text set in `font` draws it.
fn rasterize_in(ctx: &egui::Context, font: egui::FontId, ch: char) -> RasterizedGlyph {
    let job = egui::text::LayoutJob::simple(ch.to_string(), font, Color32::WHITE, f32::INFINITY);
    ctx.fonts_mut(|fonts| {
        let galley = fonts.layout_job(job);
        let uv = galley.rows[0].glyphs[0].uv_rect;
        let image = fonts.image();
        let [width, height] = image.size;
        let columns = usize::from(uv.min[0])..usize::from(uv.max[0]).min(width);
        let rows = usize::from(uv.min[1])..usize::from(uv.max[1]).min(height);
        let pixels = rows
            .flat_map(|y| columns.clone().map(move |x| (x, y)))
            .map(|position| image[position])
            .collect();
        RasterizedGlyph {
            pixels,
            colored: uv.colored,
        }
    })
}

#[test]
fn emoji_fallback_face_resolves_for_doc_emoji() {
    let ctx = context_with_fonts(definitions_with_color_font());
    for ch in DOC_EMOJI {
        assert!(
            has_glyph(&ctx, ch),
            "emoji {ch} should resolve through the fallback chain"
        );
    }
}

/// A cmap hit with an empty outline would pass `has_glyph`, so require real
/// coverage in the atlas.
#[test]
fn doc_emoji_rasterize_with_visible_ink() {
    let ctx = context_with_fonts(definitions_with_color_font());
    for ch in SOLID_DOC_EMOJI {
        let ink = rasterize(&ctx, ch).ink_pixels();
        assert!(
            ink > 20,
            "emoji {ch} rasterized blank (only {ink} ink pixels)"
        );
    }
}

/// Needs the vendored epaint with the swash patch: each emoji must come from
/// the color face and leave saturated pixels of its own hue in the atlas.
#[test]
fn doc_emoji_render_in_color() {
    let ctx = context_with_fonts(definitions_with_color_font());
    let green: fn(&Color32) -> u8 = Color32::g;
    let red: fn(&Color32) -> u8 = Color32::r;
    for (ch, hue, channel, counterpart) in [
        ('\u{1F7E2}', "green", green, red),
        ('\u{1F534}', "red", red, green),
    ] {
        let glyph = rasterize(&ctx, ch);
        assert!(
            glyph.colored,
            "emoji {ch} did not take the color-glyph path"
        );
        let peak = glyph.peak(channel, counterpart);
        assert!(
            peak > 128,
            "emoji {ch} rendered without color (peak {hue} = {peak})"
        );
    }
}

#[test]
fn doc_emoji_stay_visible_without_a_color_font() {
    let ctx = context_with_fonts(definitions_without_color_font());
    for ch in DOC_EMOJI {
        assert!(
            has_glyph(&ctx, ch),
            "emoji {ch} should resolve to a monochrome face"
        );
        assert!(
            !rasterize(&ctx, ch).colored,
            "emoji {ch} has no color face to come from"
        );
    }
    for ch in SOLID_DOC_EMOJI {
        let ink = rasterize(&ctx, ch).ink_pixels();
        assert!(
            ink > 20,
            "emoji {ch} rasterized blank (only {ink} ink pixels)"
        );
    }
}

#[test]
fn bundled_monochrome_face_covers_doc_emoji_on_its_own() {
    let ctx = context_with_fonts(only_face(
        definitions_without_color_font(),
        MONOCHROME_EMOJI_KEY,
    ));
    for ch in DOC_EMOJI {
        assert!(has_glyph(&ctx, ch), "bundled NotoEmoji lacks {ch}");
    }
}

#[test]
fn color_emoji_face_never_claims_ui_symbols() {
    let ctx = context_with_fonts(only_face(definitions_with_color_font(), COLOR_EMOJI_KEY));
    // Default-emoji characters resolve through the color face...
    for ch in ['\u{1F7E2}', '\u{1F534}', '\u{26AA}', '\u{2705}'] {
        assert!(has_glyph(&ctx, ch), "{ch} should come from ColorEmoji");
    }
    // ...but general UI symbols must not, even though Noto Color Emoji
    // maps them — they belong to regular text faces.
    for ch in [
        '\u{25B6}', '\u{25BC}', '\u{26A0}', '\u{2714}', '\u{2192}', '\u{2713}',
    ] {
        assert!(
            !has_glyph(&ctx, ch),
            "{ch} (UI symbol) must not be claimed by the color emoji face"
        );
    }
}

/// Control for the test above: without the emoji-only restriction the very
/// same face does claim the UI symbols, so their absence there is the
/// restriction at work and not a gap in the fixture.
#[test]
fn color_emoji_font_maps_the_ui_symbols_it_is_kept_from_claiming() {
    let mut definitions = only_face(definitions_with_color_font(), COLOR_EMOJI_KEY);
    let restricted = definitions
        .font_data
        .get(COLOR_EMOJI_KEY)
        .expect("color face installed");
    let mut unrestricted = FontData::clone(restricted);
    unrestricted.emoji_only = false;
    definitions
        .font_data
        .insert(COLOR_EMOJI_KEY.to_owned(), unrestricted.into());

    let ctx = context_with_fonts(definitions);
    for ch in UI_SYMBOLS_IN_COLOR_FONT {
        assert!(has_glyph(&ctx, ch), "fixture should map {ch}");
    }
}

/// The color face is found through the platform's font collection, so it
/// loads wherever a distribution keeps the file (Debian and Ubuntu use
/// `/usr/share/fonts/truetype/noto/`, Arch `/usr/share/fonts/noto/`).
#[test]
fn color_face_goes_ahead_of_the_built_in_emoji_faces_and_monochrome_last() {
    let definitions = definitions_with_color_font();

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let chain = &definitions.families[&family];
        let position = |key: &str| chain.iter().position(|name| name == key);
        let color = position(COLOR_EMOJI_KEY).expect("color face in the chain");
        for built_in in BUILTIN_EMOJI_KEYS {
            assert!(color < position(built_in).expect("built-in emoji face"));
        }
        assert_eq!(chain.last().map(String::as_str), Some(MONOCHROME_EMOJI_KEY));
    }
}

#[test]
fn machine_without_a_color_emoji_font_gets_the_monochrome_face_only() {
    let definitions = definitions_without_color_font();

    assert!(!definitions.font_data.contains_key(COLOR_EMOJI_KEY));
    let mut expected = FontDefinitions::default().families[&FontFamily::Proportional].clone();
    expected.push(MONOCHROME_EMOJI_KEY.to_owned());
    assert_eq!(definitions.families[&FontFamily::Proportional], expected);
}

/// A family that merely carries the name cannot stand in for the color face.
#[test]
fn family_without_emoji_glyphs_is_not_taken_for_the_color_face() {
    let mut fonts = Fonts::none();
    fonts.register(
        &bundled_face(LATIN_FACE),
        COLOR_EMOJI_FAMILY,
        FontWeight::NORMAL,
    );

    let definitions = emoji_definitions(fonts);

    assert!(!definitions.font_data.contains_key(COLOR_EMOJI_KEY));
}

/// The monochrome face is part of the binary: an installed copy has no
/// source tree to read it from.
#[test]
fn monochrome_face_is_compiled_into_the_binary() {
    let definitions = definitions_without_color_font();
    let face = &definitions.font_data[MONOCHROME_EMOJI_KEY];
    assert!(
        matches!(face.font, Cow::Borrowed(_)),
        "the face was read from a file at run time"
    );
}

/// Definitions a caller built without the standard families get the faces
/// registered, and no family chain is invented for them.
#[test]
fn definitions_without_family_chains_only_gain_the_font_data() {
    let mut fonts = Fonts::none();
    fonts.register_as_is(COLOR_EMOJI_FIXTURE);
    let mut definitions = FontDefinitions::empty();
    definitions.families.clear();

    install_emoji_fonts(&mut fonts.system(), &mut definitions);

    let mut faces: Vec<&str> = definitions.font_data.keys().map(String::as_str).collect();
    faces.sort_unstable();
    assert_eq!(faces, [COLOR_EMOJI_KEY, MONOCHROME_EMOJI_KEY]);
    assert!(definitions.families.is_empty());
}

/// Every family the app sets text in, on a desktop with a sans (regular and
/// bold), script fallbacks and the color emoji font.
fn text_families() -> [FontFamily; 6] {
    let named = |family: &str| FontFamily::Name(family.into());
    [
        FontFamily::Proportional,
        FontFamily::Monospace,
        named(STRONG_FONT_FAMILY),
        named(DOCUMENT_BODY_FAMILY),
        named(DOCUMENT_HEADING_FAMILY),
        named(DOCUMENT_EMPHASIS_FAMILY),
    ]
}

/// The chains `preset` installs where the color emoji font is the fixture.
fn preset_definitions(preset: FontPreset) -> FontDefinitions {
    let mut fonts = Fonts::multilingual();
    fonts.register_as_is(COLOR_EMOJI_FIXTURE);
    let config = FontConfig {
        preset,
        ..FontConfig::default()
    };
    font_definitions(&mut fonts.system(), Some("en-US"), &config)
}

/// Bold text, headings and emphasis draw an emoji from the same face as the
/// text around them: in color where the machine has a color emoji font.
#[test]
fn emoji_are_in_color_in_every_weight_of_every_preset() {
    for preset in FontPreset::ALL {
        let ctx = context_with_fonts(preset_definitions(preset));
        for family in text_families() {
            let glyph = rasterize_in(&ctx, egui::FontId::new(16.0, family.clone()), '\u{1F534}');
            assert!(
                glyph.colored && glyph.peak(Color32::r, Color32::g) > 128,
                "{preset:?} draws 🔴 in {family:?} without color"
            );
        }
    }
}

/// Every chain holds the same emoji faces in the same order, so no weight
/// resolves an emoji differently from another.
#[test]
fn every_chain_ends_in_the_same_emoji_faces() {
    let emoji_faces = [
        COLOR_EMOJI_KEY,
        BUILTIN_EMOJI_KEYS[0],
        BUILTIN_EMOJI_KEYS[1],
        MONOCHROME_EMOJI_KEY,
    ];
    for preset in FontPreset::ALL {
        let definitions = preset_definitions(preset);
        for family in text_families() {
            let chain = &definitions.families[&family];
            let emoji: Vec<&str> = chain
                .iter()
                .map(String::as_str)
                .filter(|name| emoji_faces.contains(name))
                .collect();
            assert_eq!(emoji, emoji_faces, "{preset:?} {family:?}");
            assert_eq!(chain.last().map(String::as_str), Some(MONOCHROME_EMOJI_KEY));
        }
    }
}

/// A variation selector picks the look of the character before it and is
/// never drawn itself: ❤️ is as wide as ❤, in every family.
#[test]
fn a_variation_selector_takes_no_room() {
    let ctx = context_with_fonts(preset_definitions(FontPreset::Reader));
    let width = |family: &FontFamily, text: &str| {
        let font = egui::FontId::new(16.0, family.clone());
        let job =
            egui::text::LayoutJob::simple(text.to_owned(), font, Color32::WHITE, f32::INFINITY);
        ctx.fonts_mut(|fonts| fonts.layout_job(job).rect.width())
    };
    for family in text_families() {
        let plain = width(&family, "a\u{2764}b");
        for selector in ['\u{FE0E}', '\u{FE0F}'] {
            let selected = width(&family, &format!("a\u{2764}{selector}b"));
            assert!(
                (selected - plain).abs() < f32::EPSILON,
                "{selector:?} is {} wide in {family:?}",
                selected - plain
            );
        }
    }
}
