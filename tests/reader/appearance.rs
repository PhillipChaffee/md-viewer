//! Zoom, text size, line spacing, the popover that holds them, and quit.

use super::support::Reader;

const SIZES: [&str; 5] = ["Small", "Normal", "Large", "Extra large", "Huge"];

fn article() -> Reader {
    Reader::with_docs(&[(
        "article.md",
        "# Tides\n\nThe sea keeps its own hours.\n\n- ebb\n- flow\n- slack water",
    )])
}

/// The zoom the menu reports, such as `125%`.
fn zoom(reader: &mut Reader) -> String {
    reader.click("More");
    let names = reader.names();
    let readout = names
        .iter()
        .find_map(|name| name.strip_prefix("Reset Zoom, "));
    let zoom = readout.unwrap().to_owned();
    reader.press("Escape");
    zoom
}

#[test]
fn ctrl_plus_and_minus_zoom_by_a_quarter() {
    let mut reader = article();
    assert_eq!(zoom(&mut reader), "100%");

    reader.press("Ctrl++");
    assert_eq!(zoom(&mut reader), "125%");

    reader.press("Ctrl+-");
    reader.press("Ctrl+-");
    assert_eq!(zoom(&mut reader), "80%");
}

#[test]
fn ctrl_zero_goes_back_to_full_size() {
    let mut reader = article();
    reader.press("Ctrl++");
    reader.press("Ctrl++");

    reader.press("Ctrl+0");

    assert_eq!(zoom(&mut reader), "100%");
}

#[test]
fn zoom_stops_at_50_and_300_percent() {
    let mut reader = article();

    for _ in 0..8 {
        reader.press("Ctrl+-");
    }
    assert_eq!(zoom(&mut reader), "50%");

    for _ in 0..16 {
        reader.press("Ctrl++");
    }
    assert_eq!(zoom(&mut reader), "300%");
}

#[test]
fn the_menu_zooms_too() {
    let mut reader = article();

    reader.click("More");
    reader.click("Zoom In");
    reader.press("Escape");
    assert_eq!(zoom(&mut reader), "125%");

    reader.click("More");
    reader.click("Reset Zoom, 125%");
    reader.press("Escape");
    assert_eq!(zoom(&mut reader), "100%");
}

#[test]
fn zooming_in_makes_the_text_bigger() {
    let mut reader = article();
    let before = reader.text_area("The sea keeps its own hours.");

    reader.press("Ctrl++");

    let after = reader.text_area("The sea keeps its own hours.");
    assert!(
        after.width > before.width * 1.2,
        "{before:?} then {after:?}"
    );
}

#[test]
fn the_text_size_steps_through_five_sizes() {
    let mut reader = article();
    reader.click("Appearance");
    reader.assert_sees("Normal");

    reader.click("Smaller text");
    reader.assert_sees("Small");
    assert!(!reader.is_enabled("Smaller text"), "Small is the smallest");

    for size in &SIZES[1..] {
        reader.click("Larger text");
        reader.assert_sees(size);
    }
    assert!(!reader.is_enabled("Larger text"), "Huge is the largest");
}

#[test]
fn a_larger_text_size_sets_the_document_larger() {
    let mut reader = article();
    let before = reader.text_area("The sea keeps its own hours.");
    reader.click("Appearance");

    reader.click("Larger text");

    let after = reader.text_area("The sea keeps its own hours.");
    assert!(after.height > before.height, "{before:?} then {after:?}");
    assert!(after.width > before.width, "{before:?} then {after:?}");
}

#[test]
fn line_spacing_moves_the_lines_apart() {
    let mut reader = article();
    let gap = |reader: &Reader| reader.text_area("flow").top - reader.text_area("ebb").top;
    reader.click("Appearance");
    assert!(reader.is_on("Comfortable"));
    let comfortable = gap(&reader);

    reader.click("Compact");
    let compact = gap(&reader);
    reader.click("Airy");
    let airy = gap(&reader);

    assert!(reader.is_on("Airy"));
    assert!(compact < comfortable, "{compact} then {comfortable}");
    assert!(comfortable < airy, "{comfortable} then {airy}");
}

#[test]
fn escape_closes_the_appearance_popover() {
    let mut reader = article();
    reader.click("Appearance");
    reader.assert_sees("Line spacing");

    reader.press("Escape");

    reader.assert_hides("Line spacing");
}

#[test]
fn clicking_the_page_closes_the_appearance_popover() {
    let mut reader = article();
    reader.click("Appearance");
    let page = reader.column();

    reader.click_point(page.left + 20.0, page.bottom() - 20.0);

    reader.assert_hides("Line spacing");
}

#[test]
fn ctrl_q_quits() {
    let mut reader = article();
    assert!(!reader.has_quit());

    reader.press("Ctrl+Q");

    assert!(reader.has_quit());
}

#[test]
fn quit_in_the_menu_quits() {
    let mut reader = article();

    reader.click("More");
    reader.click("Quit");

    assert!(reader.has_quit());
}

#[test]
fn page_down_and_the_wheel_move_through_a_long_document() {
    let text = super::docs::long_document(80);
    let mut reader = Reader::with_docs(&[("long.md", &text)]);
    let opening = "Paragraph 1 mentions the lighthouse once.";
    assert!(reader.in_view(opening));

    reader.press("PageDown");
    assert!(!reader.in_view(opening));
    reader.press("PageUp");
    assert!(reader.in_view(opening));

    reader.scroll_down(900.0);
    assert!(!reader.in_view(opening));
}
