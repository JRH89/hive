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

## Attachment (tracked separately from process state)
- `attached: BTreeSet<ClientId>` and `controller: Option<ClientId>`.
- Attaching makes you controller ("last attach wins"). The old controller becomes an observer and gets `ControlLost`. In the TUI, Leader then `a` re-attaches to take control back.
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
