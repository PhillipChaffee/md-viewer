//! Drawing mermaid diagrams in an application's own colors.
//!
//! merman (the mermaid renderer) draws every diagram in mermaid's default
//! light theme. Its theme support is partial: flowcharts honor
//! `themeVariables`, sequence diagrams and most other kinds ignore them
//! (probed against merman 0.3.0, see `tests.rs`). So the colors are replaced
//! in the finished SVG instead, in three steps:
//!
//! 1. usvg resolves the SVG's stylesheet, presentation attributes and
//!    inheritance and writes the result back as plain elements that carry
//!    their final `fill`/`stroke` as `#rrggbb` attributes.
//! 2. Every one of those colors is replaced. What it becomes depends on how
//!    it is used (text, fill or stroke) and on which role the original color
//!    plays in mermaid's default theme.
//! 3. The caller rasterizes the recolored SVG.
//!
//! No color is left untouched, so a diagram kind that has no entry in the
//! role table still comes out in the tone of the page.

use egui::Color32;

#[cfg(all(test, feature = "mermaid"))]
mod tests;

/// The colors an application wants mermaid diagrams drawn in.
///
/// Diagrams are rasterized with a transparent background, so `page` is never
/// painted as a backdrop. It tells the renderer what the diagram will sit on.
///
/// For legible diagrams, `text` and `line` need a contrast of at least 4.5:1
/// against `page`, `node_fill`, `secondary_fill` and `note_fill`, and
/// `accent` needs to stand out from `page`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MermaidColors {
    /// The surface the diagram is shown on. Used for label backings that
    /// must hide the edge they sit on, for interiors mermaid paints white,
    /// and for text on `accent` fills.
    pub page: Color32,
    /// Labels and titles.
    pub text: Color32,
    /// Edges, arrowheads, messages and other dark strokes.
    pub line: Color32,
    /// Flowchart nodes, sequence actors, classes, states and entities.
    pub node_fill: Color32,
    /// Borders of nodes, actors, clusters and notes, lifelines and grids.
    pub node_stroke: Color32,
    /// Subgraphs, activation bars, alternate rows and other quiet fills.
    pub secondary_fill: Color32,
    /// Notes.
    pub note_fill: Color32,
    /// Emphasized fills, such as gantt task bars.
    pub accent: Color32,
}

/// How an element uses a color.
#[cfg(feature = "mermaid")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paint {
    Text,
    Fill,
    Stroke,
}

/// The job a color does in mermaid's default theme.
#[cfg(feature = "mermaid")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    NodeFill,
    NodeStroke,
    SecondaryFill,
    NoteFill,
    Accent,
    AccentSoft,
}

#[cfg(feature = "mermaid")]
type Rgb = [u8; 3];

/// Label backings are drawn at least this opaque, so the edge under a label
/// does not strike through its text.
#[cfg(feature = "mermaid")]
const BACKING_OPACITY: f32 = 0.85;

/// The role of a color of mermaid's default theme, or `None` for any other
/// color. Keys are the `#rrggbb` values usvg resolves the theme's CSS to.
#[cfg(feature = "mermaid")]
fn default_theme_role(rgb: Rgb) -> Option<Role> {
    Some(match rgb {
        // mainBkg: nodes, actors, classes, states, entities
        [0xec, 0xec, 0xff] => Role::NodeFill,
        // nodeBorder, actorBorder, clusterBorder / noteBorderColor,
        // taskBorderColor
        [0x93, 0x70, 0xdb] | [0xda, 0xce, 0xf3] | [0xaa, 0xaa, 0x33] | [0x53, 0x4f, 0xbc] => {
            Role::NodeStroke
        }
        // clusterBkg
        [0xff, 0xff, 0xde] => Role::SecondaryFill,
        // noteBkgColor
        [0xff, 0xf5, 0xad] => Role::NoteFill,
        // taskBkgColor, sectionBkgColor
        [0x8a, 0x90, 0xdd] | [0x66, 0x66, 0xff] => Role::Accent,
        // activeTaskBkgColor
        [0xbf, 0xc7, 0xff] => Role::AccentSoft,
        _ => return None,
    })
}

#[cfg(feature = "mermaid")]
impl MermaidColors {
    fn role(&self, role: Role) -> Rgb {
        match role {
            Role::NodeFill => rgb(self.node_fill),
            Role::NodeStroke => rgb(self.node_stroke),
            Role::SecondaryFill => rgb(self.secondary_fill),
            Role::NoteFill => rgb(self.note_fill),
            Role::Accent => rgb(self.accent),
            Role::AccentSoft => mix(rgb(self.accent), rgb(self.page), 0.5),
        }
    }

    /// The color that replaces `original`, a color of the default theme or
    /// one the diagram's author chose.
    ///
    /// `use_roles` is false for charts whose colors tell data series apart:
    /// there the theme's fills must stay distinct from each other.
    fn themed(&self, original: Rgb, paint: Paint, use_roles: bool) -> Rgb {
        if paint == Paint::Text {
            // Light text was placed on a dark or saturated fill.
            return if luma(original) > 0.6 {
                rgb(self.page)
            } else {
                rgb(self.text)
            };
        }
        if let Some(role) = default_theme_role(original).filter(|_| use_roles) {
            return self.role(role);
        }
        let ink = inkiness(original);
        if !is_neutral(original) {
            return self.retoned(original, ink);
        }
        match paint {
            _ if ink >= 0.5 => rgb(self.line),
            Paint::Stroke => rgb(self.node_stroke),
            _ if ink < 0.02 => rgb(self.page),
            _ => rgb(self.secondary_fill),
        }
    }

    /// A color with no role, moved to where it sat between paper and ink in
    /// the default theme while keeping its tint.
    fn retoned(&self, original: Rgb, ink: f32) -> Rgb {
        let base = mix(rgb(self.page), rgb(self.text), ink);
        let gray = luma(original) * 255.0;
        std::array::from_fn(|i| {
            let tint = f32::from(original[i]) - gray;
            (f32::from(base[i]) + tint).round().clamp(0.0, 255.0) as u8
        })
    }
}

#[cfg(feature = "mermaid")]
fn rgb(color: Color32) -> Rgb {
    [color.r(), color.g(), color.b()]
}

#[cfg(feature = "mermaid")]
fn mix(from: Rgb, to: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| {
        let (a, b) = (f32::from(from[i]), f32::from(to[i]));
        (a + (b - a) * t).round() as u8
    })
}

/// Perceived brightness in `0.0..=1.0`.
#[cfg(feature = "mermaid")]
fn luma(rgb: Rgb) -> f32 {
    (0.2126 * f32::from(rgb[0]) + 0.7152 * f32::from(rgb[1]) + 0.0722 * f32::from(rgb[2])) / 255.0
}

/// Where a color sits between the default theme's paper (white, 0.0) and its
/// ink (`#333`, 1.0).
#[cfg(feature = "mermaid")]
fn inkiness(rgb: Rgb) -> f32 {
    ((1.0 - luma(rgb)) / 0.8).clamp(0.0, 1.0)
}

/// Whether a color is a gray.
#[cfg(feature = "mermaid")]
fn is_neutral(rgb: Rgb) -> bool {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    max - min <= 16
}

#[cfg(feature = "mermaid")]
fn parse_hex(value: &str) -> Option<Rgb> {
    let hex = value.strip_prefix('#').filter(|hex| hex.len() == 6)?;
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Recolor a merman SVG to `colors` and drop its background.
///
/// Returns `None` when usvg cannot parse the SVG.
#[cfg(feature = "mermaid")]
pub(crate) fn themed_svg(
    svg: &str,
    colors: &MermaidColors,
    options: &resvg::usvg::Options,
) -> Option<String> {
    let use_roles = !is_data_series_chart(svg);
    let tree = resvg::usvg::Tree::from_str(&without_root_background(svg), options).ok()?;
    let resolved = tree.to_string(&resvg::usvg::WriteOptions {
        // Keep `<text>` elements: they are how text colors are told apart
        // from shape fills.
        preserve_text: true,
        ..Default::default()
    });
    Some(recolor(&resolved, colors, use_roles))
}

/// The root `<svg ...>` tag of a document.
#[cfg(feature = "mermaid")]
fn root_tag(svg: &str) -> Option<std::ops::Range<usize>> {
    let start = svg.find("<svg")?;
    Some(start..start + tag_len(&svg[start..]))
}

/// Pie charts color each slice from the theme's fill palette.
#[cfg(feature = "mermaid")]
fn is_data_series_chart(svg: &str) -> bool {
    root_tag(svg).is_some_and(|tag| svg[tag].contains("aria-roledescription=\"pie\""))
}

/// merman sets `background-color: white` on the root element, which usvg
/// paints as an opaque backdrop. Remove it so the page shows through.
#[cfg(feature = "mermaid")]
fn without_root_background(svg: &str) -> std::borrow::Cow<'_, str> {
    const PROPERTY: &str = "background-color:";
    let Some(tag) = root_tag(svg) else {
        return svg.into();
    };
    let Some(start) = svg[tag.clone()].find(PROPERTY).map(|at| tag.start + at) else {
        return svg.into();
    };
    let value = &svg[start + PROPERTY.len()..tag.end];
    let value_len = value.find([';', '"']).unwrap_or(value.len());
    let end = start + PROPERTY.len() + value_len;
    let end = end + usize::from(svg[end..].starts_with(';'));
    format!("{}{}", &svg[..start], &svg[end..]).into()
}

/// Length of the tag at the start of `xml`, up to and including its `>`.
/// A `>` inside a quoted attribute value does not end the tag.
#[cfg(feature = "mermaid")]
fn tag_len(xml: &str) -> usize {
    let mut quoted = false;
    for (at, byte) in xml.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'>' if !quoted => return at + 1,
            _ => {}
        }
    }
    xml.len()
}

/// Replace every color in an SVG written by usvg.
#[cfg(feature = "mermaid")]
fn recolor(svg: &str, colors: &MermaidColors, use_roles: bool) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    let mut in_text = false;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let (tag, after) = rest[start..].split_at(tag_len(&rest[start..]));
        if tag.starts_with("</text") {
            in_text = false;
        } else if tag.starts_with("<text") {
            in_text = true;
        }
        match Element::parse(tag) {
            Some(element) => element.write_recolored(colors, in_text, use_roles, &mut out),
            None => out.push_str(tag),
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A start tag split into its parts. Values are kept as written.
#[cfg(feature = "mermaid")]
struct Element<'a> {
    name: &'a str,
    attributes: Vec<(&'a str, &'a str)>,
    /// `>` or `/>`.
    close: &'a str,
}

#[cfg(feature = "mermaid")]
impl<'a> Element<'a> {
    /// Parse a start tag whose attribute values are all double-quoted, as
    /// usvg writes them. Anything else (end tags, comments, declarations)
    /// is `None`.
    fn parse(tag: &'a str) -> Option<Self> {
        let body = tag.strip_prefix('<')?;
        let (body, close) = match body.strip_suffix("/>") {
            Some(body) => (body, "/>"),
            None => (body.strip_suffix('>')?, ">"),
        };
        let name_len = body
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(body.len());
        let (name, mut rest) = body.split_at(name_len);
        if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return None;
        }
        let mut attributes = Vec::new();
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            let (key, value_and_rest) = rest.split_once("=\"")?;
            let (value, after) = value_and_rest.split_once('"')?;
            attributes.push((key, value));
            rest = after;
        }
        Some(Self {
            name,
            attributes,
            close,
        })
    }

    fn attribute(&self, name: &str) -> Option<&'a str> {
        self.attributes
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
    }

    /// Whether this shape is a see-through light patch behind a label.
    fn is_label_backing(&self) -> bool {
        let opacity = self
            .attribute("fill-opacity")
            .and_then(|value| value.parse::<f32>().ok());
        let fill = self.attribute("fill").and_then(parse_hex);
        matches!((fill, opacity), (Some(fill), Some(opacity)) if opacity < 1.0 && luma(fill) > 0.85)
    }

    fn write_recolored(
        &self,
        colors: &MermaidColors,
        in_text: bool,
        use_roles: bool,
        out: &mut String,
    ) {
        use std::fmt::Write as _;

        let backing = !in_text && self.is_label_backing();
        let paint = |shape_paint| if in_text { Paint::Text } else { shape_paint };
        out.push('<');
        out.push_str(self.name);
        for (key, value) in &self.attributes {
            let themed = match (*key, parse_hex(value)) {
                ("fill", Some(_)) if backing => Some(rgb(colors.page)),
                ("fill" | "stop-color", Some(original)) => {
                    Some(colors.themed(original, paint(Paint::Fill), use_roles))
                }
                ("stroke", Some(original)) => {
                    Some(colors.themed(original, paint(Paint::Stroke), use_roles))
                }
                _ => None,
            };
            let _ = match themed {
                Some([r, g, b]) => write!(out, " {key}=\"#{r:02x}{g:02x}{b:02x}\""),
                None if backing && *key == "fill-opacity" => {
                    let opacity = value.parse::<f32>().unwrap_or(1.0).max(BACKING_OPACITY);
                    write!(out, " {key}=\"{opacity}\"")
                }
                None => write!(out, " {key}=\"{value}\""),
            };
        }
        out.push_str(self.close);
    }
}
