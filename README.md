```text
◇ HIVE
One terminal. Many agents.
```

A lightweight, Rust-powered terminal multiplexer for AI coding agents.

> **Status: early development (Phase 4 of 6).** Several agents side by side, grouped by repository, optionally each in its own git worktree. Detach, reattach and automatic reconnection. See [Roadmap](#roadmap).

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
- Sessions are grouped into workspaces by repository, with the current git branch shown live
- Optionally run an agent in its own git worktree (`hive new codex -w`), so agents don't overwrite each other. Hive warns when two agents share a directory
- Worktree cleanup never discards uncommitted changes without `--force` and confirmation, and never deletes branches

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
| `hive new codex -w` / `-b <branch>` | Launch in a new git worktree (default branch `hive/<agent>-<id>`) |
| `hive list` | List sessions |
| `hive attach <id>` | Attach to a session |
| `hive kill <id>` | Terminate a session |
| `hive status` | Daemon and session summary |
| `hive workspace list\|add <dir>\|rename\|rm` | Manage workspaces |
| `hive worktree list\|rm <path> [--force]` | List or remove worktrees (refuses dirty ones without `--force`) |
| `hive daemon start\|stop\|status` | Manage `hived` (`stop` terminates all sessions) |

## Keys
The leader is **Ctrl+Space**. Everything else goes to the agent.

| Keys | Action |
|---|---|
| `Ctrl+Space j` / `k` | Next / previous session |
| `Ctrl+Space 1`–`9` | Jump to session |
| `Ctrl+Space s` | Focus the sidebar (`↑↓` select, `enter` open, `esc` back) |
| `Ctrl+Space n` | New session (pick an agent; `w` toggles its own worktree) |
| `Ctrl+Space w` | Workspaces (open, add, rename, remove) |
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

[worktrees]
# dir = "~/worktrees"  # default: $XDG_DATA_HOME/hive/worktrees

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
4. ✅ Workspaces and git worktrees
5. Polish: command palette, themes, configurable keys
6. Testing and distribution

## License
Not yet chosen.
