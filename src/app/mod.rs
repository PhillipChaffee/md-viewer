//! The application state. `startup` builds it; the other modules each own
//! one part of a frame.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use eframe::egui;
#[cfg(feature = "mcp")]
use egui_mcp_bridge::{McpBridge, McpUiExt};

use crate::app::lightbox::LightboxState;
use crate::app::tabs::PathPicker;
use crate::explorer::FileExplorer;
use crate::recent::RecentEntry;
use crate::search::SearchState;
use crate::system_fonts::{FontPreset, TextSizeClass};
use crate::tab::Tab;
use crate::watcher::FileWatcher;

mod action;
mod content;
mod explorer_panel;
mod frame;
mod lightbox;
mod menu_bar;
mod outline;
mod search_bar;
mod settings;
mod shortcuts;
mod startup;
mod tab_bar;
mod tabs;
#[cfg(test)]
mod tests;
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
    dark_mode: bool,
    zoom_level: f32,
    show_outline: bool,
    full_width_content: bool,
    math_scale: f32,
    // Selection/highlight background color override. `None` = theme default.
    highlight_color: Option<egui::Color32>,
    // Mirrors `last_applied_dark_mode`: only rebuild Visuals when this changes.
    last_applied_highlight_color: Option<egui::Color32>,
    // Hyperlink text color override. `None` = theme default.
    link_color: Option<egui::Color32>,
    last_applied_link_color: Option<egui::Color32>,
    show_color_settings_dialog: bool,
    // Document body/heading font. `None` = auto-detected system sans-serif.
    selected_font_family: Option<String>,
    // Body-font families the picker offers, scanned in a background thread:
    // one canonical name per family, only families that can actually serve
    // as the body font. Empty until the scan lands (the dialog shows a
    // scanning note; "System Default" always works).
    available_font_families: Vec<String>,
    // Lowercased copy of `available_font_families`, precomputed once so the
    // font-picker search filter doesn't re-lowercase every name on every
    // frame it's open (see docs/EGUI_WORKFLOW.md: never allocate in the
    // render loop what can be cached).
    available_font_families_lower: Vec<String>,
    // In-flight background scan producing `available_font_families`.
    pending_font_family_scan: Option<Receiver<Vec<String>>>,
    // Mirrors the font setup inputs (picked family, preset, size class): only
    // reload fonts when this tuple changes.
    last_applied_font_config: Option<(Option<String>, FontPreset, TextSizeClass)>,
    show_font_dialog: bool,
    // Transient UI-only search text for the font picker (not persisted).
    font_filter: String,
    // Disables egui's glyph pixel-snapping when true. Applied unconditionally
    // every frame (see ctx.set_zoom_factor below) since it's a single write
    // into egui's own memory, not worth a last-applied change-gate.
    smooth_text_rendering: bool,
    // Which markdown viewer's font choices lead the font chains.
    font_preset: FontPreset,
    // Shared text-size class across all presets.
    text_size_class: TextSizeClass,
    // The native file and folder dialogs. Fields rather than direct calls so
    // a test can answer in the user's place.
    file_picker: PathPicker,
    folder_picker: PathPicker,
    watch_enabled: bool,
    error_message: Option<String>,
    is_dragging: bool,
    // File watcher state (inotify for local, poll for GVFS/remote)
    watcher: Option<FileWatcher>,
    watcher_rx: Option<Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>>,
    watcher_retry_count: u32,
    // Set of paths being watched (individual tab files)
    watched_paths: HashSet<PathBuf>,
    // Explorer directories being watched non-recursively: the root plus each
    // currently-expanded directory. Mirrors the lazy explorer tree so startup
    // doesn't recursively walk the whole root subtree (issue: ~6s hang on a
    // huge home directory).
    watched_explorer_dirs: HashSet<PathBuf>,
    // Tab being hovered for close button
    hovered_tab: Option<usize>,
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
    last_applied_dark_mode: Option<bool>,
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
    // Recently opened files (most-recent first), shown on the welcome page
    recent_files: Vec<RecentEntry>,
    // Welcome page: whether the recent list is expanded ("Show more")
    welcome_show_all: bool,
    // MCP bridge for E2E testing
    #[cfg(feature = "mcp")]
    mcp_bridge: McpBridge,
}
