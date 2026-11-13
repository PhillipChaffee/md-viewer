//! The font families the document is set in, and the Reader preset's own
//! faces, which are compiled into the binary.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};
use egui_commonmark_extended::STRONG_FONT_FAMILY;

use crate::system_fonts::CODE_SCALE;

#[cfg(test)]
mod tests;

/// Family of the document's body text.
pub(crate) const DOCUMENT_BODY_FAMILY: &str = "MarkdownBody";
/// Family of the document's headings: a bold face where there is one.
pub(crate) const DOCUMENT_HEADING_FAMILY: &str = "MarkdownHeading";
/// Family of emphasized text when the document's faces include a true
/// italic.
pub(crate) const DOCUMENT_EMPHASIS_FAMILY: &str = "MarkdownEmphasis";

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

/// Register the families the document pane names: body, strong, heading and
/// emphasis. Every font configuration gets all four, because egui panics on
/// a family nobody registered.
///
/// With `bundled`, each family leads with its Source Serif 4 face. The rest
/// of every chain is what the chrome uses, so a glyph the serif lacks (CJK,
/// emoji) still resolves. Without it, the families are the chrome's own
/// regular and bold chains.
pub(crate) fn install_document_families(definitions: &mut FontDefinitions, bundled: bool) {
    if bundled {
        align_code_baseline(definitions);
    }
    let regular = chain(definitions, &FontFamily::Proportional);
    let bold = chain(definitions, &FontFamily::Name(STRONG_FONT_FAMILY.into()));
    let families = [
        (DOCUMENT_BODY_FAMILY, READER_REGULAR, &regular),
        (STRONG_FONT_FAMILY, READER_SEMIBOLD, &bold),
        (DOCUMENT_HEADING_FAMILY, READER_BOLD, &bold),
        (DOCUMENT_EMPHASIS_FAMILY, READER_ITALIC, &regular),
    ];
    for (family, (key, face), fallbacks) in families {
        let mut faces = Vec::with_capacity(fallbacks.len() + 1);
        if bundled {
            definitions
                .font_data
                .insert(key.to_owned(), FontData::from_static(face).into());
            faces.push(key.to_owned());
        }
        faces.extend(fallbacks.iter().cloned());
        definitions
            .families
            .insert(FontFamily::Name(family.into()), faces);
    }
}

/// Shift the code face down so that code, set [`CODE_SCALE`] the size of the
/// serif around it, sits on the serif's baseline.
///
/// egui hangs every face from the top of the line by its own ascent, so a
/// smaller face with a lower ascent floats above the text beside it. The
/// shift is a visual tweak of the face: it moves no line. It also centers
/// the lines of a code block in their box, which the same ascent gap left
/// top-heavy.
fn align_code_baseline(definitions: &mut FontDefinitions) {
    let Some((key, shift)) = code_baseline_shift(definitions) else {
        return;
    };
    definitions
        .font_data
        .entry(key)
        .and_modify(|face| Arc::make_mut(face).tweak.y_offset_factor = shift);
}

/// The face that leads the monospace chain, and how far down to shift it,
/// as a fraction of its size. `None` when there is no such face or one of
/// the two faces cannot be read.
fn code_baseline_shift(definitions: &FontDefinitions) -> Option<(String, f32)> {
    let key = definitions.families.get(&FontFamily::Monospace)?.first()?;
    let code = definitions.font_data.get(key)?;
    let serif_ascent = ascent_em(READER_REGULAR.1, 0)?;
    let code_ascent = ascent_em(&code.font, code.index)?;
    Some((key.clone(), serif_ascent / CODE_SCALE - code_ascent))
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
