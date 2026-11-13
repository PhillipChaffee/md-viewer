//! Typography configuration for egui_commonmark
//!
//! Provides line height and spacing controls for improved readability.

/// Specifies a measurement that can be either a multiplier of font size or absolute pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Measurement {
    /// Multiplier relative to font size (e.g., 1.5 means 150% of font size)
    Multiplier(f32),
    /// Absolute pixel value
    Pixels(f32),
}

impl Measurement {
    /// Resolve the measurement to pixels given a font size
    pub fn resolve(&self, font_size: f32) -> f32 {
        match self {
            Measurement::Multiplier(m) => font_size * m,
            Measurement::Pixels(p) => *p,
        }
    }
}

impl Default for Measurement {
    fn default() -> Self {
        Measurement::Multiplier(1.0)
    }
}

/// Typography configuration for markdown rendering.
///
/// Controls line height, paragraph spacing, and heading spacing for improved readability.
/// Based on WCAG 2.1 SC 1.4.12 guidelines and peer-reviewed HCI research.
#[derive(Debug, Clone, Default)]
pub struct TypographyConfig {
    /// Line height for body text. Applied via egui's TextFormat.line_height.
    /// Research: 1.5× recommended (WCAG 2.1 SC 1.4.12, Reading University)
    /// Default: None (uses font's built-in line height)
    pub line_height: Option<Measurement>,

    /// Line height for code blocks (tighter than prose).
    /// Research: 1.2-1.4× recommended for code (PPIG Conference)
    /// Default: None (uses 1.3× when recommended() is called)
    pub code_line_height: Option<Measurement>,

    /// Extra spacing between paragraphs.
    /// Research: 2× font size recommended (WCAG 1.4.12)
    /// Default: None (uses default egui spacing)
    pub paragraph_spacing: Option<Measurement>,

    /// Extra spacing before headings.
    /// Research: More space above than below creates visual grouping
    /// Default: None
    pub heading_spacing_above: Option<Measurement>,

    /// Extra spacing after headings.
    /// Research: line-height × 0.5 ≈ space below heading
    /// Default: None
    pub heading_spacing_below: Option<Measurement>,
}

impl TypographyConfig {
    /// Create a new typography config with evidence-based defaults.
    ///
    /// Based on peer-reviewed HCI research, WCAG 2.1, and vision science:
    /// - Line height: 1.5× (WCAG 2.1 SC 1.4.12, Reading University)
    /// - Code line height: 1.3× (PPIG Conference research)
    /// - Paragraph spacing: 2.0× font size (WCAG 1.4.12)
    /// - Heading above: 2.0× font size
    /// - Heading below: 0.75× font size (line-height × 0.5)
    pub fn recommended() -> Self {
        Self {
            line_height: Some(Measurement::Multiplier(1.5)),
            code_line_height: Some(Measurement::Multiplier(1.3)),
            paragraph_spacing: Some(Measurement::Multiplier(2.0)),
            heading_spacing_above: Some(Measurement::Multiplier(2.0)),
            heading_spacing_below: Some(Measurement::Multiplier(0.75)),
        }
    }

    /// Check if any typography settings are configured
    pub fn is_configured(&self) -> bool {
        self.line_height.is_some()
            || self.code_line_height.is_some()
            || self.paragraph_spacing.is_some()
            || self.heading_spacing_above.is_some()
            || self.heading_spacing_below.is_some()
    }

    /// Resolve line height to pixels given a font size.
    /// Returns None if line_height is not configured.
    pub fn resolve_line_height(&self, font_size: f32) -> Option<f32> {
        self.line_height.map(|m| m.resolve(font_size))
    }

    /// Resolve code line height to pixels given a font size.
    /// Returns None if code_line_height is not configured.
    pub fn resolve_code_line_height(&self, font_size: f32) -> Option<f32> {
        self.code_line_height.map(|m| m.resolve(font_size))
    }

    /// Resolve paragraph spacing to pixels given a font size.
    /// Returns 0.0 if not configured.
    pub fn resolve_paragraph_spacing(&self, font_size: f32) -> f32 {
        self.paragraph_spacing
            .map(|m| m.resolve(font_size))
            .unwrap_or(0.0)
    }

    /// Resolve heading spacing above to pixels given a font size.
    /// Returns 0.0 if not configured.
    pub fn resolve_heading_above(&self, font_size: f32) -> f32 {
        self.heading_spacing_above
            .map(|m| m.resolve(font_size))
            .unwrap_or(0.0)
    }

    /// Resolve heading spacing below to pixels given a font size.
    /// Returns 0.0 if not configured.
    pub fn resolve_heading_below(&self, font_size: f32) -> f32 {
        self.heading_spacing_below
            .map(|m| m.resolve(font_size))
            .unwrap_or(0.0)
    }
}

/// How a fenced code block is boxed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodeBlockStyle {
    /// Background of the block. Replaces the syntax theme's own background.
    pub fill: egui::Color32,
    /// Outline of the block; [`egui::Stroke::NONE`] for none.
    pub stroke: egui::Stroke,
    pub corner_radius: u8,
    /// Space between the box and the code, on every side.
    pub padding: i8,
}

impl CodeBlockStyle {
    /// Drop the line break that ends the block's last line, which would
    /// otherwise lay out as an empty line above the bottom padding.
    pub(crate) fn trim_final_newline(job: &mut egui::text::LayoutJob) {
        if !job.text.ends_with('\n') {
            return;
        }
        let end = job.text.len() - 1;
        job.text.truncate(end);
        for section in &mut job.sections {
            section.byte_range.start = section.byte_range.start.min(end);
            section.byte_range.end = section.byte_range.end.min(end);
        }
        job.sections.retain(|section| !section.byte_range.is_empty());
    }
}

/// Opt-in styling of block elements and of the document column.
///
/// The default reproduces the renderer's built-in look, so consumers that do
/// not set it see no change.
#[derive(Debug, Clone, Default)]
pub struct BlockStyle {
    /// Font family for headings. Callers must register the family with egui
    /// before setting it. `None` keeps the body family.
    pub heading_font_family: Option<egui::FontFamily>,
    /// A true italic family for emphasis, used instead of slanting the body
    /// face. Callers must register the family with egui before setting it.
    pub emphasis_font_family: Option<egui::FontFamily>,
    /// Line height of headings as a multiple of their own font size. `None`
    /// derives it from the body line height. Only used when a body line
    /// height is configured.
    pub heading_line_height: Option<f32>,
    /// Size of inline code as a multiple of the text around it. `None` is
    /// the same size.
    pub inline_code_scale: Option<f32>,
    /// Outline and check mark color of a read-only task list checkbox.
    /// `None` keeps the widget outline color.
    pub task_marker: Option<egui::Color32>,
    /// `None` keeps the built-in box: the syntax theme's background, the
    /// widget outline, and an 8 px padding.
    pub code_block: Option<CodeBlockStyle>,
    /// Color of the bar beside a blockquote. `None` keeps the weak text color.
    pub blockquote_bar: Option<egui::Color32>,
    /// Find a fenced block's syntax by language name (` ```rust `) as well
    /// as by file extension (` ```rs `). Off, a block tagged with a name
    /// that is not also an extension falls back to the built-in
    /// highlighter, which ignores the configured syntax theme.
    pub syntax_by_language_name: bool,
    /// Draw a thematic break as a short centered hairline instead of a
    /// full-width separator.
    pub short_rule: bool,
    /// Center the document column in the space the renderer is given instead
    /// of starting it at the left edge. Only has an effect when
    /// `default_width` is narrower than that space.
    pub center_content: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::misc::{CommonMarkOptions, Style};
    use egui::text::LayoutJob;

    fn format_of(ui: &egui::Ui, rich_text: egui::RichText) -> egui::TextFormat {
        let mut job = LayoutJob::default();
        rich_text.append_to(
            &mut job,
            ui.style(),
            egui::FontSelection::Default,
            egui::Align::LEFT,
        );
        job.sections.remove(0).format
    }

    /// Options with a 1.6 body line height and `block_style`.
    fn options_with(block_style: BlockStyle) -> CommonMarkOptions<'static> {
        let mut options = CommonMarkOptions::default();
        options.typography.line_height = Some(Measurement::Multiplier(1.6));
        options.block_style = block_style;
        options
    }

    fn family(name: &str) -> egui::FontFamily {
        egui::FontFamily::Name(name.into())
    }

    #[test]
    fn the_default_block_style_changes_nothing() {
        egui::__run_test_ui(|ui| {
            let plain = options_with(BlockStyle::default());
            for style in [
                Style::default(),
                Style {
                    heading: Some(0),
                    ..Default::default()
                },
                Style {
                    emphasis: true,
                    ..Default::default()
                },
                Style {
                    code: true,
                    ..Default::default()
                },
            ] {
                let with_options = format_of(ui, style.to_richtext_with_options(ui, "text", &plain));
                let without = format_of(
                    ui,
                    style.to_richtext_with_typography(ui, "text", Some(&plain.typography)),
                );
                assert_eq!(with_options, without);
            }
        });
    }

    #[test]
    fn headings_take_the_heading_family_and_the_tight_line_height() {
        egui::__run_test_ui(|ui| {
            let options = options_with(BlockStyle {
                heading_font_family: Some(family("Heading")),
                heading_line_height: Some(1.2),
                ..Default::default()
            });
            let body_size = egui::TextStyle::Body.resolve(ui.style()).size;

            for level in 0..4 {
                let heading = Style {
                    heading: Some(level),
                    ..Default::default()
                };
                let format = format_of(ui, heading.to_richtext_with_options(ui, "text", &options));
                assert_eq!(format.font_id.family, family("Heading"), "level {level}");
                assert!(format.font_id.size > body_size);
                let line_height = format.line_height.expect("a heading line height");
                assert!((line_height - format.font_id.size * 1.2).abs() < 1e-3);
            }

            // Body text is untouched.
            let body = format_of(
                ui,
                Style::default().to_richtext_with_options(ui, "text", &options),
            );
            assert_ne!(body.font_id.family, family("Heading"));
            assert!((body.line_height.unwrap() - body_size * 1.6).abs() < 1e-3);
        });
    }

    #[test]
    fn strong_text_in_a_heading_keeps_the_heading_face_and_code_keeps_monospace() {
        egui::__run_test_ui(|ui| {
            let mut options = options_with(BlockStyle {
                heading_font_family: Some(family("Heading")),
                ..Default::default()
            });
            options.use_strong_font_family = true;

            let strong_heading = Style {
                heading: Some(1),
                strong: true,
                ..Default::default()
            };
            let format = format_of(
                ui,
                strong_heading.to_richtext_with_options(ui, "text", &options),
            );
            assert_eq!(format.font_id.family, family("Heading"));

            let code_heading = Style {
                heading: Some(1),
                code: true,
                ..Default::default()
            };
            let format = format_of(ui, code_heading.to_richtext_with_options(ui, "text", &options));
            assert_eq!(format.font_id.family, egui::FontFamily::Monospace);
        });
    }

    #[test]
    fn emphasis_uses_the_italic_family_instead_of_a_slant() {
        egui::__run_test_ui(|ui| {
            let options = options_with(BlockStyle {
                emphasis_font_family: Some(family("Italic")),
                ..Default::default()
            });
            let emphasis = Style {
                emphasis: true,
                ..Default::default()
            };

            let format = format_of(ui, emphasis.to_richtext_with_options(ui, "text", &options));
            assert_eq!(format.font_id.family, family("Italic"));
            assert!(!format.italics, "a true italic is not slanted again");

            // Bold italic has no face of its own: the strong face, slanted.
            let strong_emphasis = Style {
                emphasis: true,
                strong: true,
                ..Default::default()
            };
            let format = format_of(
                ui,
                strong_emphasis.to_richtext_with_options(ui, "text", &options),
            );
            assert_ne!(format.font_id.family, family("Italic"));
            assert!(format.italics);
        });
    }

    #[test]
    fn inline_code_is_scaled_against_the_text_around_it() {
        egui::__run_test_ui(|ui| {
            let options = options_with(BlockStyle {
                inline_code_scale: Some(0.5),
                ..Default::default()
            });
            let body_size = egui::TextStyle::Body.resolve(ui.style()).size;
            let code = Style {
                code: true,
                ..Default::default()
            };

            let format = format_of(ui, code.to_richtext_with_options(ui, "text", &options));

            assert!((format.font_id.size - body_size * 0.5).abs() < 1e-3);
            assert_eq!(format.font_id.family, egui::FontFamily::Monospace);
        });
    }

    #[test]
    fn the_final_newline_of_a_code_job_is_dropped_with_its_section() {
        let mut job = LayoutJob::default();
        job.append("let a = 1;", 0.0, egui::TextFormat::default());
        job.append("\n", 0.0, egui::TextFormat::default());
        CodeBlockStyle::trim_final_newline(&mut job);
        assert_eq!(job.text, "let a = 1;");
        assert_eq!(job.sections.len(), 1);
        assert_eq!(job.sections[0].byte_range, 0..10);

        // A newline that shares a section with code shortens the section.
        let mut job = LayoutJob::default();
        job.append("a\nb\n", 0.0, egui::TextFormat::default());
        CodeBlockStyle::trim_final_newline(&mut job);
        assert_eq!(job.text, "a\nb");
        assert_eq!(job.sections[0].byte_range, 0..3);

        // Nothing to drop.
        let mut job = LayoutJob::default();
        job.append("a", 0.0, egui::TextFormat::default());
        CodeBlockStyle::trim_final_newline(&mut job);
        assert_eq!(job.text, "a");
        assert_eq!(job.sections[0].byte_range, 0..1);
    }
}
