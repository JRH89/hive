---
status: accepted
date: 2026-10-31
---
# ADR-0021 Windows: named pipe with token auth

## Context
Unix uses a 0600 socket in a 0700 dir plus a peer-uid check. Windows has no `SO_PEERCRED`, and a named pipe's default DACL grants Everyone read access. The daemon must still own PTYs (ConPTY via portable-pty) and processes.

## Decision
| Piece | Unix | Windows |
|---|---|---|
| Endpoint | `$RUNTIME/hived.sock` | `\\.\pipe\hive-<fnv64(runtime dir)>` (isolated daemons get their own) |
| Auth | file mode + peer uid | `reject_remote_clients`; client sends a 64-hex token from `$RUNTIME/hived.token` first; daemon compares in constant time, drops others after 5 s |
| Runtime dir | `$XDG_RUNTIME_DIR/hive` or `/tmp/hive-<uid>` | `%LOCALAPPDATA%\hive\run` (user-private ACL) |
| Stop signals | SIGTERM/SIGINT | Ctrl+C/Ctrl+Break |
| Session kill | SIGTERM group, then SIGKILL | `taskkill /T`, then `/T /F` |
| Daemon spawn | `setsid` | `DETACHED_PROCESS \| CREATE_NEW_PROCESS_GROUP` |
| Cross-version stop | SIGTERM by pid | `taskkill /T /F` (sessions return as Lost) |
| Agent detection | foreground pgid + argv | none: the session pid stands in; status only via `hive report-agent` (ADR-0016's `Unknown`) |

Code: `hive_core::transport` (client), `hive-daemon/src/listener.rs` (server). `handle_client` is generic over the stream.

## Consequences
- Token is a second factor on top of the private runtime dir: a user who can't read the token can't talk to the daemon.
- Token comes from std's randomly keyed SipHash (no new dependency).
- Shell-script integration tests stay Unix-only; `tests/windows.rs` covers the pipe, token and a ConPTY `cmd.exe` session; `scripts/smoke.ps1` covers the CLI.
- No screen-based agent status on Windows until process inspection is ported.
