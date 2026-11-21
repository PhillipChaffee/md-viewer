//! The real window, on a private X display.

use super::support::{wait_for, Home, Icon, Running, Screen, Window};

/// The paper of the two themes a first run meets: Sepia, then Dusk.
const SEPIA: &str = "F8F1E3";
const DUSK: &str = "2C2C2E";

/// A pixel in the page's left margin: right of the window's edge, left of
/// the text column, halfway down.
const MARGIN: (u32, u32) = (60, 400);

/// A spot inside the dot the top row paints beside the edit switch while
/// a document has unsaved edits, in a window of the size the app opens at.
const UNSAVED_DOT: (u32, u32) = (911, 20);

/// The square the notice that a file changed on disk paints its warning
/// sign in, under the top row at the window's left edge: its corner and
/// its side. The page is blank there without the notice.
const NOTICE_SIGN: (u32, u32, u32) = (14, 47, 20);

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

/// Open `notes.md` with `args` and type an edit into its source. Returns
/// the app and its window.
fn edit_notes<'screen>(
    home: &Home,
    screen: &'screen Screen,
    args: &[&str],
) -> (Running, Window<'screen>) {
    home.doc("notes.md", "# Notes\n\nSome text.");
    let app = show(home, screen, args);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    let (x, y) = MARGIN;
    assert!(window.pixel_becomes(home.root(), x, y, SEPIA));

    assert!(window.press_ctrl("e", || true));
    // The source can take a moment to be there to type into: the edit is
    // typed until the dot that marks unsaved edits is painted.
    let (dot_x, dot_y) = UNSAVED_DOT;
    let edited = wait_for(|| {
        window.click(600, 300);
        window.type_text("Mine. ");
        (window.pixel(home.root(), dot_x, dot_y) != SEPIA).then_some(())
    });
    assert!(edited.is_some(), "the edit was never taken");
    (app, window)
}

/// Open `notes.md`, type an edit into its source, press Ctrl+Q, and wait
/// for the prompt about unsaved edits, which is seen by the shade it lays
/// over the page. Returns the app, still running, and its window.
fn quit_with_an_unsaved_edit<'screen>(
    home: &Home,
    screen: &'screen Screen,
) -> (Running, Window<'screen>) {
    let args = ["--foreground", "notes.md", "--no-watch"];
    let (mut app, window) = edit_notes(home, screen, &args);
    let (x, y) = MARGIN;
    let shaded = window.press_ctrl("q", || window.pixel(home.root(), x, y) != SEPIA);

    assert!(shaded, "no prompt came up");
    assert!(app.is_alive(), "the app quit with edits unsaved");
    (app, window)
}

#[test]
fn ctrl_q_with_unsaved_edits_asks_and_save_writes_them_before_the_window_closes() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    let on_disk = || std::fs::read_to_string(home.doc_path("notes.md")).unwrap();
    let (mut app, window) = quit_with_an_unsaved_edit(&home, &screen);
    assert!(!on_disk().contains("Mine."), "nothing is written unasked");

    window.press("Return");

    assert_eq!(app.exit_code(), Some(0), "Save did not go on to quit");
    assert!(on_disk().contains("Mine. "), "Save wrote {:?}", on_disk());
    assert!(screen.has_no_window());
}

#[test]
fn discard_at_the_quit_prompt_closes_the_window_and_writes_nothing() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    let (mut app, window) = quit_with_an_unsaved_edit(&home, &screen);

    window.press("Tab");
    window.press("Return");

    assert_eq!(app.exit_code(), Some(0), "Discard did not go on to quit");
    let on_disk = std::fs::read_to_string(home.doc_path("notes.md")).unwrap();
    assert_eq!(on_disk, "# Notes\n\nSome text.");
    assert!(screen.has_no_window());
}

#[test]
fn a_file_changed_under_unsaved_edits_is_announced_and_a_save_asks_before_it_writes() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    let theirs = "# Notes\n\nWritten by another program.";
    let on_disk = || std::fs::read_to_string(home.doc_path("notes.md")).unwrap();
    let (mut app, window) = edit_notes(&home, &screen, &["--foreground", "notes.md"]);
    let (sign_x, sign_y, side) = NOTICE_SIGN;
    let announced = || window.paints_in(home.root(), sign_x, sign_y, side);
    assert!(!announced(), "a notice before the file changed");

    home.doc("notes.md", theirs);

    assert!(
        wait_for(|| announced().then_some(())).is_some(),
        "no notice"
    );
    assert_eq!(on_disk(), theirs, "the notice alone wrote the file");

    // The question is seen by the shade it lays over the page.
    let (x, y) = MARGIN;
    let asked = window.press_ctrl("s", || window.pixel(home.root(), x, y) != SEPIA);
    assert!(asked, "Ctrl+S asked nothing");
    assert_eq!(on_disk(), theirs, "the save wrote before it was answered");

    // The keyboard starts on Cancel.
    window.press("Return");
    assert!(window.pixel_becomes(home.root(), x, y, SEPIA));
    assert_eq!(on_disk(), theirs, "Cancel wrote the file");

    let asked = window.press_ctrl("s", || window.pixel(home.root(), x, y) != SEPIA);
    assert!(asked, "Ctrl+S asked nothing the second time");
    window.press("shift+Tab");
    window.press("Return");
    let written = wait_for(|| on_disk().contains("Mine. ").then_some(()));
    assert!(written.is_some(), "Overwrite wrote {:?}", on_disk());
    assert!(app.is_alive());
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

/// The README: the icon is a dark M on warm paper, and the same M reversed
/// on a near-black tile while a dark theme is on.
#[test]
fn a_first_run_opens_with_the_light_tile_for_its_icon() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");

    let _app = show(&home, &screen, &["--foreground", "notes.md"]);

    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert_eq!(window.icon(), Some(Icon::LightTile));
}

#[test]
fn a_launch_into_a_saved_dark_theme_opens_with_the_dark_tile() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");
    home.left_session("(theme:Some(Night))");

    let _app = show(&home, &screen, &["--foreground", "notes.md"]);

    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert_eq!(window.icon(), Some(Icon::DarkTile));
}

/// Ctrl+D on a first run goes from Sepia to Dusk, a light theme to a dark
/// one, and an X11 window changes its icon while it runs.
#[test]
fn the_window_icon_follows_ctrl_d_from_a_light_theme_to_a_dark_one() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes\n\nSome text.");
    let _app = show(&home, &screen, &["--foreground", "notes.md"]);
    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert_eq!(window.icon(), Some(Icon::LightTile));

    let dark = window.press_ctrl("d", || window.icon() == Some(Icon::DarkTile));

    assert!(dark, "in Dusk the icon is {:?}", window.icon());
}
