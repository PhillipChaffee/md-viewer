//! What the document pane renders with, as opposed to the chrome around it:
//! its fonts and the styling of its blocks.

use std::sync::Arc;

use eframe::egui::{self, TextStyle};
use egui_commonmark_extended::{BlockStyle, CodeBlockStyle};

use crate::system_fonts::DocumentFonts;
use crate::theme::Palette;

#[cfg(test)]
mod tests;

/// Line height of headings as a multiple of their size: tight, so a
/// title that wraps reads as one block.
const HEADING_LINE_HEIGHT: f32 = 1.22;
const CODE_BLOCK_RADIUS: u8 = 8;
/// Space between a code block's box and its code.
const CODE_BLOCK_PADDING: i8 = 16;

/// The document pane's look for one theme and one font configuration. It is
/// built when either changes, never while rendering.
#[derive(Clone, Debug)]
pub(crate) struct DocumentLook {
    /// The app's style with the document's fonts in place of the chrome's.
    pub(crate) style: Arc<egui::Style>,
    /// Block styling handed to the renderer.
    pub(crate) blocks: BlockStyle,
    /// The syntect theme code blocks are highlighted with.
    pub(crate) syntax_theme: &'static str,
    /// Color of the meta line above the document.
    pub(crate) muted: egui::Color32,
}

impl DocumentLook {
    /// `chrome` is the app's current style, with the theme's visuals already
    /// in it.
    pub(crate) fn new(chrome: &egui::Style, palette: &Palette, fonts: &DocumentFonts) -> Self {
        let mut style = chrome.clone();
        style.text_styles.extend([
            (TextStyle::Body, fonts.body.clone()),
            (TextStyle::Heading, fonts.heading.clone()),
            (TextStyle::Monospace, fonts.code.clone()),
        ]);
        Self {
            style: Arc::new(style),
            blocks: BlockStyle {
                heading_font_family: Some(fonts.heading.family.clone()),
                emphasis_font_family: fonts.emphasis.clone(),
                heading_line_height: Some(HEADING_LINE_HEIGHT),
                inline_code_scale: fonts.inline_code_scale,
                task_marker: Some(palette.muted),
                code_block: Some(CodeBlockStyle {
                    fill: palette.raised,
                    stroke: egui::Stroke::NONE,
                    corner_radius: CODE_BLOCK_RADIUS,
                    padding: CODE_BLOCK_PADDING,
                }),
                blockquote_bar: Some(palette.rule),
                syntax_by_language_name: true,
                short_rule: true,
                center_content: true,
                table: None,
                list: None,
                rhythm: None,
                figure_max_width: None,
                inline_code_radius: None,
            },
            syntax_theme: palette.syntax_theme,
            muted: palette.muted,
        }
    }
}
