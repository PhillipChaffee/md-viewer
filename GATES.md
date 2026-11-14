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
| `run-gates.sh`, `ci.yml` | The cognitive gate reads `src` (arborist has no exclude switch). The TODO grep skips `vendor`. osv-scanner reads the root `Cargo.lock` only. |
| `clippy.toml` | `unwrap`, `expect` and `panic` are allowed inside tests. |
| `Cargo.toml` lint table | `redundant_pub_crate` (contradicts `unreachable_pub`) and `multiple_crate_versions` (cargo-deny owns it) are allowed. The `unexpected_cfgs` entry declares one cfg, `cfg(kani)`, which the proof harnesses are compiled under. |
| `src/**/proofs.rs` | The proof harnesses are compiled only by `cargo kani`, so clippy, rustdoc, the tests and the coverage gate never see them. rustfmt, the complexity cap and the file-length cap do. `./run-proofs.sh` and the `proofs` CI job are their gate; it is not part of `run-gates.sh`. See [PROOFS.md](PROOFS.md). |
| `src/` | Seven `#[expect(..., reason)]`: five float casts on pixel math, one percentage cast, one `struct_excessive_bools` on `MarkdownApp`. No `#[allow]`. |
| `deny.toml` | Four advisories ignored with review dates. 66 duplicate crates skipped by name, plus the template's `windows-sys` tree. Five licenses added to the allow-list. `unmaintained` and `unsound` fail for direct dependencies only (the template's setting). |
| `osv-scanner.toml` | Thirteen advisories ignored: the same four, and nine "unmaintained" notices on transitive crates. One license override for a `WITH LLVM-exception` expression. |
| `ci.yml`, `run-gates.sh` license list | The shared full-lint list, unchanged, plus `Zlib`, `BSL-1.0`, `CDLA-Permissive-2.0`, `OFL-1.1`, `Ubuntu-font-1.0`. |
| `Cargo.toml` cargo-shear | `image` is ignored: used for its features only. |
| `.typos.toml`, `.jscpd.json`, `lychee.toml`, `.markdownlint-cli2.jsonc` | `vendor/` and build output are skipped. markdownlint turns off MD013, MD025, MD033, MD036, MD041, MD060 and relaxes MD010 and MD024. |
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

## The detach change is weaker than what it replaced

`Command::process_group(0)` puts the child in its own process group, not its
own session. It leaves the shell's job, and a test asserts the new group,
but it keeps the controlling terminal, which `setsid` did not. Nobody has
closed a real terminal on it to see what happens. If that matters, `setsid`
needs an `unsafe` block and `unsafe_code` moves from `forbid` to `deny`.
