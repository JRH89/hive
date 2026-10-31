---
status: accepted
date: 2026-10-30
---
# ADR-0014: Configurable keymap of named commands

## Context
Spec §16 Phase 5 asks for a command palette, configurable keybindings and a help screen. Leader actions were a hard-coded `match` on characters in `app.rs`, so help text, palette and keys could drift apart.

## Decision
- Every leader action is a `keymap::Command` with a stable config name (`new_session`, `kill_session`, …) and a title. `App::run(cmd)` is the single dispatcher.
- `[keys] leader` and `[keys.bind] <command> = "key" | ["k1","k2"] | ""` (empty unbinds) overlay the defaults.
- Rejected at startup: unknown commands, unparsable keys, a leader without Ctrl/Alt/F-key (it would swallow typing, see [[ADR-0006 Leader key Ctrl+Space]]), one key bound to two commands, and binding digits 1–9 or the leader itself (reserved for jump and send-literal-leader).
- The palette (Leader `p`) lists every command (bound or not) with its key, then sessions, then workspaces, ranked by a small fuzzy scorer. Leader `/` is the same palette restricted to sessions.
- Help is generated from the live keymap.

## Consequences
- Adding a feature means adding a `Command` variant. It automatically appears in the palette and help and becomes bindable.
- Inside the TUI an invalid keymap falls back to defaults with an error in the status bar. The `hive` CLI rejects it before opening.
- Sidebar-mode keys (`n`, `x`, `enter`, `esc`) and picker keys stay fixed for now.
