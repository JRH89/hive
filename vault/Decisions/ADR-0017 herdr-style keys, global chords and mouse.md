---
status: accepted
date: 2026-10-30
---
# ADR-0017: herdr-style keys, global chords and mouse

Amends [[ADR-0014 Configurable keymap of named commands]].

## Context
The user asked for controls closer to herdr (whose defaults mirror tmux). Hive's leader defaults (`n` new, `p` palette, `j/k` switch) differed, there were no prefix-free chords, and the mouse was ignored (`mouse_support` was inert).

## Decision
| Area | Choice |
|---|---|
| Leader defaults | `c` new, `C` new in worktree, `n`/`p` next/prev (plus `j`/`k`), `:`/`P` palette, `b` toggle sidebar; others unchanged |
| Global chords | `[keys.global]` (command → keys, Ctrl/Alt or F-key required) plus `[keys] jump_modifier` (`alt` default) for Alt+1..9. Defaults Alt+←/→ sessions, Alt+↑/↓ workspaces, like herdr |
| New commands | `next_workspace`, `prev_workspace`, `toggle_sidebar` |
| Mouse | Captured when `[terminal] mouse_support` (default true). Sidebar clicks/wheel via row positions recorded by render (`App::sidebar_hits`). Panel events forwarded with xterm encoding (X10/UTF-8/SGR) only when the agent enabled a mouse mode; otherwise wheel → arrows on the alternate screen |

## Consequences
- Global chords take those keys from agents; users can free them with `[]`.
- Mouse capture disables native selection without Shift (documented).
- v0.1.0 users' `n`/`p` muscle memory changes (few users; matches the request).
