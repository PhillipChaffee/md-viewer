# Feature: Mermaid diagrams and math in the colors of the page

**Status:** ✅ Complete
**Branch:** `claude/beautiful-lovelace-sdpd77` (reader redesign, unit F1b)
**Date:** 2026-11-13

## Summary

Mermaid diagrams were drawn in mermaid's light theme on an opaque white
rectangle, whatever the page looked like. Formulas were composited onto the
page color, which showed as a box on every other surface. Both now follow
the page: diagrams are recolored and transparent, and formulas are plain ink
with an alpha channel.

## Features

- [x] `CommonMarkViewer::mermaid_colors(Option<MermaidColors>)`, off by default
- [x] Diagrams re-render when the colors change, one texture per diagram
- [x] The previous render stays on screen until the new one is ready
- [x] Formulas are transparent and blend on table rows, quotes and highlights
- [x] The lightbox lays a sheet of the page color under a diagram
- [x] `diagram_colors(&egui::Visuals)` derives the colors in the application

## Key Discoveries

### merman honors theme variables for flowcharts only

Probed against merman 0.3.0 with both routes mermaid offers:

| Route | Flowchart | Sequence diagram |
|---|---|---|
| `%%{init: {"theme":"base","themeVariables":{…}}}%%` | applied | ignored, output byte-identical |
| `HeadlessRenderer::with_site_config` | applied | ignored, output byte-identical |
| `theme: dark` | partly applied | ignored |

Class, state, ER, gantt and pie diagrams hard-code the default theme in
their CSS as well. So the colors are replaced in the finished SVG. The test
`merman_ignores_theme_variables_for_sequence_diagrams` pins this finding and
fails when merman starts to theme sequence diagrams.

### usvg can resolve an SVG's CSS and write the result back

merman's colors live in a `<style>` block, in presentation attributes and in
inherited properties. Rewriting them as text would need a CSS selector
matcher. `usvg::Tree::to_string` with `preserve_text: true` writes the parsed
tree back with every element carrying its final `fill` and `stroke` as
`#rrggbb`, and with `<text>` kept as text. The round trip is pixel-identical
for flowchart, sequence, class, state, ER, gantt and pie diagrams (largest
channel difference 16 of 255, on antialiased edges).

On that normalized SVG the recoloring is a scan over attributes:

- inside `<text>`: the text color
- a color of the default theme: the role it plays there (node fill, border,
  cluster, note, task)
- a gray: a point on the page-to-text scale
- any other color: moved to where it sat between paper and ink, tint kept

### usvg paints `background-color` of the root element

merman writes `style="background-color: white"` on `<svg>`. usvg 0.45 turns
that into an opaque backdrop, which is why diagrams were white rectangles.
The property is removed from the root tag before parsing.

### An opaque formula shows on every surface that is not the page

The formula raster was `fg` composited over `visuals.panel_fill`. Table rows
are filled with another color, so every formula in a table sat in a box.
egui blends premultiplied textures in gamma space, so `fg` at the coverage
alpha gives the same pixels on the page and correct ones everywhere else.
The background also left the cache key: one raster serves every surface.

## Architecture

### New and modified items

| Item | Purpose |
|---|---|
| `MermaidColors` (fork, re-exported) | page, text, line, node fill and stroke, secondary fill, note fill, accent |
| `CommonMarkOptions::mermaid_colors`, `CommonMarkViewer::mermaid_colors` | the opt-in |
| `mermaid_theme::themed_svg` | normalize through usvg, recolor |
| `MermaidSlot` in `CommonMarkCache` | one cache entry per diagram source: the wanted key, its status, the texture on screen |
| `render_mermaid_image` | source and colors to raster, off the UI thread |
| `diagram::diagram_colors` | colors for the current `egui::Visuals` |

The cache key of a render is the hash of source and colors. The slot is
keyed by source alone and holds one texture, so switching themes any number
of times holds one texture per diagram. A render that arrives for colors no
longer wanted is dropped.

## Testing Notes

- Fork, backend: role mapping, the recolor scan, a flowchart and a sequence
  diagram that use no color outside the palette, transparent corner pixel.
- Fork, `tests/mermaid_colors.rs`: frames are driven through the public API
  and the texture uploads, frees and paints of each frame are read from
  `FullOutput`.
- Application: contrast of text and lines of at least 4.5:1 on white, sepia,
  dark gray and black pages; the lightbox sheet; a theme switch redraws.

## Future Improvements

- [ ] A wide left-to-right flowchart is scaled down to the reading column
      and its labels become small. Laying it out for the column width, or
      letting it use the full pane, would help more than any color.
- [ ] Authors' own `style` and `classDef` colors keep their tint but text on
      them is not checked for contrast.
