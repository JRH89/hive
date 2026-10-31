---
status: accepted
date: 2026-10-30
---
# ADR-0012: Worktrees created by the daemon, never deleted dirty

## Context
Phase 4 requires optional worktree isolation, worktree-aware launching and safe cleanup. Spec §12 says to never silently delete worktrees with uncommitted changes and to confirm destructive git operations. Agents in the same directory can overwrite each other's work.

## Decision
- `CreateSession{worktree: Some(NewWorktree{branch})}`: the **daemon** runs `git worktree add` (argv, never a shell) in the workspace's repository, then launches the session there. The default branch is `hive/<agent>-<session id>`. An existing branch is checked out; a missing one is created from HEAD. Names are validated with `git check-ref-format`.
- Location: `[worktrees] dir` in config, default `$XDG_DATA_HOME/hive/worktrees/<repo>-<fnv hash of its path>/<branch>`. This keeps repositories' own trees clean, and same-named repositories can't collide.
- `RemoveWorktree{path, force}`: refused for the main working tree, refused while a running session uses it, and refused if dirty (`git status --porcelain`, untracked files included) unless `force`. **Branches are never deleted.**
- CLI `hive worktree rm --force` asks for confirmation unless `-y`. TUI: the kill confirmation offers `w` (terminate, then remove the worktree, non-forced). A dirty worktree is kept, with a message explaining how to force it.
- Sharing warnings: the CLI (`new -d`) and the TUI (on create, plus a `!` in the sidebar) warn when running sessions share an exact directory.

## Consequences
- Running git in the daemon makes "is anyone using it?" checks authoritative. git calls run on `spawn_blocking` and never block the runtime.
- A failed agent launch after a successful `worktree add` leaves the worktree in place (nothing is deleted implicitly).
- Each new worktree is a new directory, so agents may ask to trust it again. That's expected; Hive never answers those prompts.
