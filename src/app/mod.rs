//! The application state and its construction.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use eframe::egui;
#[cfg(feature = "mcp")]
use egui_mcp_bridge::{McpBridge, McpUiExt};

use crate::app::lightbox::LightboxState;
use crate::explorer::FileExplorer;
use crate::layout::{EXPLORER_DEFAULT_WIDTH, OUTLINE_DEFAULT_WIDTH, SIDEBAR_RESIZE_GRAB_RADIUS};
use crate::persist::{restored_sidebar_width, PersistedState, APP_KEY};
use crate::recent::RecentEntry;
use crate::search::SearchState;
use crate::system_fonts::{scan_pickable_font_families, setup_fonts, FontPreset, TextSizeClass};
use crate::tab::Tab;
use crate::watcher::FileWatcher;

mod content;
mod explorer_panel;
mod frame;
mod lightbox;
mod menu_bar;
mod outline;
mod search_bar;
mod settings;
mod shortcuts;
mod tab_bar;
mod tabs;
mod watching;
mod welcome;

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
    /// Cached set of open tab paths for file explorer highlighting (avoids per-frame syscalls)
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

impl MarkdownApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        file: Option<PathBuf>,
        watch: bool,
    ) -> Self {
        // Load persisted state (needed before font setup, which reads the
        // persisted font family, preset, and text-size class).
        let persisted: PersistedState = cc
            .storage
            .and_then(|s| eframe::get_value(s, APP_KEY))
            .unwrap_or_default();
        let selected_font_family = persisted.selected_font_family;
        let font_preset = persisted.font_preset.unwrap_or_default();
        let text_size_class = persisted.text_size_class.unwrap_or_default();

        // Setup fonts with system font fallbacks for Unicode support, led by
        // the user's picked family (font picker) or the preset's emulated
        // stack; the text-size class is shared by every preset.
        setup_fonts(
            &cc.egui_ctx,
            selected_font_family.as_deref(),
            font_preset,
            text_size_class,
        );

        // Scan the picker's family list off the UI thread: probing every
        // family's Latin coverage takes ~0.5s on a stock Arch system (see
        // `scan_pickable_font_families`). Until the result lands, the font
        // dialog shows "System Default" plus a scanning note.
        let (font_family_tx, font_family_rx) = mpsc::channel();
        let scan_spawned = std::thread::Builder::new()
            .name("font-family-scan".to_owned())
            .spawn(move || {
                let _ = font_family_tx.send(scan_pickable_font_families());
            })
            .is_ok();
        let pending_font_family_scan = scan_spawned.then_some(font_family_rx);

        // Clear stale egui widget data loaded from disk (scroll offsets, panel sizes, etc.)
        // We don't persist egui memory (see persist_egui_memory), but eframe always
        // loads it if present. This purges the old blob so it doesn't waste startup time/RAM.
        cc.egui_ctx.memory_mut(|mem| mem.data = Default::default());

        // Disable egui's built-in Ctrl+/- zoom — we handle zoom ourselves
        cc.egui_ctx
            .options_mut(|opt| opt.zoom_with_keyboard = false);

        // Set constant styles once at init (never changes at runtime)
        cc.egui_ctx.style_mut(|style| {
            // Constant floating-scrollbar geometry (#139).
            //
            // egui's floating bar animates its width between `floating_width`
            // (2) and `bar_width` (10) on hover. A dormant floating bar is
            // hidden by `dormant_*_opacity: 0.0`, not by being thin, so that
            // width governs the bar's *interact* rect and nothing visible.
            // Beside a panel divider the growing and shrinking rect claims and
            // releases the pointer on alternating frames, and at the one pixel
            // just outside the resize handle the two never settle.
            //
            // Measured on Xvfb with the pointer held still: on `main` exactly
            // one x out of 36 cycles through four states, in both the painted
            // pixels and the cursor shape; with the width pinned, zero — and
            // both builds produce an identical state map across x, so this
            // removes the oscillation rather than moving it out of view.
            // `scripts/hover-stability.sh` is that measurement.
            style.spacing.scroll.floating_width = style.spacing.scroll.bar_width;
            style.url_in_tooltip = true;
            use egui::{FontId, TextStyle};
            style
                .text_styles
                .insert(TextStyle::Body, FontId::proportional(16.0));
            style
                .text_styles
                .insert(TextStyle::Heading, FontId::proportional(32.0));
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(13.0));
            style
                .text_styles
                .insert(TextStyle::Monospace, FontId::monospace(14.0));

            // Smoother scroll animation
            style.animation_time = 0.15;
            style.scroll_animation.points_per_second = 1500.0;

            // Reduce resize grab radius to prevent overlap with adjacent scrollbars
            style.interaction.resize_grab_radius_side = SIDEBAR_RESIZE_GRAB_RADIUS;
        });

        let dark_mode = persisted
            .dark_mode
            .unwrap_or_else(|| cc.egui_ctx.style().visuals.dark_mode);
        let zoom_level = persisted.zoom_level.unwrap_or(1.0).clamp(0.5, 3.0);
        let show_outline = persisted.show_outline.unwrap_or(true);
        let full_width_content = persisted.full_width_content.unwrap_or(false);
        let math_scale = persisted.math_scale.unwrap_or(1.0).clamp(1.0, 2.0);
        let highlight_color = persisted
            .highlight_color
            .map(|[r, g, b, a]| egui::Color32::from_rgba_premultiplied(r, g, b, a));
        let link_color = persisted
            .link_color
            .map(|[r, g, b, a]| egui::Color32::from_rgba_premultiplied(r, g, b, a));
        let smooth_text_rendering = persisted.smooth_text_rendering.unwrap_or(false);
        let show_explorer = persisted.show_explorer.unwrap_or(true);
        let explorer_width =
            restored_sidebar_width(persisted.explorer_width, EXPLORER_DEFAULT_WIDTH);
        let outline_width = restored_sidebar_width(persisted.outline_width, OUTLINE_DEFAULT_WIDTH);

        // Determine initial tabs
        let mut startup_error = None;
        let initial_tabs: Vec<Tab> = if let Some(ref path) = file {
            // CLI argument takes priority
            match Tab::new(path.clone()) {
                Ok(tab) => vec![tab],
                Err(error) => {
                    startup_error = Some(format!("Unable to open {}: {error}", path.display()));
                    Vec::new()
                }
            }
        } else if let Some(paths) = persisted.open_tabs {
            // Restore previous session tabs
            paths
                .into_iter()
                .filter(|p| p.exists())
                .filter_map(|path| match Tab::new(path.clone()) {
                    Ok(tab) => Some(tab),
                    Err(error) => {
                        log::warn!("Unable to restore {}: {error}", path.display());
                        None
                    }
                })
                .collect()
        } else {
            // No file and no saved session → start empty (welcome page).
            Vec::new()
        };

        let tabs = initial_tabs;

        let active_tab = persisted
            .active_tab
            .unwrap_or(0)
            .min(tabs.len().saturating_sub(1));

        // Initialize file explorer
        let mut file_explorer = FileExplorer::default();

        // Restore sort order before scanning (so initial scan uses correct order)
        if let Some(sort_order) = persisted.explorer_sort_order {
            file_explorer.sort_order = sort_order;
        }

        // Determine explorer root:
        // 1. From CLI file path
        // 2. From persisted state
        // 3. From first open tab
        // 4. Current working directory as fallback
        let explorer_root = file
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or(persisted.explorer_root.filter(|p| p.exists()))
            .or_else(|| {
                tabs.first()
                    .and_then(|t| t.path.parent().map(|p| p.to_path_buf()))
            })
            .or_else(|| std::env::current_dir().ok());

        if let Some(ref root) = explorer_root {
            file_explorer.set_root(root.clone());
        }

        // Restore expanded directories (children will lazy-load on first render)
        if let Some(expanded) = persisted.expanded_dirs {
            file_explorer.expanded_dirs = expanded.into_iter().collect();
        }

        #[cfg(feature = "mcp")]
        let mcp_bridge = McpBridge::builder().port(9877).build();
        #[cfg(feature = "mcp")]
        log::info!("MCP bridge listening on port {}", mcp_bridge.port());

        // Detect virtual display (e.g., Xvfb :99) to limit frame rate
        // Virtual displays lack vsync, causing unlimited FPS and high CPU
        let is_virtual_display = std::env::var("DISPLAY")
            .map(|d| d != ":0" && d != ":0.0" && !d.is_empty())
            .unwrap_or(false);

        let mut app = Self {
            tabs,
            active_tab,
            dark_mode,
            zoom_level,
            show_outline,
            full_width_content,
            math_scale,
            // Frame 1 always rebuilds Visuals anyway (last_applied_dark_mode
            // starts at None), so this just avoids a spurious second rebuild
            // afterward if nothing further changes.
            highlight_color,
            last_applied_highlight_color: highlight_color,
            link_color,
            last_applied_link_color: link_color,
            show_color_settings_dialog: false,
            // Already applied above via setup_fonts(); last_applied starts in
            // sync so update() doesn't redundantly reload fonts on frame 1.
            last_applied_font_config: Some((
                selected_font_family.clone(),
                font_preset,
                text_size_class,
            )),
            selected_font_family,
            available_font_families: Vec::new(),
            available_font_families_lower: Vec::new(),
            pending_font_family_scan,
            show_font_dialog: false,
            font_filter: String::new(),
            smooth_text_rendering,
            font_preset,
            text_size_class,
            watch_enabled: watch,
            error_message: startup_error,
            is_dragging: false,
            watcher: None,
            watcher_rx: None,
            watcher_retry_count: 0,
            watched_paths: HashSet::new(),
            watched_explorer_dirs: HashSet::new(),
            hovered_tab: None,
            file_explorer,
            show_explorer,
            explorer_width,
            outline_width,
            flashing_paths: HashMap::new(),
            is_virtual_display,
            egui_ctx: cc.egui_ctx.clone(),
            last_applied_dark_mode: None,
            last_applied_zoom_level: None,
            last_window_title: String::new(),
            title_dirty: true,
            open_tab_paths: HashSet::new(),
            lightbox: None,
            lightbox_scroll: 0.0,
            lightbox_open_count: 0,
            search: SearchState::default(),
            recent_files: persisted.recent_files.unwrap_or_default(),
            welcome_show_all: false,
            #[cfg(feature = "mcp")]
            mcp_bridge,
        };

        app.refresh_open_tab_paths();

        // Record a CLI-provided file as recent (an explicit open). Session-restored
        // tabs are not treated as explicit opens, so they are not recorded here.
        if let Some(f) = &file {
            let f = f.canonicalize().unwrap_or_else(|_| f.clone());
            app.record_recent(&f);
        }

        if watch {
            app.start_watching();
        }

        app
    }

    fn window_title(&self) -> String {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            format!("{} - Markdown Viewer", tab.title())
        } else {
            "Markdown Viewer".to_string()
        }
    }

    fn is_markdown_file(path: &std::path::Path) -> bool {
        path.extension()
            .map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                ext == "md" || ext == "markdown" || ext == "txt"
            })
            .unwrap_or(false)
    }
}
