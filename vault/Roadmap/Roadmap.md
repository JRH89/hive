---
tags: [roadmap]
---
# Roadmap

Full detail is in `AGENTS.md` §16. **Don't start a phase until the previous one's acceptance criteria are verified.**

## Phase 1 — Daemon-First Foundation ✅
[[Phase 1 - Foundation]]

## Phase 2 — Multiplexing
Goal: Claude Code and Codex running simultaneously. Switching doesn't interrupt either, and background output doesn't freeze the UI.
- [ ] Sidebar (sessions list, state dots, selection highlight)
- [ ] Switch sessions inside the TUI (re-`Attach` per switch, or subscribe to several sessions; decide this, see [[Risks and Open Questions]])
- [ ] Session labels and rename (`RenameSession` message)
- [ ] Create, terminate and select sessions from the UI
- [ ] Basic branding pass on the sidebar
- [ ] Integration test: two noisy sessions, UI stays responsive

## Phase 3 — Robust Persistence
- [ ] TUI auto-reconnects when the daemon connection drops
- [ ] Persist session metadata (labels, cwd) for display after a daemon restart, without implying process recovery
- [ ] Multi-client behaviour polish (observer UX)

## Phase 4 — Workspaces and Git
## Phase 5 — Product Experience and Branding
## Phase 6 — Testing and Distribution
