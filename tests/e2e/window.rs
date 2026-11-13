//! The real window, on a private X display.

use super::support::{Home, Running, Screen};

/// The paper of the two themes a first run meets: Sepia, then Dusk.
const SEPIA: &str = "F8F1E3";
const DUSK: &str = "2C2C2E";

/// A pixel in the page's left margin: right of the window's edge, left of
/// the text column, halfway down.
const MARGIN: (u32, u32) = (60, 400);

/// Start `md-viewer <args>` on `screen`, attached to the test.
fn show(home: &Home, screen: &Screen, args: &[&str]) -> Running {
    let mut command = home.md_viewer(args);
    screen.show(&mut command);
    Running::start(&mut command)
}

#[test]
fn the_window_is_titled_after_the_document() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");

    let _app = show(&home, &screen, &["--foreground", "notes.md"]);

    screen.window_titled("notes.md - Markdown Viewer");
}

#[test]
fn with_no_file_the_window_is_the_home_page() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();

    let mut app = show(&home, &screen, &["--foreground"]);

    screen.window_titled("Markdown Viewer");
    assert!(app.is_alive());
}

#[test]
fn a_relative_path_is_read_from_the_working_directory() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("chapters/one.md", "# One");

    let _app = show(
        &home,
        &screen,
        &["--foreground", "chapters/one.md", "--no-watch"],
    );

    screen.window_titled("one.md - Markdown Viewer");
}

#[test]
fn a_missing_file_still_opens_the_window() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();

    let mut app = show(&home, &screen, &["--foreground", "absent.md"]);

    screen.window_titled("Markdown Viewer");
    assert!(app.is_alive());
}

#[test]
fn ctrl_q_quits_and_the_next_launch_resumes_the_session() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");
    let mut app = show(&home, &screen, &["--foreground", "notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");

    assert!(window.press_ctrl("q", || !app.is_alive()), "still running");

    assert_eq!(app.exit_code(), Some(0));
    assert!(screen.has_no_window());
    let saved = home.saved_settings();
    assert!(
        saved.iter().any(|(_, text)| text.contains("notes.md")),
        "the session does not name the open document: {saved:?}"
    );

    let _again = show(&home, &screen, &["--foreground"]);
    screen.window_titled("notes.md - Markdown Viewer");
}

#[test]
fn the_page_is_painted_in_the_theme_and_ctrl_d_changes_it() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");
    let _app = show(&home, &screen, &["--foreground", "notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    let (x, y) = MARGIN;

    assert!(
        window.pixel_becomes(home.root(), x, y, SEPIA),
        "the page is {}",
        window.pixel(home.root(), x, y)
    );

    let dusk = window.press_ctrl("d", || window.pixel(home.root(), x, y) == DUSK);

    assert!(dusk, "the page is {}", window.pixel(home.root(), x, y));
}

#[test]
fn the_theme_survives_a_quit() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");
    let mut app = show(&home, &screen, &["--foreground", "notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    let (x, y) = MARGIN;
    assert!(window.press_ctrl("d", || window.pixel(home.root(), x, y) == DUSK));
    assert!(window.press_ctrl("q", || !app.is_alive()), "still running");

    let _again = show(&home, &screen, &["--foreground", "notes.md"]);

    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert!(
        window.pixel_becomes(home.root(), x, y, DUSK),
        "the page is {}",
        window.pixel(home.root(), x, y)
    );
}
