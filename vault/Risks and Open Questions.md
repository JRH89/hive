---
tags: [risks]
---
# Risks and Open Questions

## Open questions
- ~~Phase 2 switching model~~: resolved, re-attach. See [[ADR-0007 Switching re-attaches]].
- **Activity indicator.** Background sessions show no "new output" signal. It would need a debounced daemon broadcast, since clients don't mirror background sessions.
- **Directory for TUI-launched sessions.** The picker uses `hive`'s cwd. Phase 4 workspaces should supply it.
- **Scrollback UI.** The daemon has `scrollback_lines` of history in its emulator, but the snapshot covers only the visible screen. Options: a `Scrollback{offset}` request that returns rendered rows, or include scrollback in the snapshot. Not decided in Phases 2 or 3 (not an acceptance criterion in either).
- **Shift+Enter.** Legacy key encoding can't distinguish it. Claude Code users rely on it for newlines. This would need the kitty keyboard protocol on the outer terminal plus a decision on what to send the agent. Alt+Enter (`ESC CR`) works today.
- **Mouse.** Not captured, so native selection works. Forwarding mouse to agents that enable mouse modes needs capture plus encoding (SGR 1006). Decide on a toggle.
- **License.** Not chosen yet. The `LICENSE` file and the `license` field in `Cargo.toml` are deliberately absent until the owner picks one.

## Risks
- **Escaped grandchildren:** a process that leaves the session's process group and keeps the PTY slave open keeps the reader thread (and the `Session` memory) alive after kill.
- **Daemon CPU under endless output:** an infinite producer (`yes`) keeps `hived` at about one core in debug builds, because every byte goes through the authoritative emulator. Other sessions and the UI stay responsive (measured). Not a concern for real agents, but worth a release-build benchmark in Phase 6.
- **Slow kill of interactive shells:** bash ignores SIGTERM, so `KillSession` on a shell waits `kill_timeout_secs` (3 s) before SIGKILL. Sending SIGHUP alongside SIGTERM would end it immediately. Agents themselves exit on TERM.
- **Lock fd inherited across fork:** a PTY child holds a copy of every daemon fd between `fork` and `exec` (CLOEXEC closes it at exec). Only visible when several daemons share a process (tests); the harness retries `AlreadyRunning`.
- **3 OS threads per session.** Fine for tens of sessions. Revisit with nonblocking PTY fds plus `AsyncFd` if that ever matters.
- **Environment snapshot:** the session gets the creating client's env (needed for PATH and API keys). Secrets travel over the local socket (user-only perms, peer uid check). They are never logged; keep it that way.
- **Zombie `hived`:** the `hive` process that auto-starts the daemon doesn't wait on it. The zombie lingers only until that `hive` exits, because hived is reparented afterwards. Harmless, but a double-fork would be cleaner.
- **vt100 fidelity:** an untested agent may use sequences vt100 doesn't implement. Watch for rendering bugs in Gemini and OpenCode.
