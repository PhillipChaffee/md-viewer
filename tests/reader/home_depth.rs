//! The home page, in depth: what each row says about its document, how
//! long the list gets, and how it words the time since a document was
//! read.

use std::time::{SystemTime, UNIX_EPOCH};

use super::support::Reader;

/// Read each `(name, content)` document once, in order, and go back to
/// the home page.
fn after_reading(docs: &[(&str, &str)]) -> Reader {
    let mut launch = Reader::launch();
    for (name, content) in docs {
        launch = launch.doc(name, content);
    }
    let mut reader = launch.open();
    for (name, _) in docs {
        reader.drop_file(name);
        reader.press("Ctrl+W");
    }
    reader
}

/// The titles of the rows the home page lists, top to bottom.
fn rows(reader: &Reader) -> Vec<String> {
    let names = reader.names().into_iter();
    names
        .skip_while(|name| name != "RECENTLY READ")
        .skip(1)
        .filter(|name| name != "Show more\u{2026}" && name != "Show less")
        .collect()
}

/// The line under each row's title, top to bottom: folder, reading time
/// and when it was read.
fn details(reader: &Reader) -> Vec<String> {
    let mut lines = reader.all_text();
    lines.retain(|line| line.contains(" min read \u{b7} "));
    lines
}

/// Seconds since 1970, as the session file counts them.
fn now() -> u64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    since.as_secs()
}

/// `words` words of plain text, no heading.
fn words(words: usize) -> String {
    vec!["word"; words].join(" ")
}

#[test]
fn a_row_is_titled_by_the_documents_first_heading_or_else_its_file_name() {
    let cases = [
        ("plain.md", "# Plain title\n\nText.", "Plain title"),
        (
            "marks.md",
            "# Title *with* `marks` and [a link](x.md)\n\nText.",
            "Title with marks and a link",
        ),
        (
            "setext.md",
            "Underlined title\n================\n\nText.",
            "Underlined title",
        ),
        (
            "late.md",
            "Some words first.\n\n# Late title\n",
            "Late title",
        ),
        ("no-heading.md", "Only a paragraph here.", "no-heading"),
        (
            "second-level.md",
            "## A section, not a title\n\nText.",
            "second-level",
        ),
        ("empty.md", "", "empty"),
    ];

    for (name, content, title) in cases {
        let reader = after_reading(&[(name, content)]);

        assert_eq!(rows(&reader), [title], "{name}");
    }
}

#[test]
fn a_row_quotes_the_opening_paragraph_without_its_markup() {
    let reader = after_reading(&[(
        "tale.md",
        "# Tale\n\nIt was a **dark** and [stormy](weather.md) `night`.\n\nSecond paragraph.",
    )]);

    reader.assert_sees("It was a dark and stormy night.");
    reader.assert_hides("Second paragraph.");
    reader.assert_hides("**");
}

#[test]
fn frontmatter_is_not_quoted_as_the_opening() {
    let reader = after_reading(&[(
        "post.md",
        "---\ntitle: Hidden\ntags: [a, b]\n---\n\nThe real opening line.",
    )]);

    reader.assert_sees("The real opening line.");
    reader.assert_hides("tags:");
}

#[test]
fn a_document_with_no_paragraph_has_a_row_without_a_quote() {
    let reader = after_reading(&[("bare.md", "# Bare\n")]);

    assert_eq!(rows(&reader), ["Bare"]);
    assert_eq!(details(&reader).len(), 1);
    let texts = reader.all_text();
    let after = texts.iter().skip_while(|text| !text.contains(" min read"));
    assert_eq!(after.count(), 1, "nothing is painted under the detail line");
}

#[test]
fn a_row_names_the_folder_the_reading_time_and_when_it_was_read() {
    let reader = after_reading(&[("travel/japan.md", "# Japan\n\nTwo weeks in spring.")]);

    assert_eq!(
        details(&reader),
        ["travel \u{b7} 1 min read \u{b7} opened just now"]
    );
}

#[test]
fn the_reading_time_grows_with_the_document() {
    for (count, minutes) in [(100, 1), (1000, 5), (2300, 10), (23_000, 100)] {
        let reader = after_reading(&[("essay.md", &words(count))]);

        let detail = details(&reader).join("");
        assert!(
            detail.contains(&format!(" \u{b7} {minutes} min read \u{b7} ")),
            "{count} words: {detail}"
        );
    }
}

#[test]
fn the_page_and_its_row_agree_on_the_reading_time() {
    let mut reader = after_reading(&[("essay.md", &format!("# Essay\n\n{}", words(700)))]);
    let detail = details(&reader).join("");

    reader.click("Essay");

    let names = reader.names();
    let kicker = names
        .iter()
        .find(|name| name.ends_with(" min read"))
        .unwrap();
    let minutes = kicker.rsplit(" \u{b7} ").next().unwrap();
    assert!(detail.contains(minutes), "{kicker} against {detail}");
}

#[test]
fn a_document_read_many_times_is_listed_once_at_the_top() {
    let mut reader = after_reading(&[
        ("one.md", "# One"),
        ("two.md", "# Two"),
        ("three.md", "# Three"),
    ]);

    for _ in 0..3 {
        reader.drop_file("one.md");
        reader.press("Ctrl+W");
    }

    assert_eq!(rows(&reader), ["One", "Three", "Two"]);
}

#[test]
fn the_list_remembers_twenty_documents_and_forgets_the_oldest() {
    let docs: Vec<(String, String)> = (1..=23)
        .map(|n| (format!("story-{n:02}.md"), format!("# Story {n}")))
        .collect();
    let borrowed: Vec<(&str, &str)> = docs
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let mut reader = after_reading(&borrowed);
    assert_eq!(rows(&reader).len(), 6);

    reader.scroll_down(600.0);
    reader.click("Show more\u{2026}");

    let listed = rows(&reader);
    assert_eq!(listed.len(), 20);
    assert_eq!(listed.first().map(String::as_str), Some("Story 23"));
    assert_eq!(listed.last().map(String::as_str), Some("Story 4"));
}

#[test]
fn a_document_that_was_deleted_is_still_listed_and_says_it_is_gone() {
    let reader = after_reading(&[("kept.md", "# Kept"), ("lost.md", "# Lost")]);
    reader.delete_file("lost.md");

    let reader = reader.restart(&[]);

    assert_eq!(rows(&reader), ["Lost", "Kept"]);
    let lines = details(&reader);
    assert!(lines[0].ends_with("file not found"), "{lines:?}");
    assert!(lines[1].ends_with("opened just now"), "{lines:?}");
}

#[test]
fn clicking_a_row_whose_file_is_gone_stays_on_the_home_page() {
    let reader = after_reading(&[("lost.md", "# Lost")]);
    reader.delete_file("lost.md");
    let mut reader = reader.restart(&[]);

    reader.click("Lost");

    reader.assert_sees("RECENTLY READ");
    assert_eq!(rows(&reader), ["Lost"]);
    assert_eq!(reader.title(), "Markdown Viewer");
}

#[test]
fn the_time_since_reading_is_worded_for_its_age() {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let cases = [
        (20, "opened just now"),
        (5 * MINUTE + 20, "opened 5m ago"),
        (59 * MINUTE + 20, "opened 59m ago"),
        (3 * HOUR + MINUTE, "opened 3h ago"),
        (30 * HOUR, "opened yesterday"),
        (3 * DAY + HOUR, "opened 3d ago"),
        (15 * DAY, "opened 2w ago"),
        (95 * DAY, "opened 3mo ago"),
        (800 * DAY, "opened 26mo ago"),
    ];

    for (age, wording) in cases {
        let launch = Reader::launch().doc("old.md", "# Old story");
        let saved = format!(
            "(recent_files:Some([(path:{:?},last_opened:{},title:Some(\"Old story\"),\
             excerpt:None,word_count:Some(2))]))",
            launch.path("old.md"),
            now() - age
        );

        let reader = launch.remembering(&saved).open();

        assert_eq!(rows(&reader), ["Old story"]);
        let detail = details(&reader).join("");
        assert!(detail.ends_with(wording), "{age} s old: {detail}");
    }
}

#[test]
fn a_document_read_in_the_future_by_a_wrong_clock_reads_as_just_now() {
    let launch = Reader::launch().doc("old.md", "# Old story");
    let saved = format!(
        "(recent_files:Some([(path:{:?},last_opened:{})]))",
        launch.path("old.md"),
        now() + 100_000
    );

    let reader = launch.remembering(&saved).open();

    assert_eq!(rows(&reader), ["old"]);
    let texts = reader.all_text();
    assert!(
        texts.iter().any(|text| text.ends_with("opened just now")),
        "{texts:?}"
    );
}

#[test]
fn opening_a_document_from_the_command_line_moves_it_to_the_top_of_the_list() {
    let reader = after_reading(&[("one.md", "# One"), ("two.md", "# Two")]);

    let mut reader = reader.restart(&["one.md"]);
    reader.press("Ctrl+W");

    assert_eq!(rows(&reader), ["One", "Two"]);
}

#[test]
fn a_document_brought_back_by_the_session_keeps_its_place_in_the_list() {
    let mut reader = after_reading(&[("one.md", "# One"), ("two.md", "# Two")]);
    reader.click("One");
    reader.drop_file("two.md");
    reader.click("one.md");

    let mut reader = reader.restart(&[]);
    reader.press("Ctrl+W");
    reader.press("Ctrl+W");

    assert_eq!(rows(&reader), ["Two", "One"]);
}

#[test]
fn the_home_page_has_no_contents_and_no_document_controls() {
    let mut reader = after_reading(&[("one.md", "# One\n\n## Section")]);

    reader.assert_hides("CONTENTS");
    assert!(reader.typed().is_empty());
    reader.click("More");
    assert!(!reader.is_enabled("Close Tab"));
}

#[test]
fn the_list_can_be_read_in_every_theme() {
    let mut reader = after_reading(&[("one.md", "# One\n\nAn opening line to read.")]);

    for _ in 0..4 {
        reader.press("Ctrl+D");

        let paper = reader.page_color();
        for text in ["RECENTLY READ", "An opening line to read.", " min read"] {
            let contrast = reader.ink_of(text).contrast(paper);
            assert!(contrast >= 3.0, "{text} on {paper:?}: {contrast:.1}");
        }
    }
}
