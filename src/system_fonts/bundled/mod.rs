//! The font families the document is set in, and the Reader preset's own
//! faces, which are compiled into the binary.

use egui::{FontData, FontDefinitions, FontFamily};
use egui_commonmark_extended::STRONG_FONT_FAMILY;

use super::FontPreset;

#[cfg(test)]
mod tests;

/// Family of the document's body text.
pub(crate) const DOCUMENT_BODY_FAMILY: &str = "MarkdownBody";
/// Family of the document's headings: a bold face where there is one.
pub(crate) const DOCUMENT_HEADING_FAMILY: &str = "MarkdownHeading";
/// Family of emphasized text when the document's faces include a true
/// italic.
pub(crate) const DOCUMENT_EMPHASIS_FAMILY: &str = "MarkdownEmphasis";
/// Family of inline code: the code face set on the body's baseline.
pub(crate) const DOCUMENT_CODE_FAMILY: &str = "MarkdownCode";
/// Family of inline code in bold text: a bold code face where there is one.
pub(crate) const DOCUMENT_STRONG_CODE_FAMILY: &str = "MarkdownStrongCode";

macro_rules! bundled_face {
    ($file:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/", $file))
    };
}

/// A face compiled into the binary.
struct BundledFace {
    /// The key the face is registered under.
    key: &'static str,
    bytes: &'static [u8],
    /// How far down the face is shifted, as a fraction of its size.
    baseline_shift: f32,
}

impl BundledFace {
    fn data(&self) -> FontData {
        let mut data = FontData::from_static(self.bytes);
        data.tweak.y_offset_factor = self.baseline_shift;
        data
    }
}

/// Source Serif 4 (SIL Open Font License 1.1; see
/// `assets/fonts/SourceSerif4-LICENSE.txt`).
const READER_REGULAR: BundledFace = BundledFace {
    key: "ReaderRegular",
    bytes: bundled_face!("SourceSerif4-Regular.ttf"),
    baseline_shift: 0.0,
};
const READER_ITALIC: BundledFace = BundledFace {
    key: "ReaderItalic",
    bytes: bundled_face!("SourceSerif4-It.ttf"),
    baseline_shift: 0.0,
};
const READER_SEMIBOLD: BundledFace = BundledFace {
    key: "ReaderSemibold",
    bytes: bundled_face!("SourceSerif4-Semibold.ttf"),
    baseline_shift: 0.0,
};
const READER_BOLD: BundledFace = BundledFace {
    key: "ReaderBold",
    bytes: bundled_face!("SourceSerif4-Bold.ttf"),
    baseline_shift: 0.0,
};

/// Key of the aligned inline-code face when the document uses a system or
/// preset monospace face rather than the bundled one.
pub(crate) const DOCUMENT_INLINE_CODE_KEY: &str = "DocumentInlineCode";

/// Source Code Pro, the serif's companion code face (SIL Open Font License
/// 1.1; see `assets/fonts/SourceCodePro-LICENSE.txt`). Code blocks and
/// inline code are set in it with different shifts, so each is registered
/// under a key of its own.
const READER_CODE: BundledFace = BundledFace {
    key: "ReaderCode",
    bytes: bundled_face!("SourceCodePro-Regular.ttf"),
    baseline_shift: CODE_BLOCK_SHIFT,
};
const READER_INLINE_CODE: BundledFace = BundledFace {
    key: "ReaderInlineCode",
    bytes: bundled_face!("SourceCodePro-Regular.ttf"),
    baseline_shift: 0.0,
};
const READER_CODE_SEMIBOLD: BundledFace = BundledFace {
    key: "ReaderCodeSemibold",
    bytes: bundled_face!("SourceCodePro-Semibold.ttf"),
    baseline_shift: 0.0,
};

/// Distance from the baseline to the top of the line in Source Code Pro:
/// 984 of 1000 units.
const CODE_ASCENT_EM: f32 = 0.984;
/// Distance from the baseline to the bottom of the line in Source Code
/// Pro: 273 of 1000 units.
const CODE_DESCENT_EM: f32 = 0.273;

/// How far down the letters of a code block are shifted, as a fraction of
/// their size: half the room the block's line has over the face's own, so
/// that each line of code sits in the middle of its line.
const CODE_BLOCK_SHIFT: f32 =
    (FontPreset::Reader.code_line_height() - 1.0) * (CODE_ASCENT_EM + CODE_DESCENT_EM) / 2.0;

/// Bounds for scaling an inline code face to match the body face's x-height,
/// so an unusual font with a tiny or huge `x` never produces unreadable code.
const INLINE_CODE_SCALE_MIN: f32 = 0.75;
const INLINE_CODE_SCALE_MAX: f32 = 1.15;

/// Vertical metrics read from a font face, in fractions of its em square.
struct FaceMetrics {
    ascent_em: f32,
    x_height_em: Option<f32>,
}

/// Read the ascender and x-height of `data` in ems. Uses the actual `'x'`
/// glyph's bounding box first because some system fonts (such as macOS's
/// Iowan Old Style) omit `OS/2.sxHeight`.
fn face_metrics(data: &FontData) -> Option<FaceMetrics> {
    let face = ttf_parser::Face::parse(&data.font, data.index).ok()?;
    let em = f32::from(face.units_per_em());
    if em <= 0.0 {
        return None;
    }
    let ascent_em = f32::from(face.ascender()) / em;
    let x_height_em = face
        .glyph_index('x')
        .and_then(|id| face.glyph_bounding_box(id))
        .filter(|bb| bb.y_max > 0)
        .map(|bb| f32::from(bb.y_max) / em)
        .or_else(|| {
            face.x_height()
                .filter(|&x| x > 0)
                .map(|x| f32::from(x) / em)
        });
    Some(FaceMetrics {
        ascent_em,
        x_height_em,
    })
}

/// Compute the `FontTweak` that makes `code` match `body`'s x-height and
/// stand on `body`'s baseline.
///
/// egui hangs every face from the top of its line by `ascent_em * font_size *
/// tweak.scale` and then shifts it down by `y_offset_factor * font_size *
/// tweak.scale`. Matching `(ascent_em + y_offset_factor) * scale` to `body`'s
/// baseline keeps inline code on the prose baseline at every text size, zoom
/// level, and heading level.
fn inline_code_tweak(body: &FontData, code: &FontData) -> Option<egui::FontTweak> {
    let body_metrics = face_metrics(body)?;
    let code_metrics = face_metrics(code)?;
    let scale = match (body_metrics.x_height_em, code_metrics.x_height_em) {
        (Some(body_x), Some(code_x)) => ((body_x * body.tweak.scale) / code_x)
            .clamp(INLINE_CODE_SCALE_MIN, INLINE_CODE_SCALE_MAX),
        _ => 1.0,
    };
    let body_baseline = (body_metrics.ascent_em + body.tweak.y_offset_factor) * body.tweak.scale;
    let y_offset_factor = body_baseline / scale - code_metrics.ascent_em;
    Some(egui::FontTweak {
        scale,
        y_offset_factor,
        ..code.tweak
    })
}

/// Register the families the document pane names: body, strong, heading,
/// emphasis, code and strong code. Every font configuration gets all of them,
/// because egui panics on a family nobody registered.
///
/// With `bundled`, each family leads with its Source Serif 4 face, and the
/// monospace family and the two inline code families with their Source
/// Code Pro face. The rest of every chain is what the chrome uses, so a glyph the
/// bundled face lacks (CJK, emoji) still resolves. Without it, the families
/// are the chrome's own regular, bold and monospace chains. In both cases,
/// the inline code faces are tweaked to match the body face's x-height and
/// baseline without changing `FontFamily::Monospace` (code blocks).
pub(crate) fn install_document_families(definitions: &mut FontDefinitions, bundled: bool) {
    let named = |family: &str| FontFamily::Name(family.into());
    let regular = chain(definitions, &FontFamily::Proportional);
    let bold = chain(definitions, &named(STRONG_FONT_FAMILY));
    let code = chain(definitions, &FontFamily::Monospace);
    let mut install = |family, face: &BundledFace, fallbacks: &[String]| {
        let mut faces = Vec::with_capacity(fallbacks.len() + 1);
        if bundled {
            definitions
                .font_data
                .insert(face.key.to_owned(), face.data().into());
            faces.push(face.key.to_owned());
        }
        faces.extend_from_slice(fallbacks);
        definitions.families.insert(family, faces);
    };
    install(named(DOCUMENT_BODY_FAMILY), &READER_REGULAR, &regular);
    install(named(STRONG_FONT_FAMILY), &READER_SEMIBOLD, &bold);
    install(named(DOCUMENT_HEADING_FAMILY), &READER_BOLD, &bold);
    install(named(DOCUMENT_EMPHASIS_FAMILY), &READER_ITALIC, &regular);
    install(FontFamily::Monospace, &READER_CODE, &code);
    install(named(DOCUMENT_CODE_FAMILY), &READER_INLINE_CODE, &code);
    install(
        named(DOCUMENT_STRONG_CODE_FAMILY),
        &READER_CODE_SEMIBOLD,
        &code,
    );
    align_inline_code_faces(definitions, bundled);
}

/// Scale and shift the lead faces of the inline-code families so they match
/// the document body face's x-height and baseline.
fn align_inline_code_faces(definitions: &mut FontDefinitions, bundled: bool) {
    let named = |family: &str| FontFamily::Name(family.into());
    let Some(body_key) = definitions
        .families
        .get(&named(DOCUMENT_BODY_FAMILY))
        .and_then(|faces| faces.first())
        .cloned()
    else {
        return;
    };
    let Some(body_data) = definitions.font_data.get(&body_key).cloned() else {
        return;
    };

    if bundled {
        for key in [READER_INLINE_CODE.key, READER_CODE_SEMIBOLD.key] {
            if let Some(code_data) = definitions.font_data.get(key) {
                if let Some(tweak) = inline_code_tweak(&body_data, code_data) {
                    let mut adjusted = FontData::clone(code_data);
                    adjusted.tweak = tweak;
                    definitions
                        .font_data
                        .insert(key.to_owned(), adjusted.into());
                }
            }
        }
        return;
    }

    let Some(code_key) = definitions
        .families
        .get(&named(DOCUMENT_CODE_FAMILY))
        .and_then(|faces| faces.first())
        .cloned()
    else {
        return;
    };
    let Some(code_data) = definitions.font_data.get(&code_key) else {
        return;
    };
    let Some(tweak) = inline_code_tweak(&body_data, code_data) else {
        return;
    };
    let mut adjusted = FontData::clone(code_data);
    adjusted.tweak = tweak;
    definitions
        .font_data
        .insert(DOCUMENT_INLINE_CODE_KEY.to_owned(), adjusted.into());

    for family in [DOCUMENT_CODE_FAMILY, DOCUMENT_STRONG_CODE_FAMILY] {
        if let Some(faces) = definitions.families.get_mut(&named(family)) {
            if let Some(first) = faces.first_mut().filter(|first| **first == code_key) {
                DOCUMENT_INLINE_CODE_KEY.clone_into(first);
            }
        }
    }
}

/// The faces `family` resolves through, in order; none when it is not
/// registered.
fn chain(definitions: &FontDefinitions, family: &FontFamily) -> Vec<String> {
    definitions
        .families
        .get(family)
        .cloned()
        .unwrap_or_default()
}
