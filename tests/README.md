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

The tests in this table state behavior the app does not have yet. Each is marked
`#[should_panic(expected = "...")]`, so it passes while the bug exists and
fails on the day the bug is fixed. Remove the marker then.

| Test | The bug |
| --- | --- |
| `opening::a_file_that_could_not_be_opened_is_not_listed_as_read` | `md-viewer missing.md` puts the missing file on the recently read list |
| `editing::the_contents_follow_what_was_typed` | After an edit, the preview shows a new heading and the contents sidebar does not |
| `themes::a_shortcut_tapped_inside_one_frame_still_counts` | A shortcut whose modifier is released in the same frame as the key is ignored |
| `prose::a_quote_inside_a_quote_steps_in_again` | A quote inside a quote is set at the same indent as the quote around it |
| `prose::a_list_after_a_quote_inside_a_quote_is_set_straight` | The first item of a list that follows a quote inside a quote is pushed far to the right |
| `links::a_link_in_a_heading_can_be_followed` | A link written in a heading is drawn over the start of the heading and does nothing when clicked |
| `pictures::a_gif_is_shown` | A GIF is looked for under its name with `#0` on the end, is not found, and a warning sign is shown instead |
| `pictures::a_picture_that_appears_later_is_shown_once_the_page_is_reloaded` | A picture that failed to load once is not tried again when the page reloads |
| `lightbox::the_buttons_of_the_top_bar_do_not_act_while_a_picture_is_enlarged` | The enlarged view does not cover the top bar, so its buttons act on the window behind the picture |
| `math::a_plain_sum_between_dollar_signs_is_a_formula` | `$b + c$` is taken for prose and shown as typed, less its closing dollar sign |
| `diagrams::a_diagram_too_large_for_the_screen_does_not_stop_the_app` | A diagram drawn larger than the graphics card's biggest texture makes the app panic |
| `find_depth::following_a_link_counts_the_document_arrived_at` | With the find bar open, a link followed in place leaves the bar on `0 matches` although the new page has matches |
| `tabs_depth::the_tab_in_front_is_scrolled_into_the_strip` | With more tabs than fit, the strip does not scroll to the tab in front, so a document just opened has no tab on screen |
| `tabs_depth::with_more_tabs_than_fit_the_open_button_keeps_clear_of_the_edit_switch` | With more tabs than fit, the open-file button is drawn over the Preview and Edit switch |
| `editing_depth::ctrl_s_saves_from_the_preview_too` | Back in the preview the unsaved mark says to press Ctrl+S, and Ctrl+S saves nothing there |
| `keyboard::closing_a_tab_from_the_find_bar_leaves_the_query_alone` | With the keyboard in the find bar, Ctrl+W closes the tab and also deletes the last word of the query |
