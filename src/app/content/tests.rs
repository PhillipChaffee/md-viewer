use eframe::egui::accesskit::Role;
use eframe::egui::{Event, Key, Modifiers, MouseWheelUnit};
use egui_kittest::kittest::Queryable as _;

use self::label_harness::LabelHarness;
use super::*;
use crate::app::tests::Harness;
use crate::app::ui_harness::{frame_on, painted_text};
use crate::layout::{centered_column_left, LineSpacing, ReadingWidth, READING_COLUMN_MARGIN};
use crate::test_support::{assert_close, painted_shapes, painted_text_starting_below};
use crate::theme::ThemeId;

mod blocks;
pub(crate) mod label_harness;

/// The first text the document pane painted that starts with `prefix`,
/// with where it was painted. The top bar above the pane repeats the
/// document's title, and is passed over.
#[track_caller]
fn pane_text_starting(output: &egui::FullOutput, prefix: &str) -> egui::epaint::TextShape {
    painted_text_starting_below(output, prefix, crate::app::chrome::BAR_HEIGHT)
}

/// One document that links to `b.md` and to a file that does not exist.
fn linked_docs() -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[
        ("a.md", "Go to [the other one](b.md) or [nowhere](gone.md)."),
        ("b.md", "The other document."),
    ]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

/// A document far taller than the window, with `needle` near its end. One
/// paragraph is a single source line that wraps over many, so a position
/// estimated from line counts is well off.
fn long_doc() -> LabelHarness {
    let filler = "A paragraph of text.\n\n".repeat(60);
    let wrapped = "word ".repeat(600);
    let content = format!("{wrapped}\n\n{filler}needle\n\n{filler}");
    let mut harness = LabelHarness::with_docs(&[("a.md", &content)]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

fn open_paths(harness: &LabelHarness) -> Vec<PathBuf> {
    let tabs = &harness.state().tabs;
    tabs.iter().map(|tab| tab.path.clone()).collect()
}

/// A point in the middle of the document pane.
const OVER_DOCUMENT: egui::Pos2 = egui::pos2(600.0, 500.0);
/// A point in the menu bar, above the document pane.
const OVER_MENU_BAR: egui::Pos2 = egui::pos2(600.0, 10.0);

/// Turns the mouse wheel by `delta_y` points with the pointer at `pointer`.
fn wheel_at(harness: &mut LabelHarness, pointer: egui::Pos2, delta_y: f32) {
    harness.event(Event::PointerMoved(pointer));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: egui::vec2(0.0, delta_y),
        modifiers: Modifiers::NONE,
    });
    harness.frames(4);
}

#[test]
fn clicking_a_link_shows_its_document_in_the_same_tab() {
    let mut harness = linked_docs();

    harness.click("the other one");

    assert_eq!(open_paths(&harness), [harness.path("b.md")]);
    assert_eq!(harness.state().error_message, None);
}

#[test]
fn ctrl_clicking_a_link_opens_its_document_in_a_new_tab() {
    let mut harness = linked_docs();

    harness
        .get_by_label("the other one")
        .click_modifiers(Modifiers::CTRL);
    harness.frames(2);

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("b.md")]
    );
}

#[test]
fn a_link_to_a_missing_document_is_reported_and_changes_nothing() {
    let mut harness = linked_docs();

    harness.click("nowhere");

    assert_eq!(open_paths(&harness), [harness.path("a.md")]);
    let message = harness.state().error_message.clone().unwrap();
    assert!(message.contains("gone.md"), "{message}");
}

#[test]
fn edit_mode_shows_the_source_and_takes_typing() {
    let mut harness = LabelHarness::with_docs(&[("a.md", "# Title")]);
    harness.click("More");
    harness.click("Edit Document");

    let editor = harness.get_by_role(Role::MultilineTextInput);
    assert_eq!(editor.value().as_deref(), Some("# Title"));
    editor.click();
    harness.frames(2);
    harness.get_by_role(Role::MultilineTextInput).type_text("!");
    harness.frames(2);

    assert_eq!(harness.state().tabs[0].content, "# Title!");
}

#[test]
fn a_heading_typed_in_the_editor_is_in_the_contents_at_once() {
    let mut harness = LabelHarness::with_docs(&[("a.md", "# Title")]);
    harness.click("More");
    harness.click("Edit Document");
    harness.get_by_role(Role::MultilineTextInput).click();
    harness.frames(2);

    let editor = harness.get_by_role(Role::MultilineTextInput);
    editor.type_text("\n\n## Afterword");
    harness.frames(2);

    let tab = &harness.state().tabs[0];
    let titles: Vec<&str> = tab
        .outline
        .headers()
        .iter()
        .map(|header| header.title.as_str())
        .collect();
    assert_eq!(titles, ["Title", "Afterword"]);
    assert_eq!(tab.content_lines, 3);
}

#[test]
fn typing_in_the_editor_with_the_find_bar_open_finds_the_matches_again() {
    let mut harness = LabelHarness::with_docs(&[("a.md", "needle")]);
    harness.state_mut().open_search();
    harness.state_mut().search.query = "needle".to_owned();
    harness.click("More");
    harness.click("Edit Document");
    harness.get_by_role(Role::MultilineTextInput).click();
    harness.frames(2);
    assert!(harness.query_by_label("1 / 1").is_some());

    let editor = harness.get_by_role(Role::MultilineTextInput);
    editor.type_text("s and needle");
    harness.frames(3);

    assert_eq!(harness.state().tabs[0].content, "needles and needle");
    assert!(
        harness.query_by_label("1 / 2").is_some(),
        "the count is of the text before the edit"
    );
    let ranges: Vec<_> = harness.state().tabs[0]
        .search_matches
        .iter()
        .map(|found| found.byte_start..found.byte_end)
        .collect();
    assert_eq!(ranges, [0..6, 12..18]);
}

#[test]
fn switching_to_edit_mode_keeps_the_place_in_a_long_document() {
    let mut harness = long_doc();
    harness.key_press(Key::PageDown);
    harness.key_press(Key::PageDown);
    harness.frames(2);
    let tab = &harness.state().tabs[0];
    let place = tab.scroll_offset / tab.last_content_height;
    assert!(place > 0.05, "the preview did not scroll: {place}");

    harness.click("More");
    harness.click("Edit Document");
    harness.frames(2);

    let tab = &harness.state().tabs[0];
    let place_in_editor = tab.scroll_offset / tab.last_content_height;
    assert!(
        (place_in_editor - place).abs() < 0.02,
        "preview was at {place}, the editor opened at {place_in_editor}"
    );
}

#[test]
fn jumping_to_a_search_match_brings_it_into_view() {
    let mut harness = long_doc();
    harness.key_press_modifiers(Modifiers::CTRL, Key::F);
    harness.frames(1);

    harness.state_mut().search.query = "needle".to_owned();
    harness.frames(4);

    let tab = &harness.state().tabs[0];
    let match_y = tab.cache.active_search_y().unwrap();
    let visible = tab.scroll_offset..tab.scroll_offset + tab.last_viewport_height;
    assert!(
        visible.contains(&match_y),
        "the match at {match_y} is outside {visible:?}"
    );
}

#[test]
fn the_wheel_scrolls_the_document_under_the_pointer() {
    let mut harness = long_doc();

    wheel_at(&mut harness, OVER_DOCUMENT, -120.0);
    let after_scrolling_down = harness.state().tabs[0].scroll_offset;
    assert!(after_scrolling_down >= 120.0, "{after_scrolling_down}");

    wheel_at(&mut harness, OVER_DOCUMENT, 60.0);
    assert!(harness.state().tabs[0].scroll_offset < after_scrolling_down);
}

#[test]
fn one_wheel_step_moves_the_document_once() {
    let mut harness = long_doc();

    wheel_at(&mut harness, OVER_DOCUMENT, -120.0);

    assert_close(harness.state().tabs[0].scroll_offset, 120.0);
}

#[test]
fn one_wheel_step_moves_the_document_once_while_a_selection_is_dragged() {
    let mut harness = long_doc();
    let dragged_to = OVER_DOCUMENT + egui::vec2(60.0, 0.0);
    harness.event(Event::PointerMoved(OVER_DOCUMENT));
    harness.event(Event::PointerButton {
        pos: OVER_DOCUMENT,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(Event::PointerMoved(dragged_to));
    harness.frames(2);
    assert!(harness.ctx.dragged_id().is_some(), "no drag is under way");

    wheel_at(&mut harness, dragged_to, -120.0);

    assert_close(harness.state().tabs[0].scroll_offset, 120.0);
}

#[test]
fn the_wheel_leaves_the_document_alone_while_the_pointer_is_elsewhere() {
    let mut harness = long_doc();

    wheel_at(&mut harness, OVER_MENU_BAR, -120.0);

    assert_close(harness.state().tabs[0].scroll_offset, 0.0);
}

#[test]
fn a_search_match_inside_the_viewport_needs_no_correction() {
    // The viewport shows 1000..1600; the match must sit 20px inside either edge.
    assert_eq!(search_correction(1300.0, 1000.0, 600.0), None);
    assert_eq!(search_correction(1020.0, 1000.0, 600.0), None);
    assert_eq!(search_correction(1580.0, 1000.0, 600.0), None);
}

#[test]
fn a_search_match_outside_the_viewport_lands_a_third_of_the_way_down() {
    // 35% of a 1000px viewport is 350px above the match.
    assert_eq!(search_correction(3000.0, 500.0, 1000.0), Some(2650.0));
    // Inside the viewport but within the 20px edge band still counts as hidden.
    assert_eq!(search_correction(1510.0, 1500.0, 1000.0), Some(1160.0));
    assert_eq!(search_correction(2495.0, 1500.0, 1000.0), Some(2145.0));
}

#[test]
fn a_search_correction_never_scrolls_above_the_document_top() {
    assert_eq!(search_correction(100.0, 800.0, 1000.0), Some(0.0));
}

#[test]
fn a_search_correction_needs_a_laid_out_viewport() {
    assert_eq!(search_correction(3000.0, 500.0, 0.0), None);
}

#[test]
fn a_search_correction_is_skipped_when_the_view_is_already_at_the_target() {
    // A 40px viewport has no comfortable band, so the match counts as hidden,
    // but its target (114 - 35% of 40) is where the view already is.
    assert_eq!(search_correction(114.0, 100.0, 40.0), None);
}

#[test]
fn a_clicked_header_is_placed_below_the_viewport_top() {
    assert_eq!(header_correction(500.0, 0.0), Some(450.0));
    assert_eq!(header_correction(500.0, 2000.0), Some(450.0));
    // A header near the top cannot scroll above the document.
    assert_eq!(header_correction(30.0, 400.0), Some(0.0));
}

#[test]
fn a_header_correction_is_skipped_within_two_pixels_of_the_target() {
    assert_eq!(header_correction(500.0, 449.0), None);
    assert_eq!(header_correction(500.0, 452.0), None);
    assert_eq!(header_correction(30.0, 0.0), None);
}

#[test]
fn a_wheel_step_moves_the_view_against_the_wheel_delta() {
    assert_eq!(wheel_scroll_target(100.0, -30.0, 1000.0), Some(130.0));
    assert_eq!(wheel_scroll_target(100.0, 30.0, 1000.0), Some(70.0));
}

#[test]
fn a_wheel_step_that_would_reach_either_end_is_left_to_the_scroll_area() {
    assert_eq!(wheel_scroll_target(20.0, 30.0, 1000.0), None);
    assert_eq!(wheel_scroll_target(990.0, -30.0, 1000.0), None);
    // A document that fits the viewport has nowhere to scroll.
    assert_eq!(wheel_scroll_target(0.0, -30.0, 0.0), None);
}

#[test]
fn a_wheel_step_smaller_than_half_a_pixel_is_ignored() {
    assert_eq!(wheel_scroll_target(100.0, 0.25, 1000.0), None);
}

/// The document pane between its margins when both sidebars are closed: the
/// window is 1200 px wide, the pane keeps 8 px on the left and 10 px on the
/// right.
const PANE_LEFT: f32 = 8.0;
const PANE_RIGHT: f32 = 1190.0;

/// The left edge of a column `width` px wide in the document pane.
fn column_left(width: f32) -> f32 {
    centered_column_left(PANE_LEFT, PANE_RIGHT, width)
}

/// The app on one document with both sidebars closed, driven a frame at a
/// time, a few frames in.
fn reader(content: &str) -> Harness {
    let mut harness = Harness::with_docs(&[("a.md", content)]);
    harness.app.show_explorer = false;
    harness.app.show_outline = false;
    harness.idle();
    harness.idle();
    harness
}

/// A heading and a paragraph long enough to wrap in any column.
fn title_and_long_paragraph() -> String {
    format!("# Title\n\n{}", "word ".repeat(300))
}

fn pointer_button(pos: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// The font and color of the first run of the painted text starting with
/// `prefix`.
fn text_format(output: &egui::FullOutput, prefix: &str) -> egui::TextFormat {
    let text = pane_text_starting(output, prefix);
    text.galley.job.sections[0].format.clone()
}

#[test]
fn the_document_is_set_in_a_centered_column_of_the_reading_width() {
    let mut harness = reader(&title_and_long_paragraph());
    let fit_window = ReadingWidth::FIT_WINDOW.resolve(PANE_RIGHT - PANE_LEFT);

    for (width, column) in [
        (ReadingWidth::DEFAULT, 680.0),
        (ReadingWidth::new(400.0), 400.0),
        (ReadingWidth::FIT_WINDOW, fit_window),
    ] {
        harness.apply(AppAction::SetReadingWidth(width.get()));
        harness.idle();
        let output = harness.idle();

        let left = column_left(column);
        let body = pane_text_starting(&output, "word");
        assert_close(body.pos.x, left);
        assert_close(pane_text_starting(&output, "Title").pos.x, left);
        // The paragraph wraps at the column's right edge, not short of it.
        let widest_line = body.galley.size().x;
        assert!(
            widest_line <= column + 0.5 && widest_line > column - 80.0,
            "a {column} px column wrapped at {widest_line}"
        );
        // The margins on the two sides are equal, to the centering's pixel.
        let right_margin = PANE_RIGHT - (left + column);
        assert!(
            (right_margin - (left - PANE_LEFT)).abs() <= 1.0,
            "margins {} and {right_margin}",
            left - PANE_LEFT
        );
    }
    assert_close(harness.app.document_pane_width, PANE_RIGHT - PANE_LEFT);
}

#[test]
fn a_pane_narrower_than_the_column_narrows_it_and_keeps_the_margins() {
    let mut harness = reader(&title_and_long_paragraph());
    harness.app.show_explorer = true;
    harness.app.explorer_width = 500.0;
    // The reader's own choice: left to the room, the outline would hide.
    harness.apply(AppAction::ToggleOutline);
    harness.idle();
    let output = harness.idle();

    let pane = harness.app.document_pane_width;
    assert!(pane < 680.0, "the sidebars leave {pane}");
    let column = ReadingWidth::DEFAULT.resolve(pane);
    assert_close(
        column,
        (2.0_f32.mul_add(-READING_COLUMN_MARGIN, pane)).floor(),
    );
    let widest_line = pane_text_starting(&output, "word").galley.size().x;
    assert!(widest_line <= column + 0.5, "{widest_line} in {column}");
}

#[test]
fn a_meta_line_with_the_file_name_and_reading_time_sits_above_the_title() {
    let mut harness = reader("# Title\n\nBody.");
    let output = harness.idle();

    let kicker = pane_text_starting(&output, "a.md \u{b7}");
    assert_eq!(kicker.galley.text(), "a.md · 1 min read");
    let title = pane_text_starting(&output, "Title");
    assert_close(kicker.pos.x, title.pos.x);
    assert!(kicker.pos.y < title.pos.y);
    // Small, muted, and in the chrome's sans whatever the document is set in.
    let format = text_format(&output, "a.md \u{b7}");
    assert_eq!(format.color, harness.app.theme.palette().muted);
    assert_eq!(format.font_id, egui::FontId::proportional(KICKER_SIZE));
}

#[test]
fn the_meta_line_keeps_its_room_whether_all_or_a_slice_of_the_document_is_painted() {
    let mut harness = reader(&format!(
        "# Title\n\n{}",
        "A paragraph of text.\n\n".repeat(200)
    ));
    let positions = |output: &egui::FullOutput| {
        (
            pane_text_starting(output, "a.md \u{b7}").pos.y,
            pane_text_starting(output, "Title").pos.y,
        )
    };
    // By now the renderer paints only the slice of the document in view.
    let (kicker, title) = positions(&harness.idle());

    // A stale layout makes it paint, and measure, the whole document once.
    harness.app.relayout_documents();
    let (measured_kicker, measured_title) = positions(&harness.idle());
    let (again_kicker, again_title) = positions(&harness.idle());

    assert_close(measured_kicker, kicker);
    assert_close(measured_title, title);
    assert_close(again_kicker, kicker);
    assert_close(again_title, title);
}

#[test]
fn the_reader_sets_body_headings_emphasis_and_code_each_in_its_own_face() {
    let mut harness = reader("# Title\n\nPlain *leaning* and `mono` text.");
    let output = harness.idle();
    let family = |name: &str| egui::FontFamily::Name(name.into());

    let body = text_format(&output, "Plain");
    assert_eq!(body.font_id.family, family("MarkdownBody"));
    // Normal text, scaled up for the serif.
    assert_close(body.font_id.size, 16.0 * 1.15);
    assert_close(
        body.line_height.unwrap(),
        body.font_id.size * LineSpacing::Comfortable.line_height(),
    );

    let title = text_format(&output, "Title");
    assert_eq!(title.font_id.family, family("MarkdownHeading"));
    assert_close(title.font_id.size, body.font_id.size * 2.0);
    assert!(title.line_height.unwrap() < title.font_id.size * 1.3);

    // A true italic face, not the upright one slanted.
    let emphasis = text_format(&output, "leaning");
    assert_eq!(emphasis.font_id.family, family("MarkdownEmphasis"));
    assert!(!emphasis.italics);

    let code = text_format(&output, "mono");
    assert_eq!(code.font_id.family, egui::FontFamily::Monospace);
    assert_close(code.font_id.size, body.font_id.size * 0.875);
}

#[test]
fn a_font_preset_without_an_italic_face_slants_emphasis_instead() {
    let mut harness = reader("Plain *leaning* text.");
    harness.app.font_preset = crate::system_fonts::FontPreset::Current;
    harness.idle();
    let output = harness.idle();

    let emphasis = text_format(&output, "leaning");
    assert!(emphasis.italics);
    assert_close(text_format(&output, "Plain").font_id.size, 16.0);
}

/// The one box the frame painted in `fill` with a code block's corners.
fn code_box(output: &egui::FullOutput, fill: egui::Color32) -> egui::epaint::RectShape {
    let mut boxes = painted_shapes(output)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.fill == fill && rect.corner_radius == egui::CornerRadius::same(8) =>
            {
                Some(rect)
            }
            _ => None,
        });
    let found = boxes.next().expect("no code block box was painted");
    assert!(boxes.next().is_none(), "more than one code block box");
    found
}

#[test]
fn a_code_block_is_a_rounded_borderless_box_in_the_themes_raised_color() {
    let mut harness = reader("```rust\nfn main() {}\n```\n");

    for theme in [ThemeId::Sepia, ThemeId::Night] {
        harness.apply(AppAction::SetTheme(theme));
        harness.idle();
        let output = harness.idle();

        let block = code_box(&output, theme.palette().raised);
        assert!(block.stroke.is_empty(), "{theme:?}");
        // As wide as the column, with the code 16 px inside it.
        assert_close(block.rect.left(), column_left(680.0));
        assert!(
            (block.rect.width() - 680.0).abs() <= 1.0,
            "{:?}",
            block.rect
        );
        let code = pane_text_starting(&output, "fn main");
        assert_close(code.pos.x - block.rect.left(), 16.0);
        // One line of code and the padding: no empty line under it.
        assert_eq!(code.galley.rows.len(), 1);
        assert!(
            (block.rect.height() - (code.galley.size().y + 32.0)).abs() <= 1.0,
            "box {} for a {} line",
            block.rect.height(),
            code.galley.size().y
        );
    }
}

#[test]
fn each_theme_highlights_code_with_its_own_syntax_theme() {
    let mut harness = reader("```rust\nfn main() {}\n```\n");
    let mut keyword_color = |theme: ThemeId| {
        harness.apply(AppAction::SetTheme(theme));
        harness.idle();
        text_format(&harness.idle(), "fn").color
    };

    // Two light themes and two dark ones: light against dark is not what
    // tells them apart.
    let colors = ThemeId::ALL.map(&mut keyword_color);

    assert_ne!(colors[0], colors[1], "Paper and Sepia");
    assert_ne!(colors[2], colors[3], "Dusk and Night");
    // Going back gives the first theme's color again.
    assert_eq!(keyword_color(ThemeId::Paper), colors[0]);
}

#[test]
fn line_spacing_sets_the_pitch_of_body_lines_and_the_length_of_the_page() {
    let mut harness = reader(&"word ".repeat(400));
    let mut measure = |spacing: LineSpacing| {
        harness.apply(AppAction::SetLineSpacing(spacing));
        harness.idle();
        let output = harness.idle();
        let galley = pane_text_starting(&output, "word").galley;
        let pitch = galley.rows[1].pos.y - galley.rows[0].pos.y;
        (pitch, harness.app.tabs[0].last_content_height)
    };

    let (compact, compact_page) = measure(LineSpacing::Compact);
    let (comfortable, _) = measure(LineSpacing::Comfortable);
    let (airy, airy_page) = measure(LineSpacing::Airy);

    let body_size = 16.0 * 1.15;
    assert!((compact - body_size * 1.45).abs() < 0.6, "{compact}");
    assert!((comfortable - body_size * 1.6).abs() < 0.6, "{comfortable}");
    assert!((airy - body_size * 1.8).abs() < 0.6, "{airy}");
    assert!(
        airy_page > compact_page * 1.15,
        "{compact_page} -> {airy_page}"
    );
}

#[test]
fn dragging_a_column_edge_resizes_the_column_about_the_pane_center() {
    let mut harness = reader(&title_and_long_paragraph());
    let right_edge = column_left(680.0) + 680.0;
    let hold = egui::pos2(right_edge + 5.0, 400.0);
    let pull = egui::pos2(right_edge + 55.0, 400.0);

    frame_on(&mut harness, vec![Event::PointerMoved(hold)]);
    frame_on(&mut harness, vec![pointer_button(hold, true)]);
    frame_on(&mut harness, vec![Event::PointerMoved(pull)]);
    assert_close(harness.app.reading_width.get(), 780.0);
    assert!(harness.app.column_drag_grab.is_some());

    // The next frame lays the document out at the new width and says so.
    let dragging = frame_on(&mut harness, Vec::new());
    assert_close(
        pane_text_starting(&dragging, "word").pos.x,
        column_left(780.0),
    );
    assert!(painted_text(&dragging).contains(&"780 px".to_owned()));

    let released = frame_on(&mut harness, vec![pointer_button(pull, false)]);
    assert_eq!(harness.app.column_drag_grab, None);
    assert_close(harness.app.reading_width.get(), 780.0);
    assert!(!painted_text(&released).contains(&"780 px".to_owned()));
}

#[test]
fn double_clicking_a_column_edge_goes_back_to_the_default_width() {
    let mut harness = reader(&title_and_long_paragraph());
    harness.apply(AppAction::SetReadingWidth(400.0));
    harness.idle();
    harness.idle();
    let on_edge = egui::pos2(column_left(400.0) - 5.0, 400.0);

    frame_on(&mut harness, vec![Event::PointerMoved(on_edge)]);
    for pressed in [true, false, true, false] {
        frame_on(&mut harness, vec![pointer_button(on_edge, pressed)]);
    }

    assert_eq!(harness.app.reading_width, ReadingWidth::DEFAULT);
}

#[test]
fn the_column_edges_are_named_for_assistive_technology() {
    let harness = linked_docs();

    for label in [
        "Resize text column, left edge",
        "Resize text column, right edge",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
}

#[test]
fn the_welcome_page_has_no_column_to_resize() {
    let mut harness = linked_docs();
    harness.state_mut().close_tab(0);
    harness.frames(2);

    assert!(harness
        .query_by_label("Resize text column, left edge")
        .is_none());
}

#[test]
fn the_editor_is_as_wide_as_the_column_and_centered_like_it() {
    let mut harness = reader("# Title\n\nBody.");
    harness.app.tabs[0].set_edit_mode(true);
    harness.idle();
    let output = harness.idle();

    let source = pane_text_starting(&output, "# Title");
    let left = column_left(680.0);
    assert!(
        (left..=left + 12.0).contains(&source.pos.x),
        "source at {}, column at {left}",
        source.pos.x
    );
    let wrap_width = source.galley.job.wrap.max_width;
    assert!(
        (640.0..=680.0).contains(&wrap_width),
        "the source wraps at {wrap_width}"
    );
}

#[test]
fn a_table_squeezed_by_a_narrow_column_spreads_out_again_when_the_column_widens() {
    let table = "| Circuit | Neurons | Share of total | Format change |\n\
                 |---|---:|---:|---:|\n\
                 | Safety refusal | 50 | 0.014% | 80% |\n\
                 | Language selection | 120 | 0.031% | 64% |\n";
    let mut harness = reader(table);
    let mut header_lines = |width: f32| {
        harness.apply(AppAction::SetReadingWidth(width));
        for _ in 0..3 {
            harness.idle();
        }
        let output = harness.idle();
        pane_text_starting(&output, "Share").galley.rows.len()
    };

    assert_eq!(header_lines(680.0), 1);
    assert!(
        header_lines(ReadingWidth::MIN) > 1,
        "the narrow column wraps it"
    );
    assert_eq!(header_lines(680.0), 1, "and the wide one unwraps it");
}
