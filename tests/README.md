# Tests that survive a rewrite

The tests in this folder check what a person using md-viewer can observe.
They do not name any type, module or function inside `src/`, so you can
rename, move or rewrite all of it and these tests still compile. They fail
only when the app behaves differently.

The unit tests in `src/**/tests.rs` are a different layer. They name
internal types and move with the code they test.

## The layers

| Folder | What runs | What it can observe |
| --- | --- | --- |
| `reader/` | The whole app, in the test process, without a window | Accessible labels, roles and states; painted text and colors; the window title; files on disk; what a restart remembers |
| `e2e/` | The built `md-viewer` binary | Exit codes and output; a real window on a private X display: its title, a screenshot's pixels, key presses, the saved session |
| `fixtures/` | Nothing | Small documents the tests open |

`reader/` reaches the app through one function, `md_viewer::app(args,
creation)`. That function and `md_viewer::run()` are the crate's whole
public surface. A rewrite has to keep them and nothing else.

## The rule: tests talk only to the driver

A test file calls the driver in its layer's `support/` folder and nothing
else. It does not import egui, eframe or kittest, and it does not start
processes itself.

- `reader/support/` has `Reader`: launch, click, press, type, drop a file,
  restart, and questions such as `sees`, `is_on`, `page_color`, `column`.
  It also reads what a frame painted: the ink and coverage of a picture
  (`picture_ink`, `picture_cover`), the lines and boxes in an area
  (`marks_in`), and what the app asked of the desktop (`opened_in_browser`,
  `clipboard`).
- `e2e/support/` has `Home`, `Running`, `Screen` and `Window`.

If the way the app is built or run changes, the driver changes and no test
does. If a test needs something the driver cannot do, add a method to the
driver, named for what the person does or sees.

## Run them

```bash
cargo test --test reader
```

```bash
cargo test --test e2e
```

`cargo test` runs both, with the unit tests.

The window tests in `e2e/` need `Xvfb`, `xdotool`, ImageMagick (`import`
and `convert`) and `script`. On Debian and Ubuntu:

```bash
sudo apt-get install xvfb xdotool imagemagick
```

Without those tools the window tests fail and say what is missing. They do
not pass silently. On a machine that cannot have the tools, set
`MD_VIEWER_SKIP_WINDOW_TESTS=1` to skip them.

## Add a test

1. Pick the module for the area, for example `reader/find.rs`. A new area
   is a new file, declared in `reader/main.rs`.
2. Launch with the documents the test needs:
   `Reader::with_docs(&[("notes.md", "# Notes")])`.
3. Act as the reader would: `reader.press("Ctrl+F")`,
   `reader.click("Next match")`.
4. Assert what the reader would see: `reader.assert_sees("2 / 3")`.
5. Break the app once and watch the test fail. A test that cannot fail
   proves nothing.

Derive the expected value from what the app promises (the README, the
labels on screen), not from the code. For work that finishes on another
thread, use `reader.wait_for(...)` and never a fixed sleep.

## Known bugs

None at the moment. When a test finds behavior the app does not have yet
and the fix has to wait, state the right behavior in the test, mark it
`#[should_panic(expected = "...")]` and list it in a table here. It then
passes while the bug exists and fails on the day the bug is fixed; remove
the marker and the row then.
