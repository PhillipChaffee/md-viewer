use super::*;

#[test]
fn emoji_fallback_face_resolves_for_doc_emoji() {
    // Build the same definitions setup_fonts() would install: egui
    // defaults + the bundled monochrome emoji face appended last.
    let mut definitions = FontDefinitions::default();
    install_emoji_fonts(&mut definitions);

    let ctx = egui::Context::default();
    ctx.set_fonts(definitions);
    // Fonts aren't built until the first Context::run() pass.
    ctx.run(egui::RawInput::default(), |_ctx| {});
    let font_id = egui::FontId::proportional(16.0);
    // Debug: font_data keys and family list as egui sees them.
    ctx.fonts(|f| {
        let defs = f.definitions();
        eprintln!("font_data keys: {:?}", defs.font_data.keys().collect::<Vec<_>>());
        eprintln!(
            "Proportional chain: {:?}",
            defs.families.get(&FontFamily::Proportional)
        );
        eprintln!("Monospace chain: {:?}", defs.families.get(&FontFamily::Monospace));
    });
    // Isolate which individual face supplies each glyph: one font per chain.
    let defaults = FontDefinitions::default();
    let mut with_ours = FontDefinitions::default();
    install_emoji_fonts(&mut with_ours);
    for (label, defs_source) in [
        ("builtin:NotoEmoji-Regular", &defaults),
        ("bundled:NotoEmoji", &with_ours),
    ] {
        let name = label.split(':').nth(1).unwrap();
        let mut single = defs_source.clone();
        single.font_data.retain(|k, _| k == name);
        *single.families.get_mut(&FontFamily::Proportional).unwrap() = vec![name.to_owned()];
        let ctx2 = egui::Context::default();
        ctx2.set_fonts(single);
        ctx2.run(egui::RawInput::default(), |_ctx| {});
        let font_id = egui::FontId::proportional(16.0);
        let results: Vec<bool> = ctx2.fonts_mut(|f| {
            ['\u{1F7E2}', '\u{1F534}', '\u{1F535}', '\u{26AA}', '\u{2705}']
                .iter()
                .map(|&ch| f.has_glyph(&font_id, ch))
                .collect()
        });
        eprintln!("face {label}: 🟢🔴🔵⚪✅ = {results:?}");
    }

    for ch in ['\u{1F7E2}', '\u{1F534}', '\u{1F535}', '\u{26AA}', '\u{2705}'] {
        let resolved = ctx.fonts_mut(|f| f.has_glyph(&font_id, ch));
        assert!(
            resolved,
            "emoji {ch} should resolve through the fallback chain to NotoEmoji"
        );
    }

    // End-to-end ink check: rasterize each emoji and require actual
    // coverage in the atlas, so a cmap hit with an empty outline fails.
    for ch in ['\u{1F7E2}', '\u{1F534}', '\u{2705}'] {
        let job = egui::text::LayoutJob::simple(
            ch.to_string(),
            font_id.clone(),
            egui::Color32::WHITE,
            f32::INFINITY,
        );
        let ink = ctx.fonts_mut(|f| {
            let galley = f.layout_job(job);
            let glyph = &galley.rows[0].glyphs[0];
            let uv = glyph.uv_rect;
            let image = f.image(); // full atlas
            let [w, h] = image.size;
            let (x0, y0) = (uv.min[0] as usize, uv.min[1] as usize);
            let (x1, y1) = (uv.max[0] as usize, uv.max[1] as usize);
            let mut count = 0usize;
            for y in y0.min(h)..y1.min(h) {
                for x in x0.min(w)..x1.min(w) {
                    if image[(x, y)].a() > 16 {
                        count += 1;
                    }
                }
            }
            count
        });
        eprintln!("ink pixels for {ch:?}: {ink}");
        assert!(ink > 20, "emoji {ch} rasterized blank (only {ink} ink pixels)");
    }

    // Color check (requires vendored epaint with the swash patch): each
    // doc emoji must resolve to the ColorEmoji face and produce genuinely
    // colored atlas pixels — saturated red/green RGB channels.
    for (ch, want_channel) in [('\u{1F7E2}', "green"), ('\u{1F534}', "red")] {
        let job = egui::text::LayoutJob::simple(
            ch.to_string(),
            font_id.clone(),
            egui::Color32::WHITE,
            f32::INFINITY,
        );
        let (colored, channel_peak) = ctx.fonts_mut(|f| {
            let galley = f.layout_job(job);
            let glyph = &galley.rows[0].glyphs[0];
            let uv = glyph.uv_rect;
            let image = f.image();
            let [w, h] = image.size;
            let mut peak = 0u8;
            for y in uv.min[1] as usize..(uv.max[1] as usize).min(h) {
                for x in uv.min[0] as usize..(uv.max[0] as usize).min(w) {
                    let px = image[(x, y)];
                    let m = match want_channel {
                        "green" => px.g(),
                        _ => px.r(),
                    };
                    // Require the color channel to dominate its counterpart.
                    let other = match want_channel {
                        "green" => px.r(),
                        _ => px.g(),
                    };
                    if px.a() > 32 && m > 48 && m as u16 > other as u16 + 16 {
                        peak = peak.max(m);
                    }
                }
            }
            (uv.colored, peak)
        });
        eprintln!("{ch:?}: colored flag = {colored}, {want_channel} channel peak = {channel_peak}");
        // Debug: dump first few atlas pixels from the glyph region.
        ctx.fonts(|f| {
            let _ = f;
        });
        let job2 = egui::text::LayoutJob::simple(
            ch.to_string(),
            font_id.clone(),
            egui::Color32::WHITE,
            f32::INFINITY,
        );
        ctx.fonts_mut(|f| {
            let galley = f.layout_job(job2);
            let uv = galley.rows[0].glyphs[0].uv_rect;
            let image = f.image();
            let mut shown = 0;
            for y in uv.min[1] as usize..uv.max[1] as usize {
                for x in uv.min[0] as usize..uv.max[0] as usize {
                    let px = image[(x, y)];
                    if px.a() > 0 && shown < 8 {
                        eprintln!("  atlas px ({x},{y}) = {:?}", px);
                        shown += 1;
                    }
                }
            }
        });
        assert!(colored, "emoji {ch} did not take the color-glyph path");
        assert!(
            channel_peak > 128,
            "emoji {ch} rendered without color (peak {want_channel} = {channel_peak})"
        );
    }
}

#[test]
fn color_emoji_face_never_claims_ui_symbols() {
    let mut definitions = FontDefinitions::default();
    install_emoji_fonts(&mut definitions);
    definitions.font_data.retain(|k, _| k == "ColorEmoji");
    *definitions.families.get_mut(&FontFamily::Proportional).unwrap() =
        vec!["ColorEmoji".to_owned()];
    let ctx = egui::Context::default();
    ctx.set_fonts(definitions);
    ctx.run(egui::RawInput::default(), |_ctx| {});
    let font_id = egui::FontId::proportional(16.0);
    // Default-emoji characters resolve through the color face...
    for ch in ['\u{1F7E2}', '\u{1F534}', '\u{26AA}', '\u{2705}'] {
        let ok = ctx.fonts_mut(|f| f.has_glyph(&font_id, ch));
        assert!(ok, "{ch} should come from ColorEmoji");
    }
    // ...but general UI symbols must not, even though Noto Color Emoji
    // maps them — they belong to regular text faces.
    for ch in ['\u{25B6}', '\u{25BC}', '\u{26A0}', '\u{2714}', '\u{2192}', '\u{2713}'] {
        let claimed = ctx.fonts_mut(|f| f.has_glyph(&font_id, ch));
        assert!(
            !claimed,
            "{ch} (UI symbol) must not be claimed by the color emoji face"
        );
    }
}
