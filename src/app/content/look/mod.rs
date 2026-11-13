//! What the document pane renders with, as opposed to the chrome around it:
//! its fonts and the styling of its blocks.

use std::sync::Arc;

use eframe::egui::{self, TextStyle};
use egui_commonmark_extended::{BlockRhythm, BlockStyle, CodeBlockStyle, ListStyle, TableStyle};

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
/// Space above and below the text of a table cell.
const TABLE_CELL_PADDING: f32 = 9.0;
/// How far the rule under a table's header leans from the hairline color
/// toward the muted text color: enough to tell the header from the rows.
const TABLE_HEADER_RULE_WEIGHT: f32 = 0.45;
/// Vertical space between blocks, in multiples of the body font size.
const RHYTHM: BlockRhythm = BlockRhythm {
    paragraph: 0.9,
    block: 1.25,
    heading_above: 1.6,
    heading_below: 0.65,
    tight_item: 0.35,
    display_math: 0.35,
};
/// Corner radius of the chip behind inline code.
const INLINE_CODE_RADIUS: f32 = 4.0;
/// Radius of a list bullet in multiples of the body font size.
const BULLET_RADIUS: f32 = 0.125;
/// Width of the slot a list marker hangs in, in multiples of the body font
/// size.
const LIST_INDENT: f32 = 1.5;

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
                table: Some(TableStyle {
                    rule: egui::Stroke::new(1.0_f32, palette.rule),
                    header_rule: egui::Stroke::new(
                        1.0_f32,
                        palette
                            .rule
                            .lerp_to_gamma(palette.muted, TABLE_HEADER_RULE_WEIGHT),
                    ),
                    cell_padding: TABLE_CELL_PADDING,
                    fill_width: true,
                }),
                list: Some(ListStyle {
                    marker: palette.muted,
                    bullet_radius: BULLET_RADIUS,
                    indent: LIST_INDENT,
                }),
                rhythm: Some(RHYTHM),
                // The pane decides this each frame: see `preview_pane`.
                figure_max_width: None,
                inline_code_radius: Some(INLINE_CODE_RADIUS),
            },
            syntax_theme: palette.syntax_theme,
            muted: palette.muted,
        }
    }
}
