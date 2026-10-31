---
status: accepted
date: 2026-10-30
---
# ADR-0011: Daemon-owned workspaces grouped by repository

## Context
Spec §12 and Phase 4 ask for workspaces that group sessions by repository, can be created and named, remember their metadata, and show git branches. "Sessions appear under the correct workspace" has to hold for sessions started from the CLI, the TUI and relaunches, including sessions in linked worktrees that live outside the repository directory.

## Decision
- The **daemon** owns workspaces `{id, name, root}`, persisted in `sessions.json` (store v2; v1 files are migrated). Workspaces survive graceful shutdown; sessions don't.
- **Every session has a workspace.** On create without an explicit one, the cwd is resolved to a key: its repository's *main* worktree root, or the directory itself outside git. The workspace with the deepest root containing the key wins. If none matches, one is created, named after the directory.
- `CreateWorkspace{path}` resolves to the same key and is idempotent per root. `RemoveWorkspace` only forgets, and is refused while any session (running, exited or lost) belongs to it.
- Git discovery reads `.git`, `commondir` and `HEAD` directly (crate `hive-workspace`), stopping at filesystem boundaries like git does. No subprocess, so it is cheap.
- Branches are re-read every 2 s for running sessions; `SessionUpdated` is published only on change.

## Consequences
- Linked worktrees, wherever they are, join their repository's workspace.
- An explicit umbrella workspace (e.g. `~/src`) captures repositories under it, because the deepest-root rule prefers user-created ancestors over auto-creating.
- The 2 s refresh is a bounded timer, not a busy loop. A branch switch appears within about 2 s.
- New crate `hive-workspace`, which the daemon, CLI and TUI share. The CLI and TUI use it to find "the workspace of the current directory".
