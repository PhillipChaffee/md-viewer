//! The session, in depth: each remembered thing alone and all of them
//! together, sessions written by older versions, and session files that
//! are damaged.

use super::docs::{kitchen_sink, three_linked, KITCHEN_SINK};
use super::support::{Launch, Reader, Rgb};

/// The paper of each theme, in the order Ctrl+D steps from a first run.
const PAPERS: [(&str, &str); 4] = [
    ("Dusk", "#2C2C2E"),
    ("Night", "#000000"),
    ("Paper", "#FDFDFB"),
    ("Sepia", "#F8F1E3"),
];

/// The paper a first run reads on.
const DEFAULT_PAPER: &str = "#F8F1E3";

/// A launch on the kitchen sink with `saved` as the session file.
fn sink_remembering(saved: &str) -> Launch {
    Reader::launch()
        .doc("sink.md", KITCHEN_SINK)
        .doc("guide.md", "# Guide\n\nHow to use the sink.")
        .args(&["sink.md"])
        .remembering(saved)
}

/// The zoom the menu shows, such as `125%`.
fn zoom(reader: &mut Reader) -> String {
    reader.click("More");
    let names = reader.names();
    let label = names
        .iter()
        .find_map(|name| name.strip_prefix("Reset Zoom, "));
    let zoom = label.unwrap().to_owned();
    reader.press("Escape");
    zoom
}

/// Whether the menu row `label` is switched on.
fn menu_is_on(reader: &mut Reader, label: &str) -> bool {
    reader.click("More");
    let on = reader.is_on(label);
    reader.press("Escape");
    on
}

#[test]
fn each_theme_comes_back() {
    for (presses, (theme, paper)) in (1..).zip(PAPERS) {
        let mut reader = kitchen_sink();
        for _ in 0..presses {
            reader.press("Ctrl+D");
        }

        let mut reader = reader.restart(&["sink.md"]);

        assert_eq!(reader.page_color(), Rgb::hex(paper), "{theme}");
        reader.click("Appearance");
        assert!(reader.is_on(theme), "{theme} is the one chosen");
    }
}

#[test]
fn the_theme_comes_back_on_the_home_page_too() {
    let mut reader = Reader::first_run();
    reader.press("Ctrl+D");
    let dusk = reader.page_color();
    assert_ne!(dusk, Rgb::hex(DEFAULT_PAPER));

    let reader = reader.restart(&[]);

    assert_eq!(reader.page_color(), dusk);
}

#[test]
fn the_order_of_the_files_and_the_open_folders_come_back() {
    let mut reader = Reader::with_docs(&[
        ("readme.md", "# Read me"),
        ("changes.md", "# Changes"),
        ("guides/setup.md", "# Setup"),
    ]);
    reader.press("Ctrl+Shift+E");
    reader.wait_for("the folder to be listed", |reader| {
        reader.count("guides") == 1
    });
    reader.click("guides");
    reader.click("Explorer options");
    reader.click("Name Z-A");

    let mut reader = reader.restart(&["readme.md"]);
    reader.wait_for("the open folder to be listed", |reader| {
        reader.count("setup.md") == 1
    });

    let names = reader.names();
    let at = |name: &str| names.iter().position(|listed| listed == name);
    assert!(at("readme.md") < at("changes.md"), "{names:?}");
    reader.click("Explorer options");
    assert!(reader.is_on("Name Z-A"));
}

#[test]
fn everything_comes_back_together() {
    let mut reader = Reader::launch()
        .doc("sink.md", KITCHEN_SINK)
        .doc("guide.md", "# Guide\n\nHow to use the sink.")
        .doc("extra.md", "# Extra\n\nOne more.")
        .args(&["sink.md"])
        .window(1600.0, 900.0)
        .open();
    reader.drop_files(&["guide.md", "extra.md"]);
    reader.click("guide.md");
    reader.press("Ctrl+D");
    reader.press("Ctrl+D");
    reader.press("Ctrl+Alt+Right");
    reader.press("Ctrl++");
    reader.press("Ctrl+Shift+E");
    reader.press("Ctrl+Shift+O");
    reader.click("Appearance");
    reader.click("Larger text");
    reader.click("Compact");
    reader.press("Escape");
    let column = reader.column().width;

    let mut reader = reader.restart(&[]);

    for tab in ["sink.md", "guide.md", "extra.md"] {
        assert_eq!(reader.count(tab), 2, "{tab}: a tab and a row in the files");
    }
    assert_eq!(reader.title(), "guide.md - Markdown Viewer");
    assert_eq!(reader.page_color(), Rgb::hex("#000000"));
    assert!((reader.column().width - column).abs() < 1.0);
    reader.assert_sees("FILES");
    reader.assert_hides("CONTENTS");
    assert_eq!(zoom(&mut reader), "125%");
    reader.click("Appearance");
    reader.assert_sees("Large");
    assert!(reader.is_on("Compact"));
}

#[test]
fn a_restart_opens_documents_in_the_preview_with_no_find_bar() {
    let mut reader = three_linked();
    reader.press("Ctrl+E");
    reader.press("Ctrl+F");
    reader.type_text("alpha");

    let reader = reader.restart(&[]);

    assert!(reader.typed().is_empty(), "{:?}", reader.typed());
    assert_eq!(reader.count("Close find bar"), 0);
    reader.assert_sees("Alpha");
}

#[test]
fn a_first_run_starts_from_the_defaults() {
    let mut reader = kitchen_sink();

    assert_eq!(reader.page_color(), Rgb::hex(DEFAULT_PAPER));
    assert!((reader.column().width - 680.0).abs() < 1.0);
    reader.assert_hides("FILES");
    reader.assert_sees("CONTENTS");
    assert_eq!(zoom(&mut reader), "100%");
    assert!(menu_is_on(&mut reader, "Live Reload"));
    assert!(!menu_is_on(&mut reader, "Full Width"));
    reader.click("Appearance");
    reader.assert_sees("Normal");
    assert!(reader.is_on("Comfortable"));
    assert!(reader.is_on("Sepia"));
}

#[test]
fn a_session_from_before_the_themes_maps_dark_mode_to_a_theme() {
    for (saved, theme, paper) in [
        ("(dark_mode: Some(true))", "Dusk", "#2C2C2E"),
        ("(dark_mode: Some(false))", "Paper", "#FDFDFB"),
        ("(dark_mode: None)", "Sepia", DEFAULT_PAPER),
    ] {
        let mut reader = sink_remembering(saved).open();

        assert_eq!(reader.page_color(), Rgb::hex(paper), "{saved}");
        reader.click("Appearance");
        assert!(reader.is_on(theme), "{saved}");
    }
}

#[test]
fn a_saved_theme_wins_over_the_old_dark_mode_switch() {
    let reader = sink_remembering("(theme: Some(Night), dark_mode: Some(false))").open();

    assert_eq!(reader.page_color(), Rgb::hex("#000000"));
}

#[test]
fn a_session_from_before_the_resizable_column_maps_full_width() {
    let mut full = sink_remembering("(full_width_content: Some(true))").open();
    let mut fixed = sink_remembering("(full_width_content: Some(false))").open();

    assert!(menu_is_on(&mut full, "Full Width"));
    assert!(full.column().width > 800.0, "{:?}", full.column());
    assert!(!menu_is_on(&mut fixed, "Full Width"));
    assert!((fixed.column().width - 680.0).abs() < 1.0);
}

#[test]
fn a_saved_width_wins_over_the_old_full_width_switch() {
    let saved = "(reading_width: Some(520.0), full_width_content: Some(true))";

    let reader = sink_remembering(saved).open();

    assert!((reader.column().width - 520.0).abs() < 1.0);
}

#[test]
fn an_old_session_keeps_its_tabs_zoom_and_sidebars() {
    let launch = Reader::launch()
        .doc("a.md", "# Alpha\n\n## Part")
        .doc("b.md", "# Beta\n\n## Part");
    let saved = format!(
        "(dark_mode: Some(true), zoom_level: Some(1.25), show_outline: Some(false), \
         full_width_content: Some(false), show_explorer: Some(true), \
         open_tabs: Some([{:?}, {:?}]), active_tab: Some(1))",
        launch.path("a.md"),
        launch.path("b.md")
    );

    let mut reader = launch.remembering(&saved).open();

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    reader.assert_sees("FILES");
    reader.assert_hides("CONTENTS");
    assert_eq!(reader.page_color(), Rgb::hex("#2C2C2E"));
    assert_eq!(zoom(&mut reader), "125%");
}

#[test]
fn an_old_list_of_read_documents_without_summaries_is_still_listed() {
    let launch = Reader::launch().doc("old/tale.md", "# A tale");
    let saved = format!(
        "(recent_files: Some([(path: {:?}, last_opened: 1700000000)]))",
        launch.path("old/tale.md")
    );

    let reader = launch.remembering(&saved).open();

    reader.assert_sees("RECENTLY READ");
    assert_eq!(reader.count("tale"), 1, "listed by its file name");
    reader.assert_sees("old \u{b7} ");
}

#[test]
fn a_damaged_session_file_starts_like_a_first_run() {
    let damaged = [
        "",
        "garbage",
        "(",
        "{}",
        "[]",
        "null",
        "(theme: Some(Chartreuse))",
        "(zoom_level: Some(\"big\"))",
        "(open_tabs: Some(7))",
        "(theme: Some(Night), reading_width: )",
        "\u{0}\u{1}\u{fffd}",
    ];

    for saved in damaged {
        let mut reader = Reader::launch().remembering(saved).open();

        reader.assert_sees("Nothing read yet.");
        assert_eq!(reader.title(), "Markdown Viewer");
        assert_eq!(reader.page_color(), Rgb::hex(DEFAULT_PAPER), "{saved:?}");
        assert_eq!(zoom(&mut reader), "100%", "{saved:?}");
    }
}

#[test]
fn a_session_with_one_thing_in_it_restores_that_and_defaults_the_rest() {
    let mut reader = sink_remembering("(theme: Some(Night))").open();

    assert_eq!(reader.page_color(), Rgb::hex("#000000"));
    assert!((reader.column().width - 680.0).abs() < 1.0);
    assert_eq!(zoom(&mut reader), "100%");
    reader.assert_sees("CONTENTS");
    reader.assert_hides("FILES");
}

#[test]
fn values_out_of_range_are_brought_back_into_range() {
    let cases = [
        ("(zoom_level: Some(50.0))", "300%"),
        ("(zoom_level: Some(0.0))", "50%"),
        ("(zoom_level: Some(-3.0))", "50%"),
    ];

    for (saved, shown) in cases {
        let mut reader = sink_remembering(saved).open();

        assert_eq!(zoom(&mut reader), shown, "{saved}");
        reader.assert_sees("Kitchen Sink");
    }
}

#[test]
fn a_column_saved_too_narrow_or_too_wide_still_fits_the_page() {
    let narrow = sink_remembering("(reading_width: Some(5.0))").open();
    let wide = sink_remembering("(reading_width: Some(90000.0))").open();

    assert!(
        (narrow.column().width - 280.0).abs() < 1.0,
        "{:?}",
        narrow.column()
    );
    assert!(wide.column().width > 800.0, "{:?}", wide.column());
    assert!(wide.column().right() <= 1200.0, "{:?}", wide.column());
}

#[test]
fn sidebars_saved_too_narrow_come_back_wide_enough_to_use() {
    let saved = "(show_explorer: Some(true), show_outline: Some(true), \
                 explorer_width: Some(2.0), outline_width: Some(-40.0))";

    let reader = sink_remembering(saved).open();

    reader.assert_sees("FILES");
    assert!(reader.area_of("CONTENTS").width >= 40.0);
    assert!(reader.column().left >= 60.0, "{:?}", reader.column());
}

#[test]
fn a_front_tab_number_past_the_end_shows_the_last_tab() {
    let launch = Reader::launch()
        .doc("a.md", "# Alpha")
        .doc("b.md", "# Beta");
    let saved = format!(
        "(open_tabs: Some([{:?}, {:?}]), active_tab: Some(99))",
        launch.path("a.md"),
        launch.path("b.md")
    );

    let reader = launch.remembering(&saved).open();

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    reader.assert_sees("Beta");
}

#[test]
fn saved_tabs_that_are_gone_or_are_folders_are_left_out() {
    let launch = Reader::launch()
        .doc("a.md", "# Alpha")
        .doc("b.md", "# Beta")
        .folder("folder.md");
    let saved = format!(
        "(open_tabs: Some([{:?}, {:?}, {:?}, {:?}]), active_tab: Some(0))",
        launch.path("missing.md"),
        launch.path("folder.md"),
        launch.path("a.md"),
        launch.path("b.md")
    );

    let reader = launch.remembering(&saved).open();

    reader.assert_hides("Unable to open");
    assert_eq!(reader.count("a.md"), 1);
    assert_eq!(reader.count("b.md"), 1);
    assert_eq!(reader.count("folder.md"), 0);
    assert_eq!(reader.count("missing.md"), 0);
}

#[test]
fn a_session_survives_two_restarts_in_a_row() {
    let mut reader = three_linked();
    reader.drop_file("b.md");
    reader.press("Ctrl+D");

    let reader = reader.restart(&[]).restart(&[]);

    assert_eq!(reader.title(), "b.md - Markdown Viewer");
    assert_eq!(reader.count("a.md"), 1);
    assert_eq!(reader.page_color(), Rgb::hex("#2C2C2E"));
}
