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

/// Record that the renderer painted the heading of `tab` at `y`.
fn measure_heading(tab: &mut Tab, y: f32) {
    let key = header_position_key(tab.outline.headers()[0].source_start);
    tab.cache.record_header_content_y(&key, y);
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
