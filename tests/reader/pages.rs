//! Pages and pictures the rendering tests open.

use super::support::{Area, Launch, Reader};

/// How far apart, in pixels, two positions may be and still be one place.
pub(crate) const NEAR: f32 = 1.5;

/// A 120 by 80 picture of one flat blue, `#3366CC`.
pub(crate) const BLUE_PNG: &[u8] = include_bytes!("../fixtures/blue.png");
/// A 90 by 60 picture of one flat red, `#CC3333`.
pub(crate) const RED_JPEG: &[u8] = include_bytes!("../fixtures/red.jpg");
/// A 60 by 40 picture of one flat green, `#33AA55`.
pub(crate) const GREEN_GIF: &[u8] = include_bytes!("../fixtures/green.gif");
/// A 1600 by 200 picture of one flat blue: wider than any column here.
pub(crate) const WIDE_PNG: &[u8] = include_bytes!("../fixtures/wide.png");
/// A 200 by 100 drawing of one flat green, `#22AA44`.
pub(crate) const GREEN_SVG: &str = concat!(
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">"#,
    r##"<rect width="200" height="100" fill="#22aa44"/></svg>"##
);

/// How far past the edge of the text column a mark drawn on it reaches.
const EDGE: f32 = 4.0;

/// The four themes, in the order Ctrl+D steps through them from Paper.
pub(crate) const THEMES: [&str; 4] = ["Paper", "Sepia", "Dusk", "Night"];

/// `md-viewer page.md` on a page that holds `text`.
pub(crate) fn page(text: &str) -> Reader {
    Reader::with_docs(&[("page.md", text)])
}

/// A launch of `md-viewer page.md` on a page that holds `text`, with every
/// picture of this module beside it.
pub(crate) fn illustrated(text: &str) -> Launch {
    Reader::launch()
        .doc("page.md", text)
        .bytes("blue.png", BLUE_PNG)
        .bytes("red.jpg", RED_JPEG)
        .bytes("green.gif", GREEN_GIF)
        .bytes("wide.png", WIDE_PNG)
        .doc("shape.svg", GREEN_SVG)
        .args(&["page.md"])
}

/// The text `line` makes of each of `items`, one after another.
pub(crate) fn each<T>(items: impl IntoIterator<Item = T>, line: impl Fn(T) -> String) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(&line(item));
    }
    text
}

/// Read on `theme`, picked in the appearance popover.
pub(crate) fn read_on(reader: &mut Reader, theme: &str) {
    reader.click("Appearance");
    reader.click(theme);
    reader.press("Escape");
}

/// The room in the text column beside the text `label`, on its left: where
/// a list item's bullet or a quote's bar is drawn.
pub(crate) fn left_of(reader: &Reader, label: &str) -> Area {
    let text = reader.text_area(label);
    let column = reader.column();
    // A mark on the column's edge straddles it.
    let left = column.left - EDGE;
    Area {
        left,
        width: text.left - left,
        ..text
    }
}

/// The whole row of the text column that the text `label` is on.
pub(crate) fn row_of(reader: &Reader, label: &str) -> Area {
    let text = reader.text_area(label);
    let column = reader.column();
    Area {
        left: column.left,
        width: column.width,
        ..text
    }
}

/// The room in the text column between the texts `above` and `below`.
pub(crate) fn between(reader: &Reader, above: &str, below: &str) -> Area {
    let column = reader.column();
    let top = reader.text_area(above).bottom();
    Area {
        left: column.left - EDGE,
        top,
        width: 2.0_f32.mul_add(EDGE, column.width),
        height: reader.text_area(below).top - top,
    }
}

/// The pictures painted between the texts `above` and `below`, once at
/// least one is: formulas and diagrams are drawn in the background.
pub(crate) fn pictures_between(reader: &mut Reader, above: &str, below: &str) -> Vec<Area> {
    let within = |reader: &Reader| {
        let (top, bottom) = (
            reader.text_area(above).bottom(),
            reader.text_area(below).top,
        );
        let mut pictures = reader.pictures();
        pictures.retain(|picture| picture.top >= top - NEAR && picture.bottom() <= bottom + NEAR);
        pictures
    };
    reader.wait_for("a picture to be drawn", |reader| !within(reader).is_empty());
    within(reader)
}

/// The one picture painted between the texts `above` and `below`.
#[track_caller]
pub(crate) fn picture_between(reader: &mut Reader, above: &str, below: &str) -> Area {
    let pictures = pictures_between(reader, above, below);
    assert_eq!(pictures.len(), 1, "between `{above}` and `{below}`");
    pictures[0]
}
