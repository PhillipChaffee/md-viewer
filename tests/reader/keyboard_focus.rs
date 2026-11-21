//! Where the keyboard is: the ring round whatever the Tab key reached.

use super::support::{Reader, Rgb};

/// A folder of documents, open on `readme.md` with the files sidebar
/// showing.
fn desk() -> Reader {
    let mut reader = Reader::launch()
        .doc("readme.md", "# Read me\n\n## Setup\n\nStart here.")
        .doc("changes.md", "# Changes\n\nWhat is new.")
        .doc("notes.md", "# Notes\n\nKept short.")
        .args(&["readme.md"])
        .open();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("changes.md") == 1
    });
    reader
}

/// Take the pointer off the sidebars.
fn point_away(reader: &mut Reader) {
    reader.hover("More");
}

/// The ring a button of the top row has while the keyboard is on it.
#[track_caller]
fn button_ring(reader: &mut Reader) -> Rgb {
    reader.tab_to("More");
    let ring = reader.ring_around("More");
    ring.expect("a top-row button the keyboard is on has a ring")
}

/// Reach `label` with the Tab key and check that it has the ring of a
/// top-row button then, and none at rest or under the pointer.
#[track_caller]
fn assert_ring_follows_the_keyboard(reader: &mut Reader, label: &str) {
    let ring = button_ring(reader);
    assert_eq!(reader.ring_around(label), None, "{label}, at rest");
    reader.hover(label);
    assert_eq!(reader.ring_around(label), None, "{label}, pointed at");
    point_away(reader);

    reader.tab_to(label);

    assert_eq!(reader.ring_around(label), Some(ring), "{label}, focused");
    reader.press("Tab");
    assert_eq!(reader.ring_around(label), None, "{label}, left behind");
}

#[test]
fn a_file_the_keyboard_is_on_has_the_ring_of_a_top_row_button() {
    let mut reader = desk();

    assert_ring_follows_the_keyboard(&mut reader, "changes.md");
}

#[test]
fn a_heading_of_the_contents_the_keyboard_is_on_has_the_ring_too() {
    let mut reader = desk();

    assert_ring_follows_the_keyboard(&mut reader, "Setup");
}

#[test]
fn the_menu_button_of_each_sidebar_has_the_ring_while_the_keyboard_is_on_it() {
    let mut reader = desk();

    assert_ring_follows_the_keyboard(&mut reader, "Explorer options");
    assert_ring_follows_the_keyboard(&mut reader, "Outline options");
}
