//! The application state. `startup` builds it; the other modules each own
//! one part of a frame.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use eframe::egui;

use crate::app::content::look::DocumentLook;
use crate::app::lightbox::LightboxState;
use crate::app::mcp::Mcp;
use crate::app::tabs::PathPicker;
use crate::app::watching::LiveReload;
use crate::explorer::FileExplorer;
use crate::layout::{LineSpacing, ReadingWidth};
use crate::recent::RecentEntry;
use crate::search::SearchState;
use crate::system_fonts::FontConfig;
use crate::tab::Tab;
use crate::theme::ThemeId;

mod action;
mod appearance;
mod chrome;
mod content;
mod drop;
mod error_bar;
mod explorer_panel;
mod frame;
mod lightbox;
mod mcp;
mod outline;
mod overflow_menu;
#[cfg(kani)]
mod proofs;
mod search_bar;
mod settings;
mod shortcuts;
mod sidebar;
mod startup;
mod tab_bar;
mod tabs;
#[cfg(test)]
mod tests;
mod top_bar;
#[cfg(test)]
mod ui_harness;
mod watching;
mod welcome;

/// A zoom factor as a whole percentage, for the status readouts.
#[expect(
    clippy::cast_possible_truncation,
    reason = "zoom factors are clamped to at most 10, so the rounded percentage is far inside i32"
)]
fn zoom_percent(zoom: f32) -> i32 {
    (zoom * 100.0).round() as i32
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent on/off setting or a per-frame latch, not a state of one machine"
)]
pub(crate) struct MarkdownApp {
    tabs: Vec<Tab>,
    active_tab: usize,
    /// The active tab the strip last scrolled into view, so that it does
    /// so once for each tab that comes to the front and not every frame.
    revealed_tab: Option<usize>,
    /// The reading theme; its palette is every color the app paints.
    theme: ThemeId,
    zoom_level: f32,
    /// Whether the outline is shown: the reader's choice, or until they
    /// make one, whether the window has room for it beside the text.
    show_outline: bool,
    /// Who decides that: the reader, once they have toggled it.
    outline_rule: outline::OutlineRule,
    /// The heading the reader is in, which the outline marks.
    heading_spy: outline::HeadingSpy,
    /// Width of the text column, which sits centered in the document
    /// pane.
    reading_width: ReadingWidth,
    /// Width of the document pane as last laid out, so a width step can
    /// start from the column that is shown. Infinite until a frame has
    /// measured it.
    document_pane_width: f32,
    /// While an edge of the text column is being dragged: how far from
    /// the edge the pointer took hold, so the edge does not jump to it.
    column_drag_grab: Option<f32>,
    line_spacing: LineSpacing,
    /// The fonts and block styling the document pane renders with, rebuilt
    /// when the theme or the fonts change (never per frame).
    document_look: DocumentLook,
    math_scale: f32,
    // Selection/highlight background color override. `None` = theme default.
    highlight_color: Option<egui::Color32>,
    // Mirrors `last_applied_theme`: only rebuild Visuals when this changes.
    last_applied_highlight_color: Option<egui::Color32>,
    // Hyperlink text color override. `None` = theme default.
    link_color: Option<egui::Color32>,
    last_applied_link_color: Option<egui::Color32>,
    show_color_settings_dialog: bool,
    /// The document's font family, preset and size class. Every preset
    /// shares the one size class.
    fonts: FontConfig,
    // Body-font families the picker offers, scanned in a background thread:
    // one canonical name per family, only families that can actually serve
    // as the body font. Empty until the scan lands (the dialog shows a
    // scanning note; "System Default" always works).
    available_font_families: Vec<String>,
    // Lowercased copy of `available_font_families`, precomputed once so the
    // font-picker search filter doesn't re-lowercase every name on every
    // frame it's open: never allocate in the render loop what can be cached.
    available_font_families_lower: Vec<String>,
    // In-flight background scan producing `available_font_families`.
    pending_font_family_scan: Option<Receiver<Vec<String>>>,
    // The `fonts` egui last loaded: fonts are reloaded only when they differ.
    applied_fonts: Option<FontConfig>,
    show_font_dialog: bool,
    // Transient UI-only search text for the font picker (not persisted).
    font_filter: String,
    // Disables egui's glyph pixel-snapping when true. Applied unconditionally
    // every frame (see ctx.set_zoom_factor below) since it's a single write
    // into egui's own memory, not worth a last-applied change-gate.
    smooth_text_rendering: bool,
    // The native file and folder dialogs. Fields rather than direct calls so
    // a test can answer in the user's place.
    file_picker: PathPicker,
    folder_picker: PathPicker,
    live_reload: LiveReload,
    error_message: Option<String>,
    // File explorer state
    file_explorer: FileExplorer,
    show_explorer: bool,
    explorer_width: f32,
    outline_width: f32,
    // Flash effect for updated files (path -> start time)
    flashing_paths: HashMap<PathBuf, Instant>,
    // True if running on virtual display (e.g., Xvfb :99) - limits frame rate
    is_virtual_display: bool,
    // Stored context for waking egui from the watcher bridge thread
    egui_ctx: egui::Context,
    // Track state to avoid unconditional repaints
    last_applied_theme: Option<ThemeId>,
    // Last zoom level handed to set_zoom_factor; skips redundant writes
    last_applied_zoom_level: Option<f32>,
    last_window_title: String,
    title_dirty: bool,
    /// Cached set of open tab paths for file explorer highlighting (avoids per-frame syscalls).
    open_tab_paths: HashSet<PathBuf>,
    // Lightbox overlay for enlarged mermaid diagrams
    lightbox: Option<LightboxState>,
    // Scroll delta captured from raw_input_hook for lightbox zoom (stripped from RawInput)
    lightbox_scroll: f32,
    // Counter for unique lightbox IDs (prevents stale egui state between opens)
    lightbox_open_count: u64,
    // Find-bar state (current-document search)
    search: SearchState,
    // Recently opened files (most-recent first), shown on the home page
    recent_files: Vec<RecentEntry>,
    // Welcome page: whether the recent list is expanded ("Show more")
    welcome_show_all: bool,
    /// The home page's rows, built from `recent_files` when they change
    /// and not while painting. `None` until the page next shows.
    home_list: Option<welcome::HomeList>,
    // MCP bridge for E2E testing
    mcp: Mcp,
}
