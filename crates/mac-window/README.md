# mac-window

What md-viewer asks of AppKit about its own window on macOS. On every other
system each function here does nothing.

| Function | What it does on a Mac |
| --- | --- |
| `center_window_buttons` | Makes the title bar as tall as the app's top row, so the close, minimize and zoom buttons sit on that row's centre line |
| `hand_quit_key_to_app` | Takes ⌘Q off the menu bar's Quit item, so the key reaches the app and it can ask about unsaved edits |
| `ask_before_quit` | Has macOS ask the app before it quits it: for Quit in the menu bar and the Dock, and for a log-out, a restart or a shut-down |

The app crate forbids unsafe code. Reading a window out of a raw window
handle and walking up from a window button to the title bar are both
unsafe, so they live here, in one function, `mac::title_bar`. Giving the
application's delegate the method macOS asks before it quits is unsafe as
well, and lives in a second, `mac::add_should_terminate`. The
[gates](../../GATES.md) record both exceptions.

The arithmetic is in `placement` and the answer to a request to quit in
`quit_answer`, which are compiled and tested on every system. The AppKit calls in `src/mac.rs` are compiled on macOS only and no
test runs them.
