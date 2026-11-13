//! Color and monochrome emoji fallback faces.

use egui::{FontData, FontDefinitions, FontFamily};

#[cfg(test)]
mod tests;

/// Bundled monochrome emoji face, appended last to both family chains.
///
/// Color-emoji coverage comes first via the system `NotoColorEmoji` face (see
/// `install_emoji_fonts`); this monochrome face fills any emoji codepoints
/// the color font lacks. Appended last so it only supplies glyphs nothing
/// else covers.
/// (See docs/devlog/014-font-fallback.md.)
const EMOJI_FONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/NotoEmoji-Regular.ttf"
);

/// System color-emoji face (CBDT/CBLC bitmap font), appended after the
/// regular system fallbacks but before the monochrome emoji face.
const COLOR_EMOJI_FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/noto/NotoColorEmoji.ttf",
    "/usr/share/fonts/TTF/NotoColorEmoji.ttf",
];

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
pub(crate) fn install_emoji_fonts(definitions: &mut FontDefinitions) {
    install_emoji_fonts_from(definitions, COLOR_EMOJI_FONT_PATHS, EMOJI_FONT_PATH);
}

/// `install_emoji_fonts` with the font locations passed in: the first readable
/// entry of `color_paths` becomes the color face, `monochrome_path` the
/// monochrome one. Either may be missing; the chains then keep what they had.
fn install_emoji_fonts_from(
    definitions: &mut FontDefinitions,
    color_paths: &[&str],
    monochrome_path: &str,
) {
    if let Some((path, bytes)) = read_first_available(color_paths) {
        install_color_emoji_face(definitions, bytes);
        log::info!("Loaded system color emoji font from {path}");
    }
    match std::fs::read(monochrome_path) {
        Ok(bytes) => {
            install_monochrome_emoji_face(definitions, bytes);
            log::info!("Loaded bundled monochrome emoji font from {monochrome_path}");
        }
        Err(error) => log::warn!("Emoji font {monochrome_path} unavailable: {error}"),
    }
}

/// Contents of the first path that can be read, with the path it came from.
fn read_first_available<'path>(paths: &[&'path str]) -> Option<(&'path str, Vec<u8>)> {
    paths.iter().find_map(|path| match std::fs::read(path) {
        Ok(bytes) => Some((*path, bytes)),
        Err(error) => {
            log::debug!("Color emoji font {path} unavailable: {error}");
            None
        }
    })
}

/// Register `bytes` as the color emoji face, ahead of epaint's built-in
/// monochrome emoji faces in both chains.
fn install_color_emoji_face(definitions: &mut FontDefinitions, bytes: Vec<u8>) {
    let mut data = FontData::from_owned(bytes);
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

/// Register `bytes` as the monochrome emoji face, last in both chains.
fn install_monochrome_emoji_face(definitions: &mut FontDefinitions, bytes: Vec<u8>) {
    definitions.font_data.insert(
        MONOCHROME_EMOJI_KEY.to_owned(),
        FontData::from_owned(bytes).into(),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(chain) = definitions.families.get_mut(&family) {
            chain.push(MONOCHROME_EMOJI_KEY.to_owned());
        }
    }
}
