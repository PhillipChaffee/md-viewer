# Feature: Quiet sidebars and the home list

**Status:** ✅ Complete
**Branch:** `worktree-agent-abe4e319f660f0e6c` (reader redesign, unit F2b)
**Date:** 2026-11-13
**Lines Changed:** `src/app/sidebar/`, `src/app/explorer_panel/`, `src/app/outline/`, `src/app/welcome/`, `src/recent/`

## Summary

The explorer and the outline are drawn from one set of parts
(`src/app/sidebar/`): the `surface` color, no frame, one hairline on the edge
shared with the page, a small spaced-capitals header, 28 px rows, the current
row in `text` and the rest in `muted`, and a rounded `raised` wash under the
pointer. Each sidebar's toolbar is one "⋯" in its header.

The page shown with no tab open is a reading list: the wordmark, two quiet
buttons, and each recently read document with its title, a meta line and the
opening of its text.

## Features

- [x] Explorer: "FILES" and the folder name; expand all, collapse all, refresh
      and the four sort orders in the "⋯" menu; a folder opens from its whole
      row; the file being read is inked.
- [x] Outline: "CONTENTS"; expand all and collapse all in the "⋯" menu; the
      heading being read is inked, and the list scrolls to keep it on screen.
- [x] A fresh install opens with the explorer hidden. The outline shows while
      the window has room for it beside the text column. A toggle by the reader
      wins over both and is what the session saves.
- [x] `RecentEntry` carries `title`, `excerpt` and `word_count`, taken from
      the document once, when it is recorded.
- [x] Home page: rows with a serif title, a sans meta line
      (`folder · N min read · opened 2h ago`), a two-line excerpt, hairlines,
      a whole-row wash and click; a file that is gone is muted and takes no
      click; an empty state.

## Key Discoveries

### A derived flag has to survive being written directly

`show_outline` is read by the menus and written by tests and by other code as
a plain flag. Recomputing it every frame from "has the reader chosen, and is
there room" would silently undo every such write. `OutlineRule::Room` keeps
the last verdict and changes `show_outline` only on the frame the verdict
changes, so the flag stays an ordinary flag between those frames. A session
starts with `had_room: true`, matching the default `show_outline: true`, so a
write made before the first frame is not undone either.

### `Button` and `RadioButton` take different text styles in a menu

`ui.button` falls back to `TextStyle::Button` for its text and `ui.radio`
takes the body style, so a menu that mixes them has two text sizes.
`ui.style_mut().override_font_id` at the top of the menu closure gives both
the same one (`sidebar::menu_text`).

### A row's chevron as a control of its own

The outline row is one clickable rectangle. The fold control is a second
`ui.interact` over the chevron, added after the row: egui gives a click to the
last widget added under the pointer, so the chevron folds and the rest of the
row jumps, and `contains_pointer` keeps the wash on the row for both.

### Scrolling a virtualized list to a row that is not laid out

`ScrollArea::show_rows` lays out only the rows on screen, so the row to show
has no `Response` to call `scroll_to_me` on. Inside the closure the `Ui`
starts `row_range.start` rows down the list, so the target's rectangle is
`ui.max_rect().top() + (row - row_range.start) * row_height`, and
`ui.scroll_to_rect(rect, None)` reaches the enclosing scroll area. `None`
moves the list only when the row is out of view.

### The first paragraph is not the first `Paragraph` event

A README often opens with a paragraph of badge images, a quote, or a list.
`recent::summary` walks pulldown-cmark events and takes the first paragraph
that is not inside a quote, list, table, footnote or definition list and has
at least one letter or digit once image alt text is left out.

## Architecture

### New/Modified Structs

```rust
// src/recent/mod.rs
pub(crate) struct RecentEntry {
    pub(crate) path: PathBuf,
    pub(crate) last_opened: u64,
    #[serde(default)] pub(crate) title: Option<String>,
    #[serde(default)] pub(crate) excerpt: Option<String>,
    #[serde(default)] pub(crate) word_count: Option<u32>,
}

// src/recent/summary/mod.rs
pub(crate) struct DocumentSummary { title, excerpt, word_count }

// src/app/outline/room/mod.rs
pub(crate) enum OutlineRule { Reader, Room { had_room: bool } }

// src/app/outline/spy/mod.rs
pub(crate) struct HeadingSpy { /* last view looked at, heading found */ }

// src/app/welcome/list/mod.rs
pub(crate) struct HomeList { rows: Vec<HomeRow>, has_more, show_all, built_at }
```

`MarkdownApp` gains `outline_rule`, `heading_spy` and `home_list`.
`PersistedState::show_outline` is `None` until the reader toggles the outline.

### New Functions

| Function | Purpose |
|----------|---------|
| `sidebar::frame`, `paint_inner_edge`, `header`, `row`, `caption`, `menu_text` | The parts both sidebars are drawn from |
| `DocumentSummary::of(content, document_title)` | Title, excerpt and word count of a loaded document |
| `push_recent(list, path, now, summary)` | Records a file with its summary; `None` keeps what was known |
| `HeadingSpy::look(&tab)` | The heading in view, recomputed only when the view or the document changed |
| `room::has_room(window, explorer, reading_width)` | Whether the outline fits beside the text column |
| `MarkdownApp::toggle_outline()` | The reader's toggle; `AppAction::ToggleOutline` calls it |
| `HomeList::current(slot, recents, show_all, now, exists)` | The home page's rows, rebuilt on change or after 30 s |
| `ReadingWidth::fixed()` | The column width the chrome measures by |
| `tab::minutes_for_words(words)` | Reading time from a stored word count |

## Testing Notes

- The parts in `sidebar/` are tested on a bare `egui::Context` by the shapes a
  frame paints; the sidebars and the home page through `LabelHarness`, by
  label and by painted text and color.
- The scratch directory of `LabelHarness` is named `md-viewer-label-test-…`,
  which starts like the wordmark. A test that looks a painted text up by
  prefix finds both; the wordmark is looked up whole.
- A tooltip under kittest shows after about 40 steps of a resting pointer.
- Screenshots on Xvfb: both sidebars on Sepia and Night, the home page on
  Sepia and Dusk, the empty home page, and a fresh install at 900 and 1300 px.

## Future Improvements

- [ ] Refresh a recent entry's summary when its file reloads on disk; it is
      taken when the file is opened or navigated to.
- [ ] Mark the folder that holds the file being read when the folder is closed,
      as the outline does for a folded heading.
- [ ] The outline builds its list of unfolded headings every frame. It could
      be kept on the tab and rebuilt when a heading folds.
- [ ] Thumbnails on the home list, from a document's first image.
