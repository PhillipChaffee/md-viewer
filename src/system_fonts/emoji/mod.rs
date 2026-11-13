//! Color and monochrome emoji fallback faces.

use egui::{FontData, FontDefinitions, FontFamily};

#[cfg(test)]
mod tests;

/// Bundled monochrome emoji face, appended last to both family chains.
///
/// Color-emoji coverage comes first via the system NotoColorEmoji face (see
/// `install_emoji_fonts`); this monochrome face fills any emoji codepoints
/// the color font lacks. Appended last so it only supplies glyphs nothing
/// else covers.
/// (See docs/devlog/014-font-fallback.md.)
const EMOJI_FONT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/NotoEmoji-Regular.ttf");

/// System color-emoji face (CBDT/CBLC bitmap font), appended after the
/// regular system fallbacks but before the monochrome emoji face.
const COLOR_EMOJI_FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/noto/NotoColorEmoji.ttf",
    "/usr/share/fonts/TTF/NotoColorEmoji.ttf",
];

/// Install emoji faces into the fallback chains.
///
/// The color face is only useful with the vendored epaint patch that
/// rasterizes color-bitmap glyphs via swash (ab_glyph alone renders them
/// blank). With the patch, emoji render in full color; without it, the
/// monochrome face still provides visible glyphs.
pub(crate) fn install_emoji_fonts(definitions: &mut FontDefinitions) {
    for path in COLOR_EMOJI_FONT_PATHS {
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut data = FontData::from_owned(bytes);
                // Restrict to default-emoji characters so the color face
                // never claims general symbols (▶ ⚠ ✔ …) used by the UI.
                data.emoji_only = true;
                definitions
                    .font_data
                    .insert("ColorEmoji".to_owned(), data.into());
                for family in [
                    FontFamily::Proportional,
                    FontFamily::Monospace,
                ] {
                    if let Some(chain) = definitions.families.get_mut(&family) {
                        // Insert ahead of epaint's built-in monochrome emoji
                        // faces, which would otherwise claim the glyph first.
                        let pos = chain
                            .iter()
                            .position(|n| n == "NotoEmoji-Regular" || n == "emoji-icon-font")
                            .unwrap_or(chain.len());
                        chain.insert(pos, "ColorEmoji".to_owned());
                    }
                }
                log::info!("Loaded system color emoji font from {path}");
                break;
            }
            Err(e) => log::debug!("Color emoji font {path} unavailable: {e}"),
        }
    }
    match std::fs::read(EMOJI_FONT_PATH) {
        Ok(bytes) => {
            definitions
                .font_data
                .insert("NotoEmoji".to_owned(), FontData::from_owned(bytes).into());
            if let Some(family) = definitions.families.get_mut(&FontFamily::Proportional) {
                family.push("NotoEmoji".to_owned());
            }
            if let Some(family) = definitions.families.get_mut(&FontFamily::Monospace) {
                family.push("NotoEmoji".to_owned());
            }
            log::info!("Loaded bundled monochrome emoji font from {EMOJI_FONT_PATH}");
        }
        Err(e) => log::warn!("Emoji font {EMOJI_FONT_PATH} unavailable: {e}"),
    }
}
