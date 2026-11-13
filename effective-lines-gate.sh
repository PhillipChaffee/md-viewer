#!/usr/bin/env bash
#
# File-length gate for a strict Rust repository: fails when any Rust
# file exceeds the maximum effective-line count.
#
# "Effective lines" counts a physical line unless it is blank, is a
# whole-line comment (a line that opens with "//" — covering "///" and
# "//!", or a line the nesting-aware "/* */" scanner leaves with no code
# outside the comment), or lies inside a "/* */" block — the same
# definition eslint gives TypeScript's max-lines (skipBlankLines +
# skipComments). Attributes count as code. The counting is deliberately
# line-shaped, not a token stream, and that shape carries a documented
# imprecision (shell coverage-gate style): the "//" and "/*" tokens
# inside a string literal — a raw string r#"..."# most visibly — are
# classified as comment markers, so such lines can shift the count a
# line or two, always in the conservative (lower-count) direction. The
# counter is a stdlib-only Rust program embedded in this script and
# compiled by the rustc the pinned toolchain already provides, so the
# repository it gates gains no Rust source its linters, coverage, or
# dependency gates would have to see. A file that cannot be read fails
# the gate: fail closed, never silently.
#
# Exemption: a file whose leading comment-and-blank section carries the
# canonical "// Code generated ... DO NOT EDIT." header line before any
# counted code is exempt — the house principle that generated code is
# not reviewed here. The scan walks every *.rs file from the repository
# root, skipping exactly what cargo never reviews: hidden directories,
# target/ (build output), and vendor/ (vendored third-party sources).
# Test files are capped identically: no test carve-out (a table too big
# for the cap is data and belongs in a fixture).
#
# Threshold 1000: ratified per-language to match python's effective-lines
# cap; Sonar's per-language file defaults are 1000 *raw* lines, and 1000
# *effective* is stricter than any of them because blanks and comments
# drop out. Raise it only with a written reason here, never silently.
# Remedy for a violation: split the file by responsibility
# (clippy::too_many_lines stays as the per-function axis).
#
# Usage: run from the repository root: ./effective-lines-gate.sh
set -u -o pipefail

readonly max_effective_lines=1000

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

root="$(pwd -P)"

# The counter is embedded: the gate carries its own tool and adds no Rust
# source to the repository it gates — so it cannot trip the linters,
# coverage, or dependency gates it runs beside. Stdlib only.
cat >"$tmp/effective_lines.rs" <<'EOF'
// Effective-lines counter for the house file-length gate. A line counts
// unless it is blank, is a whole-line comment, or lies inside a nesting-
// aware block comment; the classification is line-shaped (see the script
// header). Usage: effective-lines <max> <root>. Exit 0 clean, 1
// violations, 2 tooling or usage error.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: effective-lines <max-effective-lines> <root>");
        return 2;
    }
    let max: usize = match args[1].parse() {
        Ok(v) if v >= 1 => v,
        _ => {
            eprintln!("effective-lines: <max> must be a positive integer");
            return 2;
        }
    };
    let root = match fs::canonicalize(&args[2]) {
        Ok(p) if p.is_dir() => p,
        _ => {
            eprintln!("effective-lines: not a directory: {}", args[2]);
            return 2;
        }
    };
    let mut files = Vec::new();
    if let Err(err) = walk(&root, &mut files) {
        eprintln!("effective-lines: walk failed: {}", err);
        return 2;
    }
    files.sort();
    let mut violations = 0;
    for path in &files {
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                eprintln!(
                    "effective-lines: cannot read {}: {}",
                    path.display(),
                    err
                );
                return 2;
            }
        };
        let (count, generated) = count_effective(&src);
        if generated || count <= max {
            continue;
        }
        violations += 1;
        println!(
            "{}: {} effective lines exceeds the maximum of {}",
            path.display(),
            count,
            max
        );
    }
    if violations > 0 {
        eprintln!(
            "effective-lines: FAIL — {} file(s) exceed the {} effective-line maximum",
            violations,
            max
        );
        return 1;
    }
    0
}

// The scan walks every *.rs file under root, skipping exactly what cargo
// never reviews: hidden directories, target/ (build output), and vendor/
// (vendored third-party sources). Symlinks are skipped outright so a
// linked tree cannot loop the walk; an unreadable directory fails the
// gate (exit 2), never a silent skip.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        if file_type.is_dir() {
            if name.starts_with('.') || name == "target" || name == "vendor" {
                continue;
            }
            walk(&entry.path(), out)?;
        } else if file_type.is_file() && name.ends_with(".rs") {
            out.push(entry.path());
        }
    }
    Ok(())
}

// Returns (effective-line count, is-generated). The canonical generated
// header (cross-language convention) exempts the file when it appears in
// the leading comment-and-blank section, before any counted code.
fn count_effective(src: &str) -> (usize, bool) {
    let mut depth = 0usize;
    let mut count = 0usize;
    let mut generated = false;
    let mut code_seen = false;
    for line in src.lines() {
        let trimmed = line.trim();
        if depth == 0 {
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("//") {
                if !code_seen && is_generated_header(trimmed) {
                    generated = true;
                }
                continue;
            }
        }
        if scan_line(line, &mut depth) {
            code_seen = true;
            count += 1;
        }
    }
    (count, generated)
}

// The canonical generated-file header line: "// Code generated " + a
// non-empty attribution + " DO NOT EDIT." — the same shape the house go
// gate matches (go.dev/s/generatedcode).
fn is_generated_header(trimmed: &str) -> bool {
    match trimmed.strip_prefix("// Code generated ") {
        Some(rest) => match rest.strip_suffix(" DO NOT EDIT.") {
            Some(middle) => !middle.is_empty(),
            None => false,
        },
        None => false,
    }
}

// One physical line against the block-comment state: returns whether any
// of the line lies outside a comment. "//" ends the scan of a line
// outside a block; inside a block only "/*" and "*/" move the depth, so
// prose containing "//" cannot be mistaken for a line comment.
fn scan_line(line: &str, depth: &mut usize) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut code = 0usize;
    while i < bytes.len() {
        if *depth == 0 {
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                *depth += 1;
                i += 2;
            } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
                break;
            } else {
                code += 1;
                i += 1;
            }
        } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
            *depth -= 1;
            i += 2;
        } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            *depth += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    code > 0
}
EOF

rustc --edition 2021 -O -o "$tmp/effective_lines" "$tmp/effective_lines.rs" >"$tmp/toolerr.txt" 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
	echo "effective-lines-gate: FAIL — the counter could not compile; fix the tooling, never skip the gate" >&2
	sed 's/^/  /' "$tmp/toolerr.txt" >&2
	exit 1
fi

"$tmp/effective_lines" "$max_effective_lines" "$root" >"$tmp/violations.txt" 2>"$tmp/toolerr.txt"
rc=$?

if [ "$rc" -eq 1 ]; then
	sed 's/^/effective-lines: /' "$tmp/violations.txt" >&2
	cat "$tmp/toolerr.txt" >&2
	exit 1
fi
if [ "$rc" -ne 0 ]; then
	echo "effective-lines-gate: FAIL — the counter could not run; fix the tooling, never skip the gate" >&2
	sed 's/^/  /' "$tmp/toolerr.txt" >&2
	exit 1
fi
echo "effective-lines-gate: PASS — no Rust file exceeds ${max_effective_lines} effective lines"
