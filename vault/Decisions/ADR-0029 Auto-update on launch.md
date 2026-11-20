---
tags: [adr]
status: accepted
date: 2026-11-19
---
# ADR-0029: Auto-update on launch

**Context:** User: "when someone types hive in the terminal to open it it should auto update if there's a later release like codex and claude do". `hive update` existed but had to be run by hand.

**Decision:**
- Bare `hive` (no subcommand) checks `github.com/jrh89/hive/releases/latest` (the redirect's `/tag/vX.Y.Z`, via `curl -fsSI --max-time 3`) and, if newer, runs the same installer as `hive update` (checksum-verified), then re-execs the new binary with the same args.
- Checked at most hourly: the answer (or a failed check, as empty) is cached in `$HIVE_STATE_DIR/latest-release`.
- Off with `[general] auto_update = false`; skipped once with `HIVE_NO_AUTO_UPDATE=1`, for debug builds, and when stdout isn't a terminal. The re-exec sets `HIVE_NO_AUTO_UPDATE=1` so it can't loop.
- `current_exe()` is read before installing: on Linux a replaced running binary's path reads as `... (deleted)`.
- Never blocks opening: any failure prints a warning and the current version runs.
- The daemon is not restarted (that would end sessions); the existing outdated-daemon notice covers it.

**Consequences:**
- Drafts are never installed: `releases/latest` only points at published releases, and release.yml only makes drafts.
- Subcommands (`hive new`, `hive ls`, scripts, agents calling `hive report-agent`) never update or pay the check.
- Startup pays at most a 3 s check once an hour when offline.
