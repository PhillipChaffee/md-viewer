//! Tabs whose documents share a name: each shows its folder too.

use super::support::{Launch, Reader};

/// The longest a tab is, in pixels.
const WIDEST_TAB: f32 = 200.0;

/// A launch with a document that starts `# Notes` at each of `paths`,
/// each saying which one it is.
fn notes_at(paths: &[&str]) -> Launch {
    let mut launch = Reader::launch();
    for path in paths {
        launch = launch.doc(path, &format!("# Notes\n\nKept at {path}."));
    }
    launch
}

/// Each of `paths` open in a tab of its own, in that order, each a
/// document that starts `# Notes`.
fn notes_open(paths: &[&str]) -> Reader {
    let mut reader = notes_at(paths).args(&paths[..1]).open();
    reader.drop_files(&paths[1..]);
    reader
}

/// Put the document in front in edit mode and type `text` at its end.
fn type_at_the_end(reader: &mut Reader, text: &str) {
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.press("Ctrl+End");
    reader.type_text(text);
}

#[test]
fn two_tabs_with_one_name_each_show_their_folder() {
    let mut reader = notes_open(&["guides/notes.md", "drafts/notes.md"]);

    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · drafts"]);
    assert_eq!(reader.tab_in_front().as_deref(), Some("Notes · drafts"));

    reader.click("tab:Notes · guides");

    reader.assert_sees("Kept at guides/notes.md.");
    assert_eq!(reader.tab_reads("tab:Notes · guides"), "Notes · guides");
    assert_eq!(reader.tab_reads("tab:Notes · drafts"), "Notes · drafts");
}

#[test]
fn three_tabs_with_one_name_each_show_their_folder() {
    let reader = notes_open(&["guides/notes.md", "drafts/notes.md", "archive/notes.md"]);

    assert_eq!(
        reader.tabs(),
        ["Notes · guides", "Notes · drafts", "Notes · archive"]
    );
}

#[test]
fn folders_of_one_name_show_the_folder_above_them_too() {
    let reader = notes_open(&["a/guides/notes.md", "b/guides/notes.md"]);

    assert_eq!(reader.tabs(), ["Notes · a/guides", "Notes · b/guides"]);
}

#[test]
fn only_the_folders_that_share_a_name_show_the_folder_above() {
    let reader = notes_open(&["a/guides/notes.md", "drafts/notes.md", "b/guides/notes.md"]);

    assert_eq!(
        reader.tabs(),
        ["Notes · a/guides", "Notes · drafts", "Notes · b/guides"]
    );
}

#[test]
fn documents_of_one_name_in_one_folder_show_their_files() {
    let reader = notes_open(&["guides/monday.md", "guides/tuesday.md"]);

    assert_eq!(
        reader.tabs(),
        ["Notes · guides/monday.md", "Notes · guides/tuesday.md"]
    );
}

#[test]
fn a_tab_with_a_name_of_its_own_is_left_as_it_is() {
    let mut reader = notes_at(&["guides/notes.md", "drafts/notes.md"])
        .doc("guides/alpha.md", "# Alpha\n\nFirst.")
        .args(&["guides/alpha.md"])
        .open();
    let alone = reader.area_of("tab:Alpha");

    reader.drop_files(&["guides/notes.md", "drafts/notes.md"]);

    assert_eq!(reader.tabs(), ["Alpha", "Notes · guides", "Notes · drafts"]);
    assert_eq!(reader.tab_reads("tab:Alpha"), "Alpha");
    assert_eq!(reader.area_of("tab:Alpha"), alone);
}

#[test]
fn the_last_tab_of_a_name_goes_back_to_the_name_alone() {
    let mut reader = notes_open(&["guides/notes.md", "drafts/notes.md", "archive/notes.md"]);

    reader.middle_click("tab:Notes · drafts");

    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · archive"]);

    reader.middle_click("tab:Notes · archive");

    assert_eq!(reader.tabs(), ["Notes"]);
    assert_eq!(reader.tab_reads("tab:Notes"), "Notes");
}

#[test]
fn typing_a_heading_another_tab_has_shows_both_folders() {
    let mut reader = notes_at(&["guides/notes.md"])
        .doc("drafts/plan.md", "Nothing yet.")
        .args(&["guides/notes.md"])
        .open();
    reader.drop_file("drafts/plan.md");
    assert_eq!(reader.tabs(), ["Notes", "plan.md"]);

    type_at_the_end(&mut reader, "\n\n# Notes");

    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · drafts"]);

    reader.type_text(" of mine");

    assert_eq!(reader.tabs(), ["Notes", "Notes of mine"]);
}

#[test]
fn a_file_that_changes_to_a_heading_another_tab_has_shows_both_folders() {
    let mut reader = notes_at(&["guides/notes.md"])
        .doc("drafts/plan.md", "# Plan\n\nNothing yet.")
        .args(&["guides/notes.md"])
        .open();
    reader.drop_file("drafts/plan.md");
    assert_eq!(reader.tabs(), ["Notes", "Plan"]);

    reader.write_file("drafts/plan.md", "# Notes\n\nSomething now.");
    reader.wait_for("the page to reload", |reader| reader.sees("Something now."));

    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · drafts"]);
}

#[test]
fn following_a_link_to_a_name_another_tab_has_shows_both_folders() {
    let mut reader = notes_at(&["guides/notes.md", "drafts/notes.md"])
        .doc("drafts/index.md", "# Index\n\nSee [the notes](notes.md).")
        .args(&["guides/notes.md"])
        .open();
    reader.drop_file("drafts/index.md");
    assert_eq!(reader.tabs(), ["Notes", "Index"]);

    reader.click("the notes");

    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · drafts"]);

    reader.click("Go back");

    assert_eq!(reader.tabs(), ["Notes", "Index"]);
}

#[test]
fn one_document_in_two_tabs_keeps_its_name_alone() {
    let mut reader = notes_at(&["guides/notes.md"])
        .doc("guides/index.md", "# Index\n\nSee [the notes](notes.md).")
        .args(&["guides/notes.md"])
        .open();
    reader.drop_file("guides/index.md");

    reader.click("the notes");

    assert_eq!(reader.tabs(), ["Notes", "Notes"]);
}

#[test]
fn one_document_in_two_tabs_shows_its_folder_beside_another_of_its_name() {
    let mut reader = notes_at(&["guides/notes.md", "drafts/notes.md"])
        .doc("guides/index.md", "# Index\n\nSee [the notes](notes.md).")
        .args(&["guides/notes.md"])
        .open();
    reader.drop_file("guides/index.md");
    reader.click("the notes");

    reader.drop_file("drafts/notes.md");

    assert_eq!(
        reader.tabs(),
        ["Notes · guides", "Notes · guides", "Notes · drafts"]
    );
}

#[test]
fn a_restored_session_shows_the_folders_again() {
    let reader = notes_open(&["guides/notes.md", "drafts/notes.md"]);

    let reader = reader.restart(&[]);

    assert_eq!(reader.tabs(), ["Notes · guides", "Notes · drafts"]);
}

#[test]
fn a_long_name_is_cut_short_before_its_folder() {
    let heading = "# Minutes of the Quarterly Planning Committee Meeting\n\nAgreed.";
    let mut reader = Reader::launch()
        .doc("guides/minutes.md", heading)
        .doc("drafts/minutes.md", heading)
        .args(&["guides/minutes.md"])
        .open();
    reader.drop_file("drafts/minutes.md");

    let tabs = reader.tabs();
    assert_eq!(tabs.len(), 2);
    for (tab, folder) in tabs.iter().zip(["guides", "drafts"]) {
        let label = format!("tab:{tab}");
        assert!(tab.ends_with(&format!("Meeting · {folder}")), "{tab}");
        let reads = reader.tab_reads(&label);
        assert!(reads.starts_with("Minutes of"), "{reads}");
        assert!(reads.ends_with(&format!("… · {folder}")), "{reads}");
        let width = reader.area_of(&label).width;
        assert!(width <= WIDEST_TAB, "{tab} is {width} wide");
    }
}

#[test]
fn a_long_folder_leaves_the_start_of_the_name_in_view() {
    let reader = notes_open(&[
        "correspondence-with-the-harbour-board-1998/notes.md",
        "correspondence-with-the-harbour-board-2004/notes.md",
    ]);

    for tab in reader.tabs() {
        let label = format!("tab:{tab}");
        let reads = reader.tab_reads(&label);
        assert!(reads.starts_with("Notes · corr"), "{reads}");
        assert!(reads.ends_with('…'), "{reads}");
        assert!(reader.area_of(&label).width <= WIDEST_TAB, "{tab}");
    }
}
