use super::*;
use std::collections::BTreeSet;

/// A dark page with light ink. Every color is distinct so a test can tell
/// which role a mapped color came from.
fn dusk() -> MermaidColors {
    MermaidColors {
        page: Color32::from_rgb(0x2c, 0x2c, 0x2e),
        text: Color32::from_rgb(0xda, 0xd7, 0xd0),
        line: Color32::from_rgb(0xae, 0xab, 0xa5),
        node_fill: Color32::from_rgb(0x3a, 0x3a, 0x3c),
        node_stroke: Color32::from_rgb(0x6e, 0x6c, 0x69),
        secondary_fill: Color32::from_rgb(0x33, 0x33, 0x35),
        note_fill: Color32::from_rgb(0x4a, 0x44, 0x30),
        accent: Color32::from_rgb(0x7a, 0xa2, 0xd6),
    }
}

/// A warm light page with dark ink.
fn sepia() -> MermaidColors {
    MermaidColors {
        page: Color32::from_rgb(0xf8, 0xf1, 0xe3),
        text: Color32::from_rgb(0x2b, 0x26, 0x22),
        line: Color32::from_rgb(0x5e, 0x57, 0x50),
        node_fill: Color32::from_rgb(0xf0, 0xe7, 0xd3),
        node_stroke: Color32::from_rgb(0xb5, 0xa9, 0x94),
        secondary_fill: Color32::from_rgb(0xf4, 0xec, 0xdb),
        note_fill: Color32::from_rgb(0xf3, 0xe3, 0xb3),
        accent: Color32::from_rgb(0x9a, 0x5b, 0x2e),
    }
}

fn themed(colors: &MermaidColors, original: &str, paint: Paint) -> Color32 {
    let [r, g, b] = colors.themed(parse_hex(original).unwrap(), paint, true);
    Color32::from_rgb(r, g, b)
}

fn usvg_options() -> resvg::usvg::Options<'static> {
    resvg::usvg::Options::default()
}

fn merman() -> merman::render::HeadlessRenderer {
    merman::render::HeadlessRenderer::new()
}

fn merman_svg(source: &str) -> String {
    merman().render_svg_readable_sync(source).unwrap().unwrap()
}

/// Every `#rrggbb` an element of `svg` is filled or stroked with.
fn paint_colors(svg: &str) -> BTreeSet<Rgb> {
    let mut found = BTreeSet::new();
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        let (tag, after) = rest[start..].split_at(tag_len(&rest[start..]));
        rest = after;
        let Some(element) = Element::parse(tag) else {
            continue;
        };
        for key in ["fill", "stroke"] {
            found.extend(element.attribute(key).and_then(parse_hex));
        }
    }
    found
}

fn palette(colors: &MermaidColors) -> BTreeSet<Rgb> {
    [
        colors.page,
        colors.text,
        colors.line,
        colors.node_fill,
        colors.node_stroke,
        colors.secondary_fill,
        colors.note_fill,
        colors.accent,
    ]
    .map(rgb)
    .into()
}

const FLOWCHART: &str = "flowchart LR
    A[Prompt] --> B{Compare}
    B -->|large delta| C[Candidates]
    subgraph Group
      C
    end";

const SEQUENCE: &str = "sequenceDiagram
    participant U as User
    participant M as Model
    U->>M: prompt
    activate M
    Note right of M: thinking
    M-->>U: refusal template
    deactivate M
    loop Every minute
        U->>M: ping
    end";

#[test]
fn default_theme_colors_take_the_role_they_play() {
    let c = dusk();
    assert_eq!(themed(&c, "#ececff", Paint::Fill), c.node_fill);
    assert_eq!(themed(&c, "#9370db", Paint::Stroke), c.node_stroke);
    assert_eq!(themed(&c, "#dacef3", Paint::Stroke), c.node_stroke);
    assert_eq!(themed(&c, "#aaaa33", Paint::Stroke), c.node_stroke);
    assert_eq!(themed(&c, "#ffffde", Paint::Fill), c.secondary_fill);
    assert_eq!(themed(&c, "#fff5ad", Paint::Fill), c.note_fill);
    assert_eq!(themed(&c, "#8a90dd", Paint::Fill), c.accent);
}

#[test]
fn an_active_gantt_task_is_a_softer_accent() {
    let c = sepia();
    let soft = themed(&c, "#bfc7ff", Paint::Fill);
    let between = |i: usize| {
        let (lo, hi) = (c.accent[i].min(c.page[i]), c.accent[i].max(c.page[i]));
        lo < soft[i] && soft[i] < hi
    };
    assert!((0..3).all(between), "{soft:?} is not between accent and page");
}

#[test]
fn grays_move_onto_the_page_to_ink_scale() {
    let c = dusk();
    // Dark grays are ink: edges, arrowheads, activation borders.
    assert_eq!(themed(&c, "#333333", Paint::Stroke), c.line);
    assert_eq!(themed(&c, "#000000", Paint::Fill), c.line);
    assert_eq!(themed(&c, "#666666", Paint::Stroke), c.line);
    // White is paper.
    assert_eq!(themed(&c, "#ffffff", Paint::Fill), c.page);
    // Light grays are quiet fills, or faint rules when stroked.
    assert_eq!(themed(&c, "#f4f4f4", Paint::Fill), c.secondary_fill);
    assert_eq!(themed(&c, "#d3d3d3", Paint::Stroke), c.node_stroke);
}

#[test]
fn text_is_the_text_color_whatever_it_was() {
    let c = dusk();
    assert_eq!(themed(&c, "#333333", Paint::Text), c.text);
    assert_eq!(themed(&c, "#000000", Paint::Text), c.text);
    // ER diagrams draw relationship labels in the node border color.
    assert_eq!(themed(&c, "#9370db", Paint::Text), c.text);
    // White text sits on a task bar or a sequence number.
    assert_eq!(themed(&c, "#ffffff", Paint::Text), c.page);
}

#[test]
fn a_color_without_a_role_keeps_its_tint_and_follows_the_page() {
    // A pastel pink an author put on a node.
    let pink = "#ffccee";
    let on_dark = themed(&dusk(), pink, Paint::Fill);
    let on_light = themed(&sepia(), pink, Paint::Fill);
    // Close to the page in brightness on both, so the text stays legible...
    assert!(luma(rgb(on_dark)) < 0.4, "{on_dark:?}");
    assert!(luma(rgb(on_light)) > 0.75, "{on_light:?}");
    // ...and still pink: red strongest, green weakest.
    for mapped in [on_dark, on_light] {
        assert!(mapped.r() > mapped.b() && mapped.b() > mapped.g(), "{mapped:?}");
    }
}

#[test]
fn a_saturated_color_without_a_role_stays_recognizable() {
    let red = themed(&dusk(), "#ff0000", Paint::Fill);
    assert!(red.r() > red.g() + 60 && red.r() > red.b() + 60, "{red:?}");
}

#[test]
fn pie_slices_stay_distinct() {
    let c = sepia();
    let slices: BTreeSet<Rgb> = ["#ececff", "#ffffde", "#b5ff20"]
        .into_iter()
        .map(|slice| c.themed(parse_hex(slice).unwrap(), Paint::Fill, false))
        .collect();
    assert_eq!(slices.len(), 3);
    assert!(!slices.contains(&rgb(c.node_fill)));
}

#[test]
fn recolor_tells_text_from_shapes() {
    let c = dusk();
    let svg = concat!(
        r##"<svg width="10" height="10" xmlns="http://www.w3.org/2000/svg">"##,
        r##"<path fill="#333333" stroke="#333333" d="M 0 0 L 1 1"/>"##,
        r##"<text xml:space="preserve"><tspan fill="#333333" stroke="none">a &lt; b</tspan></text>"##,
        r##"<path fill="none" stroke="#9370db" d="M 0 0"/>"##,
        r##"</svg>"##,
    );
    let expected = concat!(
        r##"<svg width="10" height="10" xmlns="http://www.w3.org/2000/svg">"##,
        r##"<path fill="#aeaba5" stroke="#aeaba5" d="M 0 0 L 1 1"/>"##,
        r##"<text xml:space="preserve"><tspan fill="#dad7d0" stroke="none">a &lt; b</tspan></text>"##,
        r##"<path fill="none" stroke="#6e6c69" d="M 0 0"/>"##,
        r##"</svg>"##,
    );
    assert_eq!(recolor(svg, &c, true), expected);
}

#[test]
fn recolor_leaves_text_content_and_references_alone() {
    let c = dusk();
    let svg = concat!(
        r##"<!-- fill="#333333" -->"##,
        r##"<text><tspan fill="#333333">fill="#333333" > x</tspan></text>"##,
        r##"<path fill="url(#gradient1)" marker-end="url(#a>b)" stroke="none"/>"##,
    );
    let expected = concat!(
        r##"<!-- fill="#333333" -->"##,
        r##"<text><tspan fill="#dad7d0">fill="#333333" > x</tspan></text>"##,
        r##"<path fill="url(#gradient1)" marker-end="url(#a>b)" stroke="none"/>"##,
    );
    assert_eq!(recolor(svg, &c, true), expected);
}

#[test]
fn a_label_backing_becomes_the_page_and_hides_the_edge_under_it() {
    let c = sepia();
    let svg = r##"<path fill="#e8e8e8" fill-opacity="0.5019608" stroke="none" d="M 0 0"/>"##;
    assert_eq!(
        recolor(svg, &c, true),
        r##"<path fill="#f8f1e3" fill-opacity="0.85" stroke="none" d="M 0 0"/>"##
    );
    // An opaque light gray is an ordinary quiet fill.
    let opaque = r##"<path fill="#e8e8e8" d="M 0 0"/>"##;
    assert_eq!(
        recolor(opaque, &c, true),
        r##"<path fill="#f4ecdb" d="M 0 0"/>"##
    );
}

#[test]
fn only_the_root_background_is_removed() {
    let svg = concat!(
        r##"<svg id="m" style="max-width: 10px; background-color: white;" viewBox="0 0 1 1">"##,
        r##"<g style="background-color: white;"/></svg>"##,
    );
    assert_eq!(
        without_root_background(svg),
        concat!(
            r##"<svg id="m" style="max-width: 10px; " viewBox="0 0 1 1">"##,
            r##"<g style="background-color: white;"/></svg>"##,
        )
    );
    let plain = r##"<svg id="m" viewBox="0 0 1 1"><g style="background-color: white;"/></svg>"##;
    assert_eq!(without_root_background(plain), plain);
}

#[test]
fn a_themed_flowchart_uses_only_the_given_colors() {
    let c = dusk();
    let themed = themed_svg(&merman_svg(FLOWCHART), &c, &usvg_options()).unwrap();
    let used = paint_colors(&themed);
    let allowed = palette(&c);
    let stray: Vec<_> = used.difference(&allowed).collect();
    assert!(stray.is_empty(), "colors outside the palette: {stray:?}");
    for role in [c.node_fill, c.node_stroke, c.line, c.secondary_fill].map(rgb) {
        assert!(used.contains(&role), "{role:?} is unused in {used:?}");
    }
}

#[test]
fn a_themed_sequence_diagram_uses_only_the_given_colors() {
    let c = sepia();
    let themed = themed_svg(&merman_svg(SEQUENCE), &c, &usvg_options()).unwrap();
    let used = paint_colors(&themed);
    let allowed = palette(&c);
    let stray: Vec<_> = used.difference(&allowed).collect();
    assert!(stray.is_empty(), "colors outside the palette: {stray:?}");
    for role in [c.node_fill, c.node_stroke, c.line, c.note_fill].map(rgb) {
        assert!(used.contains(&role), "{role:?} is unused in {used:?}");
    }
}

#[test]
fn a_themed_diagram_has_no_backdrop() {
    let svg = merman_svg(FLOWCHART);
    let c = sepia();
    let themed = themed_svg(&svg, &c, &usvg_options()).unwrap();
    let tree = resvg::usvg::Tree::from_str(&themed, &usvg_options()).unwrap();
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(&tree, Default::default(), &mut pixmap.as_mut());
    assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 0);

    // The untouched SVG is what paints the white backdrop.
    let tree = resvg::usvg::Tree::from_str(&svg, &usvg_options()).unwrap();
    resvg::render(&tree, Default::default(), &mut pixmap.as_mut());
    assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 255);
}

#[test]
fn an_svg_that_does_not_parse_is_not_themed() {
    assert_eq!(themed_svg("<svg", &dusk(), &usvg_options()), None);
}

/// Why the colors are replaced in the SVG instead of being handed to merman:
/// merman 0.3 applies `themeVariables` to flowcharts but not to sequence
/// diagrams. If this starts failing, merman learned to theme them and the
/// recoloring may no longer be needed.
#[test]
fn merman_ignores_theme_variables_for_sequence_diagrams() {
    let mut config = merman::MermaidConfig::empty_object();
    config.set_value("theme", "base".into());
    config.set_value("themeVariables.primaryColor", "#112233".into());
    config.set_value("themeVariables.actorBkg", "#112233".into());
    let configured = merman().with_site_config(config.clone());
    let directive = "%%{init: {'theme':'base','themeVariables':{'primaryColor':'#112233'}}}%%\n";

    let render = |renderer: &merman::render::HeadlessRenderer, source: &str| {
        renderer.render_svg_readable_sync(source).unwrap().unwrap()
    };
    assert_eq!(render(&configured, SEQUENCE), merman_svg(SEQUENCE));
    assert_eq!(
        render(&merman(), &format!("{directive}{SEQUENCE}")),
        merman_svg(SEQUENCE)
    );
    // Flowcharts do pick the variable up, by either route.
    assert!(render(&configured, FLOWCHART).contains("#112233"));
    assert!(render(&merman(), &format!("{directive}{FLOWCHART}")).contains("#112233"));
}
