#!/usr/bin/env bash
# Shows that the proofs are not vacuous: plants each one-line fault from
# scripts/proof-mutants.tsv in the shipped code, runs the harnesses named
# beside it, and expects the verifier to reject the faulty code.
#
# Each row of the table is four tab-separated fields: the file, a
# `cargo kani --harness` filter, the text to replace, and the faulty text
# (`\n` stands for a line break; an empty last field deletes the text).
# The text to replace must occur exactly once in the file, so a row that
# no longer matches the code fails loudly, not silently.
#
# Slow (one verifier run per row), so it is not a gate. Run it after
# changing a proof or the code under one. Arguments narrow it to the rows
# whose file or filter contains one of them: `kill-proof-mutants.sh layout`.

set -u -o pipefail

cd "$(dirname "$0")/.." || exit 1
table="scripts/proof-mutants.tsv"
log="$(mktemp)"
backup="$(mktemp)"
mutated=""
restore() {
	[ -z "$mutated" ] || cp "$backup" "$mutated"
	mutated=""
}
trap 'restore; rm -f "$log" "$backup"' EXIT

wanted() {
	[ "$#" -eq 2 ] && return 0
	local row="$1 $2" pattern
	for pattern in "${@:3}"; do
		case "$row" in *"$pattern"*) return 0 ;; esac
	done
	return 1
}

killed=0
survived=0
while IFS=$'\t' read -r file harness original mutant; do
	case "$file" in '' | '#'*) continue ;; esac
	wanted "$file" "$harness" "$@" || continue

	shown="$original -> $mutant"
	original="${original//\\n/$'\n'}"
	mutant="${mutant//\\n/$'\n'}"
	occurrences="$(ORIGINAL="$original" perl -0ne 'print scalar(() = /\Q$ENV{ORIGINAL}\E/g)' "$file")"
	if [ "$occurrences" -ne 1 ]; then
		echo "STALE     $file: $shown ($occurrences occurrences of the text to replace)"
		survived=$((survived + 1))
		continue
	fi
	cp "$file" "$backup"
	mutated="$file"
	ORIGINAL="$original" MUTANT="$mutant" perl -0pi -e 's/\Q$ENV{ORIGINAL}\E/$ENV{MUTANT}/' "$file"

	cargo kani -Z unstable-options --harness-timeout 15m --output-format terse \
		--harness "$harness" >"$log" 2>&1
	status=$?
	restore

	if [ "$status" -ne 0 ] && grep -q '^VERIFICATION:- FAILED' "$log" &&
		! grep -qE 'could not compile|timed out|out of memory' "$log"; then
		echo "KILLED    $file: $shown ($harness)"
		killed=$((killed + 1))
	else
		echo "SURVIVED  $file: $shown ($harness)"
		survived=$((survived + 1))
	fi
done <"$table"

echo "$killed mutants killed, $survived survived"
[ "$survived" -eq 0 ]
