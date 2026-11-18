//! A picture that could not be loaded is asked for again when its page is
//! loaded again.
//!
//! egui's loaders remember a failure for as long as the app runs, so the
//! renderer has to drop it. A new `CommonMarkCache` is what a page load
//! looks like from here.

#![cfg(feature = "load-images")]

use std::time::{Duration, Instant};

use egui::load::TexturePoll;
use egui_commonmark_extended::{CommonMarkCache, CommonMarkViewer};

const WARNING: &str = "\u{26a0}";

fn painted_text(shape: &egui::Shape, text: &mut String) {
    match shape {
        egui::Shape::Text(shape) => text.push_str(shape.galley.text()),
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| painted_text(shape, text)),
        _ => {}
    }
}

/// One frame of `markdown`, and the text it painted.
fn frame(ctx: &egui::Context, cache: &mut CommonMarkCache, markdown: &str) -> String {
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            CommonMarkViewer::new().show(ui, cache, markdown);
        });
    });
    let mut text = String::new();
    for clipped in &output.shapes {
        painted_text(&clipped.shape, &mut text);
    }
    text
}

/// Frames of `markdown` until `done` says so of the text one painted.
/// Pictures are read on another thread.
#[track_caller]
fn frames_until(
    ctx: &egui::Context,
    cache: &mut CommonMarkCache,
    markdown: &str,
    what: &str,
    done: impl Fn(&str) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(&frame(ctx, cache, markdown)) {
        assert!(Instant::now() < deadline, "waited too long for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn is_loaded(ctx: &egui::Context, uri: &str) -> bool {
    let load = ctx.try_load_texture(uri, egui::TextureOptions::default(), Default::default());
    matches!(load, Ok(TexturePoll::Ready { .. }))
}

#[test]
fn a_picture_that_failed_is_asked_for_again_by_the_next_page_load() {
    let dir = std::env::temp_dir().join(format!("late-picture-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("late.png");
    let _ = std::fs::remove_file(&file);
    let uri = format!("file://{}", file.display());
    let markdown = format!("![late]({uri})");
    let ctx = egui::Context::default();

    let mut first_load = CommonMarkCache::default();
    frames_until(&ctx, &mut first_load, &markdown, "the warning", |text| {
        text.contains(WARNING)
    });

    image::RgbaImage::from_pixel(4, 4, image::Rgba([51, 102, 204, 255]))
        .save(&file)
        .unwrap();
    let still = frame(&ctx, &mut first_load, &markdown);
    assert!(still.contains(WARNING), "the same page load does not retry");

    let mut reload = CommonMarkCache::default();
    frames_until(&ctx, &mut reload, &markdown, "the picture", |_| {
        is_loaded(&ctx, &uri)
    });
    let shown = frame(&ctx, &mut reload, &markdown);
    assert!(!shown.contains(WARNING), "{shown:?}");

    std::fs::remove_dir_all(&dir).unwrap();
}
