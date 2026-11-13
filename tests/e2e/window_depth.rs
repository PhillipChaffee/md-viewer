//! The real window, in depth: its title in each state, the ways out and
//! what each one saves, and a file changing under a real window.

use super::support::{kill, wait_for, Home, Running, Screen};

/// The paper a first run reads on, and the paper of the Dusk theme.
const SEPIA: &str = "F8F1E3";
const DUSK: &str = "2C2C2E";

/// A pixel in the page's left margin, clear of the text column.
const MARGIN: (u32, u32) = (60, 400);

/// A pixel in the middle of the text column, well below the title.
const MID_PAGE: (u32, u32) = (590, 400);

/// Start `md-viewer --foreground <args>` on `screen`, attached to the test.
fn show(home: &Home, screen: &Screen, args: &[&str]) -> Running {
    let mut line = vec!["--foreground"];
    line.extend(args);
    let mut command = home.md_viewer(&line);
    screen.show(&mut command);
    Running::start(&mut command)
}

#[test]
fn the_window_is_titled_after_the_file_however_it_is_named() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    let absolute = home.root().join("docs/far away/plan b.md");
    let absolute = absolute.to_str().unwrap();

    home.doc("my notes/trip plan.md", "# Trip");
    home.doc("-dash.md", "# Dash");
    home.doc("far away/plan b.md", "# Plan B");
    home.doc("sub/keep.md", "# Keep");
    home.doc("notes.txt", "# Text");

    for (name, args) in [
        ("trip plan.md", vec!["my notes/trip plan.md"]),
        ("-dash.md", vec!["--", "-dash.md"]),
        ("plan b.md", vec![absolute]),
        ("notes.txt", vec!["./sub/../notes.txt", "--no-watch"]),
    ] {
        let _app = show(&home, &screen, &args);

        screen.window_titled(&format!("{name} - Markdown Viewer"));
    }
}

#[test]
fn a_folder_on_the_command_line_opens_the_home_page() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("folder/inner.md", "# Inner");

    let mut app = show(&home, &screen, &["folder"]);

    screen.window_titled("Markdown Viewer");
    assert!(app.is_alive());
}

#[test]
fn closing_the_only_tab_retitles_the_window_and_keeps_it_open() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes");
    let mut app = show(&home, &screen, &["notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");

    let retitled = window.press_ctrl("w", || window.title() == "Markdown Viewer");

    assert!(retitled, "the window is titled {:?}", window.title());
    assert!(app.is_alive());
}

#[test]
fn quitting_from_the_home_page_exits_cleanly_and_the_next_launch_is_the_home_page() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    let mut app = show(&home, &screen, &[]);
    let window = screen.window_titled("Markdown Viewer");

    assert!(window.press_ctrl("q", || !app.is_alive()), "still running");

    assert_eq!(app.exit_code(), Some(0));
    let _again = show(&home, &screen, &[]);
    screen.window_titled("Markdown Viewer");
}

#[test]
fn the_settings_are_kept_in_the_users_data_folder_under_the_apps_name() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes");
    let mut app = show(&home, &screen, &["notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert!(home.saved_settings().is_empty(), "nothing is saved yet");

    assert!(window.press_ctrl("q", || !app.is_alive()), "still running");

    let saved = home.saved_settings();
    let data = home.root().join("data").join("md-viewer");
    assert!(!saved.is_empty(), "quitting saved nothing");
    for (path, _) in &saved {
        assert!(
            path.starts_with(&data),
            "{} is outside {data:?}",
            path.display()
        );
    }
}

#[test]
fn a_file_on_the_command_line_becomes_the_session_the_next_launch_resumes() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("first.md", "# First");
    home.doc("second.md", "# Second");
    for name in ["first.md", "second.md"] {
        let mut app = show(&home, &screen, &[name]);
        let window = screen.window_titled(&format!("{name} - Markdown Viewer"));
        assert!(window.press_ctrl("q", || !app.is_alive()), "still running");
        assert!(screen.has_no_window());
    }

    let _again = show(&home, &screen, &[]);

    screen.window_titled("second.md - Markdown Viewer");
}

#[test]
fn a_session_whose_document_was_deleted_resumes_on_the_home_page() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes");
    let mut app = show(&home, &screen, &["notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert!(window.press_ctrl("q", || !app.is_alive()), "still running");
    home.delete("notes.md");

    let mut again = show(&home, &screen, &[]);

    screen.window_titled("Markdown Viewer");
    assert!(again.is_alive());
}

#[test]
fn a_terminated_app_goes_away_and_the_next_launch_still_opens() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes");
    let mut app = show(&home, &screen, &["notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");

    kill(&window.pid());

    assert!(screen.has_no_window());
    assert!(wait_for(|| (!app.is_alive()).then_some(())).is_some());
    let mut again = show(&home, &screen, &["notes.md"]);
    screen.window_titled("notes.md - Markdown Viewer");
    assert!(again.is_alive());
}

#[test]
fn two_launches_give_two_windows() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("one.md", "# One");
    home.doc("two.md", "# Two");

    let mut first = show(&home, &screen, &["one.md"]);
    let mut second = show(&home, &screen, &["two.md"]);

    screen.window_titled("one.md - Markdown Viewer");
    screen.window_titled("two.md - Markdown Viewer");
    assert!(first.is_alive() && second.is_alive());
}

#[test]
fn the_real_window_repaints_when_the_file_changes() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "");
    let _app = show(&home, &screen, &["notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    let (x, y) = MID_PAGE;
    assert!(
        window.pixel_becomes(home.root(), x, y, SEPIA),
        "an empty page is {} at its middle",
        window.pixel(home.root(), x, y)
    );

    let block = format!("```text\n{}```\n", "a line of code\n".repeat(40));
    home.doc("notes.md", &block);

    let repainted = wait_for(|| (window.pixel(home.root(), x, y) != SEPIA).then_some(()));
    assert!(repainted.is_some(), "the page never showed the code block");
}

/// A home with `a.md` and `b.md`, and a session left by an earlier run
/// that had both open, `b.md` in front, plus `more` of its own.
fn home_with_two_tabs_saved(more: &str) -> Home {
    let home = Home::new();
    home.doc("a.md", "# Alpha");
    home.doc("b.md", "# Beta");
    home.left_session(&format!(
        "(open_tabs: Some([\"{}\", \"{}\"]), active_tab: Some(1){more})",
        home.doc_path("a.md").display(),
        home.doc_path("b.md").display()
    ));
    home
}

#[test]
fn a_session_left_by_an_earlier_run_is_resumed_from_its_file() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = home_with_two_tabs_saved("");

    let _app = show(&home, &screen, &[]);

    screen.window_titled("b.md - Markdown Viewer");
}

#[test]
fn a_session_file_from_before_the_themes_opens_in_the_theme_it_maps_to() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = home_with_two_tabs_saved(", dark_mode: Some(true), full_width_content: Some(false)");

    let _app = show(&home, &screen, &[]);

    let window = screen.window_titled("b.md - Markdown Viewer");
    let (x, y) = MARGIN;
    assert!(
        window.pixel_becomes(home.root(), x, y, DUSK),
        "the page is {}",
        window.pixel(home.root(), x, y)
    );
}

#[test]
fn the_title_follows_the_tab_in_front_and_the_last_tab_closed() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = home_with_two_tabs_saved("");
    let mut app = show(&home, &screen, &[]);
    let window = screen.window_titled("b.md - Markdown Viewer");

    for (key, title) in [
        ("Tab", "a.md - Markdown Viewer"),
        ("w", "b.md - Markdown Viewer"),
        ("w", "Markdown Viewer"),
    ] {
        let retitled = window.press_ctrl(key, || window.title() == title);

        assert!(
            retitled,
            "after Ctrl+{key} the title is {:?}",
            window.title()
        );
    }
    assert!(app.is_alive());
}

#[test]
fn a_damaged_settings_file_does_not_stop_the_app_and_is_replaced_on_quit() {
    let Some(screen) = Screen::start() else {
        return;
    };
    for damaged in [
        "",
        "not a settings file",
        "{\"md-viewer-state\": \"(theme: \"}",
        "\u{0}\u{1}",
    ] {
        let home = Home::new();
        home.doc("notes.md", "# Notes");
        home.left_settings_file(damaged);
        let mut app = show(&home, &screen, &[]);
        let window = screen.window_titled("Markdown Viewer");
        let (x, y) = MARGIN;
        assert!(
            window.pixel_becomes(home.root(), x, y, SEPIA),
            "{damaged:?}"
        );

        assert!(window.press_ctrl("q", || !app.is_alive()), "still running");

        assert_eq!(app.exit_code(), Some(0), "{damaged:?}");
        let saved = home.saved_settings();
        assert!(
            saved.iter().any(|(_, text)| text.contains("Sepia")),
            "the settings were not written afresh: {saved:?}"
        );
    }
}
