//! The contents sidebar, in depth: which headings it lists, how they nest
//! and fold, which one is marked while reading, and the width of both
//! sidebars.

use std::fmt::Write as _;

use super::docs::long_document;
use super::support::Reader;

/// Headings four levels deep, with one title used twice.
const HANDBOOK: &str = "# Handbook\n\nWelcome.\n\n\
    ## Setup\n\nBefore you start.\n\n\
    ### Install\n\nGet the package.\n\n\
    #### Linux\n\nUse the package manager.\n\n\
    ### Configure\n\nEdit the file.\n\n\
    ## Usage\n\nDay to day.\n\n\
    ### Install\n\nInstalling plugins.\n\n\
    ## FAQ\n\nAsk away.\n";

fn handbook() -> Reader {
    Reader::with_docs(&[("handbook.md", HANDBOOK)])
}

/// The headings the contents sidebar lists, top to bottom, without the
/// controls that fold them.
fn entries(reader: &Reader) -> Vec<String> {
    let header = reader.area_of("CONTENTS");
    let mut listed = reader.names_from(header.left, header.bottom());
    listed.retain(|name| !name.starts_with("Collapse ") && !name.starts_with("Expand "));
    listed
}

/// The entries painted unlike most of the others: the heading being read.
fn marked_entries(reader: &Reader) -> Vec<String> {
    // The sidebar is painted before the page, so an entry's ink comes first.
    let ink = |entry: &String| reader.ink_of_each(entry).first().copied();
    // An entry below the window's bottom edge is not painted at all.
    let mut listed = entries(reader);
    listed.retain(|entry| ink(entry).is_some());
    let common = listed
        .iter()
        .map(ink)
        .max_by_key(|one| listed.iter().filter(|entry| ink(entry) == *one).count());
    let unlike = listed.iter().filter(|entry| Some(ink(entry)) != common);
    unlike.cloned().collect()
}

/// Twelve chapters, each with a section called `Notes`.
fn chapters_with_notes() -> Reader {
    let mut text = String::from("# Journal\n\n");
    for chapter in 1..=12 {
        let _ = write!(
            text,
            "## Chapter {chapter}\n\nThe story of chapter {chapter}.\n\n\
             ### Notes\n\nNotes kept for chapter {chapter}.\n\n\
             More of chapter {chapter}.\n\nStill more of chapter {chapter}.\n\n"
        );
    }
    Reader::with_docs(&[("journal.md", &text)])
}

/// Where a sidebar's surface ends: the first pixel from `x`, stepping by
/// `step`, that is not the color at `x`.
fn edge_from(reader: &Reader, x: f32, step: f32) -> f32 {
    let y = 400.0;
    let surface = reader.color_at(x, y);
    let mut at = x;
    while reader.color_at(at, y) == surface {
        at += step;
        assert!((0.0..1600.0).contains(&at), "no edge found from {x}");
    }
    at
}

/// The contents sidebar's left edge.
fn contents_edge(reader: &Reader) -> f32 {
    edge_from(reader, reader.area_of("CONTENTS").left, -1.0)
}

/// The files sidebar's right edge. Its header reads `FILES <folder>`.
fn files_edge(reader: &Reader) -> f32 {
    edge_from(reader, 100.0, 1.0)
}

#[test]
fn every_heading_is_listed_in_the_order_of_the_page() {
    let reader = handbook();

    assert_eq!(
        entries(&reader),
        [
            "Handbook",
            "Setup",
            "Install",
            "Linux",
            "Configure",
            "Usage",
            "Install",
            "FAQ"
        ]
    );
}

#[test]
fn deeper_headings_sit_further_in() {
    let reader = handbook();

    let fold = |heading: &str| reader.area_of(&format!("Collapse {heading}")).left;
    assert!(fold("Handbook") <= fold("Setup"));
    assert!(fold("Setup") < fold("Install"));
    assert!((fold("Setup") - fold("Usage")).abs() < 0.5, "same level");
}

#[test]
fn only_a_heading_with_headings_under_it_can_be_folded() {
    let reader = handbook();

    for parent in ["Handbook", "Setup", "Install", "Usage"] {
        assert_eq!(reader.count(&format!("Collapse {parent}")), 1, "{parent}");
    }
    for leaf in ["Linux", "Configure", "FAQ"] {
        assert_eq!(reader.count(&format!("Collapse {leaf}")), 0, "{leaf}");
    }
}

#[test]
fn folding_a_heading_hides_all_under_it_and_nothing_else() {
    let mut reader = handbook();

    reader.click("Collapse Setup");

    assert_eq!(
        entries(&reader),
        ["Handbook", "Setup", "Usage", "Install", "FAQ"]
    );
    assert_eq!(reader.count("Expand Setup"), 1);
    reader.assert_sees("Use the package manager.");
}

#[test]
fn unfolding_brings_back_what_was_under_the_heading() {
    let mut reader = handbook();
    let whole = entries(&reader);
    reader.click("Collapse Setup");

    reader.click("Expand Setup");

    assert_eq!(entries(&reader), whole);
}

#[test]
fn a_fold_inside_a_fold_is_remembered() {
    let mut reader = handbook();
    reader.click("Collapse Install");
    assert_eq!(reader.count("Linux"), 1, "only the heading in the page");

    reader.click("Collapse Setup");
    reader.click("Expand Setup");

    assert_eq!(reader.count("Expand Install"), 1);
    assert_eq!(reader.count("Linux"), 1);
    assert_eq!(reader.count("Configure"), 2);
}

#[test]
fn the_options_menu_folds_and_unfolds_everything() {
    let mut reader = handbook();
    let whole = entries(&reader);

    reader.click("Outline options");
    reader.click("Collapse All");
    assert_eq!(entries(&reader), ["Handbook"]);

    reader.click("Outline options");
    reader.click("Expand All");
    assert_eq!(entries(&reader), whole);
}

#[test]
fn a_flat_outline_has_no_options_menu() {
    let reader = Reader::with_docs(&[("flat.md", "# One\n\nText.\n\n# Two\n\nText.")]);

    assert_eq!(entries(&reader), ["One", "Two"]);
    assert_eq!(reader.count("Outline options"), 0);
}

#[test]
fn an_entry_reads_as_the_heading_does_without_its_markup() {
    let reader = Reader::with_docs(&[(
        "styled.md",
        "# The *big* `code` [linked](other.md) day\n\nText.\n\nSecond title\n============\n\n\
         Under line\n----------\n\nText.\n",
    )]);

    assert_eq!(
        entries(&reader),
        ["The big code linked day", "Second title", "Under line"]
    );
}

#[test]
fn a_hash_line_inside_a_code_block_is_not_a_heading() {
    let reader = Reader::with_docs(&[(
        "code.md",
        "# Real\n\n```sh\n# a comment, not a heading\n```\n\n## Also real\n",
    )]);

    assert_eq!(entries(&reader), ["Real", "Also real"]);
}

#[test]
fn the_entries_follow_the_tab_in_front() {
    let mut reader = Reader::with_docs(&[
        ("handbook.md", HANDBOOK),
        ("other.md", "# Other\n\n## Only section\n"),
    ]);

    reader.drop_file("other.md");
    assert_eq!(entries(&reader), ["Other", "Only section"]);

    reader.click("handbook.md");
    assert_eq!(entries(&reader).len(), 8);
}

#[test]
fn a_fold_is_kept_while_another_tab_is_in_front() {
    let mut reader = Reader::with_docs(&[("handbook.md", HANDBOOK), ("other.md", "# Other")]);
    reader.click("Collapse Setup");

    reader.drop_file("other.md");
    reader.click("handbook.md");

    assert_eq!(reader.count("Expand Setup"), 1);
}

#[test]
fn each_of_two_headings_with_one_title_leads_to_its_own_place() {
    let mut reader = chapters_with_notes();

    for chapter in [9, 3] {
        // In reading order the sidebar's entries come before the page's.
        reader.click_nth("Notes", chapter - 1);
        reader.wait_for("the chapter's notes to scroll into view", |reader| {
            reader.in_view(&format!("Notes kept for chapter {chapter}."))
        });

        let above = format!("Notes kept for chapter {}.", chapter - 1);
        assert!(!reader.in_view(&above), "chapter {chapter}");
    }
}

#[test]
fn the_mark_moves_to_the_heading_being_read_while_scrolling() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(80))]);

    reader.scroll_down(2000.0);
    let marked = marked_entries(&reader);
    assert_eq!(marked.len(), 1, "{marked:?}");
    assert!(reader.in_view(&marked[0]), "{marked:?} is on screen");

    reader.scroll_down(700.0);
    let further = marked_entries(&reader);
    assert_eq!(further.len(), 1, "{further:?}");
    assert_ne!(further, marked);

    reader.scroll_up(100_000.0);
    let at_the_top = marked_entries(&reader);
    assert!(!at_the_top.contains(&further[0]), "{at_the_top:?}");
}

#[test]
fn clicking_an_entry_marks_it() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(80))]);

    reader.click("Section 12");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Paragraph 12 mentions the lighthouse once.")
    });

    assert_eq!(marked_entries(&reader), ["Section 12"]);
}

#[test]
fn an_entry_far_down_the_page_can_be_reached_and_the_first_one_leads_back() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(20))]);

    reader.click("Section 20");
    reader.wait_for("the last section to scroll into view", |reader| {
        reader.in_view("Paragraph 20 mentions the lighthouse once.")
    });
    assert!(!reader.in_view("Section 1"));

    reader.click("Long read");
    reader.wait_for("the top to scroll into view", |reader| {
        reader.in_view("Section 1")
    });
}

#[test]
fn dragging_the_contents_sidebars_edge_makes_it_wider() {
    let mut reader = handbook();
    let edge = contents_edge(&reader);
    let column = reader.column();

    reader.drag_point(edge, 400.0, -90.0, 0.0);

    let moved = contents_edge(&reader);
    assert!((edge - moved - 90.0).abs() <= 3.0, "{edge} to {moved}");
    assert!(reader.column().left < column.left, "the column made room");
}

#[test]
fn dragging_the_files_sidebars_edge_makes_it_wider() {
    let mut reader = handbook();
    reader.press("Ctrl+Shift+E");
    let edge = files_edge(&reader);
    let column = reader.column();

    reader.drag_point(edge, 400.0, 110.0, 0.0);

    let moved = files_edge(&reader);
    assert!((moved - edge - 110.0).abs() <= 3.0, "{edge} to {moved}");
    assert!(reader.column().left > column.left, "the column made room");
}

#[test]
fn a_sidebar_cannot_be_dragged_away_to_nothing() {
    let mut reader = handbook();
    let edge = contents_edge(&reader);

    reader.drag_point(edge, 400.0, 190.0, 0.0);

    reader.assert_sees("CONTENTS");
    let width = 1200.0 - contents_edge(&reader);
    assert!((40.0..150.0).contains(&width), "{width} px wide");
}

#[test]
fn both_widths_come_back_after_a_restart() {
    let launch = Reader::launch().doc("handbook.md", HANDBOOK);
    let mut reader = launch.args(&["handbook.md"]).window(1600.0, 800.0).open();
    reader.press("Ctrl+Shift+E");
    let (files, contents) = (files_edge(&reader), contents_edge(&reader));
    reader.drag_point(files, 400.0, 64.0, 0.0);
    reader.drag_point(contents, 400.0, -48.0, 0.0);
    let (files, contents) = (files_edge(&reader), contents_edge(&reader));

    let reader = reader.restart(&["handbook.md"]);

    assert!((files_edge(&reader) - files).abs() <= 1.0);
    assert!((contents_edge(&reader) - contents).abs() <= 1.0);
}

#[test]
fn the_files_sidebar_stays_in_a_narrow_window_while_the_contents_make_way() {
    let mut reader = handbook();
    reader.press("Ctrl+Shift+E");

    reader.resize_window(700.0, 800.0);

    reader.assert_sees("FILES");
    reader.assert_hides("CONTENTS");
    reader.assert_sees("Welcome.");
}

#[test]
fn contents_hidden_by_the_reader_stay_hidden_when_the_window_grows() {
    let mut reader = handbook();
    reader.press("Ctrl+Shift+O");

    reader.resize_window(1600.0, 900.0);

    reader.assert_hides("CONTENTS");
}

#[test]
fn zooming_in_far_enough_makes_the_contents_make_way() {
    let mut reader = handbook();
    reader.assert_sees("CONTENTS");

    for _ in 0..4 {
        reader.press("Ctrl++");
    }
    reader.assert_hides("CONTENTS");

    reader.press("Ctrl+0");
    reader.assert_sees("CONTENTS");
}
