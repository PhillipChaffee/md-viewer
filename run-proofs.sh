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
# PROOF_JOBS is how many harnesses run at once (default 2; the largest
# takes about 3 GB). PROOF_TIMEOUT bounds one harness (default 15m).

set -u -o pipefail

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

started=$(date +%s)
cargo kani -Z unstable-options \
	--harness-timeout "${PROOF_TIMEOUT:-15m}" \
	--jobs "${PROOF_JOBS:-2}" \
	--output-format terse \
	"$@" >"$log" 2>&1
status=$?
elapsed=$(($(date +%s) - started))

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
	declared="$(git grep -c '^#\[kani::proof\]' -- 'src/*.rs' | awk -F: '{ total += $2 } END { print total + 0 }')"
	verified="$(echo "$summary" | awk '{ print $3 }')"
	[ "$declared" -eq "$verified" ] ||
		fail "$declared harnesses are declared in src/ but $verified were verified"
fi

echo "$summary"
echo "proofs pass in ${elapsed}s"
