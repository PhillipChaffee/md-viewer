//! Closing a document with unsaved edits: the prompt that asks first, what
//! each answer does, and the same when several documents or the whole app
//! are closed.

use super::docs::{long_document, three_linked};
use super::support::Reader;

const SOURCE: &str = "# Draft\n\nFirst thoughts.";
const EDITED: &str = "# Draft\n\nFirst thoughts. Mine.";
const ASKS_DRAFT: &str = "Save changes to “Draft” (draft.md)?";

/// Put the document in front in edit mode and type `text` at its end.
fn edit(reader: &mut Reader, text: &str) {
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.press("Ctrl+End");
    reader.type_text(text);
}

/// `draft.md` open with an edit that is not saved.
fn unsaved_draft() -> Reader {
    let mut reader = Reader::with_docs(&[("draft.md", SOURCE)]);
    edit(&mut reader, " Mine.");
    reader
}

/// `a.md`, `b.md` and `c.md` in three tabs with `c.md` in front, the first
/// two with an edit that is not saved.
fn two_unsaved_of_three() -> Reader {
    let mut reader = three_linked();
    edit(&mut reader, " Edit A.");
    reader.drop_file("b.md");
    edit(&mut reader, " Edit B.");
    reader.drop_file("c.md");
    reader
}

/// What `name` held when `three_linked` wrote it.
fn as_written(name: &str) -> &'static str {
    match name {
        "a.md" => "# Alpha\n\nGo [to beta](b.md) or [to gamma](c.md) or [nowhere](missing.md).",
        "b.md" => "# Beta\n\nGo [to alpha](a.md) or [on to gamma](c.md).",
        other => panic!("{other} is not one of the two that are edited"),
    }
}

/// Which of the three documents still have a tab. A tab carries its
/// document's heading.
fn tabs(reader: &Reader) -> Vec<&'static str> {
    let open = reader.tabs();
    let documents = [("a.md", "Alpha"), ("b.md", "Beta"), ("c.md", "Gamma")];
    let with_a_tab = documents
        .into_iter()
        .filter(|(_, heading)| open.iter().any(|tab| tab == heading));
    with_a_tab.map(|(file, _)| file).collect()
}

#[test]
fn closing_a_tab_with_unsaved_edits_asks_and_touches_nothing() {
    let mut reader = unsaved_draft();

    reader.press("Ctrl+W");

    reader.assert_sees(ASKS_DRAFT);
    for answer in ["Save", "Discard", "Cancel"] {
        assert_eq!(reader.count(answer), 1, "{answer}");
    }
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    assert_eq!(reader.read_file("draft.md"), SOURCE);
    assert_eq!(reader.files_in(""), ["draft.md"]);
}

#[test]
fn save_writes_the_edits_and_closes_the_tab() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.click("Save");

    assert_eq!(reader.read_file("draft.md"), EDITED);
    assert_eq!(reader.files_in(""), ["draft.md"]);
    assert_eq!(reader.title(), "Markdown Viewer");
    reader.assert_hides(ASKS_DRAFT);
}

#[test]
fn discard_closes_the_tab_and_leaves_the_file_as_it_was() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.click("Discard");

    assert_eq!(reader.read_file("draft.md"), SOURCE);
    assert_eq!(reader.files_in(""), ["draft.md"]);
    assert_eq!(reader.title(), "Markdown Viewer");
    reader.assert_hides(ASKS_DRAFT);
}

#[test]
fn cancel_leaves_the_tab_open_with_its_edits() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.click("Cancel");

    reader.assert_hides(ASKS_DRAFT);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    assert_eq!(reader.typed(), [EDITED]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), SOURCE);
}

#[test]
fn escape_answers_as_cancel_does() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.press("Escape");

    reader.assert_hides(ASKS_DRAFT);
    assert_eq!(reader.typed(), [EDITED]);
    assert_eq!(reader.count("Unsaved changes"), 1);
    assert_eq!(reader.read_file("draft.md"), SOURCE);
}

#[test]
fn clicking_beside_the_prompt_leaves_it_asking() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.click_point(20.0, 780.0);

    reader.assert_sees(ASKS_DRAFT);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
}

#[test]
fn enter_saves_because_the_keyboard_starts_on_save() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.press("Enter");

    assert_eq!(reader.read_file("draft.md"), EDITED);
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn tab_moves_the_keyboard_from_save_to_discard_to_cancel() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");
    reader.press("Tab");
    reader.press("Enter");
    assert_eq!(
        reader.read_file("draft.md"),
        SOURCE,
        "Discard writes nothing"
    );
    assert_eq!(reader.title(), "Markdown Viewer", "Discard closes");

    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");
    reader.press("Tab");
    reader.press("Tab");
    reader.press("Enter");
    reader.assert_hides(ASKS_DRAFT);
    assert_eq!(reader.typed(), [EDITED], "Cancel keeps the tab");
}

#[test]
fn a_save_that_fails_keeps_the_tab_and_says_why() {
    let mut reader = Reader::with_docs(&[("notes/draft.md", SOURCE)]);
    edit(&mut reader, " Mine.");
    reader.delete_folder("notes");
    reader.press("Ctrl+W");

    reader.click("Save");

    reader.assert_hides("Save changes to");
    reader.assert_sees("Unable to save");
    reader.assert_sees("draft.md");
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    assert_eq!(reader.typed(), [EDITED]);
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn a_tab_whose_edits_are_saved_closes_without_a_question() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+S");

    reader.press("Ctrl+W");

    reader.assert_hides("Save changes to");
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn every_way_to_close_a_tab_asks() {
    type Close = fn(&mut Reader);
    let ways: [(&str, Close); 5] = [
        ("the shortcut", |reader| reader.press("Ctrl+W")),
        ("the menu", |reader| {
            reader.click("More");
            reader.click("Close Tab");
        }),
        ("the tab's close button", |reader| {
            reader.hover("tab:Alpha");
            reader.click("Close tab");
        }),
        ("a middle click on the tab", |reader| {
            reader.middle_click("tab:Alpha");
        }),
        ("the tab's menu", |reader| {
            reader.right_click("tab:Alpha");
            reader.click("Close");
        }),
    ];

    for (way, close) in ways {
        let mut reader = three_linked();
        reader.drop_file("b.md");
        reader.click("tab:Alpha");
        edit(&mut reader, " Edit A.");

        close(&mut reader);

        assert!(reader.sees("Save changes to “Alpha” (a.md)?"), "{way}");
        assert_eq!(tabs(&reader), ["a.md", "b.md"], "{way}");
        assert_eq!(reader.read_file("a.md"), as_written("a.md"), "{way}");
    }
}

#[test]
fn a_middle_click_in_the_files_sidebar_asks_too() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("draft.md") >= 1
    });
    let rows = reader.count("draft.md");

    reader.middle_click_nth("draft.md", rows - 1);

    reader.assert_sees(ASKS_DRAFT);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
}

#[test]
fn a_tab_behind_the_one_in_front_is_asked_about_by_name() {
    let mut reader = two_unsaved_of_three();

    reader.middle_click("tab:Beta");

    reader.assert_sees("Save changes to “Beta” (b.md)?");
    reader.click("Discard");
    assert_eq!(tabs(&reader), ["a.md", "c.md"]);
    assert_eq!(reader.title(), "c.md - Markdown Viewer");
    assert_eq!(reader.read_file("b.md"), as_written("b.md"));
}

/// What the prompt asks, word for word.
fn question(reader: &Reader) -> String {
    let mut asked = reader.names();
    asked.retain(|name| name.starts_with("Save changes to"));
    assert_eq!(asked.len(), 1, "{asked:?}");
    asked.remove(0)
}

#[test]
fn the_prompt_names_the_document_as_its_tab_does_and_by_its_file() {
    let mut reader = Reader::with_docs(&[
        ("titled.md", "# A Tale of Two Tabs\n\nIt was."),
        ("plain.md", "No heading in here."),
        ("named.md", "# named.md\n\nNamed as its file is."),
    ]);
    reader.drop_files(&["plain.md", "named.md"]);
    let cases = [
        (
            "A Tale of Two Tabs",
            "Save changes to “A Tale of Two Tabs” (titled.md)?",
        ),
        ("plain.md", "Save changes to plain.md?"),
        ("named.md", "Save changes to named.md?"),
    ];

    for (tab, asks) in cases {
        reader.click(&format!("tab:{tab}"));
        edit(&mut reader, " More.");

        reader.press("Ctrl+W");

        assert_eq!(question(&reader), asks);
        reader.click("Cancel");
    }
}

#[test]
fn the_prompt_follows_a_heading_that_was_just_typed() {
    let mut reader = Reader::with_docs(&[("plain.md", "No heading in here.")]);
    reader.press("Ctrl+E");
    reader.click_text_field(0);
    reader.press("Ctrl+Home");
    reader.type_text("# Fresh\n\n");
    assert_eq!(reader.tabs(), ["Fresh"]);

    reader.press("Ctrl+W");

    assert_eq!(question(&reader), "Save changes to “Fresh” (plain.md)?");
}

#[test]
fn a_long_heading_and_a_long_file_name_fit_in_the_prompt() {
    let heading = "Unabridged ".repeat(40);
    let file = format!("{}.md", "a-very-long-file-name-".repeat(8));
    let mut reader = Reader::with_docs(&[(&file, &format!("# {heading}\n\nText."))]);
    edit(&mut reader, " More.");

    reader.press("Ctrl+W");

    let asked = question(&reader);
    assert!(asked.starts_with("Save changes to “Unabridged Unabridged "));
    assert!(asked.ends_with(&format!("…” ({file})?")), "{asked}");
    let (sheet, window) = (reader.text_area(&asked), (1200.0, 800.0));
    assert!(sheet.width <= 340.0, "{sheet:?}");
    assert!(sheet.height <= 200.0, "a few lines at most: {sheet:?}");
    for answer in ["Save", "Discard", "Cancel"] {
        let button = reader.area_of(answer);
        assert!(
            button.top > sheet.bottom(),
            "{answer} is under the question"
        );
        assert!(button.left >= sheet.left && button.right() <= sheet.right());
        assert!(button.bottom() < window.1 && button.right() < window.0);
    }
    for word in ["Unabridged", "a-very-long-file-name-"] {
        let painted = reader.all_text().join("\n");
        assert!(painted.contains(word), "{word} is painted");
    }
}

#[test]
fn close_others_asks_about_each_unsaved_document_in_turn() {
    let mut reader = two_unsaved_of_three();
    reader.right_click("tab:Gamma");
    reader.click("Close Others");

    reader.assert_sees("Save changes to “Alpha” (a.md)?");
    reader.click("Save");

    reader.assert_sees("Save changes to “Beta” (b.md)?");
    assert_eq!(
        tabs(&reader),
        ["a.md", "b.md", "c.md"],
        "nothing closes before the last answer"
    );
    reader.click("Discard");

    reader.assert_hides("Save changes to");
    assert_eq!(reader.count("tab:Alpha") + reader.count("tab:Beta"), 0);
    assert_eq!(reader.title(), "c.md - Markdown Viewer");
    reader.assert_sees("The end of the line.");
    assert_eq!(
        reader.read_file("a.md"),
        format!("{} Edit A.", as_written("a.md"))
    );
    assert_eq!(reader.read_file("b.md"), as_written("b.md"));
}

#[test]
fn cancel_stops_close_others_and_closes_nothing() {
    let mut reader = two_unsaved_of_three();
    reader.right_click("tab:Gamma");
    reader.click("Close Others");
    reader.click("Discard");

    reader.click("Cancel");

    reader.assert_hides("Save changes to");
    assert_eq!(tabs(&reader), ["a.md", "b.md", "c.md"]);
    reader.click("tab:Alpha");
    assert_eq!(
        reader.typed(),
        [format!("{} Edit A.", as_written("a.md"))],
        "a discard that closed nothing threw nothing away"
    );
    assert_eq!(reader.read_file("a.md"), as_written("a.md"));
}

#[test]
fn a_save_before_a_cancel_stays_saved() {
    let mut reader = two_unsaved_of_three();
    reader.right_click("tab:Gamma");
    reader.click("Close Others");
    reader.click("Save");

    reader.click("Cancel");

    assert_eq!(tabs(&reader), ["a.md", "b.md", "c.md"]);
    assert_eq!(
        reader.read_file("a.md"),
        format!("{} Edit A.", as_written("a.md"))
    );
    assert_eq!(reader.read_file("b.md"), as_written("b.md"));
}

#[test]
fn cancel_at_the_first_question_of_close_others_closes_nothing() {
    let mut reader = three_linked();
    edit(&mut reader, " Edit A.");
    reader.drop_file("b.md");
    reader.drop_file("c.md");
    reader.right_click("tab:Gamma");
    reader.click("Close Others");

    reader.click("Cancel");

    assert_eq!(tabs(&reader), ["a.md", "b.md", "c.md"]);
}

#[test]
fn close_others_does_not_ask_about_the_tab_it_keeps() {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    edit(&mut reader, " Edit B.");

    reader.right_click("tab:Beta");
    reader.click("Close Others");

    reader.assert_hides("Save changes to");
    assert_eq!(reader.count("tab:Alpha"), 0);
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn every_way_to_quit_asks_while_edits_are_unsaved() {
    type Quit = fn(&mut Reader);
    let ways: [(&str, Quit); 3] = [
        ("the shortcut", |reader| reader.press("Ctrl+Q")),
        ("the menu", |reader| {
            reader.click("More");
            reader.click("Quit");
        }),
        ("the window's close button", Reader::close_window),
    ];

    for (way, quit) in ways {
        let mut reader = unsaved_draft();

        quit(&mut reader);

        assert!(reader.sees(ASKS_DRAFT), "{way}");
        assert!(!reader.has_quit(), "{way}");
        assert_eq!(reader.read_file("draft.md"), SOURCE, "{way}");

        reader.click("Cancel");
        assert!(!reader.has_quit(), "{way}: Cancel");
        assert_eq!(reader.typed(), [EDITED], "{way}: Cancel");
    }
}

#[test]
fn the_windows_close_button_quits_at_once_with_nothing_unsaved() {
    let mut reader = three_linked();

    reader.close_window();

    assert!(reader.has_quit());
}

#[test]
fn save_while_quitting_writes_the_edits_and_quits() {
    let mut reader = unsaved_draft();
    reader.close_window();

    reader.click("Save");

    assert!(reader.has_quit());
    assert_eq!(reader.read_file("draft.md"), EDITED);
}

#[test]
fn discard_while_quitting_quits_and_the_next_run_shows_the_file() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+Q");

    reader.click("Discard");

    assert!(reader.has_quit());
    assert_eq!(reader.read_file("draft.md"), SOURCE);
    let reader = reader.restart(&[]);
    assert_eq!(reader.title(), "draft.md - Markdown Viewer");
    reader.assert_sees("First thoughts.");
    reader.assert_hides("Mine.");
    assert_eq!(reader.read_file("draft.md"), SOURCE);
}

#[test]
fn a_save_that_fails_while_quitting_stays_in_the_app() {
    let mut reader = Reader::with_docs(&[("notes/draft.md", SOURCE)]);
    edit(&mut reader, " Mine.");
    reader.delete_folder("notes");
    reader.press("Ctrl+Q");

    reader.click("Save");

    assert!(!reader.has_quit());
    reader.assert_sees("Unable to save");
    assert_eq!(reader.typed(), [EDITED]);
}

#[test]
fn quitting_asks_about_each_unsaved_document_and_keeps_the_tabs_for_next_time() {
    let mut reader = two_unsaved_of_three();
    reader.press("Ctrl+Q");

    reader.assert_sees("Save changes to “Alpha” (a.md)?");
    reader.click("Save");
    assert!(!reader.has_quit(), "b.md is still to be answered for");
    reader.assert_sees("Save changes to “Beta” (b.md)?");
    reader.click("Discard");

    assert!(reader.has_quit());
    assert_eq!(
        reader.read_file("a.md"),
        format!("{} Edit A.", as_written("a.md"))
    );
    assert_eq!(reader.read_file("b.md"), as_written("b.md"));
    let reader = reader.restart(&[]);
    assert_eq!(tabs(&reader), ["a.md", "b.md", "c.md"]);
}

#[test]
fn cancel_while_quitting_keeps_the_app_and_every_tab() {
    let mut reader = two_unsaved_of_three();
    reader.press("Ctrl+Q");
    reader.click("Discard");

    reader.click("Cancel");

    assert!(!reader.has_quit());
    assert_eq!(tabs(&reader), ["a.md", "b.md", "c.md"]);
    reader.click("tab:Alpha");
    assert_eq!(
        reader.typed(),
        [format!("{} Edit A.", as_written("a.md"))],
        "a discard that never quit threw nothing away"
    );
}

#[test]
fn quitting_again_after_a_cancel_asks_again() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+Q");
    reader.click("Cancel");

    reader.press("Ctrl+Q");

    reader.assert_sees(ASKS_DRAFT);
    assert!(!reader.has_quit());
}

#[test]
fn the_apps_shortcuts_wait_while_the_prompt_asks() {
    let mut reader = unsaved_draft();
    let page = reader.page_color();
    reader.press("Ctrl+W");

    for shortcut in [
        "Ctrl+D",
        "Ctrl+F",
        "Ctrl+E",
        "Ctrl+W",
        "Ctrl+Shift+E",
        "Ctrl++",
        "F5",
    ] {
        reader.press(shortcut);

        assert!(reader.sees(ASKS_DRAFT), "{shortcut}");
    }
    reader.click("Cancel");

    assert_eq!(reader.page_color(), page, "Ctrl+D changed no theme");
    assert_eq!(reader.typed(), [EDITED], "no find bar, still editing");
    reader.assert_hides("FILES");
}

#[test]
fn nothing_under_the_prompt_takes_a_click_or_typing() {
    let mut reader = unsaved_draft();
    reader.press("Ctrl+W");

    reader.click_text_field(0);
    reader.type_text(" Typed under it.");

    reader.click("Cancel");
    assert_eq!(reader.typed(), [EDITED]);
}

const ASKS_ALPHA: &str = "Save changes to “Alpha” (a.md)?";
const ALPHA_EDITED: &str =
    "# Alpha\n\nGo [to beta](b.md) or [to gamma](c.md) or [nowhere](missing.md). Edit A.";

/// Type an edit into the document in front and leave it unsaved, shown
/// as the reader reads it, where its links can be clicked.
fn edit_and_read(reader: &mut Reader, text: &str) {
    edit(reader, text);
    reader.press("Ctrl+E");
}

/// `a.md` of the three linked documents with an edit that is not saved.
fn unsaved_alpha() -> Reader {
    let mut reader = three_linked();
    edit_and_read(&mut reader, " Edit A.");
    reader
}

/// List the folder in the files sidebar.
fn show_files(reader: &mut Reader) {
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| reader.sees("c.md"));
}

/// A way to put another document in the tab in front, and the file that
/// then shows.
type Route = (&'static str, fn(&mut Reader), &'static str);

/// Every way out of `a.md` that shows another document in its tab, each
/// from `a.md` with an edit that is not saved.
fn routes_out_of_alpha() -> Vec<(Reader, Route)> {
    let onward: [Route; 2] = [
        ("a link", |reader| reader.click("to beta"), "b.md"),
        (
            "a file in the sidebar",
            |reader| {
                show_files(reader);
                reader.click("c.md");
            },
            "c.md",
        ),
    ];
    let back: [Route; 3] = [
        ("the Back button", |reader| reader.click("Go back"), "c.md"),
        ("Alt+Left", |reader| reader.press("Alt+Left"), "c.md"),
        (
            "Back in the menu",
            |reader| {
                reader.click("More");
                reader.click("Back");
            },
            "c.md",
        ),
    ];
    let forward: [Route; 3] = [
        (
            "the Forward button",
            |reader| reader.click("Go forward"),
            "b.md",
        ),
        ("Alt+Right", |reader| reader.press("Alt+Right"), "b.md"),
        (
            "Forward in the menu",
            |reader| {
                reader.click("More");
                reader.click("Forward");
            },
            "b.md",
        ),
    ];
    let from_alpha = onward.map(|route| (unsaved_alpha(), route));
    let came_from_gamma = back.map(|route| {
        let mut reader = Reader::launch()
            .doc("a.md", as_written("a.md"))
            .doc("b.md", as_written("b.md"))
            .doc("c.md", "# Gamma\n\nBack [to alpha](a.md).")
            .args(&["c.md"])
            .open();
        reader.click("to alpha");
        edit_and_read(&mut reader, " Edit A.");
        (reader, route)
    });
    let went_back_from_beta = forward.map(|route| {
        let mut reader = three_linked();
        reader.click("to beta");
        reader.click("Go back");
        edit_and_read(&mut reader, " Edit A.");
        (reader, route)
    });
    let mut routes = Vec::from(from_alpha);
    routes.extend(came_from_gamma);
    routes.extend(went_back_from_beta);
    routes
}

#[test]
fn every_way_to_another_document_in_the_tab_asks_and_touches_nothing() {
    for (mut reader, (way, go, _)) in routes_out_of_alpha() {
        go(&mut reader);

        assert!(reader.sees(ASKS_ALPHA), "{way}");
        assert_eq!(reader.title(), "a.md - Markdown Viewer", "{way}");
        assert_eq!(reader.read_file("a.md"), as_written("a.md"), "{way}");
        assert_eq!(reader.count("Unsaved changes"), 1, "{way}");
    }
}

#[test]
fn save_writes_the_edits_and_then_goes_to_the_other_document() {
    for (mut reader, (way, go, shown)) in routes_out_of_alpha() {
        go(&mut reader);

        reader.click("Save");

        assert_eq!(reader.read_file("a.md"), ALPHA_EDITED, "{way}");
        assert_eq!(
            reader.title(),
            format!("{shown} - Markdown Viewer"),
            "{way}"
        );
        assert!(!reader.sees("Save changes to"), "{way}");
        assert_eq!(reader.count("Unsaved changes"), 0, "{way}");
    }
}

#[test]
fn discard_goes_to_the_other_document_and_leaves_the_file_as_it_was() {
    for (mut reader, (way, go, shown)) in routes_out_of_alpha() {
        go(&mut reader);

        reader.click("Discard");

        assert_eq!(reader.read_file("a.md"), as_written("a.md"), "{way}");
        assert_eq!(
            reader.title(),
            format!("{shown} - Markdown Viewer"),
            "{way}"
        );
        assert!(!reader.sees("Save changes to"), "{way}");
        assert_eq!(reader.count("Unsaved changes"), 0, "{way}");
    }
}

#[test]
fn cancel_and_escape_stay_on_the_document_with_its_edits() {
    type Answer = fn(&mut Reader);
    let answers: [(&str, Answer); 2] = [
        ("Cancel", |reader| reader.click("Cancel")),
        ("Escape", |reader| reader.press("Escape")),
    ];
    for (answer, give) in answers {
        for (mut reader, (way, go, _)) in routes_out_of_alpha() {
            go(&mut reader);

            give(&mut reader);

            let case = format!("{answer} after {way}");
            assert!(!reader.sees("Save changes to"), "{case}");
            assert_eq!(reader.title(), "a.md - Markdown Viewer", "{case}");
            assert_eq!(reader.count("Unsaved changes"), 1, "{case}");
            assert_eq!(reader.read_file("a.md"), as_written("a.md"), "{case}");
            reader.press("Ctrl+E");
            assert_eq!(reader.typed(), [ALPHA_EDITED], "{case}");
        }
    }
}

#[test]
fn the_edits_a_discard_gave_up_are_gone_when_the_reader_comes_back() {
    let mut reader = unsaved_alpha();
    reader.click("to beta");
    reader.click("Discard");

    reader.click("Go back");

    assert_eq!(reader.title(), "a.md - Markdown Viewer");
    reader.assert_hides("Save changes to");
    assert_eq!(reader.count("Unsaved changes"), 0);
    reader.press("Ctrl+E");
    assert_eq!(reader.typed(), [as_written("a.md")]);
}

#[test]
fn a_save_that_fails_stays_on_the_document_and_says_why() {
    let mut reader = Reader::launch()
        .doc("notes/a.md", "# Alpha\n\nGo [to beta](../b.md).")
        .doc("b.md", "# Beta")
        .args(&["notes/a.md"])
        .open();
    edit_and_read(&mut reader, " Mine.");
    reader.click("to beta");
    reader.assert_sees(ASKS_ALPHA);
    reader.delete_folder("notes");

    reader.click("Save");

    reader.assert_hides("Save changes to");
    reader.assert_sees("Unable to save");
    assert_eq!(reader.title(), "a.md - Markdown Viewer");
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn a_document_with_nothing_unsaved_is_left_without_a_question() {
    let mut reader = unsaved_alpha();
    reader.press("Ctrl+S");

    reader.click("to beta");

    reader.assert_hides("Save changes to");
    assert_eq!(reader.title(), "b.md - Markdown Viewer");
}

#[test]
fn a_jump_within_the_document_does_not_ask() {
    // Short enough for the whole source to be on screen and be clicked
    // into, and too long for the page to show its last heading.
    let text = format!("[Down](#section-12) the page.\n\n{}", long_document(12));
    let mut reader = Reader::with_docs(&[("long.md", &text)]);
    edit_and_read(&mut reader, " Mine.");
    assert_eq!(reader.count("Unsaved changes"), 1);
    let last = "Paragraph 12 mentions the lighthouse once.";

    reader.assert_sees("CONTENTS");
    reader.click("Long read");
    reader.wait_for("the top to scroll into view", |reader| {
        reader.in_view("Down") && !reader.in_view(last)
    });
    reader.assert_hides("Save changes to");

    reader.click("Down");
    reader.wait_for("the heading to scroll into view", |reader| {
        reader.in_view(last)
    });
    reader.assert_hides("Save changes to");
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn a_way_that_leads_nowhere_does_not_ask() {
    let mut reader = unsaved_alpha();

    reader.click("nowhere");

    reader.assert_hides("Save changes to");
    reader.assert_sees("Unable to open");
    assert_eq!(reader.count("Unsaved changes"), 1);

    show_files(&mut reader);
    reader.click("a.md");

    reader.assert_hides("Save changes to");
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn another_document_in_a_tab_of_its_own_does_not_ask() {
    let mut reader = unsaved_alpha();

    reader.ctrl_click("to beta");

    reader.assert_hides("Save changes to");
    assert_eq!(tabs(&reader), ["a.md", "b.md"]);
    reader.click("tab:Alpha");
    assert_eq!(reader.count("Unsaved changes"), 1);
}

#[test]
fn the_shortcuts_wait_while_the_prompt_asks_about_leaving() {
    let mut reader = unsaved_alpha();
    reader.click("to beta");

    for shortcut in ["Alt+Left", "Ctrl+W", "Ctrl+E", "Ctrl+F"] {
        reader.press(shortcut);

        assert!(reader.sees(ASKS_ALPHA), "{shortcut}");
    }
    reader.click("Discard");

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    assert_eq!(tabs(&reader), ["b.md"]);
}
