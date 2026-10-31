```text
◇ HIVE
One terminal. Many agents.
```

A lightweight, Rust-powered terminal multiplexer for AI coding agents.

> **Status: early development (Phase 3 of 6).** Several agents side by side with a session sidebar, detach and reattach, and automatic reconnection. Workspaces and git worktrees are planned and **not implemented yet**. See [Roadmap](#roadmap).

## What works today
- Launch Claude Code, Codex, Gemini CLI, OpenCode, a shell, or any configured command in a real PTY
- Full terminal emulation: colours, cursor, alternate screen, wide characters, bracketed paste
- Agents run inside a background daemon (`hived`). Detach, close the terminal, or let the UI crash, and they keep running
- Reattach and get the exact current screen back
- Run several agents at once and switch between them from a sidebar
- Rename and terminate sessions from the UI
- Resizing follows your terminal
- If `hived` restarts, the UI reconnects on its own and shows which sessions ended (`◌`); press enter on one to relaunch it
- Several terminals can view the same session; only one types at a time, and taking over is explicit (`Ctrl+Space a`)

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
| `Ctrl+Space j` / `k` | Next / previous session |
| `Ctrl+Space 1`–`9` | Jump to session |
| `Ctrl+Space s` | Focus the sidebar (`↑↓` select, `enter` open, `esc` back) |
| `Ctrl+Space n` | New session (pick an agent) |
| `Ctrl+Space r` | Rename session |
| `Ctrl+Space x` | Terminate session (asks to confirm) |
| `Ctrl+Space d` | Detach |
| `Ctrl+Space a` | Take input control (when another window controls the session) |
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
2. ✅ Multiplexing: sidebar, switching, several agents at once
3. ✅ Robust persistence: auto-reconnect, multi-client polish
4. Workspaces and git worktrees
5. Polish: command palette, themes, configurable keys
6. Testing and distribution

## License
Not yet chosen.
