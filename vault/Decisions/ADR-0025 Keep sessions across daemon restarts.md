---
tags: [adr, phase-8]
status: accepted
date: 2026-11-01
---
# ADR-0025: Keep sessions across daemon restarts and restore them from the TUI

**Context:** After `hive daemon stop` to pick up v0.4.15, the user found "all my space and agents were gone". [[ADR-0009 Persist session metadata, mark lost sessions]] kept sessions only after a crash. A graceful stop forgot them, so an upgrade, the most common restart, lost the most.

**Decision:**
- A graceful shutdown persists live sessions **before** terminating them, then closes the store. A clean stop and a crash look the same afterwards: `Lost` records.
- `StoredSession.running` (serde default) records the catalog agent in the foreground (from presence). A `Lost` record carries it back as presence (status `Unknown`).
- `hive daemon stop` lists the sessions it will end and asks (only on a TTY; `-y` skips). It writes `restore.json` in the state dir, which the next daemon merges once (deduped by id) and deletes. This covers daemons before 0.4.16, which forget on a stop.
- TUI: Enter relaunches one, the new `Command::RestoreSessions` (leader `R`) relaunches all (CreateSession + KillSession of the record each). A one-time status line on start says how many ended.
- A relaunch resumes the agent that was running: `hive_agents::catalog::resume_args` (claude `--continue`, codex `resume --last`). Launched directly → args appended. Typed into a POSIX shell → `shell -c "agent args…; exec shell"`, so the terminal stays. Other agents → the old command.

**Consequences:**
- Upgrades no longer lose the layout of spaces and agents; conversations come back for Claude Code and Codex.
- Ended sessions stay listed until restored or dismissed (`x`). Tests that remove workspaces dismiss them first.
- Supersedes ADR-0009's "graceful shutdown forgets them". Processes still never survive (spec §4).
