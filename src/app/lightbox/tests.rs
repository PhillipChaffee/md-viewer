//! The lightbox: zooming toward the cursor, and the clicks that close it or
//! leave it open.

use eframe::egui::{self, Modifiers, Pos2, Rect, Vec2};

use super::{LightboxState, LightboxTexture, DIAGRAM_SHEET_MARGIN};
use crate::app::tests::Harness;
use crate::app::ui_harness::{click_on, frame_on, painted_text, UiHarness};
use crate::test_support::{assert_close, painted_shapes};

const VIEWPORT: Vec2 = egui::vec2(500.0, 500.0);

/// More frames than egui takes to fade a new overlay in.
const FADE_IN_FRAMES: usize = 12;

fn lightbox(base_size: Vec2, zoom: f32, scroll_offset: Vec2) -> LightboxState {
    LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::default()),
        base_size,
        zoom,
        open_id: 1,
        scroll_offset,
    }
}

/// Which point of the picture, as a fraction of its size, sits under
/// `pointer` when the picture is `display` wide and scrolled by `offset`.
fn point_under(pointer: Vec2, offset: Vec2, display: Vec2) -> Vec2 {
    let padding = (display.max(VIEWPORT) - display) * 0.5;
    (pointer + offset - padding) / display
}

#[track_caller]
fn assert_same_point(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() < 1e-4,
        "expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn zooming_in_keeps_the_point_under_the_cursor_in_place() {
    let base = egui::vec2(1000.0, 800.0);
    let (pointer, scrolled) = (egui::vec2(120.0, 400.0), egui::vec2(300.0, 150.0));
    let state = lightbox(base, 2.0, scrolled);

    let offset = state.zoom_to_cursor_offset(pointer, 1.0, VIEWPORT);

    assert_same_point(
        point_under(pointer, offset, base * 2.0),
        point_under(pointer, scrolled, base),
    );
}

#[test]
fn a_small_centered_picture_stays_under_the_cursor_as_it_outgrows_the_viewport() {
    let base = egui::vec2(100.0, 100.0);
    let pointer = egui::vec2(230.0, 270.0);
    let state = lightbox(base, 10.0, Vec2::ZERO);

    let offset = state.zoom_to_cursor_offset(pointer, 1.0, VIEWPORT);

    assert_same_point(
        point_under(pointer, offset, base * 10.0),
        point_under(pointer, Vec2::ZERO, base),
    );
}

#[test]
fn the_offset_never_leaves_the_scrollable_range() {
    let base = egui::vec2(1000.0, 1000.0);
    let far_corner = lightbox(base, 1.0, egui::vec2(1500.0, 1500.0));
    let near_corner = lightbox(base, 1.0, Vec2::ZERO);
    let fitting = lightbox(base, 0.25, egui::vec2(1500.0, 1500.0));

    let bottom_right = far_corner.zoom_to_cursor_offset(Vec2::ZERO, 2.0, VIEWPORT);
    let top_left = near_corner.zoom_to_cursor_offset(VIEWPORT, 2.0, VIEWPORT);
    let no_scroll = fitting.zoom_to_cursor_offset(Vec2::ZERO, 2.0, VIEWPORT);

    assert_same_point(bottom_right, egui::vec2(500.0, 500.0));
    assert_same_point(top_left, Vec2::ZERO);
    assert_same_point(no_scroll, Vec2::ZERO);
}

#[test]
fn zoom_steps_stop_at_a_tenth_and_at_ten_times() {
    let mut state = lightbox(egui::vec2(10.0, 10.0), 1.0, Vec2::ZERO);

    state.zoom_by(4.0);
    assert_close(state.zoom, 4.0);
    state.zoom_by(4.0);
    assert_close(state.zoom, 10.0);
    state.zoom_by(0.001);
    assert_close(state.zoom, 0.1);
}

/// The app with a lightbox over it showing a picture `base_size` large.
fn with_lightbox(base_size: Vec2) -> Harness {
    showing(lightbox(base_size, 1.0, Vec2::ZERO))
}

/// The app with `state` as its lightbox, laid out and painted: egui sizes a
/// new overlay in one frame and shows it from the next.
fn showing(state: LightboxState) -> Harness {
    let mut harness = Harness::with_three_docs();
    harness.app.lightbox = Some(state);
    harness.idle();
    harness.idle();
    harness
}

/// The rectangles the lightbox lays over the app, largest first: the dimmed
/// backdrop, the picture's viewport, and the close button.
fn overlay_rects(harness: &Harness) -> [Rect; 3] {
    let mut rects: Vec<Rect> = harness.ctx.memory(|memory| {
        let layers = memory.areas().visible_layer_ids();
        let overlays = layers
            .into_iter()
            .filter(|layer| layer.order == egui::Order::Tooltip);
        overlays
            .filter_map(|layer| memory.area_rect(layer.id))
            .collect()
    });
    rects.sort_by(|a, b| b.area().total_cmp(&a.area()));
    rects.try_into().unwrap()
}

fn zoom_of(harness: &Harness) -> f32 {
    harness.app.lightbox.as_ref().unwrap().zoom
}

fn pan_of(harness: &Harness) -> Vec2 {
    harness.app.lightbox.as_ref().unwrap().scroll_offset
}

#[test]
fn a_click_on_the_picture_keeps_the_lightbox_and_a_click_beside_it_closes() {
    let mut harness = with_lightbox(egui::vec2(100.0, 100.0));
    let [_, viewport, _] = overlay_rects(&harness);

    click_on(&mut harness, viewport.center());
    assert!(harness.app.lightbox.is_some());

    click_on(&mut harness, viewport.left_top() + egui::vec2(10.0, 10.0));
    assert!(harness.app.lightbox.is_none());
}

#[test]
fn a_click_on_the_dimmed_margin_closes_the_lightbox() {
    let mut harness = with_lightbox(egui::vec2(100.0, 100.0));
    let [backdrop, viewport, _] = overlay_rects(&harness);
    let margin = backdrop.left_top() + egui::vec2(10.0, 10.0);
    assert!(!viewport.contains(margin));

    click_on(&mut harness, margin);

    assert!(harness.app.lightbox.is_none());
}

#[test]
fn a_click_on_a_top_bar_button_reaches_the_scrim_and_not_the_button() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().lightbox = Some(lightbox(egui::vec2(100.0, 100.0), 1.0, Vec2::ZERO));
    harness.frames(FADE_IN_FRAMES);

    harness.click_exact("Find in document");

    assert!(!harness.app().search.is_open, "the button behind acted");
    assert!(
        harness.app().lightbox.is_none(),
        "the scrim was not clicked"
    );
}

#[test]
fn the_close_button_shows_a_hand_and_closes_the_lightbox() {
    let mut harness = with_lightbox(egui::vec2(100.0, 100.0));
    let [_, _, close_button] = overlay_rects(&harness);

    let hovering = egui::Event::PointerMoved(close_button.center());
    frame_on(&mut harness, vec![hovering]);
    let output = harness.idle();
    assert_eq!(
        output.platform_output.cursor_icon,
        egui::CursorIcon::PointingHand
    );

    click_on(&mut harness, close_button.center());
    assert!(harness.app.lightbox.is_none());
}

#[test]
fn a_double_click_on_the_picture_returns_to_full_size_unpanned() {
    let mut harness = with_lightbox(egui::vec2(4000.0, 4000.0));
    let [_, viewport, _] = overlay_rects(&harness);
    let pointer = viewport.center() + viewport.size() * 0.25;
    frame_on(&mut harness, vec![egui::Event::PointerMoved(pointer)]);
    frame_on(&mut harness, vec![wheel(3.0)]);
    harness.idle();
    assert!(zoom_of(&harness) > 1.0);
    assert_ne!(pan_of(&harness), Vec2::ZERO);

    click_on(&mut harness, pointer);
    click_on(&mut harness, pointer);
    harness.idle();

    assert_close(zoom_of(&harness), 1.0);
    assert_eq!(pan_of(&harness), Vec2::ZERO);
}

fn wheel(lines: f32) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, lines),
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn the_wheel_zooms_toward_the_pointer_and_the_picture_stays_panned_there() {
    let mut harness = with_lightbox(egui::vec2(4000.0, 4000.0));
    let [_, viewport, _] = overlay_rects(&harness);
    let pointer: Pos2 = viewport.center() + viewport.size() * 0.25;
    frame_on(&mut harness, vec![egui::Event::PointerMoved(pointer)]);

    frame_on(&mut harness, vec![wheel(1.0)]);

    assert_close(zoom_of(&harness), 1.08);
    let pan = pan_of(&harness);
    assert!(pan.x > 0.0 && pan.y > 0.0, "panned to {pan:?}");
    assert_same_point(
        point_under_in(viewport, pointer, pan, 4000.0 * 1.08),
        point_under_in(viewport, pointer, Vec2::ZERO, 4000.0),
    );

    harness.idle();
    assert_same_point(pan_of(&harness), pan);
}

/// Like [`point_under`], for a square picture `display` wide in `viewport`.
fn point_under_in(viewport: Rect, pointer: Pos2, offset: Vec2, display: f32) -> Vec2 {
    let display = Vec2::splat(display);
    let padding = (display.max(viewport.size()) - display) * 0.5;
    (pointer - viewport.min + offset - padding) / display
}

#[test]
fn without_a_pointer_the_wheel_zooms_in_place() {
    let mut harness = with_lightbox(egui::vec2(4000.0, 4000.0));

    frame_on(&mut harness, vec![wheel(-1.0)]);

    assert_close(zoom_of(&harness), 0.92);
    assert_eq!(pan_of(&harness), Vec2::ZERO);
}

#[test]
fn the_zoom_level_is_shown_unless_it_is_one_hundred_percent() {
    let mut harness = with_lightbox(egui::vec2(100.0, 100.0));
    assert!(!painted_text(&harness.idle()).contains(&"100%".to_owned()));

    harness.app.lightbox.as_mut().unwrap().zoom = 1.25;

    assert!(painted_text(&harness.idle()).contains(&"125%".to_owned()));
}

#[test]
fn a_diagram_is_painted_from_the_texture_the_lightbox_owns() {
    let mut harness = Harness::with_three_docs();
    let pixels = egui::ColorImage::filled([8, 8], egui::Color32::RED);
    let diagram = harness
        .ctx
        .load_texture("diagram", pixels, egui::TextureOptions::default());
    let diagram_id = diagram.id();
    let mut state = lightbox(egui::vec2(100.0, 100.0), 1.0, Vec2::ZERO);
    state.texture = LightboxTexture::Owned(diagram);
    harness.app.lightbox = Some(state);
    harness.idle();

    let output = harness.idle();

    let painted_with_diagram = output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == diagram_id)
    });
    assert!(painted_with_diagram);
}

/// What a lightbox paints for a 100 by 100 picture from `texture`, and the
/// page color it was painted over.
fn painted_with(
    texture: impl FnOnce(&egui::Context) -> LightboxTexture,
) -> (Vec<egui::Shape>, egui::Color32) {
    let mut harness = Harness::with_three_docs();
    let mut state = lightbox(egui::vec2(100.0, 100.0), 1.0, Vec2::ZERO);
    state.texture = texture(&harness.ctx);
    harness.app.lightbox = Some(state);
    // A new overlay fades in over its first frames, tinting what it paints.
    for _ in 0..FADE_IN_FRAMES {
        harness.idle();
    }
    let shapes = painted_shapes(&harness.idle());
    (shapes, harness.ctx.style().visuals.panel_fill)
}

/// Where in `shapes` a square with sides of `side` is filled with `fill`.
fn square_at(shapes: &[egui::Shape], side: f32, fill: egui::Color32) -> Option<usize> {
    shapes.iter().position(|shape| {
        matches!(shape, egui::Shape::Rect(rect)
            if rect.fill == fill && (rect.rect.size() - egui::vec2(side, side)).length() < 0.5)
    })
}

#[test]
fn a_diagram_lies_on_a_sheet_of_the_page_color() {
    // A diagram is drawn for the page and has a transparent background: on
    // the dark backdrop alone its ink would not show.
    let (shapes, page) = painted_with(|ctx| {
        let pixels = egui::ColorImage::filled([8, 8], egui::Color32::TRANSPARENT);
        LightboxTexture::Owned(ctx.load_texture("diagram", pixels, egui::TextureOptions::default()))
    });

    let sheet = square_at(&shapes, 2.0f32.mul_add(DIAGRAM_SHEET_MARGIN, 100.0), page);
    let diagram = shapes
        .iter()
        .position(|shape| matches!(shape, egui::Shape::Mesh(_)));

    assert!(sheet.is_some(), "no sheet under the diagram");
    assert!(sheet < diagram, "the sheet is painted over the diagram");
}

#[test]
fn a_loaded_picture_gets_no_sheet() {
    let (shapes, page) = painted_with(|_| LightboxTexture::Loaded(egui::TextureId::default()));

    let sheet = square_at(&shapes, 2.0f32.mul_add(DIAGRAM_SHEET_MARGIN, 100.0), page);

    assert_eq!(sheet, None);
}
