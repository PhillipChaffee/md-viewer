# CLAUDE.md

Guidance for Claude Code when working in this repository.

## What this is

A desktop Markdown reader written in Rust with egui. `README.md` describes
the app; `GATES.md` lists every quality gate and each exception to one.

| Component | Technology |
| --- | --- |
| GUI | egui 0.33 + eframe (glow backend) |
| Markdown, math, diagrams | `vendor/egui_commonmark` (a fork), typst + mitex, merman |
| Painter | `vendor/epaint` (a patched copy) |
| File watching | notify 6.1 + notify-debouncer-mini |
| Fonts | fontique; Source Serif 4 bundled in `assets/fonts/` |

## Layout

| Path | Owns |
| --- | --- |
| `src/app/` | The window: top bar, appearance popover, tabs, sidebars, home page, document pane |
| `src/theme/` | The four palettes. No other code names a color |
| `src/layout/` | The reading column and line spacing |
| `src/tab/`, `src/links/`, `src/headers/`, `src/search/` | One document |
| `src/explorer/`, `src/watcher/` | The file tree and live reload |
| `src/system_fonts/` | Font discovery and the bundled faces |
| `vendor/` | Forked upstream crates, outside the app crate's gates |

## Rules

1. **Never work on `main`.** Check `git branch --show-current` first.
2. **A test never lives in the file it tests.** `foo/mod.rs` has its tests
   in `foo/tests.rs`, declared `#[cfg(test)] mod tests;`.
3. **Never weaken a gate to pass it.** No threshold changes, no new entries
   in the lint tables or `clippy.toml`. A one-off suppression is
   `#[expect(lint, reason = "...")]` and goes in `GATES.md`.
4. **Tests assert behavior.** Drive the app through the harnesses in
   `src/app/tests.rs` and `src/app/ui_harness.rs`, and break the code once
   to see the test fail.
5. **egui is immediate mode.** Compute when data changes, never per frame.
   UI code picks an `AppAction` (`src/app/action/`); it is applied after
   rendering. `CommonMarkCache` lives in `Tab` and is reset only on load.
6. **Read a file before changing it,** and edit it in place.

## Commands

```bash
cargo run -- file.md        # run the app
cargo test                  # the app crate's tests
./run-gates.sh              # every gate, in parallel
```

The renderer fork has its own workspace and its own tests:

```bash
cargo test --manifest-path vendor/egui_commonmark/Cargo.toml
```

Other guidance is in `.claude/rules/`.
