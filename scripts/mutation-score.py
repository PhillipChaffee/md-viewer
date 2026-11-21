"""Merge the shards' cargo-mutants outcomes and apply the mutation score floor.

Usage: python3 scripts/mutation-score.py <shard count> <outcomes.json>...

Exits 1 when a shard's outcomes are missing or the score is under the
floor. The README's "Mutation testing" section says how the score is
counted and why the floor is 85.
"""

import json
import sys

FLOOR = 85
COUNTS = ("total_mutants", "caught", "missed", "timeout", "unviable")


def main(argv):
    shards, paths = int(argv[1]), argv[2:]
    if len(paths) != shards:
        print(f"expected outcomes from {shards} shards, found {len(paths)}")
        return 1
    total = dict.fromkeys(COUNTS, 0)
    for path in paths:
        with open(path, encoding="utf-8") as file:
            outcomes = json.load(file)
        for key in COUNTS:
            total[key] += outcomes[key]
    # An unviable mutant does not compile, so no test ever runs against it.
    # A timeout counts as not caught.
    tested = total["total_mutants"] - total["unviable"]
    if tested == 0:
        print("no mutant was tested")
        return 1
    score = 100 * total["caught"] // tested
    print(
        f"mutation score {score}% (caught {total['caught']} of {tested} tested; "
        f"{total['missed']} missed, {total['timeout']} timed out, "
        f"{total['unviable']} unviable of {total['total_mutants']}; floor {FLOOR})"
    )
    return 0 if score >= FLOOR else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
