//! Opening documents, in depth: every kind of name and file on the
//! command line, and everything that can be dropped on the window.

use std::fmt::Write as _;

use super::support::{Launch, Reader};

/// A folder with one of each kind of thing a reader might try to open.
fn drawer() -> Launch {
    Reader::launch()
        .doc("a.md", "# Alpha\n\nFirst.")
        .doc("b.md", "# Beta\n\nSecond.")
        .doc("c.markdown", "# Gamma\n\nThird.")
        .doc("d.txt", "# Delta\n\nFourth.")
        .doc("photo.png", "not a picture")
        .doc("README", "# No extension")
        .doc("folder/inner.md", "# Inner")
}

#[test]
fn a_file_opens_from_the_command_line_whatever_it_is_called() {
    let names = [
        "notes.md",
        "guide.markdown",
        "plain.txt",
        "README",
        "data.json",
        "UPPER.MD",
        "many.dots.in.name.md",
        "with space.md",
        "it's \"quoted\".md",
        "\u{fc}n\u{ef} c\u{f6}d\u{e9}/\u{444}\u{430}\u{439}\u{43b} \u{6771}\u{4eac}.md",
        "deep/er/est/doc.md",
    ];

    for name in names {
        let body = format!("Body of {}.", name.len());
        let launch = Reader::launch().doc(name, &format!("# Opened\n\n{body}"));

        let reader = launch.args(&[name]).open();

        let file = name.rsplit('/').next().unwrap();
        assert_eq!(reader.title(), format!("{file} - Markdown Viewer"));
        reader.assert_sees(&body);
        reader.assert_sees(&format!("{file} \u{b7} 1 min read"));
        reader.assert_hides("Unable to open");
    }
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_opens_the_document_it_points_to() {
    let launch = Reader::launch()
        .doc("real/target.md", "# Target\n\nThe real file.")
        .link("shortcut.md", "real/target.md");

    let mut reader = launch.args(&["shortcut.md"]).open();

    reader.assert_sees("The real file.");
    reader.assert_hides("Unable to open");
    reader.drop_file("real/target.md");
    assert_eq!(
        reader.count("target.md"),
        0,
        "one document, so no tab strip"
    );
}

#[test]
fn a_folder_on_the_command_line_is_not_a_document() {
    for name in ["folder", "folder.md"] {
        let launch = Reader::launch().doc(&format!("{name}/inner.md"), "# Inner");

        let reader = launch.args(&[name]).open();

        reader.assert_sees("Unable to open");
        reader.assert_sees(name);
        reader.assert_sees("md-viewer");
        reader.assert_hides("Inner");
        assert_eq!(reader.title(), "Markdown Viewer");
    }
}

#[test]
fn files_with_nothing_to_show_open_as_empty_pages() {
    let cases = [
        ("spaces.md", "   \n\n\t\n"),
        ("frontmatter.md", "---\ntitle: Only this\n---\n"),
        ("comment.md", "<!-- nothing to see -->\n"),
    ];

    for (name, content) in cases {
        let reader = Reader::with_docs(&[(name, content)]);

        reader.assert_hides("Unable to open");
        reader.assert_sees(&format!("{name} \u{b7} 1 min read"));
        assert_eq!(reader.title(), format!("{name} - Markdown Viewer"));
    }
}

#[test]
fn a_file_of_bytes_that_are_not_text_still_opens() {
    let reader = Reader::launch()
        .bytes(
            "blob.md",
            &[0, 159, 146, 150, 0, 255, 10, 35, 32, 72, 105, 10],
        )
        .args(&["blob.md"])
        .open();

    reader.assert_hides("Unable to open");
    assert_eq!(reader.title(), "blob.md - Markdown Viewer");
    reader.assert_sees("blob.md \u{b7} 1 min read");
}

#[test]
fn a_very_large_file_opens() {
    let mut text = String::from("# Ledger\n\nThe first line of many.\n\n");
    for line in 1..=20_000 {
        let _ = write!(text, "Entry {line} in the ledger.\n\n");
    }

    let mut reader = Reader::with_docs(&[("ledger.md", &text)]);

    reader.assert_sees("The first line of many.");
    reader.assert_sees("Entry 1 in the ledger.");
    reader.assert_hides("1 min read");
    reader.scroll_down(3000.0);
    assert!(!reader.in_view("The first line of many."));
}

#[test]
fn the_flags_may_come_before_or_after_the_file() {
    for args in [
        ["--no-watch", "a.md"],
        ["a.md", "--no-watch"],
        ["--foreground", "a.md"],
        ["a.md", "--foreground"],
    ] {
        let reader = drawer().args(&args).open();

        reader.assert_sees("Alpha");
        assert_eq!(reader.title(), "a.md - Markdown Viewer", "{args:?}");
    }
}

#[test]
fn help_and_version_are_answered_without_an_app() {
    let help = drawer().args(&["--help"]).try_open().err();
    let version = drawer().args(&["--version"]).try_open().err();

    assert!(
        help.as_deref()
            .is_some_and(|said| said.contains("Usage: md-viewer")),
        "{help:?}"
    );
    assert!(
        version
            .as_deref()
            .is_some_and(|said| said.contains(env!("CARGO_PKG_VERSION"))),
        "{version:?}"
    );
}

#[test]
fn three_files_on_the_command_line_are_refused_naming_the_extra_one() {
    let refusal = drawer().args(&["a.md", "b.md", "d.txt"]).try_open().err();

    assert!(
        refusal.as_deref().is_some_and(|said| said.contains("b.md")),
        "{refusal:?}"
    );
}

#[test]
fn a_relative_path_may_climb_and_descend() {
    let reader = Reader::launch()
        .raw_args(&["tests/reader/../fixtures/./plain.md", "--no-watch"])
        .open();

    reader.assert_sees("A plain fixture");
    assert_eq!(reader.title(), "plain.md - Markdown Viewer");
}

#[test]
fn every_kind_of_document_can_be_dropped() {
    let mut reader = drawer().args(&["a.md"]).open();

    for (name, heading) in [
        ("b.md", "Beta"),
        ("c.markdown", "Gamma"),
        ("d.txt", "Delta"),
    ] {
        reader.drop_file(name);

        reader.assert_sees(heading);
        assert!(reader.is_on(name), "{name} has the tab in front");
        assert_eq!(reader.title(), format!("{name} - Markdown Viewer"));
    }
    reader.assert_hides("Unsupported file type");
}

#[test]
fn what_is_not_a_document_is_refused_when_dropped() {
    for name in ["photo.png", "README", "folder"] {
        let mut reader = drawer().args(&["a.md"]).open();

        reader.drop_file(name);

        reader.assert_sees("Unsupported file type");
        reader.assert_sees(".md, .markdown, .txt");
        reader.assert_sees("Alpha");
        assert_eq!(reader.count("a.md"), 0, "{name} got no tab");
    }
}

#[test]
fn of_several_files_dropped_together_the_documents_open_and_the_rest_is_refused() {
    let mut reader = drawer().args(&["a.md"]).open();

    reader.drop_files(&["b.md", "photo.png", "d.txt"]);

    for tab in ["a.md", "b.md", "d.txt"] {
        assert_eq!(reader.count(tab), 1, "{tab}");
    }
    assert_eq!(reader.count("photo.png"), 0);
    reader.assert_sees("Unsupported file type");
    reader.assert_sees("Delta");
}

#[test]
fn dropping_the_document_in_front_changes_nothing() {
    let mut reader = drawer().args(&["a.md"]).open();

    reader.drop_file("a.md");

    reader.assert_sees("Alpha");
    assert_eq!(reader.count("a.md"), 0, "still one document, no tab strip");
    reader.assert_hides("Unable to open");
}

#[test]
fn the_invitation_to_drop_goes_away_when_the_files_are_taken_away() {
    let mut reader = drawer().args(&["a.md"]).open();
    reader.drag_files_over(&["b.md", "d.txt"]);
    reader.assert_sees("Drop markdown file here");

    reader.drag_files_over(&[]);

    reader.assert_hides("Drop markdown file here");
    reader.assert_sees("Alpha");
}

#[test]
fn a_later_problem_replaces_the_one_shown() {
    let mut reader = drawer().args(&["a.md"]).open();
    let gone = reader.path("gone.md");
    reader.drop_path(gone);
    reader.assert_sees("Unable to open");

    reader.drop_file("photo.png");

    reader.assert_sees("Unsupported file type");
    reader.assert_hides("Unable to open");
    assert_eq!(reader.count("Dismiss"), 1);
}

#[test]
fn a_document_dropped_on_the_home_page_after_a_refusal_opens() {
    let mut reader = drawer().open();
    reader.drop_file("photo.png");
    reader.assert_sees("Unsupported file type");

    reader.drop_file("c.markdown");

    reader.assert_sees("Gamma");
    assert_eq!(reader.title(), "c.markdown - Markdown Viewer");
}

#[test]
fn a_dropped_document_from_another_folder_opens_beside_the_first() {
    let mut reader = drawer().args(&["a.md"]).open();

    reader.drop_file("folder/inner.md");

    reader.assert_sees("Inner");
    assert!(reader.is_on("inner.md"));
    reader.click("a.md");
    reader.assert_sees("First.");
}
