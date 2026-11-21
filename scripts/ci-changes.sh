#!/usr/bin/env bash
# Says which CI jobs a change needs, from the paths it touches. Prints one
# line for $GITHUB_OUTPUT:
#   code=true|false    the Rust jobs: test, proofs, doc-comments, renderer
#                      and the cross-platform cargo check
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

set -u -o pipefail

everything() {
	printf 'code=true\n'
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
while IFS= read -r path; do
	case "$path" in
	src/* | crates/* | vendor/* | assets/* | tests/* | data/* | scripts/* | .github/*) code=true ;;
	docs/* | screenshots/* | LICENSE* | THIRD_PARTY_NOTICES | *.md) ;;
	*) code=true ;;
	esac
done <<<"$paths"

printf 'code=%s\n' "$code"
echo "$(wc -l <<<"$paths") changed paths: code=$code" >&2
