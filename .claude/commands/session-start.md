---
description: Explicitly run session start checks (usually runs automatically)
allowed-tools: Bash, Read
---

**Note:** These checks run automatically when starting implementation work (see context-awareness.md).
Use `/session-start` to explicitly re-run them.

**Steps:**

1. Show current branch and worktree:

```bash
git branch --show-current
pwd
```

1. Check if branch is up-to-date with main:

```bash
git fetch origin 2>/dev/null
git log --oneline HEAD..origin/main 2>/dev/null | head -5
```

If output is not empty, warn that the branch may be stale.

1. Show recent commits on this branch:

```bash
git log --oneline -5
```

1. Check for any work-in-progress:

```bash
git status --short
```

1. Summarize:
   - Current branch and status
   - Whether rebase is recommended
   - Any uncommitted changes
   - Relevant lessons to keep in mind
