//! What the last frame painted: the color under a point and the ink of a
//! piece of text, read from the frame's own drawing commands.

use std::collections::HashMap;
use std::fmt;

use eframe::egui::epaint::{
    textures::TexturesDelta, ClippedPrimitive, ImageData, Primitive, TextShape, Vertex, WHITE_UV,
};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Shape, TextureId};

/// An opaque color, as the hex code a designer would write.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rgb(pub(crate) u8, pub(crate) u8, pub(crate) u8);

impl Rgb {
    /// Read `#RRGGBB`.
    #[track_caller]
    pub(crate) fn hex(code: &str) -> Self {
        let digits = code.strip_prefix('#').unwrap();
        let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).unwrap();
        Self(channel(0), channel(2), channel(4))
    }

    fn of(color: Color32) -> Self {
        Self(color.r(), color.g(), color.b())
    }

    /// WCAG relative luminance: 0 is black, 1 is white.
    pub(crate) fn luminance(self) -> f64 {
        let linear = |channel: u8| {
            let level = f64::from(channel) / 255.0;
            if level <= 0.040_45 {
                level / 12.92
            } else {
                ((level + 0.055) / 1.055).powf(2.4)
            }
        };
        0.0722_f64.mul_add(
            linear(self.2),
            0.2126_f64.mul_add(linear(self.0), 0.7152 * linear(self.1)),
        )
    }

    /// WCAG contrast ratio against `other`, from 1 to 21.
    pub(crate) fn contrast(self, other: Self) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

impl fmt::Debug for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

/// How much of `point` each corner of the triangle owns, or `None` when the
/// point is outside it.
fn weights(point: Pos2, corners: [Pos2; 3]) -> Option<[f32; 3]> {
    let [a, b, c] = corners;
    let area = (b - a).x.mul_add((c - a).y, -((b - a).y * (c - a).x));
    if area.abs() <= f32::EPSILON {
        return None;
    }
    let toward_b = (point - a)
        .x
        .mul_add((c - a).y, -((point - a).y * (c - a).x))
        / area;
    let toward_c = (b - a)
        .x
        .mul_add((point - a).y, -((b - a).y * (point - a).x))
        / area;
    let own = 1.0 - toward_b - toward_c;
    [own, toward_b, toward_c]
        .iter()
        .all(|weight| *weight >= 0.0)
        .then_some([own, toward_b, toward_c])
}

/// Lay a triangle's color at `point` over `under`. Colors are
/// premultiplied, so a channel is `over + under * (1 - alpha)`.
fn laid_over(under: [f32; 4], corners: [&Vertex; 3], share: [f32; 3]) -> [f32; 4] {
    let mut over = [0.0_f32; 4];
    for (vertex, weight) in corners.iter().zip(share) {
        for (channel, level) in over.iter_mut().zip(vertex.color.to_array()) {
            *channel = f32::from(level).mul_add(weight, *channel);
        }
    }
    let keep = 1.0 - over[3] / 255.0;
    [0, 1, 2, 3].map(|channel| under[channel].mul_add(keep, over[channel]))
}

/// The flat color the frame left at `point`, in window points. Text and
/// images are skipped: this is the surface they sit on.
pub(crate) fn color_at(frame: &[ClippedPrimitive], point: Pos2) -> Rgb {
    let mut color = [0.0_f32; 4];
    for clipped in frame {
        let Primitive::Mesh(mesh) = &clipped.primitive else {
            continue;
        };
        if !clipped.clip_rect.contains(point) || mesh.texture_id != TextureId::default() {
            continue;
        }
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let corners = triangle.map(|corner| &mesh.vertices[corner as usize]);
            if corners.iter().any(|vertex| vertex.uv != WHITE_UV) {
                continue;
            }
            if let Some(share) = weights(point, corners.map(|vertex| vertex.pos)) {
                color = laid_over(color, corners, share);
            }
        }
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "each channel is a blend of bytes, so it rounds to a byte"
    )]
    let byte = |level: f32| level.round().clamp(0.0, 255.0) as u8;
    Rgb(byte(color[0]), byte(color[1]), byte(color[2]))
}

/// A text the frame painted.
pub(crate) struct Painted {
    pub(crate) text: String,
    /// The color of its first glyph.
    pub(crate) ink: Rgb,
    /// Every color its glyphs are set in, in reading order, each once.
    pub(crate) inks: Vec<Rgb>,
    /// Whether a line is drawn through any of it.
    pub(crate) struck_out: bool,
    /// Whether a line is drawn under any of it.
    pub(crate) underlined: bool,
    /// The color laid behind its first glyph, when it is marked.
    pub(crate) marker: Option<Rgb>,
    /// The middle of its first line, where a person would click on it. A
    /// text that wraps has its own middle off its words.
    pub(crate) first_line: Pos2,
}

/// The middle of the glyphs on the first line of `text`.
fn first_line(text: &TextShape) -> Pos2 {
    let line = text.galley.rows.first();
    let ends =
        line.and_then(|line| Some((line, line.row.glyphs.first()?, line.row.glyphs.last()?)));
    ends.map_or(text.pos, |(line, first, last)| {
        let across = (first.pos.x + last.pos.x + last.advance_width) / 2.0;
        text.pos + line.pos.to_vec2() + egui::vec2(across, line.row.size.y / 2.0)
    })
}

/// Every text the frame painted, in paint order.
pub(crate) fn painted_text(shapes: &[Shape]) -> Vec<Painted> {
    let mut texts = Vec::new();
    for shape in shapes {
        match shape {
            Shape::Vec(members) => texts.extend(painted_text(members)),
            Shape::Text(text) => {
                let section = text.galley.job.sections.first();
                let own = section.map_or(Color32::PLACEHOLDER, |section| section.format.color);
                let ink = text
                    .override_text_color
                    .unwrap_or(if own == Color32::PLACEHOLDER {
                        text.fallback_color
                    } else {
                        own
                    });
                let sections = &text.galley.job.sections;
                let mut inks: Vec<Rgb> = Vec::new();
                for color in sections.iter().map(|section| section.format.color) {
                    let ink = Rgb::of(if color == Color32::PLACEHOLDER {
                        ink
                    } else {
                        color
                    });
                    if !inks.contains(&ink) {
                        inks.push(ink);
                    }
                }
                let struck = |section: &egui::text::LayoutSection| {
                    section.format.strikethrough != egui::Stroke::NONE
                };
                let underlined = |section: &egui::text::LayoutSection| {
                    section.format.underline != egui::Stroke::NONE
                };
                let marker = section
                    .map(|section| section.format.background)
                    .filter(|behind| *behind != Color32::TRANSPARENT);
                texts.push(Painted {
                    text: text.galley.text().to_owned(),
                    ink: Rgb::of(ink),
                    inks,
                    struck_out: sections.iter().any(struck),
                    underlined: sections.iter().any(underlined),
                    marker: marker.map(Rgb::of),
                    first_line: first_line(text),
                });
            }
            _ => {}
        }
    }
    texts
}

/// A picture the frame painted: where, and which pixels.
#[derive(Clone, Copy)]
pub(crate) struct Picture {
    pub(crate) rect: Rect,
    texture: TextureId,
}

/// Where the frame painted pictures: images, formulas and diagrams, which
/// reach the screen as textures and not as text.
pub(crate) fn pictures(shapes: &[Shape]) -> Vec<Picture> {
    let mut found = Vec::new();
    for shape in shapes {
        match shape {
            Shape::Vec(members) => found.extend(pictures(members)),
            Shape::Rect(rect) => found.extend(rect.brush.as_ref().map(|brush| Picture {
                rect: rect.rect,
                texture: brush.fill_texture_id,
            })),
            Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => {
                found.push(Picture {
                    rect: mesh.calc_bounds(),
                    texture: mesh.texture_id,
                });
            }
            _ => {}
        }
    }
    found
}

/// The pixels of every picture the app has handed to the screen and not
/// taken back.
#[derive(Default)]
pub(crate) struct Pixels(HashMap<TextureId, ColorImage>);

impl Pixels {
    /// Keep up with the pictures one frame set, changed or dropped.
    pub(crate) fn note(&mut self, frame: &TexturesDelta) {
        for (id, change) in &frame.set {
            let ImageData::Color(image) = &change.image;
            // The font atlas is patched glyph by glyph; a picture arrives
            // whole.
            if change.pos.is_none() {
                self.0.insert(*id, ColorImage::clone(image.as_ref()));
            }
        }
        for id in &frame.free {
            self.0.remove(id);
        }
    }

    /// The color of each pixel of `picture` that is painted at all, with
    /// how strongly it is painted, from 0 to 1.
    fn painted(&self, picture: Picture) -> impl Iterator<Item = (Rgb, f64)> + '_ {
        let pixels = self.0.get(&picture.texture).map(|image| &image.pixels);
        pixels.into_iter().flatten().filter_map(|pixel| {
            let strength = f64::from(pixel.a()) / 255.0;
            // Stored premultiplied: divide the strength back out.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a channel divided by its own alpha is at most a byte"
            )]
            let whole = |channel: u8| (f64::from(channel) / strength).round().min(255.0) as u8;
            (pixel.a() > 0).then(|| {
                let color = Rgb(whole(pixel.r()), whole(pixel.g()), whole(pixel.b()));
                (color, strength)
            })
        })
    }

    /// The ink of `picture`: the average of the colors it paints, each
    /// weighed by how strongly it is painted. `None` for a picture that
    /// paints nothing.
    pub(crate) fn ink(&self, picture: Picture) -> Option<Rgb> {
        let mut sum = [0.0_f64; 4];
        for (Rgb(red, green, blue), strength) in self.painted(picture) {
            for (total, channel) in sum.iter_mut().zip([red, green, blue]) {
                *total = f64::from(channel).mul_add(strength, *total);
            }
            sum[3] += strength;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an average of bytes is a byte"
        )]
        let mean = |total: f64| (total / sum[3]).round() as u8;
        (sum[3] > 0.0).then(|| Rgb(mean(sum[0]), mean(sum[1]), mean(sum[2])))
    }

    /// How much of the rectangle of `picture` is painted, from 0 for an
    /// empty sheet to 1 for a filled box.
    pub(crate) fn cover(&self, picture: Picture) -> f64 {
        let image = self.0.get(&picture.texture);
        let all = image.map_or(0, |image| image.pixels.len());
        let painted: f64 = self.painted(picture).map(|(_, strength)| strength).sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a picture has far fewer pixels than f64 counts exactly"
        )]
        let all = all.max(1) as f64;
        painted / all
    }
}

/// Where the frame drew lines, boxes and dots: everything that is neither
/// text nor a picture, each by the rectangle that bounds it.
pub(crate) fn marks(shapes: &[Shape]) -> Vec<Rect> {
    let mut found = Vec::new();
    for shape in shapes {
        match shape {
            Shape::Vec(members) => found.extend(marks(members)),
            Shape::Noop | Shape::Text(_) | Shape::Mesh(_) | Shape::Callback(_) => {}
            Shape::Rect(rect) if rect.brush.is_some() => {}
            drawn => found.push(drawn.visual_bounding_rect()),
        }
    }
    found.retain(Rect::is_positive);
    found
}
