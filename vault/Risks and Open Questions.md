---
tags: [risks]
---
# Risks and Open Questions

## Open questions
- **Phase 2 switching model.** Option A: one attachment per connection, and switching re-attaches (simple, but the snapshot is re-sent on every switch). Option B: subscribe to several sessions per connection and tag `Output` by session (it already carries `session`). Leaning towards B for instant switching, with a cap on how many sessions are mirrored.
- **Scrollback UI.** The daemon has `scrollback_lines` of history in its emulator, but the snapshot covers only the visible screen. Options: a `Scrollback{offset}` request that returns rendered rows, or include scrollback in the snapshot. Decide in Phase 2.
- **Shift+Enter.** Legacy key encoding can't distinguish it. Claude Code users rely on it for newlines. This would need the kitty keyboard protocol on the outer terminal plus a decision on what to send the agent. Alt+Enter (`ESC CR`) works today.
- **Mouse.** Not captured, so native selection works. Forwarding mouse to agents that enable mouse modes needs capture plus encoding (SGR 1006). Decide on a toggle.
- **License.** Not chosen yet. The `LICENSE` file and the `license` field in `Cargo.toml` are deliberately absent until the owner picks one.

## Risks
- **Escaped grandchildren:** a process that leaves the session's process group and keeps the PTY slave open keeps the reader thread (and the `Session` memory) alive after kill.
- **3 OS threads per session.** Fine for tens of sessions. Revisit with nonblocking PTY fds plus `AsyncFd` if that ever matters.
- **Environment snapshot:** the session gets the creating client's env (needed for PATH and API keys). Secrets travel over the local socket (user-only perms, peer uid check). They are never logged; keep it that way.
- **Zombie `hived`:** the `hive` process that auto-starts the daemon doesn't wait on it. The zombie lingers only until that `hive` exits, because hived is reparented afterwards. Harmless, but a double-fork would be cleaner.
- **vt100 fidelity:** an untested agent may use sequences vt100 doesn't implement. Watch for rendering bugs in Gemini and OpenCode.
