use super::*;
use crate::MermaidColors;
use egui::Color32;

const FLOWCHART: &str = "flowchart LR
    A[Prompt] --> B{Compare}
    B -->|large delta| C[Candidates]";

fn night() -> MermaidColors {
    MermaidColors {
        page: Color32::BLACK,
        text: Color32::from_rgb(0xc8, 0xc6, 0xc1),
        line: Color32::from_rgb(0xa0, 0x9e, 0x9a),
        node_fill: Color32::from_rgb(0x1c, 0x1c, 0x1e),
        node_stroke: Color32::from_rgb(0x5a, 0x58, 0x55),
        secondary_fill: Color32::from_rgb(0x12, 0x12, 0x13),
        note_fill: Color32::from_rgb(0x2e, 0x29, 0x1a),
        accent: Color32::from_rgb(0x7a, 0xa2, 0xd6),
    }
}

/// More pixels a side than any diagram here has.
const ANY_SIDE: usize = 1 << 20;

fn render(source: &str, colors: Option<&MermaidColors>) -> egui::ColorImage {
    let renderer = CommonMarkCache::default().mermaid_renderer;
    render_mermaid_image(&renderer, source, colors, ANY_SIDE)
        .unwrap()
        .image
}

/// The color most of the fully opaque pixels have.
fn dominant_opaque_color(image: &egui::ColorImage) -> Color32 {
    let mut counts = HashMap::<Color32, usize>::new();
    for pixel in image.pixels.iter().filter(|pixel| pixel.a() == 255) {
        *counts.entry(*pixel).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .unwrap()
}

#[test]
fn without_colors_a_diagram_keeps_the_default_theme_on_white() {
    let image = render(FLOWCHART, None);
    assert_eq!(image.pixels[0], Color32::WHITE);
    assert!(image.pixels.contains(&Color32::from_rgb(0xec, 0xec, 0xff)));
}

#[test]
fn with_colors_a_diagram_is_transparent_and_drawn_in_them() {
    let colors = night();
    let image = render(FLOWCHART, Some(&colors));
    assert_eq!(image.pixels[0], Color32::TRANSPARENT);
    // Nodes are the only large opaque areas.
    assert_eq!(dominant_opaque_color(&image), colors.node_fill);
    assert!(image.pixels.contains(&colors.line));
    // Nothing of the light default theme is left on the black page.
    let brightest = image
        .pixels
        .iter()
        .filter(|pixel| pixel.a() == 255)
        .map(|pixel| pixel.r().max(pixel.g()).max(pixel.b()))
        .max();
    assert!(brightest <= Some(colors.text.r()), "{brightest:?}");
}

#[test]
fn colors_do_not_change_the_size_of_a_diagram() {
    let plain = render(FLOWCHART, None);
    let themed = render(FLOWCHART, Some(&night()));
    assert_eq!(plain.size, themed.size);
}

#[test]
fn a_diagram_merman_cannot_parse_reports_the_error() {
    let renderer = CommonMarkCache::default().mermaid_renderer;
    let colors = night();
    assert!(render_mermaid_image(&renderer, "flowchart LR\n  A[", Some(&colors), ANY_SIDE).is_err());
    assert!(render_mermaid_image(&renderer, "not a diagram", Some(&colors), ANY_SIDE).is_err());
    assert!(render_mermaid_image(&renderer, "not a diagram", None, ANY_SIDE).is_err());
}

#[test]
fn a_diagram_is_rasterized_at_twice_its_size_when_the_texture_has_room() {
    let renderer = CommonMarkCache::default().mermaid_renderer;
    let rendered = render_mermaid_image(&renderer, FLOWCHART, None, ANY_SIDE).unwrap();

    let [width, height] = rendered.image.size;
    assert_eq!(width, (rendered.size.x * 2.0) as usize);
    assert_eq!(height, (rendered.size.y * 2.0) as usize);
}

#[test]
fn a_diagram_larger_than_the_biggest_texture_is_rasterized_smaller() {
    let renderer = CommonMarkCache::default().mermaid_renderer;
    let whole = render_mermaid_image(&renderer, FLOWCHART, None, ANY_SIDE).unwrap();
    let longest = whole.image.size[0].max(whole.image.size[1]);

    for max_side in [longest - 1, longest / 2, 64, 1] {
        let fitted = render_mermaid_image(&renderer, FLOWCHART, None, max_side).unwrap();

        let [width, height] = fitted.image.size;
        assert_eq!(width.max(height), max_side, "it fills the texture");
        assert_eq!(fitted.size, whole.size, "and is laid out at the same size");
    }
}
