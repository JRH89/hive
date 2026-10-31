# Hive user guide

Hive runs several interactive AI coding agents (Claude Code, Codex, Gemini CLI, OpenCode, a shell, or anything you configure) in one terminal, and keeps them running when you close it.

## Concepts

| Term | Meaning |
|---|---|
| **Daemon** (`hived`) | Background process that owns every agent, its terminal (PTY) and screen state. Started automatically by `hive`. Agents live as long as the daemon does |
| **Session** | One running agent. Has an id (never reused), a label, a state (running, exited, failed, lost) and a working directory |
| **Workspace** | A group of sessions, normally one git repository. Assigned automatically from the session's directory |
| **Worktree** | Optional separate git checkout for one agent, so two agents in the same repository don't edit the same files |
| **Controller** | The one window whose keystrokes reach a session. Other windows watching it are observers |

Hive never answers an agent's permission prompts and never reads its output to guess what it's doing.

## Everyday use

```bash
hive                    # open the interface; reattaches to the newest session
hive new claude         # start Claude Code in the current directory and attach
hive new codex -w       # start Codex in its own worktree (branch hive/codex-<id>)
hive new shell -d -l db # start a shell in the background, labelled "db"
hive list               # sessions with workspace, branch and directory
hive attach 3           # open session 3
hive kill 3             # terminate session 3 (asks the agent to quit, then forces it)
```

To leave the interface, press `Ctrl+Space d`. Agents keep running. Run `hive` again to come back to exactly the same screens.

Arguments after `--` go to the agent: `hive new claude -- --model opus`.

## The interface

```text
 ◇ HIVE  CLAUDE — api  ~/src/api (main)                  One terminal. Many agents.
 WORKSPACES              │
 ▾ api                   │   (the attached agent's terminal)
▌● api                 1 │
   main · claude         │
 ● review              2 │
   wt hive/codex-2 · co… │
 ● #1 api · api · main · running · 2 active        Ctrl+Space leader · then p commands
```

| Mark | Meaning |
|---|---|
| `●` / `*` | running |
| `○` / `o` | exited |
| `✕` / `x` | failed to start or crashed |
| `◌` / `?` | lost: it was running when the daemon stopped unexpectedly. Press enter on it to relaunch |
| `▌` / `>` | the session you are attached to |
| `wt` | runs in its own worktree |
| `!` | shares its directory with another running agent |

The second symbol in each row is the ASCII fallback (see [Themes](#themes)).

Everything you type goes to the agent, except the **leader** key (`Ctrl+Space`) followed by a command key:

| Keys | Action |
|---|---|
| `p` | Command palette: search every command, session and workspace |
| `/` | Search sessions by name, agent, workspace or branch |
| `n` | New session (pick an agent; `w` in the picker toggles its own worktree) |
| `j` / `k` | Next / previous session |
| `1`–`9` | Jump to a session |
| `s` | Focus the sidebar (`↑↓`, `enter`, `n`, `x`, `esc`) |
| `w` | Workspaces |
| `r` | Rename session |
| `x` | Terminate session (asks first; offers to remove a clean worktree) |
| `a` | Take input control from another window |
| `d` | Detach |
| `?` | Help, listing your current bindings |
| `Ctrl+Space` | Send a literal Ctrl+Space to the agent |

## Several windows

You can open the same session in two terminals. The first one controls it; the second watches, and the status bar says so. `Ctrl+Space a` in the watching window takes control. The terminal size always follows the controlling window.

## Workspaces and worktrees

Sessions are grouped by the repository their directory belongs to. Workspaces are remembered across daemon restarts. Manage them with `Ctrl+Space w` or `hive workspace list|add|rename|rm`.

`hive new <agent> -w` creates a worktree under `$XDG_DATA_HOME/hive/worktrees` (change it with `[worktrees] dir`) on a new branch. `-b <branch>` picks the branch name instead.

Safety rules:
- A worktree with uncommitted changes is never removed without `--force` and a confirmation.
- A worktree a session is still running in is never removed.
- Branches are never deleted.

`hive worktree list` shows each worktree's state, and `hive worktree rm <path>` removes one.

## Configuration

The config file is `~/.config/hive/config.toml`, or `$HIVE_CONFIG`. Every key is optional. [`examples/config.toml`](../examples/config.toml) documents them all. Run `hive doctor` after editing: it validates the file without opening the interface.

### Custom agents

```toml
[agents.aider]
command = "aider"          # run directly, never through a shell
args = ["--dark-mode"]
env = { AIDER_AUTO_COMMITS = "false" }

[agents.claude]            # override a built-in
command = "claude"
args = ["--model", "opus"]
```

Agents start with the environment of the `hive` command that launched them, plus `env`. Hive never writes environment variables to disk.

### Keys

```toml
[keys]
leader = "ctrl+a"          # needs Ctrl, Alt or an F-key

[keys.bind]
new_session = "c"
add_workspace = ["W", "f2"]
kill_session = ""          # unbind
```

Command names are listed in `examples/config.toml`. Unbound commands remain available in the palette. Digits `1`–`9` and the leader itself are reserved.

### Themes

`[general] theme` is one of `hive-dark` (default), `hive-light`, `ansi` (your terminal's own 16 colours) or `mono` (no colour).

Hive detects what your terminal supports:

| Setting | Detection | Override |
|---|---|---|
| Colours | `NO_COLOR` or `TERM=dumb` → none; `COLORTERM=truecolor` → truecolor; `TERM=*256color` → 256; otherwise 16 | `[theme] colors = "truecolor" \| "256" \| "16" \| "none"` |
| Glyphs | ASCII on the Linux console or a non-UTF-8 locale | `[theme] glyphs = "unicode" \| "ascii"` |

Nerd Fonts are never required. Individual colours can be overridden with `#rrggbb` values (`accent`, `text`, `muted`, …). An agent's own colours are shown unchanged.

## Upgrading

Install the new binaries, then restart the daemon:

```bash
hive daemon stop    # terminates all running sessions
hive                # starts the new daemon
```

If the client and a running daemon speak different protocol versions, `hive` says so instead of misbehaving.

## Uninstalling

```bash
hive daemon stop
rm ~/.local/bin/hive ~/.local/bin/hived
rm -rf ~/.config/hive ~/.local/state/hive    # config, logs, session metadata
# Worktrees under ~/.local/share/hive/worktrees may hold your work: check them first.
```
