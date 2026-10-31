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

## Phase 3 — Robust Persistence
- [ ] TUI auto-reconnects when the daemon connection drops
- [ ] Persist session metadata (labels, cwd) for display after a daemon restart, without implying process recovery
- [ ] Multi-client behaviour polish (observer UX)

## Phase 4 — Workspaces and Git
## Phase 5 — Product Experience and Branding
## Phase 6 — Testing and Distribution
