//! The colors diagrams inside a document are drawn in.

use eframe::egui::{self, Color32};
use egui_commonmark_extended::MermaidColors;

#[cfg(test)]
mod tests;

/// Lowest contrast against the page at which page-colored text stays
/// readable on an accent fill.
const MIN_ACCENT_CONTRAST: f32 = 3.0;

/// Diagram colors for a page painted with `visuals`.
///
/// A diagram is drawn as ink on the page: every fill and stroke sits on the
/// line from the page color to the text color, so it suits any theme, warm
/// or dark, without a palette of its own. Only `accent` carries a hue.
pub(crate) fn diagram_colors(visuals: &egui::Visuals) -> MermaidColors {
    let page = visuals.panel_fill;
    let text = visuals.text_color();
    let toward_text = |share: f32| page.lerp_to_gamma(text, share);
    MermaidColors {
        page,
        text,
        line: toward_text(0.75),
        node_fill: toward_text(0.06),
        node_stroke: toward_text(0.36),
        secondary_fill: toward_text(0.03),
        note_fill: toward_text(0.12),
        accent: accent(visuals.hyperlink_color, page, text),
    }
}

/// The link color, pulled toward the text color as far as it takes to carry
/// the page-colored labels a diagram writes on accent fills.
fn accent(link: Color32, page: Color32, text: Color32) -> Color32 {
    [0.0, 0.25, 0.5, 0.75]
        .into_iter()
        .map(|share| link.lerp_to_gamma(text, share))
        .find(|candidate| contrast(*candidate, page) >= MIN_ACCENT_CONTRAST)
        .unwrap_or(text)
}

/// WCAG 2 contrast ratio of two opaque colors, from 1.0 to 21.0.
fn contrast(a: Color32, b: Color32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// WCAG 2 relative luminance of an opaque color.
fn luminance(color: Color32) -> f32 {
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.0722f32.mul_add(
        linear(color.b()),
        0.2126f32.mul_add(linear(color.r()), 0.7152 * linear(color.g())),
    )
}
