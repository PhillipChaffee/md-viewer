# Feature: Reader blocks (rhythm, tables, lists, figures, code chips)

**Status:** ✅ Complete
**Branch:** `claude/beautiful-lovelace-sdpd77` (unit F2c of the reader redesign)
**Date:** 2026-11-13

## Summary

The document surface after the reader foundation (devlog 070): the space
between blocks, how tables and lists are typeset, diagrams that may leave the
text column, and inline code. All of it is opt-in styling in the vendored
renderer, switched on by `DocumentLook`; a consumer of the fork that sets none
of it renders as before.

## Features

- [x] **Rhythm.** One gap per pair of neighboring blocks instead of a blank
      body line everywhere: 0.9 em between paragraphs, 1.6 em above a heading
      and 0.65 em below it, 1.25 em around code, tables, quotes, diagrams and
      rules, 0.35 em between the items of a tight list. Gaps never add up.
- [x] **Tables.** No box, no stripes, no grid lines. A hairline between rows
      and a slightly stronger one under the header, the header in the strong
      face, 9 px above and below each cell's text, columns aligned as the
      markdown asks (`:--`, `:-:`, `--:`), and a compact table widened to
      the column. A column separator shows under the pointer and drags.
- [x] **Lists.** A task item shows a drawn box in place of its bullet; small
      muted bullets and numbers hang in a 1.5 em slot; every nested level
      steps in by one slot; a second paragraph in an item is set apart.
- [x] **Figures.** A mermaid diagram wider than the column is shown at its
      natural size, centered on the column, in up to the pane's width less
      its margins; it is scaled down only when even that is too narrow.
- [x] **Inline code** on a rounded chip the height of the text, not a square
      the height of the line.
- [x] Frontmatter and HTML tables follow the table style; a footnote starts
      with its label on the line of its text.

## Key Discoveries

### A gap is a place, not a blank line

The renderer separated blocks with `ui.label("\n")`: a label whose first line
ends the current row and whose second line is the blank row. The height of
that row is the body face's natural row height, so "paragraph spacing" was
whatever the font said, and the typography options
(`paragraph_spacing`, `heading_spacing_above`) called `ui.add_space` in a
horizontal layout, which moves the cursor sideways. They never did anything.

With a `BlockRhythm` a block that ends starts the next row at an exact place:
the bottom of everything laid out so far (`ui.min_rect().bottom()`) plus the
gap, set with `Ui::advance_cursor_after_rect` on an empty rect, which a
wrapping row layout takes as "new row here". The gap is decided when a block
*ends*, from its own kind and the kind of the block that follows (one event
of lookahead), so nothing is emitted when a block starts. That keeps the
viewport-slice contract: a split point is recorded after the gap, and a slice
that resumes there needs no gap of its own.

### The text of a list item hung below its own row

Items were 11 px apart for no stated reason. The marker slot is a box as tall
as a line of text; the item's content is laid out in `ui.horizontal_wrapped`,
whose initial rect is `interact_size.y` (18 px) tall and is bottom-aligned in
the row the slot made taller. The text therefore started `line - 18` px below
the row. With a rhythm the slot reserves width only, and the item text starts
where its row starts.

### `egui_extras` cannot move a boundary

A table resized by `egui_extras` grows the dragged column and pushes the rest
right; the renderer capped that growth at the room the table leaves, which is
none once a compact table is widened to the column. So a styled table owns its
widths: columns are `Column::exact`, and `TableLook::column_separators` lays a
drag handle on each boundary. `drag_separator` is the policy as a pure
function: widening takes spare room first and the right neighbor second,
narrowing feeds the neighbor in a table kept full, and no column goes below
its minimum. A drag that ends bumps the cache's layout revision, because rows
may have rewrapped.

### A strong cell was measured in the regular face

`docs/LESSONS.md` has a table whose last row is bold. Row heights were
measured with the body font, the bold text wrapped one line further, and the
line was clipped. Header cells are now strong too, so the measuring functions
take the face: a header cell, and a body cell with any strong text, is
measured in the strong face. That errs toward a taller row.

### Painting behind a label without moving it

egui paints the background of inline code as a square as tall as the line.
The chip is painted by the renderer instead, from a galley laid out exactly
as `Label::layout_in_ui` lays out its own (same job, wrap width, first-row
minimum height and leading space), which the font cache then hands to the
label for free. The label itself is unchanged, so no line moves, and text
selection keeps working. The chip is set on the baseline of the text around
the code, because the code face's own ascent differs.

## Architecture

### Renderer options added (all opt-in, defaults unchanged)

On `BlockStyle`:

```rust
pub table: Option<TableStyle>,          // rule, header_rule, cell_padding, fill_width
pub list: Option<ListStyle>,            // marker, bullet_radius, indent
pub rhythm: Option<BlockRhythm>,        // paragraph, block, heading_above,
                                        // heading_below, tight_item, display_math
pub figure_max_width: Option<f32>,      // room for a diagram, centered on the column
pub inline_code_radius: Option<f32>,
```

`BlockRhythm::gap(upper: BlockKind, lower: BlockKind, font_size) -> f32` and
`item_gap(loose, font_size)` are the whole spacing policy.
`FigureSpace::place(natural, top) -> Rect` is the whole figure geometry; its
size depends only on the diagram and on `figure_max_width`, so the pass that
measures the document and the pass that paints a slice agree.

### Data flow

```text
DocumentLook::new(style, palette, fonts)        once per theme / font change
  blocks.table / list / rhythm / inline_code_radius

preview_pane, every frame
  blocks.figure_max_width = Column::figure_room()   (pane less its margins)

renderer
  process_event: self.upcoming = kind of the block that follows
  block ends   -> start_row_below(gap(kind, upcoming))
  Tag::Item    -> start_row_below(item_gap(loose))
  table()      -> TableLook { frame, padding, fonts } -> rules, separators
  code block   -> FigureSpace when it is a top-level diagram
```

## Testing Notes

- `vendor/.../tests/reader_blocks.rs` reads painted shapes: gaps by kind,
  rules and separators, alignment, task boxes, chips. Its slice test paints a
  full pass and a slice at sixty scroll offsets and compares every text.
- `vendor/.../tests/figure_breakout.rs` waits for a real diagram and checks
  its size and place, inline, with room, at natural size, and in a slice.
- A mutation sweep broke the renderer changes 39 ways, one line at a time
  (a gap that adds up, a rule under the last row, a separator that never
  hides, a figure that leaves the pane); each is caught by a named test.
- A galley on a wrapping row starts at the left edge of its ui, wherever its
  text starts. Tests that compare where text starts read the first row's
  `rect_without_leading_space()`.

## Future Improvements

- [ ] Definition lists are unstyled: the term is not strong and the
      definition is not indented.
- [ ] Images keep to the column. Their size is unknown until they load, so
      a breakout would need the placeholder to reserve the final height.
- [ ] A diagram texture is rasterized at twice its natural size; one wider
      than half the GPU's texture limit cannot be uploaded.
- [ ] The inline formula baseline assumes a 1.5 line height.
