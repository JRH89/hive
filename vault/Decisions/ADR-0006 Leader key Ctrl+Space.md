---
tags: [adr]
status: accepted
date: 2026-10-30
---
# ADR-0006: Leader key is Ctrl+Space

**Constraints:** It must not collide with agent shortcuts, and every terminal must report it the same way (spec §9).

**Rejected:**
- `Ctrl+B` (tmux): Claude Code uses it to background tasks, and it breaks nested tmux.
- `Ctrl+A` (screen): readline start-of-line.
- `Ctrl+G`: Claude Code opens the editor.
- `Ctrl+]`: crossterm reports it inconsistently (as `Ctrl+5`).

**Decision:** `Ctrl+Space` (NUL on the wire). Agents almost never need it, and crossterm consistently reports it as `Char(' ') + CONTROL`. Pressing it twice sends a literal NUL.

**Current bindings (after the leader, as of Phase 2):** `j`/`k` next/prev session · `1`–`9` jump · `s` focus sidebar · `n` new (agent picker) · `r` rename · `x` kill (asks y/n) · `a` take control · `d`/`q` detach · `?` help.

Sidebar focus (also active whenever there is no live session to type into): `↑↓`/`j k` select · `enter` open · `n` `r` `x` · `1`–`9` · `esc` back to the terminal · `q` detach.

**Next:** make it configurable via `[keys]` in config (Phase 5).
