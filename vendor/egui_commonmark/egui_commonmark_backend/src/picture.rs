//! The pictures of one page: what size each is, which have been asked for,
//! and where each lies, so that a picture is read only once its place is
//! near the view.
//!
//! A picture far from the view costs the page its size and no more. The
//! size is read from the head of the file, or from bytes the window holds
//! already. Where neither tells it, as for a picture on the web, the place
//! is the small one egui gives a picture it knows nothing of, until the
//! picture has been read.

use std::collections::{HashMap, HashSet};

use egui::{load::TextureLoadResult, Rangef, Rect, Vec2};

/// A picture is read once its place is within this many window heights of
/// the window, above it or below, so that it is ready when it comes into
/// view. Further away it is not read.
pub const NEAR_IN_WINDOW_HEIGHTS: f32 = 1.0;

/// How far above and below the window a picture counts as near it.
fn reach(ctx: &egui::Context) -> f32 {
    ctx.content_rect().height() * NEAR_IN_WINDOW_HEIGHTS
}

/// The size of the picture at `uri`, in its own pixels, if that is told
/// without reading the picture: by the head of a file, or by bytes the
/// window holds. A picture on the web would have to be fetched for it.
fn size_without_reading(ctx: &egui::Context, uri: &str) -> Option<Vec2> {
    let (width, height) = if let Some(path) = uri.strip_prefix("file://") {
        // As egui's file loader reads the address: `file:///C:/…` on Windows.
        let path = if cfg!(windows) {
            path.trim_start_matches('/')
        } else {
            path
        };
        image::ImageReader::open(path)
            .ok()?
            .with_guessed_format()
            .ok()?
            .into_dimensions()
            .ok()?
    } else if uri.starts_with("http://") || uri.starts_with("https://") {
        return None;
    } else {
        let Ok(egui::load::BytesPoll::Ready { bytes, .. }) = ctx.try_load_bytes(uri) else {
            return None;
        };
        image::ImageReader::new(std::io::Cursor::new(&bytes[..]))
            .with_guessed_format()
            .ok()?
            .into_dimensions()
            .ok()?
    };
    Some(egui::vec2(width as f32, height as f32))
}

/// Ask egui's loaders for the picture at `uri`, as the page shows it.
fn load(ctx: &egui::Context, uri: &str) -> TextureLoadResult {
    if crate::animation::may_move(uri) {
        crate::animation::read(ctx, uri)
    } else {
        egui::Image::from_uri(uri)
            .fit_to_original_size(1.0)
            .load_for_size(ctx, Vec2::ZERO)
    }
}

/// How far down the page a picture lies.
#[derive(Clone, Copy)]
struct Place {
    rows: Rangef,
    /// The pass of the window that laid it out.
    pass: u64,
}

/// What one page knows of its pictures, by their addresses.
#[derive(Default)]
pub(crate) struct PagePictures {
    /// The size of each picture the page has laid out: `None` where
    /// nothing tells it before the picture is read.
    sizes: HashMap<String, Option<Vec2>>,
    asked_for: HashSet<String>,
    unreadable: HashSet<String>,
    places: HashMap<String, Place>,
    /// How far down the page the top of the view is, if the last pass of
    /// the page knew.
    view_top: Option<f32>,
}

impl PagePictures {
    /// The size of the picture at `uri`, if it is known yet.
    pub(crate) fn size(&mut self, ctx: &egui::Context, uri: &str) -> Option<Vec2> {
        if let Some(size) = self.sizes.get(uri) {
            return *size;
        }
        let size = size_without_reading(ctx, uri);
        self.sizes.insert(uri.to_owned(), size);
        size
    }

    /// The picture at `uri` was read, and its size is `size`.
    pub(crate) fn learn_size(&mut self, uri: &str, size: Vec2) {
        self.sizes.insert(uri.to_owned(), Some(size));
    }

    pub(crate) fn was_asked_for(&self, uri: &str) -> bool {
        self.asked_for.contains(uri)
    }

    /// Have the picture at `uri` read, if it is not yet, and tell how far
    /// that has come.
    ///
    /// egui's loaders keep a failure for as long as the app runs. A page
    /// that asks for a picture for the first time was just loaded, or
    /// loaded again: a failure it finds is left from before, and is
    /// forgotten so that the picture is asked for once more.
    pub(crate) fn read(&mut self, ctx: &egui::Context, uri: &str) -> TextureLoadResult {
        let first = self.asked_for.insert(uri.to_owned());
        let loaded = load(ctx, uri);
        if first && loaded.is_err() {
            ctx.forget_image(uri);
            return load(ctx, uri);
        }
        loaded
    }

    /// Whether the picture at `uri` could not be read when the page last
    /// tried. A page that finds it readable again takes it back.
    pub(crate) fn is_unreadable(&mut self, ctx: &egui::Context, uri: &str) -> bool {
        if !self.unreadable.contains(uri) {
            return false;
        }
        let still = self.read(ctx, uri).is_err();
        if !still {
            self.unreadable.remove(uri);
        }
        still
    }

    pub(crate) fn could_not_be_read(&mut self, uri: &str) {
        self.unreadable.insert(uri.to_owned());
    }

    /// The picture at `uri` was laid out at `rect` of the window. Returns
    /// whether that is near the window.
    pub(crate) fn placed(&mut self, ui: &egui::Ui, uri: &str, rect: Rect) -> bool {
        // The page scrolls in the outermost scroll area, whose content
        // starts at the top of the page.
        let stack = ui.stack();
        let scroll_areas = stack
            .iter()
            .filter(|ui| ui.kind() == Some(egui::UiKind::ScrollArea));
        if let Some(page) = scroll_areas.last() {
            let page_top = page.max_rect.top();
            let rows = Rangef::new(rect.top() - page_top, rect.bottom() - page_top);
            let pass = ui.ctx().cumulative_pass_nr();
            let place = self
                .places
                .entry(uri.to_owned())
                .or_insert(Place { rows, pass });
            // A picture the page shows twice lies wherever either does.
            let both = Rangef::new(place.rows.min.min(rows.min), place.rows.max.max(rows.max));
            place.rows = if place.pass == pass { both } else { rows };
            place.pass = pass;
        }
        let window = ui.ctx().content_rect().y_range().expand(reach(ui.ctx()));
        window.intersects(rect.y_range())
    }

    /// The top of the view is `offset` down the page, or the pass that is
    /// about to lay the page out cannot tell.
    pub(crate) fn view_is_at(&mut self, offset: Option<f32>) {
        self.view_top = offset;
    }

    /// The pictures whose places are near the view, wherever the page was
    /// last laid out. Only the part of a page in view is laid out in most
    /// frames, and a picture just outside that part is not among it.
    pub(crate) fn near_the_view(&self, ctx: &egui::Context) -> Vec<String> {
        let Some(view_top) = self.view_top else {
            return Vec::new();
        };
        let window = ctx.content_rect().height();
        let near = Rangef::new(view_top, view_top + window).expand(reach(ctx));
        let places = self.places.iter();
        let near = places.filter(|(_, place)| near.intersects(place.rows));
        near.map(|(uri, _)| uri.clone()).collect()
    }
}
