//! One frame of the app: what it applies to egui, what it persists, and how
//! the clicks its panels report turn into navigation.

use std::fs;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{self, accesskit, Color32, Modifiers, PointerButton};
use eframe::{App as _, Storage as _};

use crate::app::action::AppAction;
use crate::app::explorer_panel::FLASH_DURATION_MS;
use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::tests::Harness;
use crate::app::ui_harness::{
    frame_on, frame_with, frames_until, launch, painted_text, MemoryStorage, ScratchDir, UiHarness,
};
use crate::app::MarkdownApp;
use crate::explorer::{ExplorerScanResult, SortOrder};
use crate::layout::{LineSpacing, ReadingWidth};
use crate::system_fonts::{FontPreset, TextSizeClass};
use crate::test_support::assert_close;
use crate::theme::{self, ThemeId};

const LONG_DOCUMENT_PARAGRAPHS: usize = 400;

fn long_document() -> String {
    "A paragraph of text.\n\n".repeat(LONG_DOCUMENT_PARAGRAPHS)
}

fn window_titles(output: &egui::FullOutput) -> Vec<String> {
    let viewports = output.viewport_output.values();
    viewports
        .flat_map(|viewport| &viewport.commands)
        .filter_map(|command| match command {
            egui::ViewportCommand::Title(title) => Some(title.clone()),
            _ => None,
        })
        .collect()
}

fn wheel(lines: f32) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, lines),
        modifiers: Modifiers::NONE,
    }
}

/// Run frames until the smooth scroll a wheel event started has played out,
/// and return where the document came to rest.
fn scroll_offset_at_rest(harness: &mut Harness) -> f32 {
    const ANIMATION_FRAMES: usize = 120;
    for _ in 0..ANIMATION_FRAMES {
        harness.idle();
    }
    harness.app.tabs[0].scroll_offset
}

#[test]
fn the_wheel_scrolls_the_document_until_a_lightbox_takes_it() {
    let mut harness = Harness::with_docs(&[("a.md", &long_document())]);
    let over_document = egui::Event::PointerMoved(egui::pos2(600.0, 400.0));
    frame_on(&mut harness, vec![over_document]);
    harness.idle();

    frame_on(&mut harness, vec![wheel(-3.0)]);
    let scrolled = scroll_offset_at_rest(&mut harness);
    assert!(scrolled > 0.0, "the wheel scrolled {scrolled}");

    harness.app.lightbox = Some(LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::default()),
        base_size: egui::vec2(100.0, 100.0),
        zoom: 1.0,
        open_id: 1,
        scroll_offset: egui::Vec2::ZERO,
    });
    frame_on(&mut harness, vec![wheel(-3.0)]);

    assert_close(scroll_offset_at_rest(&mut harness), scrolled);
    let zoom = harness.app.lightbox.as_ref().unwrap().zoom;
    assert!(
        zoom < 1.0,
        "scrolling down zoomed the lightbox out to {zoom}"
    );
}

/// Every setting a session saves, changed away from its default.
fn customized_session() -> Harness {
    let mut harness = Harness::with_three_docs();
    let (b, guides) = (harness.path("b.md"), harness.path("guides"));
    fs::create_dir(&guides).unwrap();
    let app = &mut harness.app;
    app.open_in_new_tab(b);
    app.theme = ThemeId::Night;
    app.zoom_level = 1.25;
    app.show_outline = true;
    app.toggle_outline();
    app.reading_width = ReadingWidth::new(512.0);
    app.line_spacing = LineSpacing::Airy;
    app.math_scale = 1.5;
    app.highlight_color = Some(Color32::from_rgb(200, 30, 30));
    app.link_color = Some(Color32::from_rgba_premultiplied(1, 2, 3, 4));
    app.selected_font_family = Some("No Such Family".to_owned());
    app.smooth_text_rendering = true;
    app.font_preset = FontPreset::Vscode;
    app.text_size_class = TextSizeClass::Large;
    app.show_explorer = false;
    app.file_explorer.expanded_dirs.insert(guides);
    app.file_explorer.sort_order = SortOrder::NameDesc;
    app.explorer_width = 301.0;
    app.outline_width = 187.0;
    harness
}

/// The saved settings two sessions must agree on, as one comparable value.
fn settings_of(app: &MarkdownApp) -> impl PartialEq + std::fmt::Debug {
    (
        (
            app.theme,
            app.line_spacing,
            app.show_outline,
            app.smooth_text_rendering,
            app.show_explorer,
        ),
        (
            app.zoom_level.to_bits(),
            app.reading_width.get().to_bits(),
            app.math_scale.to_bits(),
            app.explorer_width.to_bits(),
            app.outline_width.to_bits(),
        ),
        (app.highlight_color, app.link_color),
        (
            app.selected_font_family.clone(),
            app.font_preset,
            app.text_size_class,
        ),
        (app.get_open_tab_paths(), app.active_tab),
        (
            app.file_explorer.root.clone(),
            app.file_explorer.expanded_dirs.clone(),
            app.file_explorer.sort_order == SortOrder::NameDesc,
        ),
    )
}

#[test]
fn a_saved_session_comes_back_on_the_next_launch() {
    let mut harness = customized_session();
    let mut storage = MemoryStorage::default();

    // What eframe does as the window closes.
    harness.app.save(&mut storage);
    storage.flush();
    let relaunched = launch(&storage, None, false);

    assert_eq!(settings_of(&relaunched), settings_of(&harness.app));
    assert_eq!(relaunched.active_tab, 1);
    let recents = |app: &MarkdownApp| -> Vec<_> {
        let entries = app.recent_files.iter();
        entries.map(|entry| entry.path.clone()).collect()
    };
    assert_eq!(recents(&relaunched), recents(&harness.app));
    assert_eq!(recents(&relaunched).len(), 2);
    assert!(
        !harness.app.persist_egui_memory(),
        "widget state is rebuilt, never restored"
    );
}

#[test]
fn each_frame_applies_theme_zoom_and_text_snapping_to_egui() {
    let mut harness = Harness::with_three_docs();
    harness.app.theme = ThemeId::Dusk;
    harness.app.zoom_level = 1.25;
    harness.app.smooth_text_rendering = true;
    // egui takes a new zoom factor at the start of the frame after it is set.
    harness.idle();
    harness.idle();
    assert_eq!(
        harness.ctx.style().visuals.panel_fill,
        ThemeId::Dusk.palette().page
    );
    assert!(harness.ctx.style().visuals.dark_mode);
    assert_close(harness.ctx.zoom_factor(), 1.25);
    let rounds_text = |harness: &Harness| -> bool {
        let options = harness.ctx.tessellation_options(|options| *options);
        options.round_text_to_pixels
    };
    assert!(!rounds_text(&harness));

    harness.app.theme = ThemeId::Paper;
    harness.app.zoom_level = 1.0;
    harness.app.smooth_text_rendering = false;
    harness.idle();
    harness.idle();
    assert!(!harness.ctx.style().visuals.dark_mode);
    assert_eq!(
        harness.ctx.style().visuals.panel_fill,
        ThemeId::Paper.palette().page
    );
    assert_close(harness.ctx.zoom_factor(), 1.0);
    assert!(rounds_text(&harness));
}

#[test]
fn a_virtual_display_gets_a_frame_rate_cap() {
    let mut harness = Harness::with_three_docs();
    harness.app.is_virtual_display = true;
    harness.idle();

    let started = Instant::now();
    harness.idle();

    assert!(started.elapsed() >= Duration::from_millis(16));
}

/// The app a few frames in, with nothing left that asks for a repaint at a
/// moment of its own choosing.
fn at_rest() -> Harness {
    let mut harness = Harness::with_three_docs();
    harness.app.pending_font_family_scan = None;
    frames_until(&mut harness, "the explorer scan", |app| {
        app.file_explorer.pending_scan.is_none()
    });
    harness.idle();
    harness
}

fn repaint_delay(output: &egui::FullOutput) -> Duration {
    output.viewport_output[&egui::ViewportId::ROOT].repaint_delay
}

#[test]
fn a_scan_still_running_keeps_the_frames_coming() {
    const POLL_INTERVAL: Duration = Duration::from_millis(100);
    let mut harness = at_rest();
    assert!(repaint_delay(&harness.idle()) > POLL_INTERVAL);
    let (scan_tx, scan_rx) = mpsc::channel::<ExplorerScanResult>();
    harness.app.file_explorer.pending_scan = Some(scan_rx);

    let output = harness.idle();
    assert!(repaint_delay(&output) <= POLL_INTERVAL);
    assert!(harness.app.file_explorer.pending_scan.is_some());

    drop(scan_tx);
    harness.idle();
    assert!(harness.app.file_explorer.pending_scan.is_none());
    assert_eq!(harness.app.file_explorer.tree.len(), 3, "the listing stays");
}

#[test]
fn a_reload_flash_is_animated_until_it_expires() {
    const ANIMATION_FRAME: Duration = Duration::from_millis(16);
    let mut harness = at_rest();
    let a = harness.path("a.md");

    harness.app.flashing_paths.insert(a.clone(), Instant::now());
    assert!(repaint_delay(&harness.idle()) <= ANIMATION_FRAME);
    assert!(harness.app.flashing_paths.contains_key(&a));

    let long_ago = Instant::now()
        .checked_sub(Duration::from_millis(FLASH_DURATION_MS * 2))
        .unwrap();
    harness.app.flashing_paths.insert(a, long_ago);
    assert!(
        repaint_delay(&harness.idle()) > ANIMATION_FRAME,
        "an expired flash stops driving the animation"
    );
    assert!(harness.app.flashing_paths.is_empty());
}

#[test]
fn the_explorer_lists_the_folder_once_its_scan_lands() {
    let mut harness = Harness::with_three_docs();

    frames_until(&mut harness, "the explorer scan", |app| {
        app.file_explorer.pending_scan.is_none()
    });

    assert_eq!(harness.app.file_explorer.tree.len(), 3);
}

#[test]
fn a_change_on_disk_reaches_the_open_tab_while_watching() {
    let mut harness = Harness::with_docs(&[("a.md", "before")]);
    harness.apply(AppAction::ToggleWatch);
    harness.idle();

    fs::write(harness.path("a.md"), "after").unwrap();

    frames_until(&mut harness, "the reload", |app| {
        app.tabs[0].content == "after"
    });
}

#[test]
fn the_window_title_follows_the_active_tab_down_to_no_tab() {
    let mut harness = Harness::with_three_docs();
    assert_eq!(window_titles(&harness.idle()), ["a.md - Markdown Viewer"]);
    assert!(window_titles(&harness.idle()).is_empty(), "nothing changed");

    harness.app.close_active_tab();

    assert_eq!(window_titles(&harness.idle()), ["Markdown Viewer"]);
}

#[test]
fn files_dragged_over_the_window_get_a_drop_hint() {
    let mut harness = Harness::with_three_docs();
    let hint = "Drop markdown file here".to_owned();
    let dragging = egui::RawInput {
        hovered_files: vec![egui::HoveredFile::default()],
        ..Default::default()
    };

    let output = frame_with(&mut harness, dragging);
    assert!(painted_text(&output).contains(&hint));

    let output = harness.idle();
    assert!(!painted_text(&output).contains(&hint));
}

#[test]
fn the_tab_close_button_closes_its_tab() {
    let mut harness = UiHarness::with_three_docs();
    let b = harness.path("b.md");
    harness.app_mut().open_in_new_tab(b);
    harness.settle();

    // Of everything that reads `b.md`, the tab is the topmost.
    let named = harness.labelled("b.md");
    let tab = named.iter().map(|(center, _)| *center);
    harness.hover_at(tab.min_by(|a, b| a.y.total_cmp(&b.y)).unwrap());
    harness.click_exact("Close tab");

    assert_eq!(harness.app().tabs.len(), 1);
    assert_eq!(harness.active_path(), harness.path("a.md"));
}

/// The explorer row for `name`: of everything labelled `name`, the one
/// lowest on screen, since the sidebar sits below the tab strip.
fn explorer_row(harness: &UiHarness, name: &str) -> egui::Pos2 {
    let labelled = harness.labelled(name);
    let lowest = labelled.iter().max_by(|(a, _), (b, _)| a.y.total_cmp(&b.y));
    lowest.unwrap().0
}

fn with_explorer_listed() -> UiHarness {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().show_explorer = true;
    harness.wait_until("the explorer scan", |harness| harness.shows("c.md"));
    harness
}

#[test]
fn an_explorer_click_replaces_the_document_and_ctrl_click_adds_a_tab() {
    let mut harness = with_explorer_listed();
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));

    harness.click_at(explorer_row(&harness, "b.md"));
    assert_eq!(harness.app().tabs.len(), 1);
    assert_eq!(harness.active_path(), b);

    let row = explorer_row(&harness, "c.md");
    harness.click_with(row, PointerButton::Primary, Modifiers::CTRL);
    assert_eq!(harness.app().tabs.len(), 2);
    assert_eq!(harness.active_path(), c);
}

#[test]
fn a_middle_click_on_an_open_file_in_the_explorer_closes_its_tab() {
    let mut harness = with_explorer_listed();
    let c = harness.path("c.md");
    harness.app_mut().open_in_new_tab(c);
    harness.settle();

    let closed_file = explorer_row(&harness, "b.md");
    harness.click_with(closed_file, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(harness.app().tabs.len(), 2, "b.md has no tab to close");

    let open_file = explorer_row(&harness, "a.md");
    harness.click_with(open_file, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(harness.app().tabs.len(), 1);
    assert_eq!(harness.active_path(), harness.path("c.md"));
}

const LINKING_DOCUMENT: &str = "# A\n\n[to b](b.md)\n\n[to details](b.md#details)\n";
const LINKED_DOCUMENT: &str = "# B\n\nIntro.\n\n## Details\n\nMore.\n";

#[test]
fn a_clicked_link_navigates_the_tab_and_everything_that_mirrors_it() {
    let mut harness =
        UiHarness::with_docs(&[("a.md", LINKING_DOCUMENT), ("b.md", LINKED_DOCUMENT)]);
    let (a, b) = (harness.path("a.md"), harness.path("b.md"));

    harness.click("to b");

    assert_eq!(harness.app().tabs.len(), 1);
    assert_eq!(harness.active_path(), b);
    assert!(harness.app().open_tab_paths.contains(&b));
    assert!(!harness.app().open_tab_paths.contains(&a));
    assert_eq!(harness.app().recent_files[0].path, b);
}

#[test]
fn a_ctrl_clicked_link_opens_its_target_in_a_new_tab_at_the_anchor() {
    let mut harness =
        UiHarness::with_docs(&[("a.md", LINKING_DOCUMENT), ("b.md", LINKED_DOCUMENT)]);
    let b = harness.path("b.md");

    let link = harness.center_of("to details");
    harness.click_with(link, PointerButton::Primary, Modifiers::CTRL);

    assert_eq!(harness.app().tabs.len(), 2);
    assert_eq!(harness.active_path(), b);
    assert_eq!(harness.app().tabs[0].path, harness.path("a.md"));
}

/// A 64 by 48 picture.
const PICTURE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="48">
<rect width="64" height="48" fill="#285ac8"/>
</svg>"##;

/// A document showing one picture, with the picture loaded.
fn with_picture(markdown: &str) -> UiHarness {
    let dir = ScratchDir::new();
    dir.write("a.md", markdown);
    dir.write("picture.svg", PICTURE_SVG);
    let mut harness = UiHarness::open(dir, Some("a.md"));
    harness.wait_until("the picture", |harness| {
        let pictures = harness.centers_with_role(accesskit::Role::Image);
        !pictures.is_empty() && !harness.ui.ctx.has_pending_images()
    });
    harness
}

#[test]
fn clicking_a_picture_opens_it_in_the_lightbox_at_full_size() {
    let mut harness = with_picture("![a picture](picture.svg)\n");
    assert!(harness.app().lightbox.is_none());

    let picture = harness.centers_with_role(accesskit::Role::Image)[0];
    harness.click_at(picture);

    let lightbox = harness.app().lightbox.as_ref().unwrap();
    assert_close(lightbox.zoom, 1.0);
    assert_eq!(lightbox.base_size, egui::vec2(64.0, 48.0));
    assert_eq!(lightbox.open_id, 1);
}

#[test]
fn clicking_a_rendered_diagram_opens_it_in_the_lightbox() {
    let mut harness = UiHarness::with_docs(&[("a.md", "```mermaid\ngraph TD\n    A --> B\n```\n")]);
    harness.wait_until("the diagram", |harness| {
        !harness.centers_with_role(accesskit::Role::Image).is_empty()
    });

    let diagram = harness.centers_with_role(accesskit::Role::Image)[0];
    harness.click_at(diagram);

    let lightbox = harness.app().lightbox.as_ref().unwrap();
    assert!(matches!(lightbox.texture, LightboxTexture::Owned(_)));
    assert!(lightbox.base_size.x > 0.0 && lightbox.base_size.y > 0.0);
}

/// Click the document's diagram and take the texture the lightbox opens
/// with, closing the lightbox again.
fn enlarged_diagram(harness: &mut UiHarness) -> egui::TextureId {
    let diagram = harness.centers_with_role(accesskit::Role::Image)[0];
    harness.click_at(diagram);
    harness.app_mut().lightbox.take().unwrap().texture.id()
}

#[test]
fn a_diagram_is_redrawn_in_the_colors_of_the_theme_switched_to() {
    let mut harness = UiHarness::with_docs(&[("a.md", "```mermaid\ngraph TD\n    A --> B\n```\n")]);
    harness.wait_until("the diagram", |harness| {
        !harness.centers_with_role(accesskit::Role::Image).is_empty()
    });
    let before = enlarged_diagram(&mut harness);
    // Without a change of theme the diagram is not drawn again.
    assert_eq!(enlarged_diagram(&mut harness), before);

    harness
        .ui
        .key_press_modifiers(Modifiers::CTRL, egui::Key::D);
    harness.settle();

    assert_ne!(enlarged_diagram(&mut harness), before);
}

#[test]
fn the_users_colors_stay_on_top_of_whichever_theme_is_picked() {
    let mut harness = Harness::with_three_docs();
    let highlight = Color32::from_rgb(200, 30, 30);
    harness.app.highlight_color = Some(highlight);
    harness.idle();

    for theme in ThemeId::ALL {
        harness.app.theme = theme;
        harness.idle();
        let visuals = harness.ctx.style().visuals.clone();
        assert_eq!(visuals.selection.bg_fill, highlight, "{theme:?}");
        assert_eq!(visuals.hyperlink_color, theme.palette().link);
        assert_eq!(visuals.panel_fill, theme.palette().page);
    }

    harness.app.highlight_color = None;
    harness.idle();
    assert_eq!(
        harness.ctx.style().visuals.selection.bg_fill,
        harness.app.theme.palette().selection
    );
}

#[test]
fn the_document_look_follows_the_theme() {
    let mut harness = Harness::with_three_docs();
    harness.idle();

    for theme in ThemeId::ALL {
        harness.app.theme = theme;
        harness.idle();
        let look = &harness.app.document_look;
        assert_eq!(look.muted, theme.palette().muted, "{theme:?}");
        assert_eq!(look.syntax_theme, theme.palette().syntax_theme);
        assert_eq!(look.style.visuals.panel_fill, theme.palette().page);
    }
}

#[test]
fn changing_fonts_rebuilds_the_document_look_and_lays_documents_out_again() {
    let mut harness = Harness::with_three_docs();
    harness.idle();
    let body_size = |harness: &Harness| {
        let styles = &harness.app.document_look.style.text_styles;
        styles[&egui::TextStyle::Body].size
    };
    let revision = |harness: &Harness| harness.app.tabs[0].cache.layout_revision();
    assert_close(body_size(&harness), 16.0 * 1.15);

    let before = revision(&harness);
    harness.app.text_size_class = TextSizeClass::Large;
    harness.idle();
    assert_close(body_size(&harness), 18.0 * 1.15);
    assert!(revision(&harness) > before);

    // A frame in which nothing changed rebuilds nothing.
    let settled = revision(&harness);
    harness.idle();
    assert_eq!(revision(&harness), settled);

    harness.app.font_preset = FontPreset::Current;
    harness.idle();
    assert_close(body_size(&harness), 18.0);
    assert_eq!(harness.app.document_look.blocks.emphasis_font_family, None);
    assert!(revision(&harness) > settled);
}

#[test]
fn the_drop_target_dims_the_window_with_the_scrim() {
    let mut harness = Harness::with_three_docs();
    let hovering = egui::RawInput {
        hovered_files: vec![egui::HoveredFile::default()],
        ..Default::default()
    };

    let output = frame_with(&mut harness, hovering);

    let dimmed = crate::test_support::painted_shapes(&output)
        .into_iter()
        .any(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == theme::SCRIM));
    assert!(dimmed);
    let prompt = crate::test_support::painted_text_starting(&output, "Drop markdown file");
    assert_eq!(prompt.fallback_color, theme::ON_SCRIM);
}
