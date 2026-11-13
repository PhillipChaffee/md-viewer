# Feature: Reader chrome (top bar, Aa popover, flat tabs)

**Status:** ✅ Complete
**Branch:** `claude/beautiful-lovelace-sdpd77` (unit F2a of the reader redesign)
**Date:** 2026-11-13

## Summary

The second unit of the "as pretty as Instapaper" redesign replaces the chrome
above the document. The menu bar, the path readout and the green LIVE badge
are gone. One 40 px bar in the page color takes their place, with a few line
icons either side of the document's title, and everything the menus offered
moves behind its last button.

## Features

- [x] **Top bar** (`src/app/top_bar/`): explorer toggle, back and forward on
      the left; the document title (first heading, else the file name) in
      small sans in the middle; Aa, find, outline and ⋯ on the right. Icons
      are `muted` at rest and `text` under the pointer or the keyboard focus.
- [x] **Title hover** shows the full path, and a dot with "Live: reloads when
      the file changes" while the file is watched.
- [x] **Aa popover** (`src/app/appearance/`): four theme swatches, text size
      A− / A+, a width slider with Fit window and Reset, three line spacings,
      and Fonts…. Every control acts at once and the popover stays open;
      a click elsewhere, Esc, or the button closes it.
- [x] **⋯ menu** (`src/app/overflow_menu/`): Open File, Open Folder, Close
      Tab · Find, Edit Document, Live Reload · Back, Forward · Show Explorer,
      Show Outline, Full Width, Smooth Text Rendering, Theme ▸, Line Spacing
      ▸, Formula Size ▸ · a zoom row · Colors…, Fonts… · Quit. Each row
      dispatches the `AppAction` its menu item did.
- [x] **Tab strip** (`src/app/tab_bar/`): hidden with one document and no
      edit in progress; flat text tabs with a 2 px accent underline under the
      active one; a close button while the pointer is over a tab; a quiet
      Preview | Edit segmented control, the unsaved dot and the save outcome
      at the right end.
- [x] **Error bar and find bar** in the same language: page color, a line
      icon, muted labels, a borderless query field.
- [x] `Ctrl+E` and `AppAction::ToggleEditMode`: the way into edit mode now
      that the Preview | Edit control is hidden with a single document.
- [x] `src/app/menu_bar/` deleted; its tests moved to the new surfaces.

## Key Discoveries

### A popup's own surface is a tab stop

`egui::Popup` senses with `Sense::click()`, which is focusable. The first Tab
after opening a popup lands on the sheet itself. `Popup::sense(Sense::CLICK)`
keeps the click-blocking and drops the stop. See `docs/LESSONS.md`.

### A nested row grows downward from its starting height

The tab strip's inner `ui.horizontal` was centered at 18 px and then grown to
34 px, which pushed the tabs 8 px low and the underline off the strip. The
tabs now go straight into the scrolled row. See `docs/LESSONS.md`.

### The Aa glyph has to take an icon's square

A text button is as wide as its text plus padding, so "Aa" at 15 px was 36 px
wide in a row laid out for 28 px squares, and the ⋯ button ended 4 px from
the window edge. `QuietButton::snug()` drops the padding for a glyph that
stands among icons.

### Painted text now includes the title twice

The bar repeats the document's first heading, so a test that looks for "the
first painted text starting with the title" finds the bar. The content tests
scope their lookup to the pane below the bar.

## Architecture

### New modules

| Module | Purpose |
| --- | --- |
| `app::chrome` | `QuietButton`, `MenuRow`, `segmented`, `hairline`, `small_label`, `control_row`, `bar_frame`, `popup_style`, `POPUP_SENSE`, and the sizes of the chrome |
| `app::top_bar` | `MarkdownApp::render_top_bar`, `history_availability` |
| `app::appearance` | `MarkdownApp::appearance_popover` |
| `app::overflow_menu` | `MarkdownApp::overflow_menu` |

### New in existing modules

| Item | Purpose |
| --- | --- |
| `icons::Icon` | Vector line icons on a 16 unit grid, painted in any ink at any size |
| `AppAction::ToggleEditMode` | Switch the active tab between preview and source |
| `AppAction::SetTextSize(TextSizeClass)` | Set the text size class |
| `TextSizeClass::{smaller, larger}` | The neighboring size class, if any |

`MarkdownApp::hovered_tab` is gone: a tab asks egui whether the pointer is
over it.

## Testing Notes

- The parts in `app::chrome` are laid out alone in an empty panel and checked
  for size, painted colors and clicks; their roles and states are read from
  the AccessKit tree.
- The bar, the popover, the menu and the tab strip are driven in the whole
  app through `UiHarness` and `LabelHarness`. `UiHarness::pick(item)` now
  opens the ⋯ menu; `is_on(label)` reads a toggle's state from the tree
  where the old tests looked for a "✓" in the label.
- Fourteen mutations (lit state, title centering, Fit window, slider range,
  zoom direction, live tick, strip visibility, the nested-row bug, an arrow
  head, the next-match button, error truncation, the popup sense, the edit
  toggle, the title's hover color) each turned at least one named test red.

## Future Improvements

- [ ] The outline and explorer toggles do not show whether their panel is
      open; the panel itself is the only cue.
- [ ] Esc closes the popover and, in the same press, an open find bar.
- [ ] The Preview | Edit control appears with the strip when editing starts,
      which moves the document down by the strip's height.
