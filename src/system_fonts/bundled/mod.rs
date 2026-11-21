//! The font families the document is set in, and the Reader preset's own
//! faces, which are compiled into the binary.

use egui::{FontData, FontDefinitions, FontFamily};
use egui_commonmark_extended::STRONG_FONT_FAMILY;

use crate::system_fonts::INLINE_CODE_SCALE;

#[cfg(test)]
mod tests;

/// Family of the document's body text.
pub(crate) const DOCUMENT_BODY_FAMILY: &str = "MarkdownBody";
/// Family of the document's headings: a bold face where there is one.
pub(crate) const DOCUMENT_HEADING_FAMILY: &str = "MarkdownHeading";
/// Family of emphasized text when the document's faces include a true
/// italic.
pub(crate) const DOCUMENT_EMPHASIS_FAMILY: &str = "MarkdownEmphasis";
/// Family of inline code in bold text: a bold code face where there is one.
pub(crate) const DOCUMENT_STRONG_CODE_FAMILY: &str = "MarkdownStrongCode";

macro_rules! bundled_face {
    ($file:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/", $file))
    };
}

/// Source Serif 4 (SIL Open Font License 1.1; see
/// `assets/fonts/SourceSerif4-LICENSE.txt`).
/// Each entry is the key the face is registered under and its file.
const READER_REGULAR: (&str, &[u8]) = ("ReaderRegular", bundled_face!("SourceSerif4-Regular.ttf"));
const READER_ITALIC: (&str, &[u8]) = ("ReaderItalic", bundled_face!("SourceSerif4-It.ttf"));
const READER_SEMIBOLD: (&str, &[u8]) =
    ("ReaderSemibold", bundled_face!("SourceSerif4-Semibold.ttf"));
const READER_BOLD: (&str, &[u8]) = ("ReaderBold", bundled_face!("SourceSerif4-Bold.ttf"));

/// Source Code Pro, the serif's companion code face (SIL Open Font License
/// 1.1; see `assets/fonts/SourceCodePro-LICENSE.txt`).
const READER_CODE: (&str, &[u8]) = ("ReaderCode", bundled_face!("SourceCodePro-Regular.ttf"));
const READER_CODE_SEMIBOLD: (&str, &[u8]) = (
    "ReaderCodeSemibold",
    bundled_face!("SourceCodePro-Semibold.ttf"),
);

/// Register the families the document pane names: body, strong, heading,
/// emphasis and strong code. Every font configuration gets all of them,
/// because egui panics on a family nobody registered.
///
/// With `bundled`, each family leads with its Source Serif 4 face, and the
/// monospace family and the strong code family with their Source Code Pro
/// face. The rest of every chain is what the chrome uses, so a glyph the
/// bundled face lacks (CJK, emoji) still resolves. Without it, the families
/// are the chrome's own regular, bold and monospace chains.
pub(crate) fn install_document_families(definitions: &mut FontDefinitions, bundled: bool) {
    let named = |family: &str| FontFamily::Name(family.into());
    let regular = chain(definitions, &FontFamily::Proportional);
    let bold = chain(definitions, &named(STRONG_FONT_FAMILY));
    let code = chain(definitions, &FontFamily::Monospace);
    let prose = |(key, face): (&'static str, &'static [u8])| (key, FontData::from_static(face));
    let families = [
        (named(DOCUMENT_BODY_FAMILY), prose(READER_REGULAR), &regular),
        (named(STRONG_FONT_FAMILY), prose(READER_SEMIBOLD), &bold),
        (named(DOCUMENT_HEADING_FAMILY), prose(READER_BOLD), &bold),
        (
            named(DOCUMENT_EMPHASIS_FAMILY),
            prose(READER_ITALIC),
            &regular,
        ),
        (FontFamily::Monospace, code_face(READER_CODE), &code),
        (
            named(DOCUMENT_STRONG_CODE_FAMILY),
            code_face(READER_CODE_SEMIBOLD),
            &code,
        ),
    ];
    for (family, (key, face), fallbacks) in families {
        let mut faces = Vec::with_capacity(fallbacks.len() + 1);
        if bundled {
            definitions.font_data.insert(key.to_owned(), face.into());
            faces.push(key.to_owned());
        }
        faces.extend(fallbacks.iter().cloned());
        definitions.families.insert(family, faces);
    }
}

/// A bundled code face, shifted down so that code set
/// [`INLINE_CODE_SCALE`] the size of the serif around it sits on the
/// serif's baseline.
///
/// egui hangs every face from the top of the line by its own ascent, so a
/// smaller face with a lower ascent floats above the text beside it. The
/// shift is a visual tweak of the face: it moves no line. It also centers
/// the lines of a code block in their box, which the same ascent gap left
/// top-heavy.
fn code_face((key, face): (&'static str, &'static [u8])) -> (&'static str, FontData) {
    let mut data = FontData::from_static(face);
    data.tweak.y_offset_factor = code_baseline_shift(face).unwrap_or(0.0);
    (key, data)
}

/// How far down to shift the code face `code`, as a fraction of its size.
/// `None` when it cannot be read.
fn code_baseline_shift(code: &[u8]) -> Option<f32> {
    let serif_ascent = ascent_em(READER_REGULAR.1, 0)?;
    Some(serif_ascent / INLINE_CODE_SCALE - ascent_em(code, 0)?)
}

/// Distance from a face's baseline to the top of its line, in ems.
fn ascent_em(data: &[u8], index: u32) -> Option<f32> {
    let face = ttf_parser::Face::parse(data, index).ok()?;
    Some(f32::from(face.ascender()) / f32::from(face.units_per_em()))
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
