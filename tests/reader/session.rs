//! What the next launch remembers of the last one.

use super::docs::{kitchen_sink, three_linked};

#[test]
fn the_open_tabs_come_back_with_the_same_one_in_front() {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    reader.drop_file("c.md");
    reader.click("b.md");

    let reader = reader.restart(&[]);

    for tab in ["a.md", "b.md", "c.md"] {
        assert_eq!(reader.count(tab), 1, "{tab}");
    }
    assert!(reader.is_on("b.md"));
    reader.assert_sees("Beta");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn a_file_on_the_command_line_replaces_the_last_session() {
    let mut reader = three_linked();
    reader.drop_file("b.md");

    let reader = reader.restart(&["c.md"]);

    reader.assert_sees("Gamma");
    assert_eq!(reader.count("a.md"), 0, "no tab strip: one document");
    assert_eq!(reader.title(), "c.md - Markdown Viewer");
}

#[test]
fn a_tab_whose_file_is_gone_does_not_come_back() {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    reader.delete_file("b.md");

    let reader = reader.restart(&[]);

    reader.assert_sees("Alpha");
    reader.assert_hides("Unable to open");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
}

#[test]
fn a_session_with_no_tabs_starts_on_the_home_page() {
    let mut reader = three_linked();
    reader.press("Ctrl+W");

    let reader = reader.restart(&[]);

    reader.assert_sees("RECENTLY READ");
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn the_sidebars_come_back_as_they_were_left() {
    let mut reader = kitchen_sink();
    reader.press("Ctrl+Shift+E");
    reader.press("Ctrl+Shift+O");
    reader.assert_sees("FILES");
    reader.assert_hides("CONTENTS");

    let mut reader = reader.restart(&["sink.md"]);

    reader.assert_sees("FILES");
    reader.assert_hides("CONTENTS");
    reader.click("More");
    assert!(reader.is_on("Show Explorer"));
    assert!(!reader.is_on("Show Outline"));
}

#[test]
fn the_zoom_comes_back() {
    let mut reader = kitchen_sink();
    reader.press("Ctrl++");
    reader.press("Ctrl++");

    let mut reader = reader.restart(&["sink.md"]);

    reader.click("More");
    assert_eq!(reader.count("Reset Zoom, 156%"), 1);
}

#[test]
fn the_text_size_and_line_spacing_come_back() {
    let mut reader = kitchen_sink();
    reader.click("Appearance");
    reader.click("Larger text");
    reader.click("Airy");

    let mut reader = reader.restart(&["sink.md"]);

    reader.click("Appearance");
    reader.assert_sees("Large");
    assert!(reader.is_on("Airy"));
    assert!(!reader.is_on("Comfortable"));
}

#[test]
fn live_reload_is_a_choice_of_each_launch() {
    let mut reader = kitchen_sink();
    reader.press("F5");

    let mut reader = reader.restart(&["sink.md"]);

    reader.click("More");
    assert!(reader.is_on("Live Reload"));
}
