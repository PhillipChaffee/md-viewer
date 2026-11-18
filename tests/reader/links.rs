//! Where a link in a document leads: another document, a heading, the
//! browser, or an error that says so.

use super::docs::long_document;
use super::support::Reader;

/// `index.md` holding `body`, beside a few documents for it to link to.
fn library(body: &str) -> Reader {
    Reader::with_docs(&[
        ("index.md", &format!("# Index\n\n{body}")),
        ("guide.md", "# Guide\n\nThe guide itself."),
        (
            "my notes.md",
            "# Notes\n\nNotes with a space in their name.",
        ),
        (
            "sub/inner.md",
            "# Inner\n\nGo [back up](../index.md) or [across](sibling.md).",
        ),
        ("sub/sibling.md", "# Sibling\n\nBeside the inner one."),
        ("old.markdown", "# Old\n\nAn old extension."),
        ("plain.txt", "Plain text, read as a document."),
        ("data.csv", "a,b\n1,2\n"),
    ])
}

/// Each way of writing a link to another document, and words only the
/// document it leads to holds.
const WAYS_TO_ANOTHER_DOCUMENT: [(&str, &str); 9] = [
    ("[go](guide.md)", "The guide itself."),
    ("[go](./guide.md)", "The guide itself."),
    ("[go](my%20notes.md)", "Notes with a space in their name."),
    ("[go](<my notes.md>)", "Notes with a space in their name."),
    ("[go](sub/inner.md)", "back up"),
    ("[go](guide.md?v=2)", "The guide itself."),
    ("[go](old.markdown)", "An old extension."),
    ("[go](plain.txt)", "Plain text, read as a document."),
    (
        "[go][ref] there.\n\n[ref]: guide.md\n\nAnd",
        "The guide itself.",
    ),
];

#[test]
fn every_way_of_writing_a_link_to_a_document_opens_it_in_place() {
    for (link, words) in WAYS_TO_ANOTHER_DOCUMENT {
        let mut reader = library(&format!("Now {link} there."));

        reader.click("go");

        assert!(reader.sees(words), "{link} did not lead to `{words}`");
        assert!(reader.is_enabled("Go back"), "{link} left no way back");
        reader.assert_hides("Unable to open");
    }
}

#[test]
fn a_link_is_followed_from_the_folder_of_the_document_it_is_in() {
    let mut reader = library("Go [down](sub/inner.md).");
    reader.click("down");

    reader.click("across");
    reader.assert_sees("Beside the inner one.");

    reader.press("Alt+Left");
    reader.click("back up");
    reader.assert_sees("Go ");
    assert_eq!(reader.title(), "index.md - Markdown Viewer");
}

#[test]
fn a_link_written_as_an_absolute_path_or_a_file_address_opens_too() {
    for scheme in ["", "file://"] {
        let mut reader = library("x");
        let link = format!("[go]({scheme}{})", reader.path("guide.md").display());
        reader.write_file("index.md", &format!("# Index\n\nNow {link} there."));
        reader.wait_for("the page to reload", |reader| reader.sees("go"));

        reader.click("go");

        assert!(reader.sees("The guide itself."), "{link}");
    }
}

#[test]
fn links_work_wherever_they_are_written() {
    let places = [
        "- a list item with [go](guide.md)",
        "> a quote with [go](guide.md)",
        "| A |\n| --- |\n| [go](guide.md) |",
        "**bold [go](guide.md) words**",
    ];

    for place in places {
        let mut reader = library(place);

        reader.click("go");

        assert!(reader.sees("The guide itself."), "in `{place}`");
    }
}

#[test]
fn a_link_in_a_heading_can_be_followed() {
    let mut reader = library("## A heading with [go](guide.md)");

    reader.click("go");

    assert!(
        reader.sees("The guide itself."),
        "a link in a heading leads nowhere"
    );
}

#[test]
fn a_link_to_the_web_is_handed_to_the_browser_and_the_page_stays() {
    let mut reader = library("Read [the site](https://example.com/docs?page=2#top).");

    reader.click("the site");

    assert_eq!(
        reader.opened_in_browser(),
        ["https://example.com/docs?page=2#top"]
    );
    reader.assert_sees("Read ");
    assert_eq!(reader.title(), "index.md - Markdown Viewer");
    assert!(!reader.is_enabled("Go back"), "nowhere to go back to");
    reader.assert_hides("Unable to open");
}

#[test]
fn a_link_inside_the_app_is_not_handed_to_the_browser() {
    let mut reader = library("Read [the guide](guide.md).");

    reader.click("the guide");

    assert!(reader.opened_in_browser().is_empty());
}

#[test]
fn addresses_that_are_not_documents_go_to_the_desktop() {
    let addresses = [
        "mailto:someone@example.com",
        "http://example.com/plain",
        "https://example.com/readme.md",
        "ftp://example.com/file.md",
        "//example.com/protocol-relative.md",
    ];

    for address in addresses {
        let mut reader = library(&format!("Now [go]({address}) there."));

        reader.click("go");

        assert_eq!(reader.opened_in_browser(), [address]);
        assert_eq!(reader.title(), "index.md - Markdown Viewer", "{address}");
        reader.assert_hides("Unable to open");
    }
}

#[test]
fn an_address_written_bare_between_angle_brackets_is_a_link() {
    let mut reader = library("See <https://example.com/auto> for more.");

    reader.click("https://example.com/auto");

    assert_eq!(reader.opened_in_browser(), ["https://example.com/auto"]);
}

#[test]
fn resting_on_a_web_link_shows_where_it_leads() {
    let mut reader = library("Read [the site](https://example.com/docs).");
    reader.assert_hides("https://example.com/docs");

    reader.hover("the site");

    reader.assert_sees("https://example.com/docs");
}

#[test]
fn a_link_is_set_apart_from_the_words_around_it() {
    let reader = library("Read [the guide](guide.md) and [the site](https://example.com).");

    let prose = reader.ink_of("Read ");

    for link in ["the guide", "the site"] {
        assert_ne!(reader.ink_of(link), prose, "{link}");
        assert!(reader.is_underlined(link), "{link}");
    }
    assert!(!reader.is_underlined("Read "));
}

#[test]
fn the_links_menu_copies_the_address_as_it_was_written() {
    let cases = [
        (
            "[go](https://example.com/a%20b)",
            "https://example.com/a%20b",
        ),
        ("[go](my%20notes.md)", "my%20notes.md"),
        ("[go](sub/inner.md#inner)", "sub/inner.md#inner"),
    ];

    for (link, address) in cases {
        let mut reader = library(&format!("Now {link} there."));

        reader.right_click("go");
        reader.click("Copy Link Address");

        assert_eq!(reader.clipboard(), Some(address));
        assert_eq!(reader.title(), "index.md - Markdown Viewer");
        assert_eq!(reader.count("Copy Link Address"), 0, "the menu closed");
    }
}

#[test]
fn a_link_whose_escapes_spell_no_name_says_it_cannot_be_opened() {
    let mut reader = library("Now [go](bad%FF.md) there.");

    reader.click("go");

    reader.assert_sees("Unable to open bad%FF.md");
    assert_eq!(reader.title(), "index.md - Markdown Viewer");
}

#[test]
fn the_error_stays_until_it_is_dismissed() {
    let mut reader = library("Now [go](missing.md) there.");
    reader.click("go");
    reader.assert_sees("Unable to open missing.md");

    reader.press("PageDown");
    reader.press("Ctrl+D");
    reader.assert_sees("Unable to open missing.md");

    reader.click("Dismiss");
    reader.assert_hides("Unable to open");
    assert_eq!(reader.count("Dismiss"), 0);
}

#[test]
fn a_good_link_can_be_followed_while_an_error_is_showing() {
    let mut reader = library("Now [go](missing.md) or [the guide](guide.md).");
    reader.click("go");

    reader.click("the guide");

    reader.assert_sees("The guide itself.");
}

#[test]
fn a_link_to_the_documents_own_title_goes_to_the_top() {
    let text = format!("{}\n\n[to the top](#long-read)", long_document(60));
    let mut reader = Reader::with_docs(&[("long.md", &text)]);
    for _ in 0..40 {
        reader.press("PageDown");
    }
    assert!(reader.in_view("to the top") && !reader.in_view("Section 1"));

    reader.click("to the top");
    reader.wait_for("the top to scroll into view", |reader| {
        reader.in_view("Section 1")
    });

    assert!(!reader.in_view("to the top"));
}

#[test]
fn a_heading_is_found_by_the_address_its_words_make() {
    let filler = "A paragraph of filler.\n\n".repeat(60);
    let cases = [
        ("Settings & Config", "#settings--config"),
        ("What's New?", "#whats-new"),
        ("Step 2: Build", "#step-2-build"),
        ("snake_case name", "#snake_case-name"),
        ("MiXeD Case", "#mixed-case"),
    ];

    for (heading, address) in cases {
        let text =
            format!("# Top\n\n[jump]({address})\n\n{filler}## {heading}\n\nThe target paragraph.");
        let mut reader = Reader::with_docs(&[("long.md", &text)]);
        assert!(!reader.in_view("The target paragraph."));

        reader.click("jump");
        reader.wait_for("the heading to scroll into view", |reader| {
            reader.in_view("The target paragraph.")
        });
    }
}

#[test]
fn a_link_to_a_heading_that_is_not_there_leaves_the_page_where_it_is() {
    let text = format!("[nowhere](#no-such-heading)\n\n{}", long_document(40));
    let mut reader = Reader::with_docs(&[("long.md", &text)]);

    reader.click("nowhere");

    assert!(reader.in_view("nowhere"));
    assert_eq!(reader.title(), "long.md - Markdown Viewer");
}

#[test]
fn ctrl_clicking_a_link_to_a_heading_opens_a_tab_at_that_heading() {
    let mut reader = Reader::with_docs(&[
        (
            "index.md",
            "# Index\n\nSee [the far section](long.md#section-50).",
        ),
        ("long.md", &long_document(60)),
    ]);

    reader.ctrl_click("the far section");
    reader.wait_for("the section to scroll into view", |reader| {
        reader.in_view("Section 50")
    });

    assert_eq!(reader.count("index.md"), 1, "the first tab is still there");
    assert_eq!(reader.title(), "long.md - Markdown Viewer");
}

#[test]
fn ctrl_clicking_a_web_link_still_goes_to_the_browser() {
    let mut reader = library("Read [the site](https://example.com/docs).");

    reader.ctrl_click("the site");

    assert_eq!(reader.opened_in_browser(), ["https://example.com/docs"]);
    assert_eq!(reader.count("index.md"), 0, "no tab strip: one document");
}

#[test]
fn the_name_of_a_document_that_exists_is_a_link_without_any_markup() {
    let mut reader = library("The setup is in guide.md and nothing is in ghost.md here.");

    reader.click("guide.md");

    reader.assert_sees("The guide itself.");
}

#[test]
fn a_document_that_stops_existing_is_reported_when_its_link_is_clicked() {
    let mut reader = library("Read [the guide](guide.md).");
    reader.delete_file("guide.md");

    reader.click("the guide");

    reader.assert_sees("Unable to open");
    reader.assert_sees("Read ");
}
