#!/usr/bin/env python3
"""Check a behavior-preserving split: compare the multiset of code lines in
src/ at a base commit against the working tree. Visibility prefixes are
normalized away, so a pure move prints only `use`, `mod` and `impl` framing.
Usage: verify-move.py <base-commit>"""
import collections, pathlib, re, subprocess, sys

def norm(line):
    line = line.strip()
    line = re.sub(r"^pub(\((crate|super|in [\w:]+)\))? ", "", line)
    return line

def lines(text):
    return collections.Counter(n for n in map(norm, text.splitlines()) if n and not n.startswith("//"))

base = sys.argv[1]
names = subprocess.run(["git", "ls-tree", "-r", "--name-only", base, "src"], capture_output=True, text=True, check=True).stdout.split()
old = collections.Counter()
for name in names:
    old += lines(subprocess.run(["git", "show", f"{base}:{name}"], capture_output=True, text=True, check=True).stdout)
new = collections.Counter()
for path in pathlib.Path("src").rglob("*.rs"):
    new += lines(path.read_text())
gone, added = old - new, new - old
framing = re.compile(r"^(use |mod |impl |#\[cfg\(test\)\]|#\[path|\}$|super::|crate::)")
real_added = {k: v for k, v in added.items() if not framing.match(k)}
print(f"lines gone: {sum(gone.values())}, added: {sum(added.values())}, added beyond framing: {sum(real_added.values())}")
for k, v in sorted(gone.items()):
    print(f"- {v}x {k}")
for k, v in sorted(real_added.items()):
    print(f"+ {v}x {k}")
