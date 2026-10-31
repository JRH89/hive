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

Hive never answers an agent's permission prompts. It shows an agent's status only from per-agent screen rules or the agent's own report, never a guess.

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
 SPACES                  │
                         │   (the focused agent's terminal)
 ● api                 2 │
   main                  │
 · docs                1 │
   main                  │
                         │
 + new            ● menu │
─────────────────────────│
 AGENTS                  │
                         │
▌● api                 1 │
   main · claude         │
 ● review              2 │
   wt hive/codex-2 · co… │
 ● #1 api · api · main · running · 2 active        Ctrl+Space leader · then : commands
```

The sidebar lists your **spaces** (one per repository, with its branch and agent count) and, below them, the **agents** of the selected space. Click a space to switch to it, an agent to open it, **+ new** to start an agent in the selected space, and **menu** for the command palette.

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

Everything you type goes to the agent, except the **leader** key (`Ctrl+Space`) followed by a command key. The defaults follow herdr and tmux:

| Keys | Action |
|---|---|
| `:` or `P` | Command palette: search every command, session and workspace |
| `/` | Search sessions by name, agent, workspace or branch |
| `c` | New session (pick an agent; `w` in the picker toggles its own worktree) |
| `C` | New session in its own git worktree |
| `n` / `p` (or `j` / `k`) | Next / previous session |
| `1`–`9` | Jump to a session |
| `s` | Focus the sidebar (`↑↓`, `enter`, `n`, `x`, `esc`) |
| `b` | Show or hide the sidebar |
| `v` or `%` / `-` or `"` | Split: new session to the right / below |
| `o` | Focus the next pane (or click it) |
| `z` | Zoom the focused pane; again to restore |
| `X` | Close the pane (the session keeps running) |
| `t` | New tab with a terminal (or click `+` in the tab bar) |
| `]` / `[` | Next / previous tab |
| `w` | Workspaces |
| `r` | Rename session |
| `x` | Terminate session (asks first; offers to remove a clean worktree) |
| `a` | Take input control from another window |
| `d` | Detach |
| `?` | Help, listing your current bindings |
| `Ctrl+Space` | Send a literal Ctrl+Space to the agent |

A few chords work **without the leader**, even while typing to an agent:

| Keys | Action |
|---|---|
| `Alt+↑` / `Alt+↓` | Previous / next space |
| `Alt+←` / `Alt+→` | Previous / next tab; with one tab, the previous / next agent in the space |
| `Alt+1`–`Alt+9` | Jump to an agent of the space |

If an agent needs one of these, rebind or unbind it under `[keys.global]` (see [Keys](#keys)).

### Panes and tabs

Split the panel to watch several agents at once. Each pane is its own terminal at its own size, with a title row showing the session and its status. Keys go to the focused pane. Switching to a session that is already in a pane focuses that pane (and its tab); otherwise the session replaces the focused pane. The header's tab bar (`1:api  2:shell  +`) is clickable: pick a tab, or `+` for a new terminal tab in the current directory. Hidden tabs cost nothing: Hive only mirrors what is on screen. The layout lasts until you quit `hive`; sessions keep running either way.

### Notes (Obsidian, optional)

If your repository holds an [Obsidian](https://obsidian.md) vault, Hive can open it beside your agents. It is off by default and adds nothing to the interface until you turn it on:

```toml
[integrations.obsidian]
enabled = true
agent = ""          # "" opens $VISUAL/$EDITOR; or any agent name, e.g. "claude"
open_in = "tab"     # or "split"
```

`Ctrl+Space N` (or "notes" in the palette) then opens the vault (the nearest folder with `.obsidian/` at or above the session's directory, or one level below the repository root) as a session labelled `notes · <vault>`. Asking again goes back to it.

### More integrations (optional)

All off by default; each adds one command (and key) when enabled:

```toml
[integrations.review]         # Ctrl+Space G: lazygit, tig or git diff beside the agent
enabled = true
command = []                  # e.g. ["lazygit"]; [] picks the first installed
open_in = "split"             # or "tab"

[integrations.github]         # Ctrl+Space H: `gh pr checks --watch` for the branch
enabled = true

[integrations.tasks]          # Ctrl+Space T: pick a just/npm/make/cargo task to run beside the agent
enabled = true

[integrations.desktop_notify] # a desktop notification when an agent finishes or needs input
enabled = true
command = ["notify-send", "--app-name=Hive"]

[integrations.obsidian]
journal = true                # append "claude finished in #3 api (main)" to today's note
journal_dir = "Daily"         # folder of daily notes in the vault
```

Each opens as an ordinary session, so it shows in the sidebar and can be closed like any other. Asking again goes back to the one already open.

### Mouse

Everything in the sidebar and the tab bar is clickable: spaces, agents, **+ new**, **menu** and tabs. Click a pane to focus it. The wheel scrolls the sidebar. In the terminal panel, clicks and the wheel go to agents that use the mouse; full-screen apps without mouse support scroll with the wheel. Hold `Shift` to select text with your terminal as usual. Turn the mouse off with `[terminal] mouse_support = false`.

### Agent status

Hive recognises the agent running in each session, even one you started from a shell, and shows what it is doing:

| Symbol | Meaning |
|---|---|
| `◐` / `~` | working |
| `◆` / `!` | needs input (a permission prompt or question) |
| `✓` / `+` | done: finished while you were looking elsewhere (clears when you open it) |
| `●` / `*` | idle, or a session without a recognised agent |

A workspace header shows its most urgent session. The terminal bell rings when a session you aren't viewing needs input or finishes (`[notifications] bell = false` turns it off). Status comes from per-agent screen rules shared with [herdr](https://herdr.dev); agents without rules show no status rather than a guess. `hive agents` lists the agents Hive knows and which are installed.

Agents can report their own status: inside a session, `"$HIVE_BIN_PATH" report-agent --agent NAME --state idle|working|blocked [--message TEXT]`, and `release-agent --agent NAME` when done.

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

jump_modifier = "alt"      # Alt+1-9 jump to sessions; "" turns it off

[keys.bind]                # after the leader
new_session = "n"
add_workspace = ["W", "f2"]
kill_session = ""          # unbind

[keys.global]              # without the leader; needs Ctrl or Alt
next_tab = "ctrl+alt+right"
prev_workspace = []        # free Alt+Up for the agent
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

```bash
hive update         # installs the latest release next to the running hive
hive daemon stop    # the old daemon keeps running until stopped; this ends running sessions
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

On Windows: `hive daemon stop`, then delete `%LOCALAPPDATA%\hive` (binaries, runtime files) and the config and state folders `hive doctor` lists. Check worktrees first.
