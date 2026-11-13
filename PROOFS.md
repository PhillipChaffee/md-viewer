# Proofs

Part of md-viewer is machine-checked: 67 proof harnesses in 21 modules,
verified by the [Kani](https://github.com/model-checking/kani) model
checker. This file says what each one proves, under which bounds, and what
is **not** proven.

Read the last part first if you are deciding how far to trust this. Most
of the app draws a window, reads files and parses Markdown, and none of
that is proven. The proofs cover the arithmetic and the small state
machines under it.

## What a proof here means

A harness calls a shipped function with *symbolic* inputs, which stand for
every value of the type at once, and asserts a property. Kani compiles the
real code, not a model of it, and CBMC searches for an input that breaks
the property or panics. "Verified" means no such input exists inside the
stated bounds. It also means none of these happen on any path the harness
reaches: a panic, an arithmetic overflow, an index out of bounds, a
division by zero, or a float operation that makes NaN out of numbers.

It does not mean the property is the right one. Each property is written
below in one sentence so you can judge that.

| Status | Meaning |
| --- | --- |
| Proven | Loop-free code, checked for every input of the stated type. |
| Bounded | Checked for every input up to a stated size or magnitude. Nothing is claimed beyond it. |
| Tested only | Has tests in `tests.rs`, no proof. |
| Not verifiable | Drawing, the GPU, fonts, the file system, the OS. No tool here can prove it. |

## Run them

```bash
cargo install --locked --version 0.68.0 kani-verifier
```

```bash
cargo kani setup
```

```bash
./run-proofs.sh
```

`cargo kani setup` downloads Kani's own nightly toolchain and CBMC. The
first run compiles every dependency under Kani (about 4 minutes on 4
cores, 1.3 GB in `target/kani`). After that the 67 harnesses take about
22 minutes one at a time (`PROOF_JOBS=1`): 1,305 s on the last full run,
on a shared 4-core machine, of which 1,164 s is solver time. Two
harnesses take 458 s of that (`the_title_keeps_clear_of_the_buttons`,
`heading_in_view_is_the_last_one_reached`), and most take under a second. `./run-proofs.sh --harness layout::proofs` runs one module.

`run-proofs.sh` fails when a harness does not verify, when a
`kani::cover!` check is not reached, or when the number of harnesses
verified differs from the number of `#[kani::proof]` attributes in `src/`.
CI runs it in the `proofs` job. It is not in `run-gates.sh`: it needs a
second build of the dependency graph and several minutes of solver time,
which is more than the other gates together.

The proofs live in `proofs.rs` beside the code, declared
`#[cfg(kani)] mod proofs;`. No ordinary build, test run or coverage run
compiles them. `cargo fmt` does format them.

## The ledger: what is proven

"Any `f32`" includes NaN and both infinities. "Finite" excludes all three.

### `src/layout/` — reading column, zoom, keyboard scroll

| Harness | Property | Inputs | Status |
| --- | --- | --- | --- |
| `reading_width_new_clamps_every_float` | Every float becomes a width in [280, 4000]; an in-range one is kept; NaN gives the default. | Any `f32` | Proven |
| `reading_width_new_is_monotonic` | A wider request never gives a narrower column. | Any two `f32` | Proven |
| `resolve_fits_the_pane` | The column shown is a whole, non-negative px count, at most the stored width, and leaves 32 px on each side; a pane under 64 px or NaN shows none. | Any width; pane any `f32` but ±∞ | Proven |
| `resolve_is_monotonic_in_the_pane` | A wider pane never shows a narrower column. | Same | Proven |
| `column_keeps_both_margins` | Where `centered_column_left` puts the column, it has at least 32 px of pane on each side. | Pane from 64 to 2^23 px | Bounded |
| `centered_column_starts_inside_the_pane` | The column's left edge is never left of the pane's. | Finite `f32` | Proven |
| `stepped_moves_the_way_it_was_asked` | A step is a valid width; a step out never narrows the shown column, a step in never widens it. | Any step; pane any `f32` but ±∞ | Proven |
| `a_step_out_and_back_is_the_identity` | Widen then narrow gives back the column that was shown. | Panes with room for the wider column, so not a column the pane is limiting or one set to fit the window | Proven |
| `from_edge_is_always_a_valid_width` | Dragging an edge anywhere gives a width in range. | Any `f32` but ±∞ | Proven |
| `restored_maps_every_saved_session_to_a_valid_width` | Every saved width and every legacy `full_width_content` flag restore to a valid width: saved width first, then the flag, then the default. | Every `Option<f32>` × `Option<bool>` | Proven |
| `fixed_is_never_fit_window` | The width the chrome measures by is never fit-window. | Any width | Proven |
| `keyboard_scroll_stays_inside_the_document` | A line or page step lands between the top and the furthest scroll offset. | Finite offset and heights | Proven |
| `keyboard_scroll_moves_the_way_the_key_points` | Down never moves the view up; up never moves it down. | Offsets inside the document | Proven |
| `whole_pixels_is_total` | Every float has a pixel count; NaN and negatives give 0. | Any `f32` | Proven |
| `line_fraction_never_divides_by_zero` | The fraction is never NaN or negative; an empty document gives 0. | Every pair of `usize` | Proven |
| `line_fraction_is_a_fraction` | For a line inside the document the fraction is in [0, 1]. | Up to 2^24 lines | Bounded |
| `zoomed_stays_in_range` | Any zoom step lands in [0.5, 3.0]. | Any factor; current zoom not NaN | Proven |
| `zoomed_moves_the_way_it_was_asked` | Zoom in never zooms out, and the reverse. | Zoom in range | Proven |
| `a_zoom_step_in_and_out_comes_back` | In then out returns to the start within 1e-6. | Zooms where neither step hits the range's end | Proven |

### `src/theme/`

| Harness | Property | Inputs | Status |
| --- | --- | --- | --- |
| `next_cycles_through_every_theme` | `Ctrl+D` visits all four themes and returns, in the order of `ALL`. | All 4 themes | Proven |
| `restored_maps_every_saved_session_to_a_theme` | A saved theme wins; a legacy `dark_mode` flag keeps light or dark; neither gives the default. | Every combination | Proven |
| `every_theme_has_its_own_palette` | No two themes share a palette or a page color. | All 16 pairs | Proven |
| `rgb_reads_the_three_channels` | `0xRRGGBB` becomes that opaque color. | Every 24-bit value | Proven |

### Tabs, search, history, startup

| Harness | Property | Inputs | Status |
| --- | --- | --- | --- |
| `tabs::closing_a_tab_keeps_the_active_index_valid` | After a close the active index names an open tab, and the same document stays active unless it was the one closed. | Every `usize` with active and closed in range | Proven |
| `tabs::closing_a_tab_is_total` | The close arithmetic never panics, even from an out-of-range index. | Every `usize` | Proven |
| `tabs::next_and_previous_tab_wrap_and_invert` | Next and previous stay in range, move, wrap, and undo each other. | Every `usize`, two tabs or more | Proven |
| `search::stepping_matches_wraps_and_inverts` | Next and previous match stay in range, wrap, and undo each other. | Every `usize` with the index in range | Proven |
| `search::stepping_forward_recovers_a_stale_index` | Stepping forward from an out-of-range index lands in range. | Every `usize` but `usize::MAX` | Proven |
| `tab::back_and_forward_undo_each_other` | Back then Forward restores the document and both history stacks; so does Forward then Back. | Stacks of up to 3 entries | Bounded |
| `tab::a_step_back_moves_exactly_one_entry` | Back moves one entry between the stacks and loses none. | Same | Bounded |
| `tab::a_visit_can_be_undone_and_clears_forward` | Opening a document clears Forward and Back returns from it. | Same | Bounded |
| `tab::reading_minutes_never_overflow` | Reading time is at least 1 minute and never overflows. | Every `usize` | Proven |
| `tab::reading_minutes_round_up` | Reading time is the least whole number of minutes covering the words. | Up to `u32::MAX` words | Bounded |
| `startup::restored_active_tab_names_an_open_tab` | Any saved tab index restores to an open tab, and to itself where it still exists. | Every `Option<usize>` × `usize` | Proven |
| `startup::restored_zoom_is_in_range` | A saved zoom restores into [0.5, 3.0]. | Every `Option<f32>` but NaN | Proven |
| `startup::restored_math_scale_is_in_range` | A saved formula size restores into [1, 2]. | Every `Option<f32>` but NaN | Proven |
| `startup::restored_zoom_passes_nan_through` | Pins finding 1 below. | NaN | Proven |
| `persist::restored_sidebar_width_is_usable` | Any saved sidebar width restores finite and at least 70 px. | Every `Option<f32>`; finite default ≥ 70 | Proven |

The three history harnesses run the two helpers `Tab` calls
(`move_history_entry`, `record_visit`) on a small stand-in for the tab.
They do not run `Tab::navigate_back` itself, which loads a file between
the two halves of a step. That wiring is tested only.

### Outline

| Harness | Property | Inputs | Status |
| --- | --- | --- | --- |
| `headers::a_heading_is_hidden_exactly_under_a_collapsed_ancestor` | A heading is hidden if and only if its parent is collapsed or hidden; no index reads out of bounds. | Up to 5 headings, any levels, any collapsed set, any index | Bounded |
| `headers::has_children_reads_only_the_next_heading` | "Has children" agrees with the next heading's level and never reads out of bounds. | Same | Bounded |
| `headers::heading_levels_are_one_to_six` | H1 to H6 map to 1 to 6. | All 6 | Proven |
| `spy::heading_in_view_names_a_heading_or_none` | The binary search ends, never overflows, and returns a valid index or none, sorted input or not. | Up to 6 headings, any `f32` positions, finite view | Bounded |
| `spy::heading_in_view_is_the_last_one_reached` | With headings in order, the result is the last one at or above the reading line. | Same, positions ascending | Bounded |
| `spy::the_end_of_the_document_is_the_last_heading` | At the end of the document the last heading is current. | Same | Bounded |
| `room::the_outline_rule_survives_a_restart` | What a session saves about the outline restores to the same rule. | Every rule × shown | Proven |
| `room::follow_reports_each_change_of_room_once` | The window never overrides a reader's choice; a change of room is reported once. | Every rule × room | Proven |
| `room::a_wider_window_keeps_its_room` | Widening the window never loses the outline. | Finite widths | Proven |
| `room::the_explorer_only_takes_room` | Opening the explorer never makes room for the outline. | Finite widths | Proven |

`header_is_hidden` looks collapsed headings up in a `HashSet`, which Kani
cannot run. The proof runs `has_collapsed_ancestor`, the function
`header_is_hidden` hands that lookup to.

### Document pane, lightbox, chrome

| Harness | Property | Inputs | Status |
| --- | --- | --- | --- |
| `content::wheel_scroll_never_stores_an_end_of_the_document` | A wheel step during a selection stores only offsets half a pixel clear of both ends. | Finite offset and delta; scroll range ≥ 0 | Proven |
| `content::corrections_never_scroll_above_the_top` | A scroll correction is never negative and never asked for within 2 px. | Finite `f32` | Proven |
| `content::a_visible_match_is_left_alone` | A search match on screen causes no scroll. | Finite `f32` | Proven |
| `lightbox::lightbox_zoom_stays_in_range` | Lightbox zoom stays in [0.1, 10]. | Zoom in range, factor not NaN | Proven |
| `lightbox::zoom_to_cursor_never_panics_or_goes_negative` | Zoom-to-cursor gives a non-negative offset and never panics. | Sizes 1 to 10^6 px, zooms in range, pointer and offset within ±10^6 | Bounded |
| `top_bar::the_title_keeps_clear_of_the_buttons` | The title keeps 12 px from the buttons when there is room, and is centered in the free span when there is not. | Finite `f32` | Proven |
| `chrome::segments_share_the_width_they_are_given` | Segment width is finite and non-negative for any count, zero included; segments never exceed the width shared. | Finite width; sum check up to 64 segments | Bounded |
| `appearance::the_width_slider_always_has_a_range` | The width slider's upper end is always at least one step above its lower end. | Any `f32` but ±∞ | Proven |
| `app::zoom_percent_is_total_and_close` | The zoom readout never panics and is within half a percent. | Any `f32` | Proven |
| `explorer_panel::the_flash_fades_to_its_floor_and_ends` | The change highlight stays in [60, 180], never brightens, and ends. | Every pair of `Duration` | Proven |

### Shortcuts, launch, watcher, fonts

| Harness | Property | Inputs | Status |
| --- | --- | --- | --- |
| `shortcuts::no_key_press_fires_two_bindings` | No two rows of the binding table share a key and are live together. | Every pair of rows × 4 focus states × 32 modifier sets | Proven |
| `shortcuts::the_lightbox_takes_the_keyboard` | With the lightbox open only its bindings are live; closed, none of them is. | Every row × focus × modifiers | Proven |
| `shortcuts::a_popup_takes_the_find_bar_and_reading_keys` | Under an open menu no find-bar or reading binding is live, Escape included. | Same | Proven |
| `shortcuts::every_binding_can_fire` | Every row has a focus and modifiers under which it is live. | Every row | Proven |
| `cli::only_a_terminal_launch_detaches_and_only_once` | The process detaches only from a terminal, without `--foreground`, and never when it is the detached copy. | All 16 cases | Proven |
| `cli::every_launch_opens_one_window_or_hands_over` | `launch` either hands over to a copy and opens no window, or opens exactly one; a failed respawn falls back to the window. | All cases | Proven |
| `watcher::watcher_retries_are_bounded` | The retry counter never overflows and allows exactly 3 retries. | Every `u32` | Proven |
| `system_fonts::text_size_steps_are_ordered_and_reversible` | Larger and smaller undo each other and move the px size the right way. | All 5 sizes | Proven |
| `system_fonts::all_lists_the_sizes_in_step_order` | `ALL` lists the sizes one step apart. | All 5 | Proven |

The shortcut harnesses read the real `BINDINGS` table. They do not run
`shortcut_actions`, which reads and consumes egui's input, nor the code that decides
which focus the app is in. Both are tested only.

**Counts: 67 harnesses. 54 proven, 13 bounded.**

## Every assumption

A harness narrows its inputs with `kani::assume`. Each use is one of
these, and none narrows an untrusted input to make a proof pass.

| Assumption | Where | Why |
| --- | --- | --- |
| Pane width is not ±∞ | `resolve` and everything built on it; `segment_share` | CBMC's model of fused multiply-add reports an error for every infinite addend. The real operation has none. `layout/tests.rs` covers ±∞, which the app does pass before the first frame. |
| Inputs are finite | Keyboard scroll, scroll corrections, wheel scroll, title placement, outline room, heading-in-view's view, `centered_column_left` | ∞ − ∞ is NaN, and Kani fails on any operation that makes NaN. With infinite inputs these functions can return NaN. That is not proven absent; see finding 2. |
| Value is not NaN | `zoomed` (current zoom), saved zoom and formula size, lightbox factor | `f32::clamp` passes NaN through. See finding 1. |
| Index is in range | Tab index, match index | The invariant the callers keep. `closing_a_tab_is_total` and `stepping_forward_recovers_a_stale_index` drop it. |
| `max_scroll >= 0` | `wheel_scroll_target` | Its one caller passes `(a - b).max(0.0)`. This link is by reading, not by proof. |
| Positions ascend | `heading_in_view_is_the_last_one_reached` | The function's documented precondition. The first spy harness drops it. |
| Domain bounds | The "Bounded" rows | Stated in each row. |
| Default width finite and ≥ 70 | `restored_sidebar_width` | Both callers pass a constant. |

Unwinding bounds: `spy` 5 and 7, `headers` 7, `tab` history 5. Kani's
unwinding assertions are on, so a bound too small fails the proof
instead of hiding a path. No stubs, no `--no-*-checks` flags.

## What the proofs found

None of these is fixed here. This change adds proofs and changes no
behavior.

1. **A NaN zoom in the settings file survives startup.** `restored_zoom`
   and `restored_math_scale` clamp, and clamping NaN gives NaN.
   `ReadingWidth::new` and `restored_sidebar_width` do handle NaN.
   `restored_zoom_passes_nan_through` pins this so the finding stays true
   until the code changes.
2. **Scroll arithmetic returns NaN for infinite inputs.**
   `keyboard_scroll_target`, the scroll corrections and `heading_in_view`
   are proven for finite inputs only.
3. **`search_correction` with a NaN viewport height scrolls to the top.**
   Its `viewport_height <= 0.0` guard is false for NaN. Kani produced this
   as a counterexample before the harness was narrowed to finite inputs.
4. **`title_center` would panic on NaN geometry.** Its guard does not
   catch NaN before `clamp`. Seen by reading, not proven; the harness
   covers finite geometry.

## Are the proofs vacuous?

Two checks say no.

- **Cover checks.** Every harness with symbolic inputs has `kani::cover!`
  statements for its interesting cases (157 in all), and `run-proofs.sh`
  fails if one is unreachable.
- **Mutants.** `scripts/proof-mutants.tsv` lists 76 one-line faults in the
  shipped code, at least one for each harness but the one that pins
  finding 1. `scripts/kill-proof-mutants.sh` plants each and expects Kani
  to reject it. Last run: 76 killed, 0 survived, in 19 minutes.

The cover gate has already earned its place: it rejected the first
version of `a_step_out_and_back_is_the_identity`, whose assumptions made
one of its two cases unreachable.

## What is not proven

Everything below is outside the proofs. A refactor here is protected by
tests, or by nothing.

| Area | Status | Why |
| --- | --- | --- |
| `vendor/egui_commonmark`, `vendor/epaint` | Unproven | Out of scope. This is the Markdown layout, math and diagrams: most of what a reader sees. |
| Everything drawn: top bar, popover, tab bar, sidebars, home page, document pane, lightbox, dialogs, icons | Not verifiable | Immediate-mode drawing through egui and the GPU. Tested through the UI harness. |
| Focus order: which of lightbox, popup, find bar or document has the keyboard; what Escape closes | Tested only | Decided from egui state each frame. Only the binding table is proven. |
| Edit mode: dirty state, save, save flash, scroll mapping | Tested only | Tied to the file system and the tab's render cache. |
| `links`: classifying and resolving link destinations, percent-decoding, slugs | Tested only | String, `url` and path code. Not attempted: Kani explores strings a few bytes long at best, which would say little about real links. |
| `search::find_matches` | Tested only | A regular expression and a substring search over the document. |
| `headers::parse_headers`, `recent::summary` | Tested only | Run the Markdown parser. |
| `recent`: the recent list, relative times | Tested only | Paths and string formatting. |
| `tab`: loading, reloading, saving, link navigation, the wiring of history | Tested only | File system. |
| `persist`: reading and writing the settings file | Tested only | serde. The proofs start from the parsed values. |
| `theme`: the 7:1 contrast claim; `diagram` contrast | Tested only | WCAG luminance needs a fractional power, which CBMC does not model. |
| `cli`: argument parsing, the respawn, the Wayland probe | Tested only | clap and the OS. |
| `watcher`, `explorer`, `system_fonts`, `dialogs` | Tested only | File system, threads, system fonts. One counter and one enum are proven. |
| Threads: the explorer scan, the font scan, the file watcher | Tested only | Kani checks one thread. |
| Fonts, the GPU, the windowing system, the file system | Not verifiable | Outside the program. |

Also trusted, not proven: Kani and CBMC themselves, the Rust compiler, and
that the nightly Kani compiles with behaves like the pinned 1.98.0 for
this code.

## Keeping the proofs through a refactor

- **Moving a function** means moving its `proofs.rs` with it and keeping
  the `#[cfg(kani)] mod proofs;` line. `run-proofs.sh` counts harnesses,
  but it cannot notice a deleted `proofs.rs`: the count drops on both
  sides. Check the "Complete - 67" line.
- **Keep the proven functions pure.** They take numbers and return
  numbers. Give one an `egui::Context`, a `Tab` or a `HashSet` and Kani
  can no longer run it.
- **Keep the names in the ledger**, or update the ledger and
  `scripts/proof-mutants.tsv` in the same change. A mutant row whose text
  no longer matches is reported as stale.
- **A harness that fails after a refactor is a finding first.** Read the
  counterexample (`cargo kani --harness NAME -Z concrete-playback
  --concrete-playback print`) before touching the harness. Never add a
  `kani::assume` to get a pass without a row in the assumptions table.
- **New arithmetic gets a harness** with at least one `kani::cover!` and
  one mutant row.
