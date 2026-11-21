# The owner's picks on the review findings (2026-11-21)

Every finding with `"pile": "will-do"` in your PR's file is accepted as written (with the verifier's adjustments, which are in the entry). For needs-call findings the owner picked:

order: Accept this order
21-N1: Give the buttons of each question their own ids
29-F1: Set the source at the text size again, in the window's face
21-B1: Do not cut on key repeats, and add a test
27-F1: Elide in names what the clashing places share and keep a pixel cap as the last resort
24-F1: Add quit_has_to_ask in close_prompt and use it
23-F3: Draw the ring on the home buttons and the fold control
28-N1: Reset the offset when a popup opens
29-F3: Accept 13 px, name it in the changelog, and set the note in the chrome's small size
22-F4: Keep ignoring drops under both overlays
28-F1: Add show_popup, use it at every Popup site and wrap the outline menu
23-F2: Rename Tab id to document_id and leave the key
26-F5: Name the boolean now and leave the heading alone
21-F3: Drop the Overwrite answer and Goal Save in PR 21, rename later in a PR of its own
21-B2: Press the keys directly again and delete the helper
24-F3: Keep the per-frame compare and record it as a known cost
26-F3: One field and one rule in add
26-F7b: Run epaint's text tests in the fork's CI job
27-F4: Leave the fields on Tab for now
28-F2: Move it into chrome, keep the frame check, fix the comment
29-F2: Do it as a refactor PR of its own, and correct the two comments now
23-N1: Do it with 23-F3 in the same commit
23-F4: Keep the repeat filter as it is
26-F8: Leave it for a follow-up PR
22-F3: Leave the two enums as they are
24-F5b: Owner accepts the transmute and GATES.md says so
24-F6: Leave it pub like placement and is_off
26-F6: Leave the code and take a release-build measurement

## Rules for acting on them
- Work only on your own PR's branch. Ordinary pushes; never force-push, never rebase; no merge of the PR.
- One commit per finding (or per pair the entry says belongs together). A bug fix has a test seen failing first. Diffs in the entries are sketches, not compiled: read the code at your branch head first.
- Deferred work stays out: renames named "later in a PR of its own" (the second half of 21-F3, 22-F5's Modal rename), 23-F1 and 23-F5 (after PRs 21/23/27 merge), 26-F8, 29-F2's refactor.
- Where a pick records a rule exception (24-F3, 24-F5b), say so in GATES.md.
- Commits by the configured author; no Co-Authored-By, Claude-Session or "Generated with" lines; no footer in the PR description. Update the PR description if what it says changes.
- Run the repository's checks (fmt, clippy -D warnings, cargo test --workspace, arborist, effective-lines gate, typos, markdownlint on edited .md) before each push.
- Report per finding: done (commit) / not done and why, and anything new you noticed.
