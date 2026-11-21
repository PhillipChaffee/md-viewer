#!/usr/bin/env bash
# Runs every Kani proof harness in src/ (the proofs.rs files; PROOFS.md
# lists them) and fails unless all three of these hold:
#   - every harness verifies;
#   - every `kani::cover!` in every harness is reached, so no harness
#     passes because its assumptions rule everything out;
#   - the number of harnesses verified is the number of `#[kani::proof]`
#     attributes in src/, so none was skipped.
#
# Needs Kani 0.68.0: `cargo install --locked --version 0.68.0 kani-verifier`
# then `cargo kani setup`. Arguments are passed on to `cargo kani`, for
# example `./run-proofs.sh --harness layout::proofs`; the count check is
# skipped then.
#
# PROOF_SHARD=K/N runs share K of N, which is how CI spreads the harnesses
# over several runners. The files that declare a harness are listed in
# path order and file number i belongs to share (i mod N) + 1, so every
# file is in exactly one share and the N shares together are the whole
# set. A share runs its files' modules (`--harness layout::proofs::`) and
# its count check compares against the `#[kani::proof]` attributes in
# those files, so a harness the filter missed, or matched twice, fails.
#
# PROOF_JOBS is how many harnesses run at once (default 2; each is a
# solver process that can take gigabytes, so raise it only with memory
# to spare). PROOF_TIMEOUT bounds one harness (default 15m).
# PROOF_LOG names a file to keep Kani's full output in.

set -u -o pipefail

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

mapfile -t files < <(git grep -l '^#\[kani::proof\]' -- 'src/*.rs')
filters=()
scope="src/"
if [ -n "${PROOF_SHARD:-}" ]; then
	usage="PROOF_SHARD is K/N with 1 <= K <= N and takes no arguments, got '$PROOF_SHARD'"
	[[ "$#" -eq 0 && "$PROOF_SHARD" =~ ^([1-9][0-9]*)/([1-9][0-9]*)$ ]] || {
		echo "proofs: $usage" >&2
		exit 1
	}
	shard="${BASH_REMATCH[1]}"
	shards="${BASH_REMATCH[2]}"
	share=()
	for i in "${!files[@]}"; do
		[ "$((i % shards + 1))" -ne "$shard" ] || share+=("${files[$i]}")
	done
	[ "${#share[@]}" -gt 0 ] || {
		echo "proofs: $usage; share $shard of $shards holds no file" >&2
		exit 1
	}
	files=("${share[@]}")
	for file in "${files[@]}"; do
		module="${file#src/}"
		module="${module%.rs}"
		filters+=(--harness "${module//\//::}::")
	done
	scope="share $shard of $shards (${files[*]})"
	echo "$scope"
fi

started=$(date +%s)
cargo kani -Z unstable-options \
	--harness-timeout "${PROOF_TIMEOUT:-15m}" \
	--jobs "${PROOF_JOBS:-2}" \
	--output-format terse \
	"${filters[@]}" "$@" >"$log" 2>&1
status=$?
elapsed=$(($(date +%s) - started))
[ -z "${PROOF_LOG:-}" ] || cp "$log" "$PROOF_LOG"

fail() {
	grep -vE 'register_tool|unstable feature|^ *\||^ *= note|^ *--> <crate|^$|^ *Compiling ' "$log"
	echo "proofs: $1" >&2
	exit 1
}

[ "$status" -eq 0 ] || fail "cargo kani exited with $status"

unreached="$(grep -E '^ \*\* [0-9]+ of [0-9]+ cover properties satisfied' "$log" |
	awk '$2 != $4' || true)"
[ -z "$unreached" ] || fail "a cover check was not reached: $unreached"

summary="$(grep -E '^Complete - ' "$log" || true)"
[ -n "$summary" ] || fail "no harness ran"

if [ "$#" -eq 0 ]; then
	declared="$(git grep -c '^#\[kani::proof\]' -- "${files[@]}" | awk -F: '{ total += $2 } END { print total + 0 }')"
	verified="$(echo "$summary" | awk '{ print $3 }')"
	[ "$declared" -eq "$verified" ] ||
		fail "$declared harnesses are declared in $scope but $verified were verified"
fi

echo "$summary"
echo "proofs pass in ${elapsed}s"
