//! Color and monochrome emoji fallback faces.

use egui::{FontData, FontDefinitions, FontFamily};
use fontique::{Collection, SourceCache};

use crate::system_fonts::install::select_family_font;

#[cfg(test)]
mod tests;

/// Bundled monochrome emoji face, appended last to both family chains.
///
/// Color-emoji coverage comes first via the system color face (see
/// `install_emoji_fonts`); this monochrome face fills any emoji codepoints
/// the color font lacks. Appended last so it only supplies glyphs nothing
/// else covers. Compiled into the binary, so an installed copy has it
/// wherever it is run from.
/// (See docs/devlog/014-font-fallback.md.)
const MONOCHROME_EMOJI_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/NotoEmoji-Regular.ttf"
));

/// Family of the system color-emoji face (a CBDT/CBLC bitmap font), which
/// goes after the regular system fallbacks but before the monochrome emoji
/// face. The platform's font configuration says where its file is, which
/// differs between distributions.
const COLOR_EMOJI_FAMILY: &str = "Noto Color Emoji";

/// 🔴, an emoji a face has to map to count as the color emoji face.
const COLOR_EMOJI_SAMPLE: &str = "\u{1F534}";

/// Key of the color face in `FontDefinitions::font_data`.
const COLOR_EMOJI_KEY: &str = "ColorEmoji";

/// Key of the bundled monochrome face in `FontDefinitions::font_data`.
const MONOCHROME_EMOJI_KEY: &str = "NotoEmoji";

/// epaint's built-in monochrome emoji faces. The color face has to sit ahead
/// of them in a chain, or they claim the glyph first.
const BUILTIN_EMOJI_KEYS: [&str; 2] = ["NotoEmoji-Regular", "emoji-icon-font"];

/// Install emoji faces into the fallback chains.
///
/// The color face is only useful with the vendored epaint patch that
/// rasterizes color-bitmap glyphs via swash (`ab_glyph` alone renders them
/// blank). With the patch, emoji render in full color; without it, the
/// monochrome face still provides visible glyphs.
///
/// The color face is the one `collection` lists under `COLOR_EMOJI_FAMILY`;
/// where none is installed the chains get the monochrome face only.
pub(crate) fn install_emoji_fonts(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
    definitions: &mut FontDefinitions,
) {
    let color_face = select_family_font(
        collection,
        source_cache,
        COLOR_EMOJI_FAMILY,
        COLOR_EMOJI_SAMPLE,
    );
    if let Some(face) = color_face {
        log::info!("Loaded system color emoji font from {}", face.source);
        install_color_emoji_face(definitions, face.data());
    }
    install_monochrome_emoji_face(definitions);
}

/// Register `data` as the color emoji face, ahead of epaint's built-in
/// monochrome emoji faces in both chains.
fn install_color_emoji_face(definitions: &mut FontDefinitions, mut data: FontData) {
    // Restrict to default-emoji characters so the color face
    // never claims general symbols (▶ ⚠ ✔ …) used by the UI.
    data.emoji_only = true;
    definitions
        .font_data
        .insert(COLOR_EMOJI_KEY.to_owned(), data.into());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(chain) = definitions.families.get_mut(&family) {
            let position = chain
                .iter()
                .position(|name| BUILTIN_EMOJI_KEYS.contains(&name.as_str()))
                .unwrap_or(chain.len());
            chain.insert(position, COLOR_EMOJI_KEY.to_owned());
        }
    }
}

/// Register the bundled monochrome emoji face, last in both chains.
fn install_monochrome_emoji_face(definitions: &mut FontDefinitions) {
    definitions.font_data.insert(
        MONOCHROME_EMOJI_KEY.to_owned(),
        FontData::from_static(MONOCHROME_EMOJI_FONT).into(),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(chain) = definitions.families.get_mut(&family) {
            chain.push(MONOCHROME_EMOJI_KEY.to_owned());
        }
    }
}
