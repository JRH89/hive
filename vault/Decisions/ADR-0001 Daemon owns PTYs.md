---
tags: [adr]
status: accepted
date: 2026-10-30
---
# ADR-0001: The daemon owns PTYs, processes and terminal state

**Context:** Spec §4 makes this non-negotiable. Retrofitting a daemon onto a TUI that owns its children is the failure mode it warns about.

**Decision:** `hived` is a separate binary from day one. `hive` never opens a PTY. It auto-starts `hived` (`Client::connect_or_start`) and talks over [[IPC Protocol]]. `hived` calls `setsid()` at startup, so terminal hangups never reach it.

**Concurrent starts:** `hived` takes an exclusive `File::try_lock` on `hived.lock` (std, Rust ≥ 1.89). The loser exits 0. The winner treats any existing socket as stale and removes it before binding. The OS releases the lock if the daemon crashes.

**Consequences:** Frontend crashes are harmless. This was verified by `kill -9` of the TUI with Claude Code still running (see [[Phase 1 - Foundation]]). Sessions do not survive a daemon exit or a reboot, and we don't claim they do.
