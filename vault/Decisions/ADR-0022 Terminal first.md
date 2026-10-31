---
status: accepted
date: 2026-10-31
---
# ADR-0022 Terminal first

## Context
"New" and splits opened an agent picker and launched the agent in the space root. The user found it unlike herdr, where every new pane is a terminal and agents are started by typing them.

## Decision
| Action | Opens |
|---|---|
| leader `c`, sidebar "new", sidebar `n` | terminal, focused pane |
| leader `v`/`-` | terminal, split |
| leader `t`, tab row `+` | terminal, new tab |
| leader `A` (`new_agent`) | agent picker (old behaviour) |

Terminals start in the focused session's cwd if it's in the selected space, else the space root. The tab row is `1  2  +` above the panes; the sidebar runs full height; no brand header.

## Consequences
- Agents are found by process detection ([[ADR-0016 Agent detection from process and herdr manifests]]); a shell running `claude` shows as claude.
- Shared-directory warnings ignore plain terminals.
- `[keys.bind] new_session` keeps its name but now means "new terminal".

## Amendment (2026-10-31): spaces follow `cd`
- Sessions' `cwd`, workspace and repo are mutable (session `Meta`). Every 2 s (`Daemon::refresh_places`, with branch refresh) the daemon reads the process's cwd (`/proc/<pid>/cwd`; macOS `lsof -Fn`; Windows: not tracked). In a repo → that repo's workspace (exact main root; created if new); else the deepest workspace containing it; else unchanged.
- TUI: every running session has a tab (`adopt_sessions`, except ones whose tab was closed here); "+ new" opens a terminal tab; the view follows the focused session's move; SPACES hides spaces with nothing open (`App::spaces`).

## Amendment (2026-10-31): worktree terminals
- Agents don't isolate themselves; isolation is a worktree. Leader `C` = `Command::NewWorktreeTerminal`: a terminal tab in a new worktree (`NewWorktree { branch: None }` → `hive/shell-<id>`), same space (the worktree's repo main root). The agent-in-worktree picker (`NewWorktreeSession`) keeps working from the palette, unbound.

## Amendment (2026-10-31): a space for every folder
- Outside git, a moved (or new) session joins the deepest workspace containing its cwd, else a new workspace rooted at the cwd. The home directory's workspace matches only home itself (`containing()` in `hive-daemon`), so `cd ~/Work` makes a `Work` space instead of staying in `~`'s. User report: two terminals in `~/Work` (a folder of repos) both stayed in the home space.

## Amendment (2026-10-31): "+ new" opens a new space
- Sidebar "+ new" (and leader `S`, `Command::NewSpace`) = `CreateSession { new_space: true }` with cwd = home (protocol v7). The daemon puts it in a fresh workspace named "new space", tracked in `Daemon::scratch`: never chosen for a directory (`containing`, repo lookup), dropped once empty (`drop_empty_scratch`, every place refresh). `cd` moves the session to the real space. The tab row's `+` / leader `t` stays "new tab in this space". Not persisted: after a daemon restart a leftover scratch space is an ordinary one.
- `hive` against an empty daemon opens a terminal (only-lost sessions still focus the sidebar for relaunch).
