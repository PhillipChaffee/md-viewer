//! Building the app at launch: one-time egui setup, the tabs and explorer to
//! start with, and the settings restored from the last session.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, FontId, TextStyle};
#[cfg(feature = "mcp")]
use egui_mcp_bridge::McpBridge;

use crate::app::content::look::DocumentLook;
use crate::app::outline::{HeadingSpy, OutlineRule};
use crate::app::MarkdownApp;
use crate::dialogs::{pick_file, pick_folder};
use crate::explorer::{FileExplorer, SortOrder};
use crate::layout::{
    ReadingWidth, EXPLORER_DEFAULT_WIDTH, OUTLINE_DEFAULT_WIDTH, SIDEBAR_RESIZE_GRAB_RADIUS,
};
use crate::persist::{restored_sidebar_width, PersistedState, APP_KEY};
use crate::search::SearchState;
use crate::system_fonts::{document_fonts, scan_pickable_font_families, setup_fonts};
use crate::tab::Tab;
use crate::theme::ThemeId;

#[cfg(test)]
mod tests;

/// Width of every scrollbar, at rest and hovered.
const SCROLLBAR_WIDTH: f32 = 6.0;

/// One-time egui setup that never changes at runtime.
fn configure_context(ctx: &egui::Context) {
    // Clear stale egui widget data loaded from disk (scroll offsets, panel sizes, etc.)
    // We don't persist egui memory (see persist_egui_memory), but eframe always
    // loads it if present. This purges the old blob so it doesn't waste startup time/RAM.
    ctx.memory_mut(|mem| mem.data = egui::util::IdTypeMap::default());

    // Disable egui's built-in Ctrl+/- zoom — we handle zoom ourselves
    ctx.options_mut(|opt| opt.zoom_with_keyboard = false);

    // Set constant styles once at init (never changes at runtime)
    ctx.style_mut(|style| {
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
        //
        // The bar is thin: it floats over the page's margin and is drawn
        // in the theme's hairline color, so it reads as part of the paper.
        let scroll = &mut style.spacing.scroll;
        scroll.bar_width = SCROLLBAR_WIDTH;
        // No track, and the handle in the widget fill colors the theme
        // sets for it: the hairline color at rest, the muted color in use.
        scroll.foreground_color = false;
        scroll.active_background_opacity = 0.0;
        scroll.interact_background_opacity = 0.0;
        scroll.active_handle_opacity = 1.0;
        style.spacing.scroll.floating_width = style.spacing.scroll.bar_width;
        style.url_in_tooltip = true;
        // Body, heading and code sizes belong to the text-size class, which
        // `setup_fonts` has already applied; setting them here would undo a
        // restored class until the user next changed it.
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(13.0));

        // Smoother scroll animation
        style.animation_time = 0.15;
        style.scroll_animation.points_per_second = 1500.0;

        // Reduce resize grab radius to prevent overlap with adjacent scrollbars
        style.interaction.resize_grab_radius_side = SIDEBAR_RESIZE_GRAB_RADIUS;
    });
}

/// Scan the picker's family list off the UI thread: probing every family's
/// Latin coverage takes ~0.5s on a stock Arch system (see
/// `scan_pickable_font_families`). Until the result lands, the font dialog
/// shows "System Default" plus a scanning note. `None` means the thread
/// could not be started, and the list stays empty.
fn spawn_font_family_scan() -> Option<Receiver<Vec<String>>> {
    let (font_family_tx, font_family_rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("font-family-scan".to_owned())
        .spawn(move || {
            let _ = font_family_tx.send(scan_pickable_font_families());
        })
        .is_ok()
        .then_some(font_family_rx)
}

/// The tabs to start with, and the message to show when the requested file
/// could not be opened. A file from the command line takes priority over the
/// previous session's tabs; with neither, the app starts on the welcome page.
fn initial_tabs(
    file: Option<&Path>,
    session_tabs: Option<Vec<PathBuf>>,
) -> (Vec<Tab>, Option<String>) {
    if let Some(path) = file {
        return match Tab::new(path.to_path_buf()) {
            Ok(tab) => (vec![tab], None),
            Err(error) => (
                Vec::new(),
                Some(format!("Unable to open {}: {error}", path.display())),
            ),
        };
    }
    let tabs = session_tabs
        .unwrap_or_default()
        .into_iter()
        .filter(|path| path.exists())
        .filter_map(|path| {
            Tab::new(path.clone())
                .inspect_err(|error| log::warn!("Unable to restore {}: {error}", path.display()))
                .ok()
        })
        .collect();
    (tabs, None)
}

/// The file explorer for this launch. Its root is the first of: the command
/// line file's folder, the persisted root if it still exists, the first open
/// tab's folder, and the working directory.
fn initial_explorer(
    file: Option<&Path>,
    first_tab: Option<&Tab>,
    sort_order: Option<SortOrder>,
    persisted_root: Option<PathBuf>,
    expanded_dirs: Option<Vec<PathBuf>>,
) -> FileExplorer {
    let mut file_explorer = FileExplorer::default();

    // Restore sort order before scanning (so initial scan uses correct order)
    if let Some(sort_order) = sort_order {
        file_explorer.sort_order = sort_order;
    }

    let root = file
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(|| persisted_root.filter(|root| root.exists()))
        .or_else(|| {
            first_tab
                .and_then(|tab| tab.path.parent())
                .map(Path::to_path_buf)
        })
        .or_else(|| std::env::current_dir().ok());
    if let Some(root) = root {
        file_explorer.set_root(root);
    }

    // Restore expanded directories (children will lazy-load on first render)
    if let Some(expanded) = expanded_dirs {
        file_explorer.expanded_dirs = expanded.into_iter().collect();
    }
    file_explorer
}

/// Whether `display`, the value of `DISPLAY`, names a virtual display (e.g.,
/// Xvfb `:99`). Virtual displays lack vsync, causing unlimited FPS and high
/// CPU, so the frame loop caps its own rate there.
fn is_virtual_display(display: Option<&str>) -> bool {
    display.is_some_and(|display| display != ":0" && display != ":0.0" && !display.is_empty())
}

/// The theme of the last session, and what the document pane renders with
/// until frame 1, which rebuilds it once the theme's visuals are applied.
fn restored_look(ctx: &egui::Context, persisted: &PersistedState) -> (ThemeId, DocumentLook) {
    let theme = ThemeId::restored(persisted.theme, persisted.dark_mode);
    let fonts = document_fonts(
        persisted.selected_font_family.as_deref(),
        persisted.font_preset.unwrap_or_default(),
        persisted.text_size_class.unwrap_or_default(),
    );
    let look = DocumentLook::new(&ctx.style(), theme.palette(), &fonts);
    (theme, look)
}

const fn color_from_persisted([r, g, b, a]: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(r, g, b, a)
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

        // Setup fonts with system font fallbacks for Unicode support, led by
        // the user's picked family (font picker) or the preset's emulated
        // stack; the text-size class is shared by every preset.
        setup_fonts(
            &cc.egui_ctx,
            persisted.selected_font_family.as_deref(),
            persisted.font_preset.unwrap_or_default(),
            persisted.text_size_class.unwrap_or_default(),
        );
        configure_context(&cc.egui_ctx);

        Self::from_persisted(&cc.egui_ctx, persisted, file, watch)
    }

    /// Build the app state from the last session's settings and the launch
    /// arguments. The context's fonts must already match `persisted`.
    fn from_persisted(
        ctx: &egui::Context,
        persisted: PersistedState,
        file: Option<PathBuf>,
        watch: bool,
    ) -> Self {
        let (theme, document_look) = restored_look(ctx, &persisted);
        let (tabs, startup_error) = initial_tabs(file.as_deref(), persisted.open_tabs);
        let active_tab = persisted
            .active_tab
            .unwrap_or(0)
            .min(tabs.len().saturating_sub(1));
        let file_explorer = initial_explorer(
            file.as_deref(),
            tabs.first(),
            persisted.explorer_sort_order,
            persisted.explorer_root,
            persisted.expanded_dirs,
        );

        #[cfg(feature = "mcp")]
        let mcp_bridge = McpBridge::builder().port(9877).build();
        #[cfg(feature = "mcp")]
        log::info!("MCP bridge listening on port {}", mcp_bridge.port());

        let highlight_color = persisted.highlight_color.map(color_from_persisted);
        let link_color = persisted.link_color.map(color_from_persisted);
        let font_preset = persisted.font_preset.unwrap_or_default();
        let text_size_class = persisted.text_size_class.unwrap_or_default();
        let reading_width =
            ReadingWidth::restored(persisted.reading_width, persisted.full_width_content);

        let mut app = Self {
            tabs,
            active_tab,
            theme,
            zoom_level: persisted.zoom_level.unwrap_or(1.0).clamp(0.5, 3.0),
            show_outline: persisted.show_outline.unwrap_or(true),
            outline_rule: OutlineRule::restored(persisted.show_outline),
            heading_spy: HeadingSpy::default(),
            reading_width,
            document_pane_width: f32::INFINITY,
            column_drag_grab: None,
            line_spacing: persisted.line_spacing.unwrap_or_default(),
            document_look,
            math_scale: persisted.math_scale.unwrap_or(1.0).clamp(1.0, 2.0),
            // Frame 1 always rebuilds Visuals anyway (last_applied_theme
            // starts at None), so this just avoids a spurious second rebuild
            // afterward if nothing further changes.
            highlight_color,
            last_applied_highlight_color: highlight_color,
            link_color,
            last_applied_link_color: link_color,
            show_color_settings_dialog: false,
            // Already applied by `new` via setup_fonts(); last_applied starts in
            // sync so update() doesn't redundantly reload fonts on frame 1.
            last_applied_font_config: Some((
                persisted.selected_font_family.clone(),
                font_preset,
                text_size_class,
            )),
            selected_font_family: persisted.selected_font_family,
            available_font_families: Vec::new(),
            available_font_families_lower: Vec::new(),
            pending_font_family_scan: spawn_font_family_scan(),
            show_font_dialog: false,
            font_filter: String::new(),
            smooth_text_rendering: persisted.smooth_text_rendering.unwrap_or(false),
            font_preset,
            text_size_class,
            file_picker: Box::new(pick_file),
            folder_picker: Box::new(pick_folder),
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
            // A fresh install opens on the document alone.
            show_explorer: persisted.show_explorer.unwrap_or(false),
            explorer_width: restored_sidebar_width(
                persisted.explorer_width,
                EXPLORER_DEFAULT_WIDTH,
            ),
            outline_width: restored_sidebar_width(persisted.outline_width, OUTLINE_DEFAULT_WIDTH),
            flashing_paths: HashMap::new(),
            is_virtual_display: is_virtual_display(std::env::var("DISPLAY").ok().as_deref()),
            egui_ctx: ctx.clone(),
            last_applied_theme: None,
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

        app.begin_session(file, watch);
        app
    }

    /// What a launch does once the state is built: note which documents
    /// are open, remember the one named on the command line, and start
    /// watching for changes.
    fn begin_session(&mut self, file: Option<PathBuf>, watch: bool) {
        self.refresh_open_tab_paths();

        // Record a CLI-provided file as recent (an explicit open). Session-restored
        // tabs are not treated as explicit opens, so they are not recorded here.
        if let Some(file) = file {
            let file = file.canonicalize().unwrap_or(file);
            self.record_recent(&file);
        }

        if watch {
            self.start_watching();
        }
    }
}
