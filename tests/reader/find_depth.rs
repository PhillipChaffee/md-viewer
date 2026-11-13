//! Find in document, in depth: what counts as a match, which match is
//! marked, and how the bar behaves while the document changes under it.

use std::fmt::Write as _;

use super::docs::long_document;
use super::support::{Reader, Rgb};

/// A page where "tent" appears three times, once capitalized.
const CAMPING: &str =
    "# Camping\n\nPack the tent.\n\nTent pegs go in last.\n\nAir the tent at home.";

/// A page of the characters a find box has to take literally.
const FIELD_NOTES: &str = "# Field notes\n\n\
    The heron stood. A HERON flew. Herons everywhere.\n\n\
    Price: $4.50 (approx.) for a.b and a*b and axb.\n\n\
    Sm\u{f6}rg\u{e5}sbord au lait, na\u{ef}ve fa\u{e7}ade, \u{6771}\u{4eac} tower, \u{6771}\u{4eac} bay.\n\n\
    Keep C:\\path\\file and 50% off, see note^2 and a|b.\n";

/// The line above the title of the camping page.
const KICKER: &str = "camping.md \u{b7} 1 min read";

fn camping() -> Reader {
    Reader::with_docs(&[("camping.md", CAMPING)])
}

/// Open the find bar and look for `query`, replacing any earlier query.
fn find(reader: &mut Reader, query: &str) {
    reader.press("Ctrl+F");
    reader.press("Ctrl+A");
    reader.type_text(query);
}

/// Which of the marked matches is painted unlike the others: the current
/// one. `None` unless exactly one stands out.
fn current_marker(reader: &Reader) -> Option<usize> {
    let colors: Vec<Rgb> = reader
        .marked()
        .into_iter()
        .map(|(_, color)| color)
        .collect();
    let unlike = |color: &Rgb| colors.iter().filter(|other| *other == color).count() == 1;
    let mut alone = colors.iter().enumerate().filter(|(_, color)| unlike(color));
    let (index, _) = alone.next()?;
    alone.next().is_none().then_some(index)
}

/// A long document with a kingfisher in sections 10, 40 and 70.
fn kingfishers() -> Reader {
    let mut text = String::from("# Birds\n\n");
    for number in 1..=80 {
        let _ = write!(
            text,
            "## Stretch {number}\n\nReeds along stretch {number}.\n\n"
        );
        if number % 30 == 10 {
            let _ = write!(text, "A kingfisher at marker {number}.\n\n");
        }
    }
    Reader::with_docs(&[("birds.md", &text)])
}

#[test]
fn the_count_follows_the_query_as_it_is_typed() {
    let mut reader = camping();
    reader.press("Ctrl+F");

    reader.type_text("pe");
    reader.assert_sees("1 / 1");

    reader.type_text("x");
    reader.assert_sees("0 matches");

    reader.press("Backspace");
    reader.assert_sees("1 / 1");
}

#[test]
fn an_empty_query_shows_no_count_and_offers_no_stepping() {
    let mut reader = camping();
    find(&mut reader, "tent");

    reader.press("Ctrl+A");
    reader.press("Backspace");

    assert_eq!(reader.typed(), [""]);
    reader.assert_hides("0 matches");
    reader.assert_hides(" / ");
    assert!(!reader.is_enabled("Next match"));
    assert!(reader.marked().is_empty());
}

#[test]
fn every_query_is_taken_literally_and_counted() {
    let cases = [
        ("heron", "1 / 3"),
        ("HERON", "1 / 3"),
        ("hErOn", "1 / 3"),
        ("herons", "1 / 1"),
        ("heron stood", "1 / 1"),
        ("a.b", "1 / 1"),
        ("a*b", "1 / 1"),
        ("a|b", "1 / 1"),
        (".*", "0 matches"),
        ("^The", "0 matches"),
        ("\\w+", "0 matches"),
        ("[a-z]", "0 matches"),
        ("$4.50", "1 / 1"),
        ("(approx.)", "1 / 1"),
        ("C:\\path\\file", "1 / 1"),
        ("50%", "1 / 1"),
        ("note^2", "1 / 1"),
        ("Sm\u{f6}rg\u{e5}sbord", "1 / 1"),
        ("na\u{ef}ve fa\u{e7}ade", "1 / 1"),
        ("\u{6771}\u{4eac}", "1 / 2"),
        ("\u{4eac} bay", "1 / 1"),
        ("zebra", "0 matches"),
    ];
    let mut reader = Reader::with_docs(&[("notes.md", FIELD_NOTES)]);

    for (query, count) in cases {
        find(&mut reader, query);

        assert_eq!(reader.typed(), [query]);
        assert!(reader.sees(count), "`{query}` should show `{count}`");
    }
}

#[test]
fn a_word_that_is_only_in_a_links_address_is_not_a_match() {
    let mut reader = Reader::with_docs(&[(
        "shop.md",
        "# Shop\n\nBuy pegs at [the outfitter](https://tent.example/tent-pegs).",
    )]);

    find(&mut reader, "tent");
    reader.assert_sees("0 matches");

    find(&mut reader, "outfitter");
    reader.assert_sees("1 / 1");
}

#[test]
fn matches_in_headings_lists_tables_and_code_are_all_counted_and_marked() {
    let mut reader = Reader::with_docs(&[(
        "gear.md",
        "# Tent care\n\n- dry the tent\n\n| Item | Note |\n| --- | --- |\n| tent | new |\n\n\
         Run `tent --pitch` now.\n\n> A tent is a home.\n",
    )]);

    find(&mut reader, "tent");

    reader.assert_sees("1 / 5");
    assert_eq!(reader.marked().len(), 5, "{:?}", reader.marked());
}

#[test]
fn every_match_is_marked_and_the_first_stands_out() {
    let mut reader = camping();
    assert!(reader.marked().is_empty());

    find(&mut reader, "tent");

    let words: Vec<String> = reader.marked().into_iter().map(|(text, _)| text).collect();
    assert_eq!(words, ["tent", "Tent", "tent"]);
    assert_eq!(current_marker(&reader), Some(0));
}

#[test]
fn the_marker_that_stands_out_moves_with_each_step_and_wraps() {
    let mut reader = camping();
    find(&mut reader, "tent");

    for (key, current) in [
        ("Enter", 1),
        ("Enter", 2),
        ("Enter", 0),
        ("Shift+Enter", 2),
        ("Up", 1),
        ("Down", 2),
    ] {
        reader.press(key);

        assert_eq!(current_marker(&reader), Some(current), "after {key}");
        reader.assert_sees(&format!("{} / 3", current + 1));
    }
}

#[test]
fn the_buttons_wrap_at_both_ends() {
    let mut reader = camping();
    find(&mut reader, "tent");

    reader.click("Previous match");
    reader.assert_sees("3 / 3");
    assert_eq!(current_marker(&reader), Some(2));

    reader.click("Next match");
    reader.assert_sees("1 / 3");
    assert_eq!(current_marker(&reader), Some(0));
}

#[test]
fn a_lone_match_stays_current_however_it_is_stepped() {
    let mut reader = camping();
    find(&mut reader, "pegs");

    for key in ["Enter", "Shift+Enter", "Down", "Up"] {
        reader.press(key);

        reader.assert_sees("1 / 1");
        assert_eq!(reader.marked().len(), 1, "after {key}");
    }
}

#[test]
fn a_new_query_starts_again_at_its_first_match() {
    let mut reader = camping();
    find(&mut reader, "tent");
    reader.press("Enter");
    reader.press("Enter");
    reader.assert_sees("3 / 3");

    find(&mut reader, "p");

    reader.assert_sees("1 / 3");
    assert_eq!(current_marker(&reader), Some(0));
}

#[test]
fn closing_the_bar_takes_the_markers_off_the_page() {
    let mut reader = camping();
    find(&mut reader, "tent");
    assert_eq!(reader.marked().len(), 3);

    reader.click("Close find bar");

    assert!(reader.marked().is_empty(), "{:?}", reader.marked());
    reader.assert_sees("Pack the tent.");
}

#[test]
fn a_bar_opened_again_starts_empty() {
    let mut reader = camping();
    find(&mut reader, "tent");
    reader.press("Enter");
    reader.press("Escape");

    reader.press("Ctrl+F");

    assert_eq!(reader.typed(), [""]);
    assert!(reader.marked().is_empty());
    reader.assert_hides(" / 3");
}

#[test]
fn the_keyboard_is_in_the_bar_while_it_is_open_and_leaves_when_it_closes() {
    let mut reader = camping();
    assert!(!reader.is_typing());

    reader.press("Ctrl+F");
    assert!(reader.is_typing());

    reader.press("Escape");
    assert!(!reader.is_typing());
}

#[test]
fn ctrl_f_takes_the_keyboard_back_to_an_open_bar() {
    let mut reader = camping();
    find(&mut reader, "tent");
    reader.click("Camping");
    assert!(!reader.is_typing(), "a click on the page leaves the bar");

    reader.press("Ctrl+F");
    reader.type_text("s");

    assert!(reader.is_typing());
    assert!(reader.typed()[0].contains('s'), "{:?}", reader.typed());
    assert_eq!(reader.count("Close find bar"), 1);
}

#[test]
fn the_down_arrow_steps_in_the_bar_and_scrolls_the_page_once_it_is_closed() {
    let mut reader = Reader::with_docs(&[("long.md", &long_document(80))]);
    find(&mut reader, "Paragraph 1 ");
    let with_the_bar = reader.text_area("Section 2").top;

    reader.press("Down");
    assert!(
        (reader.text_area("Section 2").top - with_the_bar).abs() < 1.0,
        "the arrow went to the bar, not the page"
    );

    reader.press("Escape");
    let closed = reader.text_area("Section 2").top;
    reader.press("Down");
    reader.wait_for("the page to scroll a line", |reader| {
        reader.text_area("Section 2").top < closed
    });
}

#[test]
fn each_step_scrolls_to_its_match_and_wrapping_scrolls_back() {
    let mut reader = kingfishers();
    find(&mut reader, "kingfisher");
    reader.assert_sees("1 / 3");

    for (count, marker) in [("2 / 3", 40), ("3 / 3", 70), ("1 / 3", 10)] {
        reader.press("Enter");
        reader.wait_for("the match to scroll into view", |reader| {
            reader.in_view(&format!("Stretch {marker}"))
        });

        reader.assert_sees(count);
        assert_eq!(reader.marked().len(), 1, "one kingfisher is in view");
    }
}

#[test]
fn stepping_backwards_from_the_first_match_scrolls_to_the_last() {
    let mut reader = kingfishers();
    find(&mut reader, "kingfisher");

    reader.press("Shift+Enter");
    reader.wait_for("the last match to scroll into view", |reader| {
        reader.in_view("Stretch 70")
    });

    reader.assert_sees("3 / 3");
    assert!(!reader.in_view("Stretch 10"));
}

#[test]
fn a_match_already_in_view_does_not_move_the_page() {
    let mut reader = camping();
    find(&mut reader, "tent");
    let before = reader.text_area(KICKER);

    reader.press("Enter");
    reader.press("Enter");

    assert_eq!(reader.text_area(KICKER), before);
}

#[test]
fn the_count_follows_a_live_reload() {
    let mut reader = camping();
    find(&mut reader, "tent");
    reader.assert_sees("1 / 3");

    reader.write_file("camping.md", "# Camping\n\nOne tent, and a spare tent.");
    reader.wait_for("the page to reload", |reader| reader.sees("One "));

    reader.assert_sees("1 / 2");
    assert_eq!(reader.marked().len(), 2);
    assert_eq!(reader.typed(), ["tent"]);
}

#[test]
fn a_reload_that_removes_every_match_says_so() {
    let mut reader = camping();
    find(&mut reader, "tent");

    reader.write_file("camping.md", "# Camping\n\nStay in the cabin.");
    reader.wait_for("the page to reload", |reader| {
        reader.sees("Stay in the cabin.")
    });

    reader.assert_sees("0 matches");
    assert!(reader.marked().is_empty());
}

#[test]
fn the_query_and_the_bar_stay_through_a_switch_of_tabs_and_back() {
    let mut reader = Reader::with_docs(&[
        ("one.md", "# One\n\nA tent beside a tent beside a tent."),
        ("two.md", "# Two\n\nA single tent."),
    ]);
    reader.drop_file("two.md");
    find(&mut reader, "tent");
    reader.assert_sees("1 / 1");

    reader.click("one.md");
    reader.assert_sees("1 / 3");
    assert_eq!(reader.marked().len(), 3);
    reader.press("Enter");
    reader.assert_sees("2 / 3");

    reader.press("Ctrl+Tab");
    reader.assert_sees("1 / 1");
    assert_eq!(reader.typed(), ["tent"]);
}

#[test]
fn closing_the_tab_in_front_counts_the_one_behind_it() {
    let mut reader = Reader::with_docs(&[
        ("one.md", "# One\n\nA tent beside a tent."),
        ("two.md", "# Two\n\nNo shelter here."),
    ]);
    reader.drop_file("two.md");
    find(&mut reader, "tent");
    reader.assert_sees("0 matches");

    reader.middle_click("two.md");

    reader.assert_sees("1 / 2");
    assert_eq!(reader.typed(), ["tent"]);
}

/// Known bug, listed in `tests/README.md`: a link followed in place leaves
/// the find bar counting nothing, although the query is still in it and
/// the new page has matches. This test states the right behavior and is
/// expected to fail until the app has it.
#[test]
#[should_panic(expected = "the count does not follow a link opened in place")]
fn following_a_link_counts_the_document_arrived_at() {
    let mut reader = Reader::with_docs(&[
        ("one.md", "# One\n\nA tent. Now [onward](two.md)."),
        ("two.md", "# Two\n\nTent, tent, tent and tent."),
    ]);
    find(&mut reader, "tent");
    reader.assert_sees("1 / 1");

    reader.click("onward");

    reader.assert_sees("Two");
    assert_eq!(reader.typed(), ["tent"]);
    assert!(
        reader.sees("1 / 4"),
        "the count does not follow a link opened in place"
    );
}

#[test]
fn find_counts_matches_in_the_source_while_editing() {
    let mut reader = camping();
    reader.press("Ctrl+E");

    find(&mut reader, "tent");

    reader.assert_sees("1 / 3");
    assert_eq!(reader.typed().len(), 2, "the query and the source");
    reader.press("Enter");
    reader.assert_sees("2 / 3");
}

#[test]
fn a_pasted_query_is_searched_for() {
    let mut reader = camping();
    reader.press("Ctrl+F");

    reader.paste("pegs go");

    assert_eq!(reader.typed(), ["pegs go"]);
    reader.assert_sees("1 / 1");
}

#[test]
fn the_bar_opens_on_the_home_page_and_finds_nothing_there() {
    let mut reader = Reader::first_run();

    find(&mut reader, "viewer");

    assert_eq!(reader.typed(), ["viewer"]);
    reader.assert_sees("0 matches");
    reader.press("Escape");
    assert_eq!(reader.count("Close find bar"), 0);
}

#[test]
fn the_bar_does_not_cover_the_start_of_the_page() {
    let mut reader = camping();
    let bar_less = reader.text_area(KICKER).top;

    reader.press("Ctrl+F");

    let bar_bottom = reader.area_of("Close find bar").bottom();
    let kicker = reader.text_area(KICKER);
    assert!(kicker.top > bar_less, "the page moved down for the bar");
    assert!(kicker.top >= bar_bottom, "{kicker:?} under {bar_bottom}");
}
