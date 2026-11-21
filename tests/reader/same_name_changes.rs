//! Tabs whose documents share a name, while a file changes under unsaved
//! edits, a question is asked about one of them, or the keyboard is on
//! one: each is still named and drawn so that it is told from the other.

use super::support::Reader;
use super::unsaved::edit;

const NOTICE: &str = "This file changed on disk. Your edits are not saved.";
/// What a tab is marked with while its notice waits behind another tab.
const CHANGED: &str = "Changed on disk";
/// The longest a tab is, in pixels.
const WIDEST_TAB: f32 = 200.0;

/// Each of `docs` open in a tab of its own, in that order.
fn open_each(docs: &[(&str, &str)]) -> Reader {
    let mut launch = Reader::launch();
    for (path, text) in docs {
        launch = launch.doc(path, text);
    }
    let mut reader = launch.args(&[docs[0].0]).open();
    for (path, _) in &docs[1..] {
        reader.drop_file(path);
    }
    reader
}

/// Notes in `guides` and Plan in `drafts`, Plan in front with an edit
/// that is not saved, and its file then given the heading Notes by
/// another program.
fn plan_renamed_on_disk_under_edits() -> Reader {
    let mut reader = open_each(&[
        ("guides/notes.md", "# Notes\n\nKept."),
        ("drafts/plan.md", "# Plan\n\nNothing yet."),
    ]);
    edit(&mut reader, " Mine.");
    reader.write_file("drafts/plan.md", "# Notes\n\nSomething now.");
    reader.wait_for("the notice", |reader| reader.sees(NOTICE));
    reader
}

#[test]
fn keep_mine_and_a_save_over_the_other_version_keep_the_name_of_my_text() {
    let mut reader = plan_renamed_on_disk_under_edits();

    reader.click("Keep mine");

    assert_eq!(reader.tabs(), ["Notes", "Plan"]);

    reader.press("Ctrl+S");
    reader.assert_sees("“Plan” (plan.md) changed on disk. Overwrite it?");
    reader.click("Overwrite");

    assert_eq!(reader.tabs(), ["Notes", "Plan"]);
    assert!(reader.read_file("drafts/plan.md").starts_with("# Plan"));
}

/// Notes in `guides` and Notes in `drafts`, the second in front with an
/// edit that is not saved, and its file then changed by another program.
fn namesake_changed_under_edits() -> Reader {
    let mut reader = open_each(&[
        ("guides/notes.md", "# Notes\n\nKept."),
        ("drafts/notes.md", "# Notes\n\nRough."),
    ]);
    edit(&mut reader, " Mine.");
    reader.write_file("drafts/notes.md", "# Notes\n\nWritten by another program.");
    reader.wait_for("the notice", |reader| reader.sees(NOTICE));
    reader
}

#[test]
fn both_questions_name_the_folder_of_a_file_name_two_tabs_share() {
    let mut reader = namesake_changed_under_edits();

    reader.press("Ctrl+W");

    reader.assert_sees("Save changes to “Notes” (notes.md · drafts)?");

    reader.click("Save");

    reader.assert_sees("“Notes” (notes.md · drafts) changed on disk. Overwrite it?");
    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · drafts"]);
}

#[test]
fn each_close_button_is_named_after_its_tab_folder_and_all() {
    let folders = [
        "correspondence-with-the-harbour-board-1998",
        "correspondence-with-the-harbour-board-2004",
    ];
    let files = folders.map(|folder| format!("{folder}/notes.md"));
    let mut reader = open_each(&[
        (&files[0], "# Notes\n\nOld."),
        (&files[1], "# Notes\n\nNew."),
    ]);

    let tabs = reader.tabs();
    assert_eq!(tabs, folders.map(|folder| format!("Notes · {folder}")));
    for tab in &tabs {
        assert_eq!(reader.count(&format!("tab:{tab}")), 1, "{tab}");
        assert_eq!(reader.count(&format!("Close {tab}")), 1, "{tab}");
    }

    reader.tab_to(&format!("Close {}", tabs[0]));
    reader.press("Enter");

    assert_eq!(reader.tabs(), ["Notes"]);
    reader.assert_sees("New.");
    assert_eq!(reader.count("Close Notes"), 1);
}

/// What is drawn in the square at the end of the tab named `tab`: whether
/// the cross of its close button is, and whether the dot of a changed
/// file is.
fn cross_and_dot(reader: &Reader, tab: &str) -> (bool, bool) {
    let square = reader.area_of(&format!("Close {tab}"));
    let dots = reader.dots_in(square).len();
    (reader.marks_in(square).len() > dots, dots > 0)
}

/// Where each of `tabs` starts and how wide it is.
fn places(reader: &Reader, tabs: &[String]) -> Vec<(f32, f32)> {
    let areas = tabs.iter().map(|tab| reader.area_of(&format!("tab:{tab}")));
    areas.map(|area| (area.left, area.width)).collect()
}

#[test]
fn a_marked_namesake_with_a_long_name_and_folder_keeps_its_parts_apart() {
    let heading = "# Minutes of the Quarterly Planning Committee Meeting\n\nAgreed.";
    let folders = [
        "correspondence-with-the-harbour-board-1998",
        "correspondence-with-the-harbour-board-2004",
    ];
    let files = folders.map(|folder| format!("{folder}/minutes.md"));
    let mut reader = Reader::launch()
        .doc(&files[0], heading)
        .doc(&files[1], heading)
        .args(&[&files[0]])
        .open();
    edit(&mut reader, " Mine.");
    reader.drop_file(&files[1]);
    reader.write_file(
        &files[0],
        "# Minutes of the Quarterly Planning Committee Meeting\n\nNo.",
    );
    reader.wait_for("the tab to be marked", |reader| reader.count(CHANGED) == 1);
    reader.hover("More");

    let tabs = reader.tabs();
    let marked = tabs[0].clone();
    let label = format!("tab:{marked}");
    let at_rest = places(&reader, &tabs);
    assert_eq!(cross_and_dot(&reader, &marked), (false, true), "at rest");

    for keyboard_on_it in [false, true] {
        if keyboard_on_it {
            reader.tab_to(&label);
            assert!(reader.ring_around(&label).is_some(), "the ring shows");
            assert_eq!(cross_and_dot(&reader, &marked), (true, false));
        }
        let tab = reader.area_of(&label);
        let square = reader.area_of(&format!("Close {marked}"));
        let words_end = reader.end_of("board-1998");
        assert!(tab.width <= WIDEST_TAB, "the tab is {} wide", tab.width);
        assert!(
            words_end <= square.left,
            "the folder ends at {words_end}, the square starts at {}",
            square.left
        );
        assert!(square.right() <= tab.right(), "the square is in the tab");
        assert_eq!(places(&reader, &tabs), at_rest, "nothing moves");
        let reads = reader.tab_reads(&label);
        assert!(reads.starts_with("Minutes"), "{reads}");
        assert!(reads.ends_with("board-1998"), "{reads}");
    }
}
