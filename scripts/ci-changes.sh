#!/usr/bin/env bash
# Says which CI jobs a change needs, from the paths it touches. Prints two
# lines for $GITHUB_OUTPUT:
#   code=true|false    the Rust jobs: lint, test, doc-comments, renderer
#                      and the cross-platform cargo check
#   proofs=true|false  the Kani proofs
# hygiene and version-sync are not on the list: they always run.
#
# Usage: ci-changes.sh <base> <head>   the paths that differ between two commits
#        ci-changes.sh -               the paths on standard input, one per line
#        ci-changes.sh                 no paths known
# With no paths known, with an empty list, or when git cannot compute the
# difference, every answer is true: when in doubt, run.
#
# code is false only when every path is documentation: docs/, screenshots/,
# the root license files, and Markdown outside the directories whose
# Markdown the build or a test reads (tests/fixtures and the renderer's
# examples are compiled into test binaries).
#
# proofs is true when a path is something `cargo kani` compiles (the crate,
# the crates and forks it depends on, the fonts it embeds, the manifests
# and the toolchain pin) or part of the proof machinery, this script and
# the workflows included. Integration tests, lint configuration and the
# other scripts are none of those: Kani never reads them.

set -u -o pipefail

everything() {
	printf 'code=true\nproofs=true\n'
	exit 0
}

case "$#" in
1)
	[ "$1" = - ] || everything
	paths="$(cat)"
	;;
2) paths="$(git diff --no-renames --name-only "$1" "$2" --)" || everything ;;
*) everything ;;
esac
[ -n "$paths" ] || everything

code=false
proofs=false
while IFS= read -r path; do
	case "$path" in
	src/* | crates/* | vendor/* | assets/* | .cargo/* | .github/workflows/* | \
		Cargo.toml | Cargo.lock | build.rs | rust-toolchain.toml | \
		run-proofs.sh | scripts/*proof* | scripts/ci-changes.sh)
		code=true
		proofs=true
		;;
	tests/* | data/* | scripts/* | .github/*) code=true ;;
	docs/* | screenshots/* | LICENSE* | THIRD_PARTY_NOTICES | *.md) ;;
	*) code=true ;;
	esac
done <<<"$paths"

printf 'code=%s\nproofs=%s\n' "$code" "$proofs"
echo "$(wc -l <<<"$paths") changed paths: code=$code proofs=$proofs" >&2
