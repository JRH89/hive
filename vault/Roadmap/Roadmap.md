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

## Phase 4 — Workspaces and Git ✅
[[Phase 4 - Workspaces and Git]]
- [x] Workspace creation, naming, removal; remembered across restarts ([[ADR-0011 Daemon-owned workspaces grouped by repository]])
- [x] Repository discovery and session grouping (linked worktrees join their repository)
- [x] Git branch display, refreshed while sessions run
- [x] Optional worktree creation and worktree-aware launching ([[ADR-0012 Worktrees created by the daemon, never deleted dirty]])
- [x] Safe cleanup: never removes dirty or in-use worktrees without force; branches kept
## Phase 5 — Product Experience and Branding ✅
[[Phase 5 - Product Experience and Branding]]
- [x] Final theme, theme support (4 themes, depth fallback, overrides) ([[ADR-0013 Themes degrade by colour depth and glyph set]])
- [x] Hexagonal brand mark, branded startup and empty states
- [x] Command palette, session search, help screen
- [x] Configurable keybindings ([[ADR-0014 Configurable keymap of named commands]])
- [x] Custom agent definitions documented (`examples/config.toml`)
- [x] README branding, terminal screenshots (text captures), CLI help examples
## Phase 6 — Testing and Distribution ✅
[[Phase 6 - Testing and Distribution]]
- [x] Unit, IPC, PTY lifecycle, detach/reattach, stress and resize tests; CLI end-to-end tests
- [x] CI pipeline (not yet run on GitHub)
- [x] Static Linux release binaries, draft-release workflow, checksum-verifying installer ([[ADR-0015 Static musl binaries via draft GitHub releases]])
- [x] Installation instructions, user guide, troubleshooting guide, `hive doctor`
- [x] macOS/Windows investigated (macOS type-checks; Windows needs a port)
## Phase 7 — herdr parity (in progress)
[[Phase 7 - herdr parity]]
- [x] Agent discovery: 27-agent catalog, `hive agents`, picker shows installed agents ([[ADR-0016 Agent detection from process and herdr manifests]])
- [x] Agent detection from the foreground process (incl. agents started from a shell)
- [x] Status (working/blocked/idle/done) from herdr manifests; self-report via `hive report-agent`
- [x] Sidebar status glyphs, workspace rollup, bell on done/blocked
- [x] herdr-style controls (prefix-free chords, tmux-like defaults) and mouse ([[ADR-0017 herdr-style keys, global chords and mouse]])
- [ ] Split panes and tabs (multi-attach)
- [ ] truss reports to Hive (needs the user's OK: their repo)
