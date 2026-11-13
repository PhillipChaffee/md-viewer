//! Launching from a terminal: the prompt comes back unless `--foreground`
//! is given.

use super::support::{in_a_terminal, kill, Home, Running, Screen};

#[test]
fn launched_from_a_terminal_it_hands_the_prompt_back() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes");
    let mut command = in_a_terminal(&home, &["notes.md"]);
    screen.show(&mut command);

    let mut shell = Running::start(&mut command);

    assert_eq!(shell.exit_code(), Some(0), "the prompt never came back");
    let window = screen.window_titled("notes.md - Markdown Viewer");
    kill(&window.pid());
    assert!(screen.has_no_window());
}

#[test]
fn foreground_keeps_it_attached_to_the_terminal() {
    let Some(screen) = Screen::start() else {
        return;
    };
    let home = Home::new();
    home.doc("notes.md", "# Notes");
    let mut command = in_a_terminal(&home, &["--foreground", "notes.md"]);
    screen.show(&mut command);

    let mut shell = Running::start(&mut command);

    let window = screen.window_titled("notes.md - Markdown Viewer");
    assert!(
        shell.is_alive(),
        "the prompt came back with the window open"
    );
    assert!(
        window.press_ctrl("q", || !shell.is_alive()),
        "still running"
    );
    assert_eq!(shell.exit_code(), Some(0));
}
