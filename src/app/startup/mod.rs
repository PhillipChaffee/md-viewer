//! Building the app at launch: one-time egui setup, the tabs and explorer to
//! start with, and the settings restored from the last session.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui::{self, FontId, TextStyle};

use crate::app::content::look::DocumentLook;
use crate::app::mcp::Mcp;
use crate::app::outline::{HeadingSpy, OutlineRule};
use crate::app::settings::FontPicker;
use crate::app::watching::LiveReload;
use crate::app::MarkdownApp;
use crate::dialogs::{pick_file, pick_folder};
use crate::explorer::{FileExplorer, SortOrder};
use crate::layout::SIDEBAR_RESIZE_GRAB_RADIUS;
use crate::persist::{color_to_persisted, PersistedState, APP_KEY};
use crate::search::SearchState;
use crate::system_fonts::{document_fonts, setup_fonts, FontConfig};
use crate::tab::Tab;
use crate::theme::{ColorOverrides, ThemeId};

#[cfg(kani)]
mod proofs;
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
    let expanded_dirs = expanded_dirs.into_iter().flatten().collect();
    FileExplorer::restored(sort_order.unwrap_or_default(), expanded_dirs, root)
}

/// Whether `display`, the value of `DISPLAY`, names a virtual display (e.g.,
/// Xvfb `:99`). Virtual displays lack vsync, causing unlimited FPS and high
/// CPU, so the frame loop caps its own rate there.
fn is_virtual_display(display: Option<&str>) -> bool {
    display.is_some_and(|display| display != ":0" && display != ":0.0" && !display.is_empty())
}

/// The theme of the last session, and what the document pane renders with
/// until frame 1, which rebuilds it once the theme's visuals are applied.
fn restored_look(
    ctx: &egui::Context,
    persisted: &PersistedState,
    fonts: &FontConfig,
) -> (ThemeId, DocumentLook) {
    let theme = persisted.theme();
    let look = DocumentLook::new(&ctx.style(), theme.palette(), &document_fonts(fonts));
    (theme, look)
}

/// The font choices of the last session.
fn restored_fonts(persisted: &PersistedState) -> FontConfig {
    FontConfig {
        family: persisted.selected_font_family.clone(),
        preset: persisted.font_preset(),
        size: persisted.text_size_class(),
    }
}

/// The tab a saved session comes back on: the one it saved, or the last
/// of `tab_count` where that one is gone, and the first where it saved
/// none.
fn restored_active_tab(saved: Option<usize>, tab_count: usize) -> usize {
    saved.unwrap_or(0).min(tab_count.saturating_sub(1))
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
        setup_fonts(&cc.egui_ctx, &restored_fonts(&persisted));
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
        let fonts = restored_fonts(&persisted);
        let (theme, document_look) = restored_look(ctx, &persisted, &fonts);
        // Each setting is read here, before the paths leave `persisted`.
        let zoom_level = persisted.zoom();
        let show_outline = persisted.show_outline();
        let outline_rule = OutlineRule::restored(persisted.show_outline);
        let reading_width = persisted.reading_width();
        let line_spacing = persisted.line_spacing();
        let math_scale = persisted.math_scale();
        let colors = ColorOverrides {
            highlight: persisted.highlight_color(),
            link: persisted.link_color(),
        };
        let smooth_text_rendering = persisted.smooth_text_rendering();
        let show_explorer = persisted.show_explorer();
        let explorer_width = persisted.explorer_width();
        let outline_width = persisted.outline_width();

        let (tabs, startup_error) = initial_tabs(file.as_deref(), persisted.open_tabs);
        let active_tab = restored_active_tab(persisted.active_tab, tabs.len());
        let file_explorer = initial_explorer(
            file.as_deref(),
            tabs.first(),
            persisted.explorer_sort_order,
            persisted.explorer_root,
            persisted.expanded_dirs,
        );

        let mcp = Mcp::new();

        let mut app = Self {
            tabs,
            active_tab,
            revealed_tab: None,
            theme,
            zoom_level,
            show_outline,
            outline_rule,
            heading_spy: HeadingSpy::default(),
            reading_width,
            document_pane_width: f32::INFINITY,
            column_drag_grab: None,
            line_spacing,
            document_look,
            math_scale,
            colors,
            applied_visuals: None,
            show_color_settings_dialog: false,
            // Already applied by `new` via setup_fonts(); `applied_fonts` starts
            // in sync so update() doesn't redundantly reload fonts on frame 1.
            applied_fonts: Some(fonts.clone()),
            fonts,
            font_picker: FontPicker::scanning(),
            smooth_text_rendering,
            file_picker: Box::new(pick_file),
            folder_picker: Box::new(pick_folder),
            live_reload: LiveReload::new(watch),
            error_message: startup_error,
            file_explorer,
            show_explorer,
            explorer_width,
            outline_width,
            flashing_paths: HashMap::new(),
            is_virtual_display: is_virtual_display(std::env::var("DISPLAY").ok().as_deref()),
            egui_ctx: ctx.clone(),
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
            home_list: None,
            mcp,
        };

        app.begin_session(file, watch);
        app
    }

    /// What this session saves, for `from_persisted` to start the next
    /// one from.
    pub(crate) fn persisted(&self) -> PersistedState {
        PersistedState {
            theme: Some(self.theme),
            dark_mode: None,
            zoom_level: Some(self.zoom_level),
            show_outline: self.outline_rule.saved(self.show_outline),
            reading_width: Some(self.reading_width.get()),
            full_width_content: None,
            line_spacing: Some(self.line_spacing),
            math_scale: Some(self.math_scale),
            highlight_color: self.colors.highlight.map(color_to_persisted),
            link_color: self.colors.link.map(color_to_persisted),
            selected_font_family: self.fonts.family.clone(),
            smooth_text_rendering: Some(self.smooth_text_rendering),
            font_preset: Some(self.fonts.preset),
            text_size_class: Some(self.fonts.size),
            open_tabs: Some(self.get_open_tab_paths()),
            active_tab: Some(self.active_tab),
            show_explorer: Some(self.show_explorer),
            explorer_root: self.file_explorer.root().map(Path::to_path_buf),
            expanded_dirs: Some(self.file_explorer.expanded_dirs().iter().cloned().collect()),
            explorer_sort_order: Some(self.file_explorer.sort_order()),
            explorer_width: Some(self.explorer_width),
            outline_width: Some(self.outline_width),
            recent_files: Some(self.recent_files.clone()),
        }
    }

    /// What a launch does once the state is built: note which documents
    /// are open, remember the one named on the command line, and start
    /// watching for changes.
    fn begin_session(&mut self, file: Option<PathBuf>, watch: bool) {
        self.refresh_open_tab_paths();

        // Record a CLI-provided file as recent (an explicit open). Session-restored
        // tabs are not treated as explicit opens, so they are not recorded here.
        // A file that did not open left no tab, and was never read.
        if let Some(file) = file.filter(|_| !self.tabs.is_empty()) {
            let file = file.canonicalize().unwrap_or(file);
            self.record_recent(&file);
        }

        if watch {
            self.start_watching();
        }
    }
}
