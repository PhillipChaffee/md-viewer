# Feature: full-lint retrofit

**Status:** 🚧 In Progress
**Branch:** `claude/beautiful-lovelace-sdpd77`
**Date:** 2026-11-13

## Summary

Retrofit the application crate onto the Rust baseline from
[full-lint](https://github.com/PhillipChaffee/full-lint): strict clippy
groups, denied doc lints, a cognitive-complexity cap of 15, a 1000
effective-line file cap, a 95% coverage gate, and the shared hygiene gates.
House rule added on top: a test never lives in the file it tests.

## Scope

- [ ] Gate configuration copied from the full-lint templates
- [ ] `src/` split by responsibility, tests in their own files
- [ ] Lint, doc, format, cognitive and file-length gates green
- [ ] Coverage at 95% on lines, regions and functions
- [ ] Hygiene gates green

## Key Discoveries

### The vendored forks moved from `crates/` to `vendor/`

The file-length gate skips `vendor/` and nothing else, so the directory name
is the contract. The forks keep their own manifests and upstream lint setup.

### The detach path no longer needs `unsafe`

`Command::process_group(0)` replaces the `pre_exec` call to `setsid`. The
child leaves the shell's job without an `unsafe` block, which lets
`unsafe_code` stay at `forbid`.
