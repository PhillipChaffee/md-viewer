use super::*;
use crate::test_support::ScratchDir;

/// A tab on a document of 200 lines whose one heading, "Deep", is on
/// line 50.
fn tab() -> (ScratchDir, Tab) {
    let dir = ScratchDir::new();
    let content = format!("{}## Deep\n{}", "text\n".repeat(50), "text\n".repeat(149));
    let tab = Tab::new(dir.file("doc.md", content)).unwrap();
    assert_eq!(tab.content_lines, 200);
    assert_eq!(tab.outline.headers()[0].line_number, 50);
    (dir, tab)
}

/// The key the renderer records the position of the heading of `tab` under.
fn heading_key(tab: &Tab) -> String {
    header_position_key(tab.outline.headers()[0].source_start)
}

/// Record that the renderer painted the heading of `tab` at `y`.
fn measure_heading(tab: &mut Tab, y: f32) {
    tab.cache.record_header_content_y(&heading_key(tab), y);
}

fn heading_y(tab: &Tab) -> HeaderY {
    tab.header_y(&tab.outline.headers()[0])
}

#[test]
fn a_measured_heading_is_where_the_renderer_painted_it() {
    let (_dir, mut tab) = tab();
    tab.last_content_height = 4000.0;

    measure_heading(&mut tab, 400.0);

    let measured = HeaderY {
        y: 400.0,
        is_estimate: false,
    };
    assert_eq!(heading_y(&tab), measured);
}

#[test]
fn an_unmeasured_heading_is_estimated_from_its_line_number() {
    let (_dir, mut tab) = tab();
    tab.last_content_height = 4000.0;

    // Line 50 of 200 is a quarter of the way down a 4000px document.
    let a_quarter_down = HeaderY {
        y: 1000.0,
        is_estimate: true,
    };
    assert_eq!(heading_y(&tab), a_quarter_down);
}

#[test]
fn before_layout_an_unmeasured_heading_is_estimated_at_the_top() {
    let at_the_top = HeaderY {
        y: 0.0,
        is_estimate: true,
    };
    let (_dir, mut tab) = tab();

    assert_eq!(heading_y(&tab), at_the_top, "no height measured");
    tab.last_content_height = 4000.0;
    tab.content_lines = 0;
    assert_eq!(heading_y(&tab), at_the_top, "no lines counted");
}

#[test]
fn a_measured_position_does_not_depend_on_layout_totals() {
    let (_dir, mut tab) = tab();
    tab.content_lines = 0;

    measure_heading(&mut tab, 400.0);

    let measured = HeaderY {
        y: 400.0,
        is_estimate: false,
    };
    assert_eq!(heading_y(&tab), measured);
}

#[test]
fn a_jump_to_a_measured_heading_scrolls_to_just_above_it() {
    let (_dir, mut tab) = tab();
    measure_heading(&mut tab, 400.0);

    tab.jump_to_header(0);

    assert_eq!(tab.pending_scroll_offset, Some(350.0));
    assert_eq!(tab.corrections.header_key, None, "nothing to correct");
}

#[test]
fn a_jump_to_a_measured_heading_near_the_top_scrolls_to_the_top() {
    let (_dir, mut tab) = tab();
    measure_heading(&mut tab, 20.0);

    tab.jump_to_header(0);

    assert_eq!(tab.pending_scroll_offset, Some(0.0));
    assert_eq!(tab.corrections.header_key, None);
}

#[test]
fn a_jump_to_an_unmeasured_heading_is_estimated_and_asks_to_be_corrected() {
    let (_dir, mut tab) = tab();
    tab.last_content_height = 4000.0;

    tab.jump_to_header(0);

    assert_eq!(tab.pending_scroll_offset, Some(950.0));
    assert_eq!(tab.corrections.header_key, Some(heading_key(&tab)));
}

#[test]
fn before_layout_a_jump_goes_to_the_top_and_asks_to_be_corrected() {
    let (_dir, mut tab) = tab();

    tab.jump_to_header(0);

    assert_eq!(tab.pending_scroll_offset, Some(0.0));
    assert_eq!(tab.corrections.header_key, Some(heading_key(&tab)));
}

#[test]
fn corrections_are_taken_whole_and_only_once() {
    let (_dir, mut tab) = tab();
    tab.corrections = ScrollCorrections {
        search: true,
        header_key: Some(heading_key(&tab)),
    };

    let taken = tab.take_corrections();

    assert!(taken.search);
    assert_eq!(taken.header_key, Some(heading_key(&tab)));
    assert_eq!(tab.take_corrections(), ScrollCorrections::default());
}

mod fragment {
    use std::fs;

    use super::*;
    use crate::test_support::assert_close;

    /// Above a heading, so it does not sit flush against the top edge.
    const HEADROOM: f32 = 50.0;

    /// A tab on a ten-line document whose title is on line 0 and whose
    /// "Deep Section" heading is on line 8.
    fn tab() -> (ScratchDir, Tab) {
        let dir = ScratchDir::new();
        let document = dir.file("doc.md", "# Title\n\na\n\nb\n\nc\n\n## Deep Section\n\n");
        let tab = Tab::new(document).unwrap();
        (dir, tab)
    }

    fn deep_section_key(tab: &Tab) -> String {
        header_position_key(tab.outline.headers()[1].source_start)
    }

    #[test]
    fn title_fragment_scrolls_to_the_very_top() {
        let (_dir, mut tab) = tab();

        assert!(tab.go_to_fragment("title"));

        assert_close(tab.pending_scroll_offset.unwrap(), 0.0);
        assert_eq!(tab.corrections.header_key, None);
    }

    #[test]
    fn heading_with_a_measured_position_scrolls_to_just_above_it() {
        let (_dir, mut tab) = tab();
        let key = deep_section_key(&tab);

        tab.cache.record_header_content_y(&key, 400.0);
        assert!(tab.go_to_fragment("deep-section"));
        assert_close(tab.pending_scroll_offset.unwrap(), 400.0 - HEADROOM);
        assert_eq!(tab.corrections.header_key, None, "nothing to correct");

        // A heading nearer the top than the headroom cannot scroll above 0.
        tab.cache.record_header_content_y(&key, 20.0);
        assert!(tab.go_to_fragment("deep-section"));
        assert_close(tab.pending_scroll_offset.unwrap(), 0.0);
    }

    #[test]
    fn unmeasured_heading_is_estimated_from_its_line_and_corrected_after_painting() {
        let (_dir, mut tab) = tab();
        let key = deep_section_key(&tab);
        tab.last_content_height = 1000.0;

        assert!(tab.go_to_fragment("deep-section"));

        // Line 8 of 10: eight tenths of the way down the rendered height.
        assert_close(tab.pending_scroll_offset.unwrap(), 800.0 - HEADROOM);
        assert_eq!(tab.corrections.header_key, Some(key));
    }

    #[test]
    fn document_without_a_title_still_resolves_its_section_anchors() {
        let dir = ScratchDir::new();
        let mut tab = Tab::new(dir.file("doc.md", "## Only Section\n")).unwrap();

        assert!(tab.go_to_fragment("only-section"));
        assert!(!tab.go_to_fragment("title"));
    }

    #[test]
    fn unpainted_document_leaves_the_scroll_to_the_corrective_pass() {
        let (_dir, mut tab) = tab();
        let key = deep_section_key(&tab);

        assert!(tab.go_to_fragment("deep-section"));

        assert_eq!(tab.pending_scroll_offset, None);
        assert_eq!(tab.corrections.header_key, Some(key));
    }

    #[test]
    fn same_document_fragment_scrolls_without_touching_history() {
        let dir = ScratchDir::new();
        let root = dir.path();
        let document = root.join("single.md");
        fs::write(&document, "# Title\n\ntext\n\n## A Section\n\ntext\n").unwrap();

        let mut tab = Tab::new(document).unwrap();
        let original_path = tab.path.clone();

        assert!(
            tab.navigate_to_link("#a-section").unwrap(),
            "a matching same-document fragment must navigate"
        );
        assert_eq!(
            tab.path, original_path,
            "same-document navigation stays put"
        );
        assert!(tab.history_back.is_empty(), "no history entry for a scroll");
        assert!(
            tab.corrections.header_key.is_some() || tab.pending_scroll_offset.is_some(),
            "the click must schedule the scroll"
        );

        // An unmatched fragment does nothing at all — no navigation, and the
        // scroll scheduled by the earlier match is left alone.
        let key_before = tab.corrections.header_key.clone();
        assert!(!tab.navigate_to_link("#no-such-anchor").unwrap());
        assert_eq!(tab.corrections.header_key, key_before);
        assert_eq!(tab.path, original_path);
    }
}
