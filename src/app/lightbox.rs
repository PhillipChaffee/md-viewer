//! The zoomable lightbox overlay for diagrams and images.

use eframe::egui;

use crate::app::MarkdownApp;

/// Source of a lightbox texture. Mermaid pre-rasterizes its own texture so we
/// own the handle to keep it alive; loader-managed images (egui_extras) just
/// give us an id whose lifetime is governed by the loader.
pub(crate) enum LightboxTexture {
    Owned(egui::TextureHandle),
    Loaded(egui::TextureId),
}

impl LightboxTexture {
    fn id(&self) -> egui::TextureId {
        match self {
            Self::Owned(handle) => handle.id(),
            Self::Loaded(id) => *id,
        }
    }
}

pub(crate) struct LightboxState {
    /// GPU-resident texture (mermaid-owned or loader-owned). Zoom scales it — instant.
    pub(crate) texture: LightboxTexture,
    /// Original rasterized pixel dimensions (before any zoom)
    pub(crate) base_size: egui::Vec2,
    pub(crate) zoom: f32,
    /// Unique ID per open — prevents stale egui Area/ScrollArea state between opens
    pub(crate) open_id: u64,
    /// Tracks scroll offset for zoom-to-cursor (we maintain our own copy because
    /// egui's ScrollArea may clamp the old offset when content size changes mid-zoom)
    pub(crate) scroll_offset: egui::Vec2,
}

impl MarkdownApp {
    pub(crate) fn render_lightbox(&mut self, ctx: &egui::Context) {
        let Some(lightbox) = &mut self.lightbox else {
            return;
        };

        let screen_rect = ctx.available_rect();
        let mut should_close = false;

        // 1. Semi-transparent backdrop (visual only, drawn below Tooltip layers)
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("lightbox_backdrop"),
        ))
        .rect_filled(
            screen_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
        );

        // 2. Full-screen input sink at Tooltip order — blocks all input to lower layers.
        //    Clicking on the sink (outside image/close) closes the lightbox.
        let oid = lightbox.open_id;
        egui::Area::new(egui::Id::new("lightbox_sink").with(oid))
            .order(egui::Order::Tooltip)
            .movable(false)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let r = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                if r.clicked() {
                    should_close = true;
                }
            });

        // 3. Apply scroll-wheel zoom (proportional to scroll amount for smooth feel)
        let old_zoom = lightbox.zoom;
        if self.lightbox_scroll != 0.0 {
            // ~10% per scroll notch, smooth with proportional delta
            let factor = (1.0_f32 + self.lightbox_scroll * 0.08).clamp(0.5, 2.0);
            lightbox.zoom = (lightbox.zoom * factor).clamp(0.1, 10.0);
        }
        let zoom_changed = (lightbox.zoom - old_zoom).abs() > f32::EPSILON;

        // 4. Image — GPU texture scaling, no re-rasterization
        let padding = 40.0;
        let area_rect = screen_rect.shrink(padding);
        let area_size = area_rect.size();
        let zoom = lightbox.zoom;
        let base_size = lightbox.base_size;
        let display_size = base_size * zoom;
        let tex_id = lightbox.texture.id();
        let tex_uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

        // Use fixed_pos instead of anchor to avoid position jitter when content
        // size changes (anchor recalculates position based on laid-out size,
        // causing a one-frame shift when scrollbars appear/disappear).
        let area_pos = egui::pos2(
            screen_rect.center().x - area_size.x / 2.0,
            screen_rect.center().y - area_size.y / 2.0,
        );

        // Pre-compute zoom-to-cursor offset BEFORE ScrollArea renders,
        // so it uses the correct offset on the same frame (no one-frame jitter).
        let pre_offset: Option<egui::Vec2> = if zoom_changed {
            ctx.input(|i| i.pointer.latest_pos()).map(|mouse_pos| {
                // area_pos ≈ viewport origin (no scrollbar/frame margins)
                let mouse_in_vp = mouse_pos - area_pos;

                let old_display = base_size * old_zoom;
                let new_display = display_size;

                let old_content = egui::vec2(
                    old_display.x.max(area_size.x),
                    old_display.y.max(area_size.y),
                );
                let new_content = egui::vec2(
                    new_display.x.max(area_size.x),
                    new_display.y.max(area_size.y),
                );

                let old_pad = (old_content - old_display) * 0.5;
                let new_pad = (new_content - new_display) * 0.5;

                let frac = (mouse_in_vp + lightbox.scroll_offset - old_pad) / old_display;

                let new_offset = frac * new_display + new_pad - mouse_in_vp;

                let max_offset = egui::vec2(
                    (new_content.x - area_size.x).max(0.0),
                    (new_content.y - area_size.y).max(0.0),
                );

                egui::vec2(
                    new_offset.x.clamp(0.0, max_offset.x),
                    new_offset.y.clamp(0.0, max_offset.y),
                )
            })
        } else {
            None
        };

        if let Some(offset) = pre_offset {
            lightbox.scroll_offset = offset;
        }

        egui::Area::new(egui::Id::new("lightbox_image").with(oid))
            .order(egui::Order::Tooltip)
            .movable(false)
            .fixed_pos(area_pos)
            .show(ctx, |ui| {
                ui.set_min_size(area_size);
                ui.set_max_size(area_size);

                // Only enable scroll/drag on axes where image overflows the viewport
                let mut scroll_area = egui::ScrollArea::new([
                    display_size.x > area_size.x,
                    display_size.y > area_size.y,
                ])
                .id_salt(egui::Id::new("lightbox_scroll").with(oid))
                .max_width(area_size.x)
                .max_height(area_size.y)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .scroll_source(egui::scroll_area::ScrollSource {
                    drag: true,
                    ..Default::default()
                });

                // Set the pre-computed offset so ScrollArea renders at the
                // correct position on this frame (avoids one-frame lag).
                if let Some(offset) = pre_offset {
                    scroll_area = scroll_area
                        .horizontal_scroll_offset(offset.x)
                        .vertical_scroll_offset(offset.y);
                }

                let scroll_output = scroll_area.show(ui, |ui| {
                    // Allocate content: at least area_size for centering,
                    // larger when image overflows (for scroll panning)
                    let content_size = egui::vec2(
                        display_size.x.max(area_size.x),
                        display_size.y.max(area_size.y),
                    );
                    let (content_rect, response) =
                        ui.allocate_exact_size(content_size, egui::Sense::click());
                    // Draw image centered within the content rect
                    let image_rect =
                        egui::Rect::from_center_size(content_rect.center(), display_size);
                    if ui.is_rect_visible(image_rect) {
                        let mut mesh = egui::Mesh::with_texture(tex_id);
                        mesh.add_rect_with_uv(image_rect, tex_uv, egui::Color32::WHITE);
                        ui.painter().add(egui::Shape::mesh(mesh));
                    }
                    if response.double_clicked() {
                        lightbox.zoom = 1.0;
                        lightbox.scroll_offset = egui::Vec2::ZERO;
                    }
                    // Click outside image area → close lightbox
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if !image_rect.contains(pos) {
                                should_close = true;
                            }
                        }
                    }
                });

                // Sync tracking: captures drag-to-scroll changes on non-zoom frames
                if pre_offset.is_none() {
                    lightbox.scroll_offset = scroll_output.state.offset;
                }
            });

        // 5. Close button (top-right)
        egui::Area::new(egui::Id::new("lightbox_close").with(oid))
            .order(egui::Order::Tooltip)
            .movable(false)
            .fixed_pos(egui::pos2(
                screen_rect.right() - 48.0,
                screen_rect.top() + 8.0,
            ))
            .show(ctx, |ui| {
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("\u{2715}")
                            .size(20.0)
                            .color(egui::Color32::from_gray(200)),
                    )
                    .frame(false),
                );
                if btn.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if btn.clicked() {
                    should_close = true;
                }
            });

        // 6. Zoom indicator (bottom-center) when zoom ≠ 100%
        let zoom_pct = (lightbox.zoom * 100.0).round() as i32;
        if zoom_pct != 100 {
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("lightbox_zoom_indicator").with(oid),
            ))
            .text(
                egui::pos2(screen_rect.center().x, screen_rect.bottom() - 24.0),
                egui::Align2::CENTER_CENTER,
                format!("{zoom_pct}%"),
                egui::FontId::proportional(16.0),
                egui::Color32::from_gray(180),
            );
        }

        // 7. Escape to close
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                should_close = true;
            }
        });

        if should_close {
            self.lightbox = None;
        }
    }
}
