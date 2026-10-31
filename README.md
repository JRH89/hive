```text
◇ HIVE
One terminal. Many agents.
```

A lightweight, Rust-powered terminal multiplexer for AI coding agents.

> **Status: early development (Phase 1 of 6).** One agent session at a time, with detach and reattach. Multi-session sidebar, workspaces and git worktrees are planned and **not implemented yet**. See [Roadmap](#roadmap).

## What works today
- Launch Claude Code, Codex, Gemini CLI, OpenCode, a shell, or any configured command in a real PTY
- Full terminal emulation: colours, cursor, alternate screen, wide characters, bracketed paste
- Agents run inside a background daemon (`hived`). Detach, close the terminal, or let the UI crash, and they keep running
- Reattach and get the exact current screen back
- Resizing follows your terminal

## Build
Requires Rust ≥ 1.89 on Linux.

```bash
cargo build --release
# binaries: target/release/hive and target/release/hived (keep them side by side)
```

## Quick start
```bash
hive new claude        # launch Claude Code and attach
# Ctrl+Space d         # detach; Claude keeps running
hive list              # see sessions
hive                   # reattach to the most recent session
```

## Commands
| Command | |
|---|---|
| `hive` | Open the UI (attaches to the newest live session) |
| `hive new [agent] [-l label] [-C dir] [-d] [-- args]` | Launch an agent |
| `hive list` | List sessions |
| `hive attach <id>` | Attach to a session |
| `hive kill <id>` | Terminate a session |
| `hive status` | Daemon and session summary |
| `hive daemon start\|stop\|status` | Manage `hived` (`stop` terminates all sessions) |

## Keys
The leader is **Ctrl+Space**. Everything else goes to the agent.

| Keys | Action |
|---|---|
| `Ctrl+Space d` | Detach |
| `Ctrl+Space n` | New session (default agent) |
| `Ctrl+Space x` | Terminate session (asks to confirm) |
| `Ctrl+Space a` | Take input control (when another client attached) |
| `Ctrl+Space Ctrl+Space` | Send a literal Ctrl+Space |
| `Ctrl+Space ?` | Help |

## Configuration
`~/.config/hive/config.toml`. Every key is optional.

```toml
[general]
default_agent = "claude"

[daemon]
auto_start = true
kill_timeout_secs = 3

[agents.aider]          # add or override agents
command = "aider"
args = ["--dark-mode"]
```

## Architecture
`hived` owns every agent process, PTY and the authoritative terminal state. `hive` is a client that talks to it over a per-user Unix socket. See `vault/Architecture/Overview.md`.

## Roadmap
1. ✅ Daemon-first foundation: one session, detach/reattach
2. Multiplexing: sidebar, switching, several agents at once
3. Robust persistence: auto-reconnect, multi-client polish
4. Workspaces and git worktrees
5. Polish: command palette, themes, configurable keys
6. Testing and distribution

## License
Not yet chosen.
