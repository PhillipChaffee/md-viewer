//! The zoomable lightbox overlay for diagrams and images.

use eframe::egui;

use crate::app::{zoom_percent, MarkdownApp};

/// Margin kept between the image viewport and the edges of the screen.
const VIEWPORT_PADDING: f32 = 40.0;

/// Source of a lightbox texture. Mermaid pre-rasterizes its own texture so we
/// own the handle to keep it alive; loader-managed images (`egui_extras`) just
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
    /// Original rasterized pixel dimensions (before any zoom).
    pub(crate) base_size: egui::Vec2,
    pub(crate) zoom: f32,
    /// Unique ID per open — prevents stale egui Area/ScrollArea state between opens.
    pub(crate) open_id: u64,
    /// Tracks scroll offset for zoom-to-cursor (we maintain our own copy because
    /// egui's `ScrollArea` may clamp the old offset when content size changes mid-zoom).
    pub(crate) scroll_offset: egui::Vec2,
}

impl LightboxState {
    /// Multiply the zoom by `factor`, within the range the overlay supports.
    pub(crate) fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
    }

    /// The scroll offset that keeps the image point under the pointer in
    /// place after the zoom moved from `old_zoom` to the current zoom.
    /// `pointer` is relative to the viewport origin, and `viewport_size` is
    /// the size of the area the image scrolls in.
    fn zoom_to_cursor_offset(
        &self,
        pointer: egui::Vec2,
        old_zoom: f32,
        viewport_size: egui::Vec2,
    ) -> egui::Vec2 {
        let old_display = self.base_size * old_zoom;
        let new_display = self.base_size * self.zoom;

        let old_content = old_display.max(viewport_size);
        let new_content = new_display.max(viewport_size);

        let old_pad = (old_content - old_display) * 0.5;
        let new_pad = (new_content - new_display) * 0.5;

        let frac = (pointer + self.scroll_offset - old_pad) / old_display;

        let new_offset = frac * new_display + new_pad - pointer;

        let max_offset = (new_content - viewport_size).max(egui::Vec2::ZERO);

        new_offset.clamp(egui::Vec2::ZERO, max_offset)
    }

    /// Show the image in a pannable viewport at `viewport_pos`. A
    /// `zoom_offset` is the scroll position a zoom change asked for this
    /// frame. Returns whether a click landed beside the image.
    fn show_image(
        &mut self,
        ctx: &egui::Context,
        viewport_pos: egui::Pos2,
        viewport_size: egui::Vec2,
        zoom_offset: Option<egui::Vec2>,
    ) -> bool {
        let display_size = self.base_size * self.zoom;
        let mut clicked_beside_image = false;

        egui::Area::new(egui::Id::new("lightbox_image").with(self.open_id))
            .order(egui::Order::Tooltip)
            .movable(false)
            .fixed_pos(viewport_pos)
            .show(ctx, |ui| {
                ui.set_min_size(viewport_size);
                ui.set_max_size(viewport_size);

                // Only enable scroll/drag on axes where image overflows the viewport
                let mut scroll_area = egui::ScrollArea::new([
                    display_size.x > viewport_size.x,
                    display_size.y > viewport_size.y,
                ])
                .id_salt(egui::Id::new("lightbox_scroll").with(self.open_id))
                .max_width(viewport_size.x)
                .max_height(viewport_size.y)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .scroll_source(egui::scroll_area::ScrollSource {
                    drag: true,
                    ..Default::default()
                });

                // Set the pre-computed offset so ScrollArea renders at the
                // correct position on this frame (avoids one-frame lag).
                if let Some(offset) = zoom_offset {
                    scroll_area = scroll_area
                        .horizontal_scroll_offset(offset.x)
                        .vertical_scroll_offset(offset.y);
                }

                let scroll_output = scroll_area.show(ui, |ui| {
                    clicked_beside_image = self.image_canvas(ui, viewport_size);
                });

                // Sync tracking: captures drag-to-scroll changes on non-zoom frames
                if zoom_offset.is_none() {
                    self.scroll_offset = scroll_output.state.offset;
                }
            });

        clicked_beside_image
    }

    /// Paint the image centered in its scrollable canvas and handle clicks
    /// on it: a double click resets the zoom. Returns whether a click landed
    /// beside the image.
    fn image_canvas(&mut self, ui: &mut egui::Ui, viewport_size: egui::Vec2) -> bool {
        let display_size = self.base_size * self.zoom;
        // Allocate content: at least the viewport for centering, larger
        // when the image overflows (for scroll panning)
        let content_size = display_size.max(viewport_size);
        let (content_rect, response) = ui.allocate_exact_size(content_size, egui::Sense::click());
        // Draw image centered within the content rect — GPU texture
        // scaling, no re-rasterization
        let image_rect = egui::Rect::from_center_size(content_rect.center(), display_size);
        if ui.is_rect_visible(image_rect) {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let mut mesh = egui::Mesh::with_texture(self.texture.id());
            mesh.add_rect_with_uv(image_rect, uv, egui::Color32::WHITE);
            ui.painter().add(egui::Shape::mesh(mesh));
        }
        if response.double_clicked() {
            self.zoom = 1.0;
            self.scroll_offset = egui::Vec2::ZERO;
        }
        response.clicked()
            && response
                .interact_pointer_pos()
                .is_some_and(|pos| !image_rect.contains(pos))
    }

    /// Show the zoom percentage at the bottom center unless it is 100%.
    fn paint_zoom_indicator(&self, ctx: &egui::Context, screen_rect: egui::Rect) {
        let zoom_pct = zoom_percent(self.zoom);
        if zoom_pct == 100 {
            return;
        }
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("lightbox_zoom_indicator").with(self.open_id),
        ))
        .text(
            egui::pos2(screen_rect.center().x, screen_rect.bottom() - 24.0),
            egui::Align2::CENTER_CENTER,
            format!("{zoom_pct}%"),
            egui::FontId::proportional(16.0),
            egui::Color32::from_gray(180),
        );
    }
}

/// Semi-transparent backdrop (visual only, drawn below Tooltip layers).
fn paint_backdrop(ctx: &egui::Context, screen_rect: egui::Rect) {
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("lightbox_backdrop"),
    ))
    .rect_filled(
        screen_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
    );
}

/// Full-screen input sink at Tooltip order — blocks all input to lower
/// layers. Returns whether the sink itself (outside image and close button)
/// was clicked.
fn input_sink_clicked(ctx: &egui::Context, screen_rect: egui::Rect, open_id: u64) -> bool {
    egui::Area::new(egui::Id::new("lightbox_sink").with(open_id))
        .order(egui::Order::Tooltip)
        .movable(false)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            ui.allocate_response(screen_rect.size(), egui::Sense::click())
                .clicked()
        })
        .inner
}

/// The close button in the top-right corner. Returns whether it was clicked.
fn close_button_clicked(ctx: &egui::Context, screen_rect: egui::Rect, open_id: u64) -> bool {
    egui::Area::new(egui::Id::new("lightbox_close").with(open_id))
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
            btn.clicked()
        })
        .inner
}

impl MarkdownApp {
    pub(crate) fn render_lightbox(&mut self, ctx: &egui::Context) {
        let Some(lightbox) = &mut self.lightbox else {
            return;
        };

        let screen_rect = ctx.available_rect();
        paint_backdrop(ctx, screen_rect);
        // Clicking on the sink closes the lightbox.
        let mut should_close = input_sink_clicked(ctx, screen_rect, lightbox.open_id);

        // Apply scroll-wheel zoom (proportional to scroll amount for smooth feel)
        let old_zoom = lightbox.zoom;
        if self.lightbox_scroll != 0.0 {
            // ~10% per scroll notch, smooth with proportional delta
            lightbox.zoom_by(self.lightbox_scroll.mul_add(0.08, 1.0).clamp(0.5, 2.0));
        }
        let zoom_changed = (lightbox.zoom - old_zoom).abs() > f32::EPSILON;

        let viewport_size = screen_rect.shrink(VIEWPORT_PADDING).size();
        // Use fixed_pos instead of anchor to avoid position jitter when content
        // size changes (anchor recalculates position based on laid-out size,
        // causing a one-frame shift when scrollbars appear/disappear).
        let viewport_pos = screen_rect.center() - viewport_size / 2.0;

        // Pre-compute zoom-to-cursor offset BEFORE ScrollArea renders,
        // so it uses the correct offset on the same frame (no one-frame jitter).
        let zoom_offset = if zoom_changed {
            ctx.input(|i| i.pointer.latest_pos()).map(|pointer_pos| {
                // viewport_pos ≈ viewport origin (no scrollbar/frame margins)
                lightbox.zoom_to_cursor_offset(pointer_pos - viewport_pos, old_zoom, viewport_size)
            })
        } else {
            None
        };
        if let Some(offset) = zoom_offset {
            lightbox.scroll_offset = offset;
        }

        should_close |= lightbox.show_image(ctx, viewport_pos, viewport_size, zoom_offset);
        should_close |= close_button_clicked(ctx, screen_rect, lightbox.open_id);
        lightbox.paint_zoom_indicator(ctx, screen_rect);
        should_close |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if should_close {
            self.lightbox = None;
        }
    }
}
