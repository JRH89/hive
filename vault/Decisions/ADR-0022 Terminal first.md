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
