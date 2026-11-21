//! The font families the document is set in, and the Reader preset's own
//! faces, which are compiled into the binary.

use egui::{FontData, FontDefinitions, FontFamily};
use egui_commonmark_extended::STRONG_FONT_FAMILY;

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

/// Source Code Pro, the serif's companion code face (SIL Open Font License
/// 1.1; see `assets/fonts/SourceCodePro-LICENSE.txt`).
const READER_CODE: BundledFace = BundledFace {
    key: "ReaderCode",
    bytes: bundled_face!("SourceCodePro-Regular.ttf"),
    baseline_shift: CODE_BASELINE_SHIFT,
};
const READER_CODE_SEMIBOLD: BundledFace = BundledFace {
    key: "ReaderCodeSemibold",
    bytes: bundled_face!("SourceCodePro-Semibold.ttf"),
    baseline_shift: CODE_BASELINE_SHIFT,
};

/// Inline code in the bundled faces is set this much smaller than the text
/// around it, so that a line dense with code still reads as prose.
pub(crate) const INLINE_CODE_SCALE: f32 = 0.86;

/// Distance from the baseline to the top of the line in every cut of
/// Source Serif 4, in ems: its ascender is 1036 of 1000 units.
const SERIF_ASCENT_EM: f32 = 1.036;
/// The same in both cuts of Source Code Pro: 984 of 1000 units.
const CODE_ASCENT_EM: f32 = 0.984;

/// How far down a bundled code face is shifted, as a fraction of its size,
/// so that code set [`INLINE_CODE_SCALE`] the size of the serif around it
/// sits on the serif's baseline.
///
/// egui hangs every face from the top of the line by its own ascent, so a
/// smaller face with a lower ascent floats above the text beside it. The
/// shift is a visual tweak of the face: it moves no line.
///
/// A code block is set in the same face at another size, so its letters
/// move down by the same fraction of that size. The number is not worked
/// out for a block: there it only takes the letters off the top of their
/// line, to near its middle.
const CODE_BASELINE_SHIFT: f32 = SERIF_ASCENT_EM / INLINE_CODE_SCALE - CODE_ASCENT_EM;

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
    install(
        named(DOCUMENT_STRONG_CODE_FAMILY),
        &READER_CODE_SEMIBOLD,
        &code,
    );
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
