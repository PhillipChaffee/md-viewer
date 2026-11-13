//! Kani proofs for the lightbox zoom. `PROOFS.md` lists each harness
//! with its bounds.

use std::mem::ManuallyDrop;

use eframe::egui;

use super::{LightboxState, LightboxTexture};

/// A lightbox at `zoom` with any picture size and any scroll offset. It is
/// never dropped: dropping a texture reaches into egui's texture manager,
/// which is no part of the arithmetic proven here and which the verifier
/// cannot compile.
fn lightbox(zoom: f32) -> ManuallyDrop<LightboxState> {
    ManuallyDrop::new(LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::Managed(0)),
        base_size: egui::Vec2::new(kani::any(), kani::any()),
        zoom,
        open_id: kani::any(),
        scroll_offset: egui::Vec2::new(kani::any(), kani::any()),
    })
}

/// Zooming the lightbox from any zoom in its range, by any factor that is
/// a number, stays inside the range of 10% to 1000%.
#[kani::proof]
fn lightbox_zoom_stays_in_range() {
    let (zoom, factor): (f32, f32) = (kani::any(), kani::any());
    kani::assume((0.1..=10.0).contains(&zoom) && !factor.is_nan());
    let mut state = lightbox(zoom);

    state.zoom_by(factor);

    assert!((0.1..=10.0).contains(&state.zoom));

    kani::cover!(state.zoom == 10.0 && zoom < 10.0);
    kani::cover!(state.zoom == 0.1 && zoom > 0.1);
    kani::cover!(state.zoom > 0.1 && state.zoom < 10.0 && state.zoom != zoom);
}

/// For a picture between 1 and `MAX_PX` px a side, both zooms in the
/// lightbox's range, and a viewport, pointer and scroll offset within
/// `MAX_PX` px, the scroll offset that keeps the picture under the
/// pointer never panics, is a number, and is never negative.
#[kani::proof]
fn zoom_to_cursor_never_panics_or_goes_negative() {
    /// Larger than any picture or screen.
    const MAX_PX: f32 = 1_000_000.0;
    let within = |vector: egui::Vec2, low: f32| {
        vector.x >= low && vector.x <= MAX_PX && vector.y >= low && vector.y <= MAX_PX
    };

    let (old_zoom, zoom): (f32, f32) = (kani::any(), kani::any());
    kani::assume((0.1..=10.0).contains(&old_zoom) && (0.1..=10.0).contains(&zoom));
    let state = lightbox(zoom);
    let pointer = egui::Vec2::new(kani::any(), kani::any());
    let viewport = egui::Vec2::new(kani::any(), kani::any());
    kani::assume(within(state.base_size, 1.0) && within(viewport, 0.0));
    kani::assume(within(pointer, -MAX_PX) && within(state.scroll_offset, -MAX_PX));

    let offset = state.zoom_to_cursor_offset(pointer, old_zoom, viewport);

    assert!(offset.x >= 0.0 && offset.y >= 0.0);

    kani::cover!(offset.x > 0.0 && offset.y > 0.0);
    kani::cover!(offset.x == 0.0);
}
