# Feature: Reader foundation (themes, resizable column, document typography)

**Status:** ✅ Complete
**Branch:** `claude/beautiful-lovelace-sdpd77` (unit F1a of the reader redesign)
**Date:** 2026-11-13

## Summary

The first unit of the "as pretty as Instapaper" redesign: the data and the
document surface, not the chrome. The menu bar, tab bar, sidebars and welcome
page keep their layout and only take the new colors; later units replace them.

## Features

- [x] `src/theme/`: four reading themes (Paper, Sepia default, Dusk, Night),
      one `Palette` each, and the single `visuals()` function that builds
      `egui::Visuals` from a palette and the user's color overrides.
- [x] `dark_mode: bool` is gone from the app; `theme: ThemeId` replaces it.
      Old sessions map dark to Dusk and light to Paper.
- [x] `Ctrl+D` and the old "dark mode" action step through the four themes;
      **View → Theme** picks one by name.
- [x] `layout::ReadingWidth` replaces `full_width_content`: a centered column,
      resized by dragging either edge, by `Ctrl+Alt+←/→`, or reset by a
      double click on an edge. **View → Full Width** switches between
      fit-window and the default width.
- [x] Reader font preset (bundled Source Serif 4), the default for a fresh
      install, with the chrome left in the system sans.
- [x] Palette-driven code blocks, inline code, blockquote bar, links, table
      lines, task markers and thematic breaks; one syntect theme per palette.
- [x] `layout::LineSpacing` (Compact 1.45 / Comfortable 1.6 / Airy 1.8),
      reachable from **View → Line Spacing** until the Aa popover lands.
- [x] A muted meta line above the document: `file-name · N min read`.
- [x] Thin, trackless scrollbar in the theme's hairline color.

## Key Discoveries

### A fenced block tagged `rust` never reached syntect

The renderer looked a fence tag up with `find_syntax_by_extension`. `rs` is an
extension; `rust` is a name. So every block tagged with a language *name*
fell through to egui_extras' built-in highlighter, which has two fixed
palettes and ignores the configured syntax theme. `syntax_theme_dark` and
`syntax_theme_light` had been set for months and applied to almost nothing.

Found by sampling pixels, not by reading code: Paper and Sepia were given
different syntect themes and painted the same purple (`#9986FF`), which is
neither theme's color. `BlockStyle::syntax_by_language_name` (opt-in) adds a
`find_syntax_by_token` lookup.

The same sampling exposed a second defect: the highlight cache key hashed
`dark_mode` but not the theme name, so two light themes shared cached jobs.

### egui hangs every face from the top of the line by its own ascent

Inline code set smaller than the serif around it floated about 4 px above
the baseline. epaint places a glyph at `row_top + font_ascent`; a smaller
face, or one with a lower ascent, simply sits higher. There is no baseline
alignment across labels.

The fix is a `FontTweak::y_offset_factor` on the code face, worked out from
the two faces' ascents (`ttf_parser`) so it is exact for whatever monospace
face leads the chain:

```rust
shift = serif_ascent_em / CODE_SCALE - code_ascent_em
```

It is a visual tweak, so no line moves. It also centers the lines of a code
block in their box, which the same ascent gap had left top-heavy. It is only
applied with the bundled serif: other presets keep inline code at body size,
as before (#46).

### A block placed in the wrapping row leaves its height behind

The meta line is painted by a new renderer hook (`preamble`) inside the
document column, so it scrolls with the text and every recorded y stays
relative to the same origin. The column ui is a wrapping horizontal layout.
Two traps:

- `allocate_ui_with_layout` bottom-aligns the block in a row of body height,
  so it starts one line low. The block is placed with `scope_builder` at an
  explicit rect instead.
- After `end_row`, the new row inherits the height of the row it ended, here
  the preamble's. A heading that opens the document ends that row again and
  pushes a blank line of that height above itself. The preamble sets the row
  height to zero after ending its row.

The bootstrap pass and the slice that starts at event 0 both call the same
function, which is what keeps measured and painted positions equal (see
LESSONS, "Viewport slices must reproduce the bootstrap's layout exactly").
A test paints a slice frame, a forced bootstrap frame and another slice
frame and compares the title's y.

### A table kept the columns a narrow column squeezed it into

Narrowing the text column to 400 px and widening it again left every
table with wrapped cells and room to spare. The renderer persists the
column widths a table rendered with (so a sidebar drag does not undo
the user's own column drags) and prefers them to a fresh fit even when
its layout key changes. Seen in a screenshot of the wide layout, then
pinned by a test that was red with `mark_layout_changed` alone.

`CommonMarkCache::refit_tables` bumps an epoch stored beside the
widths; a table whose epoch is stale fits afresh. The app calls it only
when the reading width itself changes, so pane changes keep the old
policy.

### `configure_context` undid a restored text size

`new` called `setup_fonts` (which applies the text-size class) and then
`configure_context`, which set Body 16 / Heading 32 / Monospace 14 again. A
session saved at Large came back at Normal until the user next changed the
size, because the last-applied gate believed the fonts were in sync. The
three inserts are gone; a test launches a Large session and reads the style.

### A `dyn Fn` type alias defaults to `'static`

`pub type PreambleFn = dyn Fn(&mut Ui)` made `Option<&'f PreambleFn>` demand
a `'static` closure, so a preamble could not borrow the tab's kicker string.
The alias carries the lifetime: `dyn Fn(&mut Ui) + 'f`.

## Architecture

### Data shapes

```rust
// src/theme/mod.rs
pub(crate) enum ThemeId { Paper, Sepia /* default */, Dusk, Night }
pub(crate) struct Palette { dark, page, surface, raised, text, heading, muted,
                            rule, link, selection, accent, positive, warning,
                            syntax_theme }
pub(crate) struct ColorOverrides { highlight: Option<Color32>, link: Option<Color32> }
pub(crate) fn visuals(palette: &Palette, overrides: ColorOverrides) -> egui::Visuals;

// src/layout/mod.rs
pub(crate) struct ReadingWidth(f32);        // [280, 4000], default 680
pub(crate) enum LineSpacing { Compact, Comfortable /* default */, Airy }

// src/system_fonts/mod.rs
pub(crate) struct DocumentFonts { body, heading, code, inline_code_scale, emphasis }

// src/app/content/look/mod.rs
pub(crate) struct DocumentLook { style: Arc<egui::Style>, blocks: BlockStyle,
                                 syntax_theme, muted }
```

### Data flow

```text
theme / color override / font config changes
  -> begin_frame: apply_theme, apply_fonts (both diff-gated)
  -> rebuild_document_look: DocumentLook::new(ctx.style(), palette, document_fonts(..))
  -> relayout_documents: every tab's cache.mark_layout_changed()

each frame, preview_pane:
  ui.set_style(look.style)            (an Arc clone)
  CommonMarkViewer .default_width(column) .block_style(look.blocks.clone())
                   .preamble(kicker)  .line_height(line_spacing)
  resize_strips(column) -> Option<ReadingWidth> -> AppAction::SetReadingWidth
```

### Renderer options added (all opt-in, defaults unchanged)

`BlockStyle` on `CommonMarkOptions`, set with `CommonMarkViewer::block_style`:
`heading_font_family`, `emphasis_font_family`, `heading_line_height`,
`inline_code_scale`, `task_marker`, `code_block` (`CodeBlockStyle`: fill,
stroke, corner radius, padding), `blockquote_bar`, `syntax_by_language_name`,
`short_rule`, `center_content`. Plus `CommonMarkViewer::preamble` and
`CommonMarkCache::refit_tables`.

## Testing Notes

- Contrast: body text and headings ≥ 7:1 on page, surface and raised of every
  theme; muted, link, positive and warning ≥ 4.5:1 on the page.
- The column: unit tests of `ReadingWidth` and of the strips driven with raw
  pointer events in a bare `egui::Context`; app-level tests read the painted
  text's x and wrap width at three column widths.
- Screenshots on Xvfb for all four themes and three widths; see the unit's
  report.

## Future Improvements

- [ ] Tables: a `raised` header row and no vertical grid lines. The table is
      an `egui_extras::TableBuilder` with resizable columns; both need more
      than a visuals change.
- [ ] Paragraph spacing is one blank body line (the renderer's newline
      label), about 1.3 em. Getting to 0.9 em means changing how the renderer
      spaces blocks.
- [ ] Task list items show a bullet and a checkbox.
- [ ] Pin egui's own light/dark preference: a system theme flip at runtime
      switches egui to its other style slot, which holds none of this.
