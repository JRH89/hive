---
tags: [roadmap]
---
# Roadmap

Full detail is in `AGENTS.md` §16. **Don't start a phase until the previous one's acceptance criteria are verified.**

## Phase 1 — Daemon-First Foundation ✅
[[Phase 1 - Foundation]]

## Phase 2 — Multiplexing ✅
[[Phase 2 - Multiplexing]]
- [x] Sidebar (sessions list, state dots, selection highlight)
- [x] Switch sessions inside the TUI (re-attach, see [[ADR-0007 Switching re-attaches]])
- [x] Session labels and rename (`RenameSession`, protocol v2)
- [x] Create (agent picker), terminate and select sessions from the UI
- [x] Basic branding pass on the sidebar
- [x] Integration tests: switching keeps sessions running; a flood doesn't starve a quiet session

## Phase 3 — Robust Persistence ✅
[[Phase 3 - Robust Persistence]]
- [x] TUI auto-reconnects when the daemon connection drops (and auto-starts a new daemon)
- [x] Persist session metadata (labels, cwd) for display after a daemon restart, without implying process recovery ([[ADR-0009 Persist session metadata, mark lost sessions]])
- [x] Multi-client behaviour polish: observe unless forced, `ControlReleased` ([[ADR-0010 Attach observes unless control is free or forced]])
- [x] Stale socket cleanup and ids that never repeat across restarts

## Phase 4 — Workspaces and Git
## Phase 5 — Product Experience and Branding
## Phase 6 — Testing and Distribution
