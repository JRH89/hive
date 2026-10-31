---
tags: [architecture]
source: crates/hive-daemon/src/session.rs
---
# Session Lifecycle

## Process state (`SessionState`)
`Starting` → `Running` → `Exited{code, signal}` or `Failed{reason}`

- Today a session is `Running` as soon as `spawn_command` returns. `Starting` is reserved for adapters that will need a readiness step.
- A spawn failure (e.g. a binary missing from PATH) is returned to the client as `Error` and is **not** registered. The CLI and TUI pre-check PATH via `launch::find_executable` for a friendlier message.
- `Failed` is used when `child.wait()` itself errors.
- An exited session **stays listed** (the final screen is still viewable) until it is killed or removed.
- `Lost`: only in a daemon started after a crash. The process is gone; the record (from `sessions.json`) can be renamed, dismissed, or relaunched from the TUI sidebar. Never attachable. See [[ADR-0009 Persist session metadata, mark lost sessions]].

## Placement
Every session has a `workspace`, resolved at create time ([[ADR-0011 Daemon-owned workspaces grouped by repository]]). The daemon keeps the session's discovered `Repo` and re-reads `HEAD` every 2 s while it runs, publishing `branch` changes. `worktree` is set when the cwd is in a linked worktree (any, not only Hive-created ones).

## Persistence
- Live sessions (not exited ones) plus lost records are written to `$HIVE_STATE_DIR/sessions.json` after each burst of lifecycle events. No env.
- Graceful shutdown writes only the lost records, so sessions stopped on purpose aren't "lost". Ids never repeat across restarts.

## Attachment (tracked separately from process state)
- `attached: BTreeSet<ClientId>` and `controller: Option<ClientId>`.
- Attaching makes you controller only if control is free; otherwise you observe. A forced attach (TUI Leader `a`) takes over, and the old controller gets `ControlLost`. When the controller detaches, observers get `ControlReleased`. See [[ADR-0010 Attach observes unless control is free or forced]].
- Only the controller's `Input`/`Resize` are honoured. This is the explicit input ownership required by spec §11.

## Lifecycle events
Only `Session::publish()` (sends `SessionUpdated` unless removed) and `Session::remove()` (sets `removed`, sends `SessionRemoved`) emit per-session broadcasts, both under the `meta` lock. See [[ADR-0008 Session events published under the session lock]].

## Size policy
- The PTY size follows the **controller's** panel size (terminal minus header, status bar and the 26-column sidebar): set on attach and on every controller resize.
- With no client attached the PTY keeps its last size. Background agents are not resized under them.
- Observers render the controller's size, clipped to their own panel.

## Termination
1. `SIGTERM` to the process group. The PTY child is a session leader, so pid == pgid and descendants are included.
2. Wait `daemon.kill_timeout_secs` (default 3).
3. If still alive, `SIGKILL` the group.

The integration test `exit_is_reported_and_kill_removes_session` covers a child that traps TERM. The test waits for an `armed` marker first, because SIGTERM otherwise beats the `trap` (that race was caught during Phase 1; see [[2026-10-30]]).

## Thread/resource teardown
Reader exits on EIO once all slave fds close. The waiter publishes the exit state. The writer exits when the `Session` (which holds the input sender) is dropped. Known edge: a grandchild that escapes the process group and keeps the slave open keeps the reader alive. See [[Risks and Open Questions]].
