//! Where a line of running text may end: where prose could break, and
//! never just because a piece of code, a link or bold text starts or ends
//! there. What touches such a piece with no space between stays on its
//! line.

use super::pages::each;
use super::support::Reader;

/// Words of many lengths, so that at every column width some line ends
/// near one of them.
const NAMES: [&str; 24] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
    "uniform", "victor", "whiskey", "xray",
];

/// Column widths to read at, each as the key that changes the width and
/// how often to press it: 560, 640, 680, 720 and 800 pixels.
const WIDTHS: [(&str, usize); 5] = [
    ("Ctrl+Alt+Left", 3),
    ("Ctrl+Alt+Left", 1),
    ("Ctrl+Alt+Left", 0),
    ("Ctrl+Alt+Right", 1),
    ("Ctrl+Alt+Right", 3),
];

/// `body` under a heading, beside a document for its links to lead to.
fn document(body: &str) -> Reader {
    Reader::with_docs(&[
        ("page.md", &format!("# Lines\n\n{body}")),
        ("guide.md", "# Guide\n\nThe guide itself."),
    ])
}

/// The lines the document is set in, as text.
fn set_lines(reader: &Reader) -> Vec<String> {
    reader.lines().into_iter().map(|line| line.text).collect()
}

/// At every column width, each of `units` is set whole on one line of the
/// document `body` makes. A unit is a run of characters with no space in
/// it, written once in `body`.
#[track_caller]
fn assert_set_whole(body: &str, units: &[String]) {
    for (key, presses) in WIDTHS {
        let mut reader = document(body);
        for _ in 0..presses {
            reader.press(key);
        }
        assert_whole_in(&reader, units, &format!("{key} x{presses}"));
    }
}

/// Each of `units` is on one line of what `reader` shows.
#[track_caller]
fn assert_whole_in(reader: &Reader, units: &[String], when: &str) {
    let lines = set_lines(reader);
    assert!(lines.len() > 3, "{when}: the text does not wrap: {lines:?}");
    for unit in units {
        assert!(
            lines.iter().any(|line| line.contains(unit.as_str())),
            "{when}: `{unit}` is split between two lines:\n{}",
            lines.join("\n")
        );
    }
}

/// What `unit` makes of each name.
fn units(unit: impl Fn(&str) -> String) -> Vec<String> {
    NAMES.into_iter().map(unit).collect()
}

#[test]
fn brackets_and_a_comma_stay_with_the_code_between_them() {
    let body = each(NAMES, |name| format!("(`{name}`), "));

    assert_set_whole(&body, &units(|name| format!("({name}),")));
}

#[test]
fn a_full_stop_stays_with_the_code_it_ends_a_sentence_on() {
    let body = each(NAMES, |name| format!("Call `{name}`. "));

    assert_set_whole(&body, &units(|name| format!("{name}.")));
}

#[test]
fn a_plural_s_stays_with_its_code() {
    let body = each(NAMES, |name| format!("two `{name}`s or "));

    assert_set_whole(&body, &units(|name| format!("{name}s")));
}

#[test]
fn a_prefix_stays_with_the_code_it_is_written_onto() {
    let body = each(NAMES, |name| format!("to re`{name}` it, "));

    assert_set_whole(&body, &units(|name| format!("re{name}")));
}

#[test]
fn code_written_straight_after_a_link_stays_with_it() {
    let body = each(NAMES, |name| format!("see [{name}](guide.md)`{name}`; "));

    assert_set_whole(&body, &units(|name| format!("{name}{name};")));
}

#[test]
fn brackets_and_a_full_stop_stay_with_the_link_between_them() {
    let body = each(NAMES, |name| format!("Go ([{name}](guide.md)). "));

    assert_set_whole(&body, &units(|name| format!("({name}).")));
}

#[test]
fn what_touches_bold_or_italic_words_stays_with_them() {
    let body = each(NAMES, |name| format!("(**{name}**), un*{name}*s; "));

    let mut whole = units(|name| format!("({name}),"));
    whole.extend(units(|name| format!("un{name}s;")));
    assert_set_whole(&body, &whole);
}

/// Two prices with a dash between them are read as the text they are, and
/// the second stays with its dollar sign.
#[test]
fn a_range_of_prices_stays_whole() {
    let body = each(10..34, |cents| {
        format!("from $1,0{cents}\u{2013}$2,0{cents}; ")
    });
    let whole: Vec<String> = (10..34)
        .map(|cents| format!("$1,0{cents}\u{2013}$2,0{cents};"))
        .collect();

    assert_set_whole(&body, &whole);
}

#[test]
fn list_items_quotes_and_headings_keep_them_together_too() {
    let text = each(NAMES, |name| format!("(`{name}`), "));

    for start in ["- ", "> ", "1. ", "## "] {
        assert_set_whole(
            &format!("{start}{text}"),
            &units(|name| format!("({name}),")),
        );
    }
}

/// Find paints each match as a piece of its own; a match inside a word is
/// no place for a line to end either.
#[test]
fn a_match_of_find_splits_no_word_between_lines() {
    let body = each(NAMES, |name| format!("(`{name}`), {name}ward and "));
    let mut whole = units(|name| format!("({name}),"));
    whole.extend(units(|name| format!("{name}ward")));

    for query in ["a", "e", "o", ")", "war"] {
        let mut reader = document(&body);
        reader.press("Ctrl+F");
        reader.type_text(query);

        assert!(!reader.marked().is_empty(), "nothing matches `{query}`");
        assert_whole_in(&reader, &whole, &format!("finding `{query}`"));
    }
}

/// Words on a line are still broken where prose breaks: between two
/// pieces with a space between them.
#[test]
fn a_line_still_ends_between_code_and_the_word_after_a_space() {
    let body = each(NAMES, |name| format!("`{name}` then "));
    let reader = document(&body);

    let lines = set_lines(&reader);

    assert!(lines.len() > 3, "the text does not wrap: {lines:?}");
    let column = reader.column();
    for line in reader.lines() {
        assert!(
            line.area.right() <= column.right() + 1.0,
            "`{}` runs past the column",
            line.text
        );
    }
}

/// A run with nowhere to break that is wider than the column cannot stay
/// whole: it wraps, and nothing is set past the column's edge.
#[test]
fn a_run_wider_than_the_column_still_wraps() {
    let long = "abcdefghij".repeat(14);
    let bodies = [
        format!("Before (`{long}`), after."),
        format!("Before ({long}`code`), after."),
        format!("Before (**{long}**`code`{long}), after."),
    ];

    for body in bodies {
        let mut reader = document(&body);
        for _ in 0..3 {
            reader.press("Ctrl+Alt+Left");
        }

        let column = reader.column();
        let lines = reader.lines();
        assert!(lines.len() > 3, "the run does not wrap: {lines:?}");
        for line in lines {
            assert!(
                line.area.right() <= column.right() + 1.0,
                "`{}` runs {} px past the column in `{body}`",
                line.text,
                line.area.right() - column.right()
            );
        }
    }
}

/// A word too long for a line, with matches of find all through it, is
/// still set as the long word it is: it fills its lines, and its pieces do
/// not each start a line.
#[test]
fn a_long_word_full_of_matches_fills_its_lines() {
    let long = "abcdefghij".repeat(14);
    let mut reader = document(&format!("Before {long} after."));
    for _ in 0..3 {
        reader.press("Ctrl+Alt+Left");
    }
    let plain = reader.lines().len();

    reader.press("Ctrl+F");
    reader.type_text("a");

    assert!(reader.marked().len() >= 14, "{:?}", reader.marked());
    let lines = set_lines(&reader);
    assert!(
        lines.len() <= plain + 1,
        "{plain} lines became {}:\n{}",
        lines.len(),
        lines.join("\n")
    );
}

/// Pieces of code written against each other, too many for one line, fill
/// the lines one after another as a long word does.
#[test]
fn a_chain_of_code_wider_than_the_column_fills_its_lines() {
    let chain = each(NAMES.iter().chain(&NAMES), |name| format!("`{name}`/"));
    let reader = document(&format!("Before {chain} after."));

    let lines = set_lines(&reader);

    assert!(lines.len() > 3, "the chain does not wrap: {lines:?}");
    assert!(
        lines.len() < 9,
        "the chain is set on {} lines:\n{}",
        lines.len(),
        lines.join("\n")
    );
}

/// Long inline code wraps at word boundaries to the column's width rather
/// than cutting mid-word at a fixed character count, and what follows it
/// stays on its last line.
#[test]
fn long_inline_code_wraps_at_word_boundaries_and_keeps_trailing_punctuation() {
    let body = concat!(
        "- Produces:\n",
        "  - `build_data.main(argv: Sequence[str] | None = None) -> int`, ",
        "for `python -m inkling_decision.build_data <filtered_full_dir> <default_dir>`.\n",
        "  - `overlap.matches(name: str, patterns: Sequence[str]) -> list[str]` ",
        "and `overlap.main(argv: Sequence[str] | None = None) -> int`.\n",
    );
    let reader = document(body);
    let lines = set_lines(&reader);

    for whole in ["int,", "<filtered_full_dir>", "list[str]", "int."] {
        assert!(
            lines.iter().any(|line| line.contains(whole)),
            "`{whole}` was broken across lines:\n{}",
            lines.join("\n")
        );
    }
    assert!(
        lines.iter().any(|line| line.contains("int, for")),
        "a forced newline followed the inline code:\n{}",
        lines.join("\n")
    );
}

/// A link that wraps is one link on each of its lines, with the bracket
/// after it kept on its last.
#[test]
fn a_link_that_wraps_can_be_followed_from_each_of_its_lines() {
    let link = each(NAMES, |name| format!("{name} "));
    let link = link.trim_end();
    let body = format!("Some words first, then ([{link}](guide.md)), and more.");

    for line in 0..2 {
        let mut reader = document(&body);
        let lines = reader.lines_of("alpha bravo");
        assert!(lines.len() >= 2, "the link is on one line: {lines:?}");
        let on = lines[line];

        reader.click_point(on.left + on.width / 2.0, on.top + on.height / 2.0);

        assert!(reader.sees("The guide itself."), "from line {line}");
    }
    let reader = document(&body);
    let lines = set_lines(&reader);
    assert!(
        lines.iter().any(|line| line.contains("xray),")),
        "the bracket left its link:\n{}",
        lines.join("\n")
    );
    assert!(
        lines.iter().any(|line| line.contains("(alpha")),
        "the bracket left its link:\n{}",
        lines.join("\n")
    );
}

/// Dragging over a bracket, the code in it and the comma after it selects
/// all three, and copying takes them in the order they are written. Each
/// piece is copied as a text of its own, and a space may be put between
/// two of them: the test reads past those.
#[test]
fn a_selection_runs_over_code_and_what_touches_it() {
    let body = each(NAMES, |name| format!("(`{name}`), "));
    let mut reader = document(&body);
    let mut lines = reader.lines().into_iter();
    let first = lines.find(|line| line.text.starts_with("(alpha)"));
    let line = first.expect("the paragraph starts a line").area;

    reader.drag_point(
        line.left + 1.0,
        line.top + line.height / 2.0,
        line.width,
        0.0,
    );
    reader.copy();

    let copied = reader.clipboard().unwrap_or_default().replace(' ', "");
    assert!(copied.starts_with("(alpha),(bravo),"), "copied `{copied}`");
}
