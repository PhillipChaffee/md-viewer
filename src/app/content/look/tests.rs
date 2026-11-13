use super::*;
use crate::system_fonts::{document_fonts, FontConfig, FontPreset, TextSizeClass};
use crate::theme::ThemeId;

fn look(theme: ThemeId, preset: FontPreset) -> DocumentLook {
    let fonts = FontConfig {
        preset,
        ..FontConfig::default()
    };
    DocumentLook::new(
        &egui::Style::default(),
        theme.palette(),
        &document_fonts(&fonts),
    )
}

#[test]
fn the_document_is_laid_out_in_its_own_fonts_and_the_rest_stays_the_chromes() {
    let chrome = egui::Style::default();
    let fonts = document_fonts(&FontConfig {
        size: TextSizeClass::Large,
        ..FontConfig::default()
    });

    let look = DocumentLook::new(&chrome, ThemeId::Sepia.palette(), &fonts);

    let styles = &look.style.text_styles;
    assert_eq!(styles[&TextStyle::Body], fonts.body);
    assert_eq!(styles[&TextStyle::Heading], fonts.heading);
    assert_eq!(styles[&TextStyle::Monospace], fonts.code);
    assert_ne!(
        styles[&TextStyle::Body],
        chrome.text_styles[&TextStyle::Body]
    );
    // What the document does not set is the chrome's.
    for kept in [TextStyle::Small, TextStyle::Button] {
        assert_eq!(styles[&kept], chrome.text_styles[&kept], "{kept:?}");
    }
    assert_eq!(look.style.visuals, chrome.visuals);
    assert_eq!(look.style.spacing, chrome.spacing);
}

#[test]
fn headings_emphasis_and_inline_code_take_the_document_fonts_faces() {
    let fonts = document_fonts(&FontConfig::default());
    let reader = look(ThemeId::Sepia, FontPreset::Reader).blocks;
    assert_eq!(reader.heading_font_family, Some(fonts.heading.family));
    assert!(reader.emphasis_font_family.is_some());
    assert_eq!(reader.emphasis_font_family, fonts.emphasis);
    assert_eq!(reader.inline_code_scale, fonts.inline_code_scale);

    // A preset with no italic face and no aligned code face keeps the
    // renderer's slanted emphasis and full-size inline code.
    let system = look(ThemeId::Sepia, FontPreset::Current).blocks;
    assert!(system.heading_font_family.is_some());
    assert_eq!(system.emphasis_font_family, None);
    assert_eq!(system.inline_code_scale, None);
}

#[test]
fn blocks_are_drawn_in_the_themes_colors() {
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        let look = look(theme, FontPreset::Reader);

        assert_eq!(
            look.blocks.code_block,
            Some(CodeBlockStyle {
                fill: palette.raised,
                stroke: egui::Stroke::NONE,
                corner_radius: 8,
                padding: 16,
            }),
            "{theme:?}"
        );
        assert_eq!(look.blocks.blockquote_bar, Some(palette.rule));
        assert_eq!(look.blocks.task_marker, Some(palette.muted));
        assert_eq!(look.syntax_theme, palette.syntax_theme);
        assert_eq!(look.muted, palette.muted);
    }
}

#[test]
fn diagrams_are_drawn_in_the_colors_of_the_style_the_look_was_built_from() {
    let mut chrome = egui::Style::default();
    for visuals in [egui::Visuals::light(), egui::Visuals::dark()] {
        chrome.visuals = visuals;
        let fonts = document_fonts(&FontConfig::default());

        let look = DocumentLook::new(&chrome, ThemeId::Sepia.palette(), &fonts);

        assert_eq!(look.diagrams, diagram_colors(&look.style.visuals));
        assert_eq!(look.diagrams.page, chrome.visuals.panel_fill);
    }
}

#[test]
fn the_document_is_a_centered_column_with_reader_block_styling() {
    let blocks = look(ThemeId::Paper, FontPreset::Reader).blocks;

    assert!(blocks.center_content);
    assert!(blocks.short_rule);
    assert!(blocks.syntax_by_language_name);
    // Headings sit tighter than any body line spacing.
    let heading_line_height = blocks.heading_line_height.unwrap();
    assert!(
        (1.1..1.3).contains(&heading_line_height),
        "{heading_line_height}"
    );
}

#[test]
fn tables_lists_and_code_chips_are_drawn_in_the_themes_colors() {
    for theme in ThemeId::ALL {
        let palette = theme.palette();
        let blocks = look(theme, FontPreset::Reader).blocks;

        let table = blocks.table.unwrap();
        assert_eq!(table.rule, egui::Stroke::new(1.0_f32, palette.rule));
        // The header rule is a hairline too, in a color of its own between
        // the row rules and the muted text.
        crate::test_support::assert_close(table.header_rule.width, 1.0);
        assert_ne!(table.header_rule.color, palette.rule, "{theme:?}");
        assert_ne!(table.header_rule.color, palette.muted, "{theme:?}");
        assert!(table.fill_width);
        assert!(table.cell_padding >= 8.0);

        assert_eq!(blocks.list.unwrap().marker, palette.muted);
        assert_eq!(blocks.inline_code_radius, Some(4.0));
    }
}

#[test]
fn headings_get_more_room_above_than_below_and_tight_lists_stay_tight() {
    let rhythm = look(ThemeId::Sepia, FontPreset::Reader)
        .blocks
        .rhythm
        .unwrap();

    assert!(rhythm.heading_above > 2.0 * rhythm.heading_below);
    assert!(rhythm.heading_below < rhythm.paragraph);
    assert!(rhythm.tight_item < 0.5 * rhythm.paragraph);
    // A paragraph gap is clearly less than a blank line of body text.
    assert!(rhythm.paragraph < 1.0 && rhythm.paragraph <= rhythm.block);
}

#[test]
fn the_room_for_figures_is_left_to_the_pane() {
    // The look is built once per theme and font change; the pane's width
    // is only known per frame.
    assert_eq!(
        look(ThemeId::Sepia, FontPreset::Reader)
            .blocks
            .figure_max_width,
        None
    );
}
