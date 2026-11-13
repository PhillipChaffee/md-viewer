//! Every color the app paints: the four reading themes and the egui visuals
//! built from them.

use egui::{Color32, CornerRadius, Stroke};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// The dimming layer behind the lightbox and the drop target. It is the same
/// on every theme: an overlay has to darken a dark page too.
pub(crate) const SCRIM: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 200);
/// Text and controls painted on top of [`SCRIM`].
pub(crate) const ON_SCRIM: Color32 = rgb(0xE8_E6_E1);
/// Secondary text painted on top of [`SCRIM`].
pub(crate) const ON_SCRIM_MUTED: Color32 = rgb(0xB4_B2_AD);

/// A color from its `0xRRGGBB` value.
const fn rgb(hex: u32) -> Color32 {
    let [_, r, g, b] = hex.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

/// A reading theme. The order is the order Ctrl+D cycles through.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ThemeId {
    /// Off-white paper with near-black ink.
    Paper,
    /// Warm paper, the default.
    #[default]
    Sepia,
    /// Dark gray, for a dim room.
    Dusk,
    /// Black, for the dark.
    Night,
}

/// The colors of one theme. Nothing else in the app names a color.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Palette {
    /// Drives egui's dark mode and the renderer's light/dark branches.
    pub(crate) dark: bool,
    /// The paper, edge to edge.
    pub(crate) page: Color32,
    /// Sidebars, popovers and dialogs: a hair off the page color.
    pub(crate) surface: Color32,
    /// Code blocks, table headers, inline code and hovered rows.
    pub(crate) raised: Color32,
    /// Body text.
    pub(crate) text: Color32,
    /// Titles, slightly stronger than body text.
    pub(crate) heading: Color32,
    /// Meta lines, icons at rest and outline entries.
    pub(crate) muted: Color32,
    /// Hairlines.
    pub(crate) rule: Color32,
    pub(crate) link: Color32,
    /// Background of selected text and of find-bar matches.
    pub(crate) selection: Color32,
    /// The active tab's underline and focused controls.
    pub(crate) accent: Color32,
    /// Good news: a saved file, a reloaded file, live reload being on.
    pub(crate) positive: Color32,
    /// Bad news: the error bar and a failed save.
    pub(crate) warning: Color32,
    /// The syntect theme code blocks are highlighted with.
    pub(crate) syntax_theme: &'static str,
}

const PAPER: Palette = Palette {
    dark: false,
    page: rgb(0xFD_FD_FB),
    surface: rgb(0xF6_F6_F3),
    raised: rgb(0xF0_F0_EC),
    text: rgb(0x1F_1F_1F),
    heading: rgb(0x11_11_11),
    muted: rgb(0x6B_6B_68),
    rule: rgb(0xE3_E3_DE),
    link: rgb(0x1A_5F_B4),
    selection: rgb(0xCC_E0_FB),
    accent: rgb(0x1A_5F_B4),
    positive: rgb(0x2B_7A_4B),
    warning: rgb(0xA8_4B_10),
    syntax_theme: "InspiredGitHub",
};

const SEPIA: Palette = Palette {
    dark: false,
    page: rgb(0xF8_F1_E3),
    surface: rgb(0xF3_EB_DA),
    raised: rgb(0xF0_E7_D3),
    text: rgb(0x2B_26_22),
    heading: rgb(0x1E_1A_17),
    muted: rgb(0x76_6C_5E),
    rule: rgb(0xE6_DC_C8),
    link: rgb(0x9A_4A_1F),
    selection: rgb(0xEA_D8_A9),
    accent: rgb(0xB2_5E_28),
    positive: rgb(0x4F_74_2E),
    warning: rgb(0xA3_45_14),
    syntax_theme: "Solarized (light)",
};

const DUSK: Palette = Palette {
    dark: true,
    page: rgb(0x2C_2C_2E),
    surface: rgb(0x26_26_28),
    raised: rgb(0x38_38_3B),
    text: rgb(0xDA_D7_D0),
    heading: rgb(0xEE_EB_E4),
    muted: rgb(0x9C_99_91),
    rule: rgb(0x40_40_44),
    link: rgb(0x9D_C1_F5),
    selection: rgb(0x47_56_73),
    accent: rgb(0xE0_A4_58),
    positive: rgb(0x8F_C4_8A),
    warning: rgb(0xF0_B2_6B),
    syntax_theme: "base16-eighties.dark",
};

const NIGHT: Palette = Palette {
    dark: true,
    page: rgb(0x00_00_00),
    surface: rgb(0x0B_0B_0B),
    raised: rgb(0x17_17_17),
    text: rgb(0xC8_C6_C1),
    heading: rgb(0xE2_E0_DB),
    muted: rgb(0x8C_8A_84),
    rule: rgb(0x26_26_26),
    link: rgb(0x8A_B0_EE),
    selection: rgb(0x2C_3A_57),
    accent: rgb(0xD0_9A_52),
    positive: rgb(0x7F_B5_7A),
    warning: rgb(0xE0_A1_5C),
    syntax_theme: "base16-ocean.dark",
};

impl ThemeId {
    pub(crate) const ALL: [Self; 4] = [Self::Paper, Self::Sepia, Self::Dusk, Self::Night];

    pub(crate) const fn palette(self) -> &'static Palette {
        match self {
            Self::Paper => &PAPER,
            Self::Sepia => &SEPIA,
            Self::Dusk => &DUSK,
            Self::Night => &NIGHT,
        }
    }

    /// The name shown in menus and under the theme swatches.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Paper => "Paper",
            Self::Sepia => "Sepia",
            Self::Dusk => "Dusk",
            Self::Night => "Night",
        }
    }

    /// The theme after this one in [`Self::ALL`], wrapping around.
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Paper => Self::Sepia,
            Self::Sepia => Self::Dusk,
            Self::Dusk => Self::Night,
            Self::Night => Self::Paper,
        }
    }

    /// The theme a saved session comes back in. Versions before the themes
    /// knew light and dark only; a session that saved neither is a fresh
    /// install.
    pub(crate) fn restored(theme: Option<Self>, legacy_dark_mode: Option<bool>) -> Self {
        theme.unwrap_or_else(|| match legacy_dark_mode {
            Some(true) => Self::Dusk,
            Some(false) => Self::Paper,
            None => Self::default(),
        })
    }
}

/// The user's own colors, which win over the theme's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColorOverrides {
    /// Background of selected text and find-bar matches.
    pub(crate) highlight: Option<Color32>,
    pub(crate) link: Option<Color32>,
}

/// One interaction state of a control: its fill, its outline and its ink.
fn widget(fill: Color32, outline: Color32, ink: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0_f32, outline),
        corner_radius: CornerRadius::same(6),
        fg_stroke: Stroke::new(1.0_f32, ink),
        expansion: 0.0,
    }
}

/// The egui visuals for `palette`, with the user's `overrides` on top.
///
/// Controls carry no outline and no fill at rest, so the chrome reads as
/// text on paper; a hovered control gets the `raised` wash.
pub(crate) fn visuals(palette: &Palette, overrides: ColorOverrides) -> egui::Visuals {
    let mut visuals = if palette.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = palette.page;
    visuals.window_fill = palette.surface;
    visuals.window_stroke = Stroke::new(1.0_f32, palette.rule);
    visuals.extreme_bg_color = palette.raised;
    visuals.faint_bg_color = palette.surface;
    visuals.code_bg_color = palette.raised;
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.muted);
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.warning;
    visuals.hyperlink_color = overrides.link.unwrap_or(palette.link);
    visuals.selection.bg_fill = overrides.highlight.unwrap_or(palette.selection);
    visuals.selection.stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.text_cursor.stroke.color = palette.text;

    // The noninteractive outline is what separators, frames and table lines
    // are drawn with: the hairline.
    let noninteractive = widget(palette.page, palette.rule, palette.text);
    // `bg_fill` at rest is the scrollbar handle; buttons take `weak_bg_fill`.
    let mut inactive = widget(palette.rule, Color32::TRANSPARENT, palette.text);
    inactive.weak_bg_fill = Color32::TRANSPARENT;
    let mut hovered = widget(palette.muted, Color32::TRANSPARENT, palette.heading);
    hovered.weak_bg_fill = palette.raised;
    // `strong()` text, which the renderer uses for headings, takes the
    // active state's ink.
    let mut active = widget(palette.muted, Color32::TRANSPARENT, palette.heading);
    active.weak_bg_fill = palette.rule;
    let open = widget(palette.raised, palette.rule, palette.text);
    visuals.widgets = egui::style::Widgets {
        noninteractive,
        inactive,
        hovered,
        active,
        open,
    };
    visuals
}
