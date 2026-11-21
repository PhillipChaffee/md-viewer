//! md-viewer patch: a row of running text ends only where prose breaks.
//! Each piece of a paragraph is a text of its own, and a row used to end
//! wherever one piece ended and the next began: after a bracket, before a
//! comma, between a word and the code written onto it.

use egui::{Context, Rect, Shape};
use egui_commonmark_extended::{CommonMarkCache, CommonMarkViewer};

/// The characters one painted text has on one of its rows, and where.
struct Stretch {
    text: String,
    rect: Rect,
}

fn collect(shape: &Shape, stretches: &mut Vec<Stretch>) {
    match shape {
        Shape::Text(text) => {
            for row in &text.galley.rows {
                let (Some(first), Some(last)) = (row.row.glyphs.first(), row.row.glyphs.last())
                else {
                    continue;
                };
                let origin = text.pos + row.pos.to_vec2();
                stretches.push(Stretch {
                    text: row.row.glyphs.iter().map(|glyph| glyph.chr).collect(),
                    rect: Rect::from_min_max(
                        origin + egui::vec2(first.pos.x, 0.0),
                        origin + egui::vec2(last.max_x(), row.row.size.y),
                    ),
                });
            }
        }
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, stretches)),
        _ => {}
    }
}

/// The rows `markdown` is set in at `width`, top to bottom: every piece
/// on a row joined left to right, with nothing put between them.
fn rows(markdown: &str, width: f32) -> Vec<String> {
    let ctx = Context::default();
    let mut cache = CommonMarkCache::default();
    let mut stretches = Vec::new();
    // Two passes let egui settle its font and layout caches.
    for pass in 0..2 {
        ctx.begin_pass(Default::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.set_width(width);
            CommonMarkViewer::new()
                .default_width(Some(width as usize))
                .show(ui, &mut cache, markdown);
        });
        let output = ctx.end_pass();
        if pass == 1 {
            for clipped in &output.shapes {
                collect(&clipped.shape, &mut stretches);
            }
        }
    }

    stretches.sort_by(|a, b| a.rect.center().y.total_cmp(&b.rect.center().y));
    let mut rows: Vec<Vec<Stretch>> = Vec::new();
    for stretch in stretches {
        let middle = stretch.rect.center().y;
        let shares = |row: &Vec<Stretch>| {
            row.iter()
                .any(|set| (set.rect.top()..set.rect.bottom()).contains(&middle))
        };
        match rows.last_mut().filter(|row| shares(row)) {
            Some(row) => row.push(stretch),
            None => rows.push(vec![stretch]),
        }
    }
    rows.into_iter()
        .map(|mut row| {
            row.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
            row.iter().map(|set| set.text.as_str()).collect()
        })
        .collect()
}

const NAMES: [&str; 12] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima",
];

/// At every width, each unit `whole` makes of a name is on one row of the
/// paragraph `written` makes of the names.
fn assert_whole(written: impl Fn(&str) -> String, whole: impl Fn(&str) -> String) {
    let markdown: String = NAMES.iter().map(|name| written(name)).collect();
    for width in (180..=480).step_by(20) {
        let rows = rows(&markdown, width as f32);
        assert!(rows.len() > 1, "{width}: the text does not wrap: {rows:?}");
        for name in NAMES {
            let unit = whole(name);
            assert!(
                rows.iter().any(|row| row.contains(&unit)),
                "{width}: `{unit}` is split between two rows:\n{}",
                rows.join("\n")
            );
        }
    }
}

#[test]
fn brackets_and_a_comma_stay_on_the_row_of_the_code_between_them() {
    assert_whole(|name| format!("(`{name}`), "), |name| format!("({name}),"));
}

#[test]
fn a_letter_written_onto_code_stays_on_its_row() {
    assert_whole(
        |name| format!("un`{name}`s and "),
        |name| format!("un{name}s"),
    );
}

#[test]
fn what_touches_a_link_or_bold_text_stays_on_its_row() {
    assert_whole(
        |name| format!("([{name}](x.md)). "),
        |name| format!("({name})."),
    );
    assert_whole(
        |name| format!("(**{name}**)*s*; "),
        |name| format!("({name})s;"),
    );
}

#[test]
fn a_row_still_ends_at_a_space_between_two_pieces() {
    let markdown: String = NAMES.iter().map(|name| format!("`{name}` and ")).collect();

    let rows = rows(&markdown, 240.0);

    assert!(rows.len() > 2, "the text does not wrap: {rows:?}");
}

/// A run with nowhere to break that is wider than the page cannot stay
/// whole: it is still set on more rows than one.
#[test]
fn a_run_wider_than_the_page_still_wraps() {
    let long = "abcdefghij".repeat(8);
    let markdown = format!("Before ({long}`code`{long}), after.");

    let rows = rows(&markdown, 240.0);

    assert!(rows.len() > 3, "the run does not wrap: {rows:?}");
}
