use super::*;
use crate::test_support::ScratchDir;

/// Headings 400 px apart, the first one 100 px down a 2000 px document
/// seen through a 600 px pane.
const POSITIONS: [f32; 4] = [100.0, 500.0, 900.0, 1300.0];

fn view_from(top: f32) -> View {
    View {
        top,
        height: 600.0,
        content_height: 2000.0,
    }
}

fn heading_at(top: f32) -> Option<usize> {
    heading_in_view(POSITIONS.len(), |index| POSITIONS[index], view_from(top))
}

#[test]
fn at_the_top_the_reader_is_in_the_first_heading_on_screen() {
    assert_eq!(heading_at(0.0), Some(0));
}

#[test]
fn above_a_first_heading_that_is_off_screen_the_reader_is_in_none() {
    let far_down = |_| 900.0;

    assert_eq!(heading_in_view(1, far_down, view_from(0.0)), None);
    assert_eq!(heading_in_view(1, far_down, view_from(300.0)), None);
    assert_eq!(heading_in_view(1, far_down, view_from(301.0)), Some(0));
}

#[test]
fn the_reader_is_in_the_last_heading_the_reading_line_has_reached() {
    // The reading line sits 62 px below the top of the pane.
    assert_eq!(heading_at(437.0), Some(0));
    assert_eq!(heading_at(438.0), Some(1));
    assert_eq!(heading_at(700.0), Some(1));
    assert_eq!(heading_at(838.0), Some(2));
}

#[test]
fn a_heading_the_outline_scrolled_to_counts_as_reached() {
    for (index, position) in POSITIONS.iter().enumerate().skip(1) {
        let scrolled_to = position - HEADER_SCROLL_MARGIN;

        assert_eq!(heading_at(scrolled_to), Some(index));
    }
}

#[test]
fn at_the_end_of_the_document_the_reader_is_in_the_last_heading() {
    // The last heading is 1900 px down a 2000 px document. The view ends
    // 1400 px down at its lowest, so the reading line never reaches it.
    let positions = [100.0, 500.0, 900.0, 1900.0];
    let heading_at = |top| heading_in_view(4, |index| positions[index], view_from(top));

    assert_eq!(heading_at(1200.0), Some(2));
    assert_eq!(heading_at(1397.0), Some(2));
    assert_eq!(heading_at(1398.0), Some(3), "within a hair of the end");
    assert_eq!(heading_at(1400.0), Some(3));
}

#[test]
fn a_document_shorter_than_the_pane_is_not_at_its_end_until_scrolled() {
    let short = View {
        top: 0.0,
        height: 600.0,
        content_height: 400.0,
    };

    assert_eq!(heading_in_view(2, |index| POSITIONS[index], short), Some(0));
}

#[test]
fn a_document_without_headings_or_layout_has_no_current_heading() {
    assert_eq!(heading_in_view(0, |_| 0.0, view_from(0.0)), None);
    let not_laid_out = View {
        top: 0.0,
        height: 0.0,
        content_height: 0.0,
    };
    assert_eq!(heading_in_view(3, |_| 0.0, not_laid_out), None);
}

/// A tab on a document of `headings` headings, laid out 2000 px tall in a
/// 600 px pane, with nothing measured.
fn tab_with(scratch: &ScratchDir, headings: usize) -> Tab {
    let sections = "## Heading\n\ntext\n\ntext\n\ntext\n\n".repeat(headings);
    let mut tab = Tab::new(scratch.file("doc.md", &sections)).unwrap();
    tab.last_content_height = 2000.0;
    tab.last_viewport_height = 600.0;
    tab
}

#[test]
fn until_headings_are_measured_their_line_numbers_place_them() {
    let scratch = ScratchDir::new();
    let mut tab = tab_with(&scratch, 4);
    let mut spy = HeadingSpy::default();

    // Four headings spread evenly: the third starts about halfway down.
    tab.scroll_offset = 1100.0;

    assert_eq!(spy.look(&tab), (Some(2), true));
}

#[test]
fn a_measured_heading_is_where_the_renderer_painted_it() {
    let scratch = ScratchDir::new();
    let mut tab = tab_with(&scratch, 4);
    let mut spy = HeadingSpy::default();
    for (header, position) in tab.outline_headers.iter().zip([0.0, 50.0, 100.0, 150.0]) {
        let key = header_position_key(header.source_start);
        tab.cache.record_header_content_y(&key, position);
    }

    tab.scroll_offset = 100.0;

    assert_eq!(spy.look(&tab), (Some(3), true));
}

#[test]
fn the_spy_reports_a_move_only_when_the_heading_changes() {
    let scratch = ScratchDir::new();
    let mut tab = tab_with(&scratch, 4);
    let mut spy = HeadingSpy::default();

    assert_eq!(spy.look(&tab), (Some(0), true));
    assert_eq!(spy.look(&tab), (Some(0), false), "nothing changed");

    tab.scroll_offset = 20.0;
    assert_eq!(spy.look(&tab), (Some(0), false), "same heading, new view");

    tab.scroll_offset = 1100.0;
    assert_eq!(spy.look(&tab), (Some(2), true));
    assert_eq!(spy.look(&tab), (Some(2), false));
}

#[test]
fn a_reloaded_document_is_looked_at_again() {
    let scratch = ScratchDir::new();
    let mut tab = tab_with(&scratch, 4);
    let mut spy = HeadingSpy::default();
    tab.scroll_offset = 1100.0;
    assert_eq!(spy.look(&tab), (Some(2), true));

    // The same view of a document that now has two headings.
    std::fs::write(
        &tab.path,
        "## One\n\n## Two\n\ntext\n\ntext\n\ntext\n\ntext\n",
    )
    .unwrap();
    tab.reload().unwrap();
    tab.last_content_height = 2000.0;
    tab.scroll_offset = 1100.0;

    assert_eq!(spy.look(&tab), (Some(1), true));
}

#[test]
fn another_tab_is_looked_at_afresh() {
    let (first_dir, second_dir) = (ScratchDir::new(), ScratchDir::new());
    let first = tab_with(&first_dir, 4);
    let mut second = tab_with(&second_dir, 4);
    second.scroll_offset = 1100.0;
    let mut spy = HeadingSpy::default();

    assert_eq!(spy.look(&first), (Some(0), true));
    assert_eq!(spy.look(&second), (Some(2), true));
    assert_eq!(spy.look(&first), (Some(0), true));
}
