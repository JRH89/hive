---
tags: [architecture, protocol]
source: crates/hive-protocol/src/lib.rs
---
# IPC Protocol

- Transport: Unix stream socket, per-user dir. The daemon checks `SO_PEERCRED` uid == own uid.
- Frame: `u32` little-endian length + `postcard` payload. Max 8 MiB (`MAX_FRAME_LEN`). See [[ADR-0003 postcard length-prefixed frames]].
- `PROTOCOL_VERSION = 6` (v2 added `RenameSession`; v3 added `Attach.force`, `ControlReleased` and `SessionState::Lost`; v4 added workspaces and worktrees; v5 added `SessionInfo.presence`, `ReportAgent`, `ReleaseAgent`, see [[ADR-0016 Agent detection from process and herdr manifests]]; v6 made a connection attach several sessions and gave `Input`/`Resize`/`Detach` a session, see [[ADR-0018 Tabs of split panes over multi-attach]]). Bump it on any incompatible change. New variants go at the **end** of the enums.
- **Compat invariant:** `ClientMessage::Hello` and `ServerMessage::Welcome` stay variant 0 with the same shape (a unit test enforces this). That way a mismatched client and daemon can still handshake and report "run `hive daemon stop`".

## Handshake
Client → `Hello{protocol_version, client}` (within 5 s) → daemon replies `Welcome{protocol_version, daemon_version, daemon_pid}`. On a version mismatch the daemon closes the connection after sending Welcome, and the client raises `VersionMismatch`.

## Client → daemon
| Message | Effect |
|---|---|
| `ListSessions` | → `SessionList` |
| `CreateSession{agent,label,command,cwd,env,size,workspace,worktree}` | spawn; → `SessionCreated`, plus a `SessionUpdated` broadcast. `command` is argv, never a shell string. `env` is the client's full environment. `workspace: None` means resolve from `cwd`; `worktree: Some{branch}` creates a linked worktree and launches there. **The broadcast can overtake the reply**, so don't let `SessionCreated` overwrite newer info |
| `Attach{session,size,force}` | adds the session to the connection's attached set (re-attaching replaces it); become **controller** if control is free or `force`, else observe; a controller resizes the PTY to `size`; → `Attached{session,snapshot,controller}`. A forced take-over sends the previous controller `ControlLost`. Fails for `Lost` sessions. See [[ADR-0010 Attach observes unless control is free or forced]] |
| `Detach{session}` | one session, or all with `None`; → `Detached{session}` per detached session |
| `Input{session,data}` / `Resize{session,size}` | silently ignored unless the sender controls that session |
| `RenameSession{session,label}` | trimmed, ≤64 chars; empty resets to the agent name; → `SessionUpdated` broadcast |
| `KillSession(id)` | TERM → (timeout) → KILL the process group, remove; → `SessionRemoved` broadcast. On a `Lost` record it just forgets it |
| `Shutdown` | terminate all sessions, `ShuttingDown` broadcast, remove socket, exit |
| `ListWorkspaces` | → `WorkspaceList` |
| `CreateWorkspace{path,name}` | root = repository main root (or `path`); idempotent per root; → `WorkspaceCreated`, plus a `WorkspaceUpdated` broadcast if new |
| `RenameWorkspace{workspace,name}` | → `WorkspaceUpdated` broadcast |
| `RemoveWorkspace(id)` | forget it (refused with any session in it); → `WorkspaceRemoved` broadcast |
| `ListWorktrees(id)` | → `WorktreeList{workspace, worktrees:[{path,branch,main,dirty,sessions}]}` |
| `RemoveWorktree{path,force}` | refused for the main tree, in use, or dirty without `force`; → `WorktreeRemoved{path}`. Branch kept |

## Daemon → client (streamed while attached)
`Output{data}`, `Resized{size}` (ordered with output, so mirrors stay in sync), `Resync{size,snapshot}` (after lag, and when an app leaves the alternate screen), `Detached{session}` (unprompted only when the session is removed; it can arrive before or after `SessionRemoved`). Every streamed message names its session, `ControlLost{session}`, `ControlReleased{session}` (to observers when the controller leaves).

## Broadcasts (every client)
`SessionUpdated(info)`, `SessionRemoved(id)`, `WorkspaceUpdated(info)`, `WorkspaceRemoved(id)`, `ShuttingDown`. If a client lags on the event channel it gets a full `WorkspaceList` and `SessionList` instead.

**Ordering guarantee:** no `SessionUpdated(id)` ever follows `SessionRemoved(id)`. Both are emitted only through `Session::publish`/`remove`, under the session's meta lock. See [[ADR-0008 Session events published under the session lock]].

## Client usage notes
- `read_frame` is **not cancel-safe**. Keep it in a dedicated task and never put it in `select!`.
- One-shot CLI commands use `Client::recv_until(pick)`. It skips unrelated broadcasts and converts `Error` into `ClientError::Daemon`.
