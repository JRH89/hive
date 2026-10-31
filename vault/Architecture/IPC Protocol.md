---
tags: [architecture, protocol]
source: crates/hive-protocol/src/lib.rs
---
# IPC Protocol

- Transport: Unix stream socket, per-user dir. The daemon checks `SO_PEERCRED` uid == own uid.
- Frame: `u32` little-endian length + `postcard` payload. Max 8 MiB (`MAX_FRAME_LEN`). See [[ADR-0003 postcard length-prefixed frames]].
- `PROTOCOL_VERSION = 2` (v2 added `RenameSession`). Bump it on any incompatible change. New variants go at the **end** of the enums.
- **Compat invariant:** `ClientMessage::Hello` and `ServerMessage::Welcome` stay variant 0 with the same shape (a unit test enforces this). That way a mismatched client and daemon can still handshake and report "run `hive daemon stop`".

## Handshake
Client → `Hello{protocol_version, client}` (within 5 s) → daemon replies `Welcome{protocol_version, daemon_version, daemon_pid}`. On a version mismatch the daemon closes the connection after sending Welcome, and the client raises `VersionMismatch`.

## Client → daemon
| Message | Effect |
|---|---|
| `ListSessions` | → `SessionList` |
| `CreateSession{agent,label,command,cwd,env,size}` | spawn; → `SessionCreated`, plus a `SessionUpdated` broadcast. `command` is argv, never a shell string. `env` is the client's full environment |
| `Attach{session,size}` | become **controller**, PTY resized to `size`; → `Attached{session,snapshot,controller}`. The previous controller gets `ControlLost` |
| `Detach` | → `Detached` |
| `Input(bytes)` / `Resize(size)` | silently ignored unless sender is controller |
| `RenameSession{session,label}` | trimmed, ≤64 chars; empty resets to the agent name; → `SessionUpdated` broadcast |
| `KillSession(id)` | TERM → (timeout) → KILL the process group, remove; → `SessionRemoved` broadcast |
| `Shutdown` | terminate all sessions, `ShuttingDown` broadcast, remove socket, exit |

## Daemon → client (streamed while attached)
`Output{data}`, `Resized{size}` (ordered with output, so mirrors stay in sync), `Resync{size,snapshot}` (after lag), `Detached` (when the session is removed).

## Broadcasts (every client)
`SessionUpdated(info)`, `SessionRemoved(id)`, `ShuttingDown`. If a client lags on the event channel it gets a full `SessionList` instead.

**Ordering guarantee:** no `SessionUpdated(id)` ever follows `SessionRemoved(id)`. Both are emitted only through `Session::publish`/`remove`, under the session's meta lock. See [[ADR-0008 Session events published under the session lock]].

## Client usage notes
- `read_frame` is **not cancel-safe**. Keep it in a dedicated task and never put it in `select!`.
- One-shot CLI commands use `Client::recv_until(pick)`. It skips unrelated broadcasts and converts `Error` into `ClientError::Daemon`.
