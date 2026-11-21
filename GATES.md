# Gates

The application crate (`src/`) runs under the Rust baseline from
[full-lint](https://github.com/PhillipChaffee/full-lint). `./run-gates.sh`
runs every gate; `.github/workflows/ci.yml` runs the same commands.

One house rule sits on top: a test never lives in the file it tests. Each
module's tests are in a sibling `tests.rs`, and its proofs, where it has
any, in a sibling `proofs.rs`. [PROOFS.md](PROOFS.md) says what is proven
and what is not.

## Where the retrofit left the crate

| Gate | Before | After |
| --- | --- | --- |
| Largest file | `main.rs`, 7384 lines | 483 effective lines |
| Functions over cognitive 15 | 23 (worst 422) | 0 |
| Clippy findings under the strict groups | about 700 | 0 |
| Coverage, lines / regions / functions | 33% / 35% / 44% | above 99% on each |
| Tests | 76 pass, 2 fail, 5 ignored | all pass, none ignored |

## Exceptions, each with its reason beside it

Everything below narrows a gate. Each carries its reason in the file named.

| Where | What it narrows |
| --- | --- |
| `Cargo.toml` `[workspace] exclude` | `vendor/` is outside clippy, rustdoc, tests and coverage. See the caveat below. |
| `run-gates.sh`, `ci.yml` | The cognitive gate reads `src` and `crates` (arborist has no exclude switch). The TODO grep skips `vendor`. osv-scanner reads the root `Cargo.lock` only. |
| `crates/mac-window/Cargo.toml` lint table | `unsafe_code` is `deny` where the workspace says `forbid`, for one function behind an `#[expect(unsafe_code, reason)]`. See the section below. |
| `crates/mac-window/src/mac.rs` | Compiled for macOS only, so on Linux, where the gates run, clippy, the tests, the coverage gate and mutation testing never see it. See the section below. |
| `clippy.toml` | `unwrap`, `expect` and `panic` are allowed inside tests. |
| `Cargo.toml` lint table | `redundant_pub_crate` (contradicts `unreachable_pub`) and `multiple_crate_versions` (cargo-deny owns it) are allowed. The `unexpected_cfgs` entry declares one cfg, `cfg(kani)`, which the proof harnesses are compiled under. |
| `src/**/proofs.rs` | The proof harnesses are compiled only by `cargo kani`, so clippy, rustdoc, the tests and the coverage gate never see them. rustfmt, the complexity cap and the file-length cap do. `./run-proofs.sh` and the `proofs` CI job are their gate; it is not part of `run-gates.sh`. See [PROOFS.md](PROOFS.md). |
| `src/` | Seven `#[expect(..., reason)]`: five float casts on pixel math, one percentage cast, one `struct_excessive_bools` on `MarkdownApp`. No `#[allow]`. |
| `deny.toml` | Four advisories ignored with review dates. 66 duplicate crates skipped by name, plus the template's `windows-sys` tree. Five licenses added to the allow-list. `unmaintained` and `unsound` fail for direct dependencies only (the template's setting). |
| `osv-scanner.toml` | Fourteen advisories ignored: the same four, and ten "unmaintained" notices on transitive crates. Two license overrides: one for a `WITH LLVM-exception` expression, and one naming this repository's own unpublished `mac-window` crate as MIT, which the registry cannot answer for. |
| `ci.yml`, `run-gates.sh` license list | The shared full-lint list, unchanged, plus `Zlib`, `BSL-1.0`, `CDLA-Permissive-2.0`, `OFL-1.1`, `Ubuntu-font-1.0`. |
| `Cargo.toml` cargo-shear | `image` is ignored: used for its features only. |
| `.typos.toml`, `.jscpd.json`, `lychee.toml`, `.markdownlint-cli2.jsonc` | `vendor/` and build output are skipped. markdownlint turns off MD013, MD025, MD033, MD036, MD041, MD060 and relaxes MD010 and MD024. |
| `lychee.toml` | The two font license files, `assets/fonts/SourceSerif4-LICENSE.txt` and `assets/fonts/SourceCodePro-LICENSE.txt`, are skipped too. They are legal text kept verbatim, and their one URL (adobe.com) answers 403 to automated clients. |
| `.gitleaks.toml` | `target/` is skipped in the working-tree scan. |

## Caveat: the renderer fork is first-party code outside the gates

`vendor/egui_commonmark` was excluded as a fork reviewed against upstream.
The reader redesign then added about 4,300 lines of this project's own code
to it (block typesetting, diagram recoloring, typography hooks), because
that is where the renderer lives. That code has the fork's own clippy run
and its own tests (267 pass) in CI, but not the complexity cap, the
file-length cap or the coverage gate: `parsers/pulldown.rs` is over 6,000
lines. Bringing the fork under the gates is the largest piece of retrofit
work left, and it is not done.

## The one unsafe function: the Mac's window buttons

On a Mac the window has no title strip of its own: the close, minimize and
zoom buttons sit in the app's top row, on its centre line. winit cannot
move them, so the app asks AppKit itself, and two of the calls that takes
are unsafe: reading the window's view out of the raw window handle eframe
gives, and walking up from a button to the view that holds the title bar.

The application crate still forbids unsafe code. The calls live in a
second workspace crate, `crates/mac-window`, in one function,
`mac::title_bar`, which has two unsafe blocks, each under a `SAFETY`
comment. The owner accepted this one exception.

| What keeps it small | How |
| --- | --- |
| No other unsafe code in the crate | Its `unsafe_code` is `deny`, and only `title_bar` carries the `#[expect]`. A second unsafe function fails the build. |
| Each block says why it is sound | `clippy::undocumented_unsafe_blocks` is `deny` in the crate. |
| The arithmetic is not in it | `placement` and `is_off` are safe, compiled on every system and unit-tested. |
| No new dependency versions | It uses the `objc2` crates the window stack already resolves to on a Mac. |

The crate's lint table is a copy of the workspace's, because a crate
inherits that table whole or not at all. The copy differs in the two
entries named above. A change to the workspace table has to be made in
both.

What no gate checks: everything in `mac.rs`. CI builds it with
`cargo check` on `macos-latest` and runs none of it. `cargo clippy
--target aarch64-apple-darwin -p mac-window -- -D warnings` lints it from
Linux, and passed when the crate was written, but is not a CI step. Nobody
has run this code on a Mac under a test: whether the buttons land where
`placement` says is checked by eye. The nightly mutation run covers the
root package only, so it does not see this crate either.

## The detach change is weaker than what it replaced

`Command::process_group(0)` puts the child in its own process group, not its
own session. It leaves the shell's job, and a test asserts the new group,
but it keeps the controlling terminal, which `setsid` did not. Nobody has
closed a real terminal on it to see what happens. If that matters, `setsid`
needs an `unsafe` block and `unsafe_code` moves from `forbid` to `deny`.
