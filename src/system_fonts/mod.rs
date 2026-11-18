//! Font presets, text sizes, and system font installation for egui.

use egui::{FontDefinitions, FontFamily, FontId, TextStyle};
use fontique::{Collection, CollectionOptions, FamilyId, SourceCache};
use serde::{Deserialize, Serialize};

use crate::system_fonts::bundled::{
    install_document_families, DOCUMENT_BODY_FAMILY, DOCUMENT_EMPHASIS_FAMILY,
    DOCUMENT_HEADING_FAMILY,
};
use crate::system_fonts::emoji::install_emoji_fonts;
use crate::system_fonts::install::{
    install_preset_mono_font, install_regular_fonts, install_strong_font_family, SystemFonts,
    LATIN_SAMPLE,
};

mod bundled;
mod emoji;
mod install;
#[cfg(kani)]
mod proofs;
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
    /// The reading look: the bundled Source Serif 4 for body text and
    /// headings, the system sans for everything around the document.
    #[default]
    Reader,
    /// The app's look before the reader: best installed general-purpose
    /// sans for body text, egui's bundled monospace face for code.
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
    pub(crate) const ALL: [Self; 4] = [Self::Reader, Self::Current, Self::Github, Self::Vscode];

    /// Menu label for this preset.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Reader => "Reader",
            Self::Current => "System",
            Self::Github => "GitHub",
            Self::Vscode => "VS Code",
        }
    }

    /// One-line explanation of what the preset emulates, shown as menu hover text.
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Reader => {
                "Reading style: bundled Source Serif 4 body and headings, sans everywhere else."
            }
            Self::Current => "Best installed sans + bundled mono code font.",
            Self::Github => {
                "github.com style: Mona Sans / Segoe UI / Noto Sans body, SF Mono / Liberation Mono code."
            }
            Self::Vscode => {
                "VS Code preview style: Segoe UI / system UI body, Droid Sans Mono code."
            }
        }
    }

    /// Line height of code blocks as a multiple of the code font size. The
    /// body line height is the user's line-spacing setting, whatever the
    /// preset. GitHub: `pre{line-height:1.45}`; VS Code preview: 1.357em.
    pub(crate) const fn code_line_height(self) -> f32 {
        match self {
            Self::Reader => 1.5,
            Self::Current => 1.3,
            Self::Github => 1.45,
            Self::Vscode => 1.36,
        }
    }

    /// Document body size as a multiple of the text-size class. A serif
    /// with a modest x-height reads smaller than a sans at the same size.
    const fn body_scale(self) -> f32 {
        match self {
            Self::Reader => 1.15,
            Self::Current | Self::Github | Self::Vscode => 1.0,
        }
    }

    /// Whether the document is set in the bundled serif faces. A family the
    /// user picked outranks the preset's own, as it does for every preset.
    const fn uses_bundled_faces(self, preferred_family: Option<&str>) -> bool {
        matches!(self, Self::Reader) && preferred_family.is_none()
    }

    /// The font stacks of the viewer this preset emulates. `None` keeps the
    /// app's own primary sans selection and egui's bundled monospace first.
    const fn emulated_stack(self) -> Option<&'static ViewerStack> {
        match self {
            Self::Reader | Self::Current => None,
            Self::Github => Some(&GITHUB),
            Self::Vscode => Some(&VSCODE),
        }
    }
}

/// The families an emulated viewer's stylesheet asks for, in resolution
/// order.
///
/// Pure macOS keywords (`-apple-system`, `BlinkMacSystemFont`) are skipped.
/// `"Adwaita Sans"` directly follows `"Segoe UI"` because modern fontconfig
/// setups substitute Segoe UI with GNOME's Adwaita Sans, which is what
/// Linux browsers actually render for stacks asking for Segoe UI first.
struct ViewerStack {
    body: &'static [&'static str],
    mono: &'static [&'static str],
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

    /// Menu label. It names no pixel size: the document's depends on the
    /// font preset too.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Normal => "Normal",
            Self::Large => "Large",
            Self::ExtraLarge => "Extra large",
            Self::Huge => "Huge",
        }
    }

    /// The next size down, if there is one.
    pub(crate) const fn smaller(self) -> Option<Self> {
        match self {
            Self::Small => None,
            Self::Normal => Some(Self::Small),
            Self::Large => Some(Self::Normal),
            Self::ExtraLarge => Some(Self::Large),
            Self::Huge => Some(Self::ExtraLarge),
        }
    }

    /// The next size up, if there is one.
    pub(crate) const fn larger(self) -> Option<Self> {
        match self {
            Self::Small => Some(Self::Normal),
            Self::Normal => Some(Self::Large),
            Self::Large => Some(Self::ExtraLarge),
            Self::ExtraLarge => Some(Self::Huge),
            Self::Huge => None,
        }
    }

    /// Base text size in px: what the chrome is set in. The document
    /// scales it by the preset (see [`document_fonts`]), and its headings
    /// and code cascade from there.
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

/// Primer `.markdown-body`.
///
/// The body stack is without macOS-only aliases; the generic `sans-serif`
/// tail is covered by the app's existing fallback chain. GitHub prepends its
/// open-source Mona Sans (primer/primitives#1332); it renders only where
/// installed locally because GitHub ships no markdown webfont.
/// `"Noto Sans Backtick Fix"` is intentionally skipped: it is a
/// `local()`-only `@font-face` shim covering just U+60, not a real family.
///
/// In the code stack generic monospace leads because `ui-monospace` precedes
/// every named face in the CSS: on Linux browsers it resolves through
/// fontconfig's generic monospace alias before any of the named proprietary
/// faces are reached.
const GITHUB: ViewerStack = ViewerStack {
    body: &[
        "Mona Sans VF",
        "Mona Sans",
        "Segoe UI",
        "Adwaita Sans", // fontconfig substitutes Segoe UI with Adwaita Sans on modern GNOME
        "Noto Sans",
        "Helvetica",
        "Arial",
    ],
    mono: &[
        "Noto Sans Mono",
        "DejaVu Sans Mono",
        "SFMono-Regular",
        "SF Mono",
        "Menlo",
        "Consolas",
        "Liberation Mono",
    ],
};

/// VS Code's markdown preview.
///
/// The body stack is without macOS-only aliases; `system-ui` is transcribed
/// to its common desktop resolutions and `sans-serif` remains covered by the
/// app's existing fallback chain.
///
/// The code stack is the Linux `editor.fontFamily` default, which VS Code
/// webviews always inject (`'Droid Sans Mono', monospace`), followed by the
/// generic monospace resolutions.
const VSCODE: ViewerStack = ViewerStack {
    body: &[
        "Segoe WPC",
        "Segoe UI",
        "Adwaita Sans", // system-ui on GNOME 47+
        "Cantarell",    // system-ui on older GNOME releases
        "Ubuntu",
        "Droid Sans",
    ],
    mono: &["Droid Sans Mono", "Noto Sans Mono", "DejaVu Sans Mono"],
};

/// Code is set this much smaller than the body text around it: a
/// monospace face looks larger than a proportional one at the same size.
const CODE_SCALE: f32 = 0.875;

/// The fonts the document pane lays out with, as opposed to the chrome
/// around it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DocumentFonts {
    pub(crate) body: FontId,
    /// The family is the heading face; the renderer sizes each level from
    /// the body size.
    pub(crate) heading: FontId,
    /// The font of code blocks.
    pub(crate) code: FontId,
    /// Size of inline code as a multiple of the text around it, where the
    /// code face is aligned to the body face's baseline at that size.
    /// `None` sets inline code at the size of the text around it.
    pub(crate) inline_code_scale: Option<f32>,
    /// A true italic family, where the document's faces have one. `None`
    /// means emphasis is the body face slanted.
    pub(crate) emphasis: Option<FontFamily>,
}

/// What the reader chose the document to be set in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FontConfig {
    /// The picked body and heading family. `None` is the preset's own.
    pub(crate) family: Option<String>,
    pub(crate) preset: FontPreset,
    pub(crate) size: TextSizeClass,
}

impl FontConfig {
    /// Whether the document is set in the bundled serif faces.
    fn uses_bundled_faces(&self) -> bool {
        self.preset.uses_bundled_faces(self.family.as_deref())
    }
}

/// The document fonts for one font configuration. The named families are
/// the ones [`setup_fonts`] registers for the same configuration.
pub(crate) fn document_fonts(config: &FontConfig) -> DocumentFonts {
    let body = config.size.px() * config.preset.body_scale();
    let bundled = config.uses_bundled_faces();
    DocumentFonts {
        body: FontId::new(body, FontFamily::Name(DOCUMENT_BODY_FAMILY.into())),
        heading: FontId::new(body * 2.0, FontFamily::Name(DOCUMENT_HEADING_FAMILY.into())),
        code: FontId::monospace(body * CODE_SCALE),
        inline_code_scale: bundled.then_some(CODE_SCALE),
        emphasis: bundled.then(|| FontFamily::Name(DOCUMENT_EMPHASIS_FAMILY.into())),
    }
}

/// Apply the shared base text size class to the context styles: the sizes
/// the chrome is set in.
fn apply_text_size_class(ctx: &egui::Context, size: TextSizeClass) {
    let base = size.px();
    ctx.style_mut(|style| {
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

pub(crate) fn setup_fonts(ctx: &egui::Context, config: &FontConfig) {
    let mut collection = Collection::new(CollectionOptions::default());
    let locale = sys_locale::get_locale();
    apply_fonts(ctx, &mut collection, locale.as_deref(), config);
}

/// Give `ctx` the faces `collection` has for `config`, at its text size.
/// The system locale steers the script fallbacks.
fn apply_fonts(
    ctx: &egui::Context,
    collection: &mut Collection,
    locale: Option<&str>,
    config: &FontConfig,
) {
    let mut fonts = SystemFonts {
        collection,
        source_cache: &mut SourceCache::default(),
    };
    let definitions = font_definitions(&mut fonts, locale, config);
    ctx.set_fonts(definitions);
    apply_text_size_class(ctx, config.size);
}

/// The font chains for one font configuration, with every face taken from
/// `fonts`: egui's bundled faces stay in as the last resort.
fn font_definitions(
    fonts: &mut SystemFonts<'_>,
    locale: Option<&str>,
    config: &FontConfig,
) -> FontDefinitions {
    let started = std::time::Instant::now();
    let family_count = fonts.collection.family_names().count();
    let mut definitions = FontDefinitions::default();
    let defaults = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let stack = config.preset.emulated_stack();
    let installed = install_regular_fonts(
        fonts,
        &mut definitions,
        locale,
        config.family.as_deref(),
        stack.map(|stack| stack.body),
    );
    let bold_count = install_strong_font_family(fonts, &mut definitions, &installed, &defaults);
    let preset_mono =
        stack.is_some_and(|stack| install_preset_mono_font(fonts, &mut definitions, stack.mono));
    install_emoji_fonts(fonts, &mut definitions);
    // Last, so the document chains end in every fallback installed above.
    install_document_families(&mut definitions, config.uses_bundled_faces());
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
    definitions
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
    pickable_family_names(&mut SystemFonts {
        collection: &mut collection,
        source_cache: &mut source_cache,
    })
}

fn pickable_family_names(fonts: &mut SystemFonts<'_>) -> Vec<String> {
    let mut names: Vec<String> = fonts.collection.family_names().map(str::to_owned).collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names.retain(|name| {
        canonical_family_id(fonts.collection, name)
            .is_some_and(|id| fonts.regular(&[id], LATIN_SAMPLE).is_some())
    });
    names
}

/// The family `name` belongs to, if `name` is that family's canonical
/// (first-seen) name. Aliases — localized names, weight-instance names, other
/// spellings — resolve to the same family and give `None`, so they collapse
/// into a single entry instead of five rows that all pick the same regular
/// face.
fn canonical_family_id(collection: &mut Collection, name: &str) -> Option<FamilyId> {
    collection
        .family_id(name)
        .filter(|id| collection.family_name(*id) == Some(name))
}
