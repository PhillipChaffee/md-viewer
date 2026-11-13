# Feature: full-lint retrofit

**Status:** ✅ Complete
**Branch:** `claude/beautiful-lovelace-sdpd77`
**Date:** 2026-11-13

## Summary

Retrofit the application crate onto the Rust baseline from
[full-lint](https://github.com/PhillipChaffee/full-lint): strict clippy
groups, denied doc lints, a cognitive-complexity cap of 15, a 1000
effective-line file cap, a 95% coverage gate, and the shared hygiene gates.
House rule added on top: a test never lives in the file it tests.

## Scope

- [x] Gate configuration copied from the full-lint templates
- [x] `src/` split by responsibility, tests in their own files
- [x] Lint, doc, format, cognitive and file-length gates green
- [x] Coverage at 95% on lines, regions and functions
- [x] Hygiene gates green

## Result

| Gate | Before | After |
| --- | --- | --- |
| Largest file | `main.rs`, 7384 lines | 483 effective lines |
| Functions over cognitive 15 | 23 (worst 422) | 0 |
| Clippy findings under the strict groups | about 700 | 0 |
| Coverage, lines / regions / functions | 33% / 35% / 44% | above 99% on each |
| Tests | 76 pass, 2 fail, 5 ignored | all pass, none ignored |

`./run-gates.sh` runs all 23 gates. One cannot be confirmed from a sandbox
whose proxy rejects github.com pages: the remote half of the link check.

## Exceptions, each with its reason beside it

- `vendor/` is outside the gates: upstream forks, reviewed against upstream.
- `clippy.toml` allows `unwrap`, `expect` and `panic` inside tests.
- `redundant_pub_crate` and `multiple_crate_versions` are allowed in the lint
  table; the first contradicts `unreachable_pub`, the second duplicates
  cargo-deny's ban check.
- `deny.toml` and `osv-scanner.toml` list the advisories and duplicate
  versions the GUI, math and diagram stacks pin, with review dates.

## Key Discoveries

### The vendored forks moved from `crates/` to `vendor/`

The file-length gate skips `vendor/` and nothing else, so the directory name
is the contract. The forks keep their own manifests and upstream lint setup.

### The detach path no longer needs `unsafe`

`Command::process_group(0)` replaces the `pre_exec` call to `setsid`. The
child leaves the shell's job without an `unsafe` block, which lets
`unsafe_code` stay at `forbid`.

### Per-agent results do not add up to a gate

Three agents each reported zero findings on their own files, and the merged
tree still failed clippy on 23 float comparisons: a lint error in one file
stops the test target from compiling, so nobody had seen that target's
findings. The same happened at every later merge (a dropped test helper, two
tests counting chrome shapes). Only a run on the merged tree counts.

### Tests that survive a redesign assert behavior

The coverage tests drive the real app through `egui_kittest` by accessible
label. When the menu bar was replaced by a top bar, the menu tests moved to
the new surface with the same assertions: "choosing X changes state Y".
