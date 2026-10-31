<p align="center"><img src="assets/branding/hive-mark.svg" width="96" alt="Hive mark"></p>

```text
   ____
  /    \
 /  >_  \    HIVE
 \      /    One terminal. Many agents.
  \____/
```

A lightweight, Rust-powered terminal multiplexer for AI coding agents.

> **Status: pre-release.** All six roadmap phases are implemented on Linux: several agents side by side, grouped by repository, optionally each in its own git worktree; detach, reattach and automatic reconnection; a command palette, themes and configurable keys. Version 0.1.0 ships static Linux binaries. See [Roadmap](#roadmap).

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

## A look
Captured from a real session with `tmux capture-pane` (paths and prompt trimmed), four shells in two repositories:

```text
 ◇ HIVE  SHELL — frontend  ~/src/webapp (main)                           One terminal. Many agents.
 WORKSPACES              │[you@host webapp]$
                         │
 ▾ webapp                │
▌● frontend            1 │
   main · shell          │
 ● tests       ╭ Commands ──────────────────────────────────────────────────────────╮
   wt hive/shel│   New session                                                    n │
 ▾ api         │   New session in its own worktree                                  │
 ● billing     │   Switch workspace                                               w │
   main · shell│   Add workspace                                                    │
 ● migrations  │   Rename workspace                                                 │
   wt db/migrat│   Next session                                                   j │
               │   Previous session                                               k │
               ╰────────────────────────────────────────────────────────────────────╯
 Command: ▏  ↑↓ choose · enter run · esc close
```

Without Unicode or colour (`TERM=linux`, `NO_COLOR=1`) the same UI falls back to plain ASCII:

```text
 <> HIVE  SHELL - migrations  ~/src/api (db/migrate)
 WORKSPACES              |[you@host db-migrate]$
 - webapp                |
 * frontend            1 |
   main . shell          |
 * tests               2 |
   wt hive/shell-2 . sh~ |
 - api                   |
 * billing             3 |
>* migrations          4 |
   wt db/migrate . shell |
 * #4 migrations . api . db/migrate . running . 4 active        Ctrl+Space leader . then : commands
```

## Install
Linux (x86_64 and aarch64) is supported. macOS and Windows are not yet.

**Prebuilt binaries** ([latest release](https://github.com/jrh89/hive/releases/latest)):

```bash
curl -fsSL https://github.com/jrh89/hive/releases/latest/download/install.sh | sh
```

The script downloads static binaries, verifies their SHA-256 checksum and installs `hive` and `hived` to `~/.local/bin`. Set `HIVE_INSTALL_DIR` to change the location or `HIVE_VERSION=v0.1.0` to pin a version.

**From source** (Rust ≥ 1.89):

```bash
git clone https://github.com/jrh89/hive && cd hive
cargo build --release
install -m 0755 target/release/hive target/release/hived ~/.local/bin/
```

Keep `hive` and `hived` in the same directory. Then run `hive doctor` to check the setup.

## Quick start
```bash
hive new claude        # launch Claude Code and attach
# Ctrl+Space d         # detach; Claude keeps running
hive list              # see sessions
hive                   # reattach to the most recent session
```

## Supported agents
| Agent | Name in Hive | Command |
|---|---|---|
| Claude Code | `claude` | `claude` |
| Codex | `codex` | `codex` |
| Gemini CLI | `gemini` | `gemini` |
| OpenCode | `opencode` | `opencode` |
| Your shell | `shell` | `$SHELL` |
| Anything else | your choice | see [custom agents](docs/user-guide.md#custom-agents) |

Agents run unmodified in a real terminal. Hive doesn't parse their output or answer their prompts.

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
| `hive doctor` | Check config, daemon, terminal support and installed agents |
| `hive daemon start\|stop\|status` | Manage `hived` (`stop` terminates all sessions) |

## Keys
The leader is **Ctrl+Space**. Everything else goes to the agent.

| Keys | Action |
|---|---|
| `Ctrl+Space p` | Command palette: every command, session and workspace, fuzzy-searchable |
| `Ctrl+Space /` | Search sessions by name, agent, workspace or branch |
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
| `Ctrl+Space ?` | Help (generated from your actual bindings) |

Every key except the digits is configurable, and so is the leader (see below).

## Configuration
`~/.config/hive/config.toml`. Every key is optional; [`examples/config.toml`](examples/config.toml) documents all of them. Mistakes are reported before the UI opens.

```toml
[general]
default_agent = "claude"
theme = "hive-dark"     # hive-dark, hive-light, ansi (your terminal's palette), mono

[theme]
# colors = "auto"       # auto, truecolor, 256, 16, none (NO_COLOR is honoured)
# glyphs = "auto"       # auto, unicode, ascii (no Nerd Fonts needed either way)
# accent = "#F2B84B"    # override any palette colour

[keys]
leader = "ctrl+space"   # must use Ctrl, Alt or an F-key

[keys.bind]
new_session = "c"
add_workspace = ["W", "f2"]
kill_session = ""       # unbind

[daemon]
auto_start = true
kill_timeout_secs = 3

[worktrees]
# dir = "~/worktrees"  # default: $XDG_DATA_HOME/hive/worktrees

[agents.aider]          # add or override agents
command = "aider"
args = ["--dark-mode"]
```

## Documentation
- [User guide](docs/user-guide.md): concepts, interface, workspaces and worktrees, configuration, upgrading
- [Troubleshooting](docs/troubleshooting.md): `hive doctor`, logs, common problems
- [`examples/config.toml`](examples/config.toml): every configuration key

## Architecture
`hived` owns every agent process, PTY and the authoritative terminal state. `hive` is a client that talks to it over a per-user Unix socket. See `vault/Architecture/Overview.md`.

## Development
```bash
cargo build                                            # target/debug/hive and hived
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                                 # unit, PTY integration and CLI tests
scripts/smoke.sh target/debug                          # end-to-end check of built binaries
```

Integration tests run real PTYs with `/bin/sh` scripts against private daemons. No agent CLIs are needed. CI (`.github/workflows/ci.yml`) runs the above, an MSRV check, and static musl builds for x86_64 and aarch64. Pushing a `v*` tag builds release archives and creates a draft GitHub release.

## Roadmap
1. ✅ Daemon-first foundation: one session, detach/reattach
2. ✅ Multiplexing: sidebar, switching, several agents at once
3. ✅ Robust persistence: auto-reconnect, multi-client polish
4. ✅ Workspaces and git worktrees
5. ✅ Polish: command palette, themes, configurable keys
6. ✅ Testing and distribution: CI, static Linux binaries, docs

## License
[MIT](LICENSE)
