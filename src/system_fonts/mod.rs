//! Font presets, text sizes, and system font installation for egui.

use egui::{FontDefinitions, FontFamily};
use fontique::{Collection, CollectionOptions, SourceCache};
use serde::{Deserialize, Serialize};

use crate::system_fonts::emoji::install_emoji_fonts;
use crate::system_fonts::install::{
    install_preset_mono_font, install_regular_fonts, install_strong_font_family, select_body_font,
};

mod emoji;
mod install;
#[cfg(test)]
mod tests;

/// Markdown font presets that emulate how popular markdown viewers pick fonts.
///
/// A preset reorders the app's font chains so the first installed face matches
/// what the emulated viewer would show on Linux. CSS-generic keywords
/// (`-apple-system`, `BlinkMacSystemFont`, `system-ui`, `ui-monospace`,
/// `sans-serif`, `monospace`) have no literal family behind them, so they are
/// transcribed to the concrete faces a Linux browser resolves them to:
/// `system-ui`/UI aliases become GNOME's Adwaita Sans / Cantarell, and generic
/// monospace becomes Noto Sans Mono / `DejaVu` Sans Mono. Anything still missing
/// falls through to the app's own fallback chain, so Unicode/CJK coverage and
/// bold rendering never regress when switching presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FontPreset {
    /// The app's own look: best installed general-purpose sans for body text,
    /// egui's bundled monospace face for code.
    #[default]
    Current,
    /// github.com rendered markdown (Primer `.markdown-body`): body
    /// `"Mona Sans VF", -apple-system, BlinkMacSystemFont, "Segoe UI",
    /// "Noto Sans", Helvetica, Arial, sans-serif`; code `ui-monospace,
    /// SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono",
    /// monospace`.
    Github,
    /// VS Code markdown preview (`markdown.preview.fontFamily` default plus
    /// the Linux `editor.fontFamily` default for code): body `-apple-system,
    /// BlinkMacSystemFont, "Segoe WPC", "Segoe UI", system-ui, "Ubuntu",
    /// "Droid Sans", sans-serif`; code `'Droid Sans Mono', monospace`.
    Vscode,
}

impl FontPreset {
    pub(crate) const ALL: [Self; 3] = [Self::Current, Self::Github, Self::Vscode];

    /// Menu label for this preset.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Current => "Default",
            Self::Github => "GitHub",
            Self::Vscode => "VS Code",
        }
    }

    /// One-line explanation of what the preset emulates, shown as menu hover text.
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Current => {
                "This app's default: best installed sans + bundled mono code font."
            }
            Self::Github => {
                "github.com style: Mona Sans / Segoe UI / Noto Sans body, SF Mono / Liberation Mono code."
            }
            Self::Vscode => {
                "VS Code preview style: Segoe UI / system UI body, Droid Sans Mono code."
            }
        }
    }

    /// `(body line-height, code line-height)` multipliers fed to the renderer.
    /// GitHub: `.markdown-body{line-height:1.5}`, `pre{line-height:1.45}`.
    /// VS Code preview default: `--markdown-line-height: 1.6`, code 1.357em.
    pub(crate) const fn line_heights(self) -> (f32, f32) {
        match self {
            Self::Current => (1.5, 1.3),
            Self::Github => (1.5, 1.45),
            Self::Vscode => (1.6, 1.36),
        }
    }

    /// Body families from the emulated viewer's CSS stack, in resolution
    /// order. `None` keeps the app's own primary sans selection.
    ///
    /// Pure macOS keywords (`-apple-system`, `BlinkMacSystemFont`) are skipped.
    /// `"Adwaita Sans"` directly follows `"Segoe UI"` because modern fontconfig
    /// setups substitute Segoe UI with GNOME's Adwaita Sans, which is what
    /// Linux browsers actually render for stacks asking for Segoe UI first.
    const fn body_families(self) -> Option<&'static [&'static str]> {
        match self {
            Self::Current => None,
            Self::Github => Some(GITHUB_BODY_FAMILIES),
            Self::Vscode => Some(VSCODE_BODY_FAMILIES),
        }
    }

    /// Code families from the emulated viewer's monospace stack, in resolution
    /// order. Empty keeps egui's bundled monospace first.
    const fn mono_families(self) -> &'static [&'static str] {
        match self {
            // GitHub puts ui-monospace first; on Linux browsers that resolves
            // through fontconfig's generic monospace alias before any of the
            // named proprietary faces are reached, so the Linux defaults lead.
            Self::Github => GITHUB_MONO_FAMILIES,
            // VS Code webviews always inject editor.fontFamily; its Linux
            // default is 'Droid Sans Mono', monospace.
            Self::Vscode => VSCODE_MONO_FAMILIES,
            Self::Current => &[],
        }
    }
}

/// Discrete base text sizes shared by every font preset. Unlike the presets
/// themselves — which only choose typefaces and line heights — the size class
/// is one app-wide setting with a single shared default, so switching presets
/// never changes the text size on its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TextSizeClass {
    Small,
    #[default]
    Normal,
    Large,
    ExtraLarge,
    Huge,
}

impl TextSizeClass {
    pub(crate) const ALL: [Self; 5] = [
        Self::Small,
        Self::Normal,
        Self::Large,
        Self::ExtraLarge,
        Self::Huge,
    ];

    /// Menu label including the concrete pixel size.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Small => "Small (14px)",
            Self::Normal => "Normal (16px)",
            Self::Large => "Large (18px)",
            Self::ExtraLarge => "Extra large (20px)",
            Self::Huge => "Huge (24px)",
        }
    }

    /// Base body size in px. Headings and code cascade from this because the
    /// renderer derives every document size from `TextStyle::Body`.
    pub(crate) const fn px(self) -> f32 {
        match self {
            Self::Small => 14.0,
            Self::Normal => 16.0,
            Self::Large => 18.0,
            Self::ExtraLarge => 20.0,
            Self::Huge => 24.0,
        }
    }
}

/// Primer `.markdown-body` body stack without macOS-only aliases; the generic
/// `sans-serif` tail is covered by the app's existing fallback chain.
///
/// GitHub prepends its open-source Mona Sans (primer/primitives#1332); it
/// renders only where installed locally because GitHub ships no markdown
/// webfont. `"Noto Sans Backtick Fix"` is intentionally skipped: it is a
/// `local()`-only `@font-face` shim covering just U+60, not a real family.
const GITHUB_BODY_FAMILIES: &[&str] = &[
    "Mona Sans VF",
    "Mona Sans",
    "Segoe UI",
    "Adwaita Sans", // fontconfig substitutes Segoe UI with Adwaita Sans on modern GNOME
    "Noto Sans",
    "Helvetica",
    "Arial",
];

/// Primer `.markdown-body` code stack. Generic monospace leads because
/// `ui-monospace` precedes every named face in the CSS and resolves on Linux.
const GITHUB_MONO_FAMILIES: &[&str] = &[
    "Noto Sans Mono",
    "DejaVu Sans Mono",
    "SFMono-Regular",
    "SF Mono",
    "Menlo",
    "Consolas",
    "Liberation Mono",
];

/// VS Code preview body stack without macOS-only aliases; `system-ui` is
/// transcribed to its common desktop resolutions and `sans-serif` remains
/// covered by the app's existing fallback chain.
const VSCODE_BODY_FAMILIES: &[&str] = &[
    "Segoe WPC",
    "Segoe UI",
    "Adwaita Sans", // system-ui on GNOME 47+
    "Cantarell",    // system-ui on older GNOME releases
    "Ubuntu",
    "Droid Sans",
];

/// VS Code preview code stack: the Linux `editor.fontFamily` default followed
/// by the generic monospace resolutions.
const VSCODE_MONO_FAMILIES: &[&str] = &["Droid Sans Mono", "Noto Sans Mono", "DejaVu Sans Mono"];

/// Apply the shared base text size class to the context styles. Headings and
/// code cascade because the renderer derives document sizes from
/// `TextStyle::Body`.
fn apply_text_size_class(ctx: &egui::Context, size: TextSizeClass) {
    let base = size.px();
    ctx.style_mut(|style| {
        use egui::{FontId, TextStyle};
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(base));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(base * 2.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(base - 2.0));
    });
}

pub(crate) fn setup_fonts(
    ctx: &egui::Context,
    preferred_family: Option<&str>,
    preset: FontPreset,
    size: TextSizeClass,
) {
    let started = std::time::Instant::now();
    let mut collection = Collection::new(CollectionOptions::default());
    let family_count = collection.family_names().count();
    let mut source_cache = SourceCache::default();
    let locale = sys_locale::get_locale();
    let mut definitions = FontDefinitions::default();
    let defaults = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let installed = install_regular_fonts(
        &mut collection,
        &mut source_cache,
        &mut definitions,
        locale.as_deref(),
        preferred_family,
        preset.body_families(),
    );
    let bold_count = install_strong_font_family(
        &mut collection,
        &mut source_cache,
        &mut definitions,
        &installed,
        &defaults,
    );
    let preset_mono_families = preset.mono_families();
    let preset_mono = if preset_mono_families.is_empty() {
        false
    } else {
        install_preset_mono_font(
            &mut collection,
            &mut source_cache,
            &mut definitions,
            preset_mono_families,
        )
    };
    install_emoji_fonts(&mut definitions);
    if installed.is_empty() {
        log::warn!("No suitable system font fallbacks found; using egui defaults.");
    } else {
        log::info!(
            "Selected {} font faces from {} families for locale {:?} in {:.1} ms",
            installed.len() + bold_count + usize::from(preset_mono),
            family_count,
            locale,
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    ctx.set_fonts(definitions);
    apply_text_size_class(ctx, size);
}

/// Enumerate the family names the font picker should offer: one canonical
/// name per family, filtered to families that can actually serve as the
/// document body font.
///
/// Fontique's raw `family_names()` is unusable for a picker. On a stock Arch
/// system it reports 742 names for only 317 distinct families — every
/// localized name and weight-instance name ("Noto Sans Black", …) is a
/// separate entry that resolves to the base family and would pick the very
/// same regular face — and roughly two thirds of the listed names
/// (script-specific families such as "Noto Sans Devanagari", emoji and
/// symbol faces) have no basic-Latin coverage, so picking them silently fell
/// back to the auto-detected default. The body-font gate here is the same
/// one `install_regular_fonts` applies when installing a picked family, so
/// every listed name is guaranteed to take effect.
///
/// Costs one face load per candidate (every face of script-only families is
/// probed and rejected) — around half a second on a stock Arch system — so
/// callers run this off the UI thread (see the egui workflow's async phase).
pub(crate) fn scan_pickable_font_families() -> Vec<String> {
    let mut collection = Collection::new(CollectionOptions::default());
    let mut source_cache = SourceCache::default();
    pickable_family_names(&mut collection, &mut source_cache)
}

fn pickable_family_names(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
) -> Vec<String> {
    let mut names: Vec<String> = collection.family_names().map(str::to_owned).collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names.retain(|name| {
        let Some(id) = collection.family_id(name) else {
            return false;
        };
        // Keep only each family's canonical (first-seen) name so aliases —
        // localized names and weight-instance names — collapse into a single
        // entry instead of five rows that all pick the same regular face.
        let canonical = collection.family_name(id).map(str::to_owned);
        if canonical.as_deref() != Some(name.as_str()) {
            return false;
        }
        select_body_font(collection, source_cache, &[id]).is_some()
    });
    names
}
