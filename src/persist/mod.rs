//! Session state persisted between runs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::explorer::SortOrder;
use crate::layout::{LineSpacing, SIDEBAR_MIN_WIDTH, ZOOM_MAX, ZOOM_MIN};
use crate::recent::RecentEntry;
use crate::system_fonts::{FontPreset, TextSizeClass};
use crate::theme::ThemeId;

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

pub(crate) const APP_KEY: &str = "md-viewer-state";

/// The smallest and the largest a formula is drawn, relative to the text.
const MATH_SCALE_MIN: f32 = 1.0;
const MATH_SCALE_MAX: f32 = 2.0;

/// Persisted state saved between sessions.
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct PersistedState {
    /// The reading theme. `None` in a session saved before the themes;
    /// see `dark_mode`.
    pub(crate) theme: Option<ThemeId>,
    /// Read only: what versions before the themes saved instead of
    /// `theme`. `ThemeId::restored` maps it to a theme.
    #[serde(default, skip_serializing)]
    pub(crate) dark_mode: Option<bool>,
    pub(crate) zoom_level: Option<f32>,
    /// `None` until the reader toggles the outline: it then shows while
    /// the window has room for it beside the text.
    pub(crate) show_outline: Option<bool>,
    /// Width of the text column in px. `None` in a session saved before
    /// the column was resizable; see `full_width_content`.
    pub(crate) reading_width: Option<f32>,
    /// Read only: what versions before the resizable column saved instead
    /// of `reading_width`. `ReadingWidth::restored` maps it to a width.
    #[serde(default, skip_serializing)]
    pub(crate) full_width_content: Option<bool>,
    /// `None` keeps the Comfortable default.
    pub(crate) line_spacing: Option<LineSpacing>,
    pub(crate) math_scale: Option<f32>,
    /// Selection/highlight background color override, stored as premultiplied
    /// RGBA bytes (`Color32`'s canonical representation — see `Color32::r/g/b/a`
    /// and `Color32::from_rgba_premultiplied`). `None` = current theme default.
    pub(crate) highlight_color: Option<[u8; 4]>,
    /// Hyperlink text color override, same premultiplied-RGBA encoding as
    /// `highlight_color`. `None` = current theme default.
    pub(crate) link_color: Option<[u8; 4]>,
    /// Preferred document body/heading font family name. `None` means use the
    /// auto-detected system sans-serif (see `system_fonts::setup_fonts`).
    pub(crate) selected_font_family: Option<String>,
    /// Disable egui's pixel-snapping for glyph positions (see
    /// `TessellationOptions::round_text_to_pixels`). Off by default, matching
    /// today's behavior; `Some(true)` trades a touch of sharpness for glyphs
    /// that no longer look unevenly spaced at fractional zoom levels.
    pub(crate) smooth_text_rendering: Option<bool>,
    /// Which markdown viewer's font choices lead the font chains.
    pub(crate) font_preset: Option<FontPreset>,
    /// Shared text-size class; `None` keeps the Normal default.
    pub(crate) text_size_class: Option<TextSizeClass>,
    pub(crate) open_tabs: Option<Vec<PathBuf>>,
    pub(crate) active_tab: Option<usize>,
    // File explorer state
    /// `None` on a fresh install, where the explorer starts hidden.
    pub(crate) show_explorer: Option<bool>,
    pub(crate) explorer_root: Option<PathBuf>,
    pub(crate) expanded_dirs: Option<Vec<PathBuf>>,
    pub(crate) explorer_sort_order: Option<SortOrder>,
    pub(crate) explorer_width: Option<f32>,
    pub(crate) outline_width: Option<f32>,
    pub(crate) recent_files: Option<Vec<RecentEntry>>,
}

/// The document zoom a saved session comes back at.
pub(crate) fn restored_zoom(saved: Option<f32>) -> f32 {
    saved.unwrap_or(1.0).clamp(ZOOM_MIN, ZOOM_MAX)
}

/// The formula size a saved session comes back at, relative to the text.
pub(crate) fn restored_math_scale(saved: Option<f32>) -> f32 {
    saved.unwrap_or(1.0).clamp(MATH_SCALE_MIN, MATH_SCALE_MAX)
}

pub(crate) fn restored_sidebar_width(width: Option<f32>, default: f32) -> f32 {
    width
        .filter(|width| width.is_finite())
        .map_or(default, |width| width.max(SIDEBAR_MIN_WIDTH))
}
