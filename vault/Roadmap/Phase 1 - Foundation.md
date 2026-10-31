---
tags: [roadmap, phase-1]
status: done
completed: 2026-10-30
---
# Phase 1: Daemon-First Foundation ✅

Goal (spec §16): prove the full architecture with one interactive agent session.

## Acceptance criteria and evidence
| Criterion | Evidence |
|---|---|
| `hive` starts or connects to the daemon | `hive status` said "not running". `hive new shell -d` then auto-started `hived` (runtime dir 0700, socket 0600) |
| An installed agent CLI can be launched | `hive new claude -d` and `hive new codex` both launched under `hived` |
| Terminal output renders correctly | Claude Code's logo (block glyphs), box rules, prompt and status lines rendered in the TUI under tmux |
| Interactive prompts work | Codex trust dialog: Down/Up moved the `›` selection |
| Keyboard input is forwarded | Typed `hello from hive` appeared at Claude's `❯` prompt |
| Resizing works | Resizing tmux 110→90 cols made Claude redraw its rules at 90. Integration test asserts `stty size` reports 40 120, then 30 100 |
| Closing the TUI does not kill the agent | Leader `d` detached and Claude was still `running`. `kill -9` of the TUI left Claude still `running` |
| Reopening Hive reconnects to the live session | Bare `hive` reattached to session 2. The snapshot still showed `hello from hive` |

Automated: `cargo test --workspace` has 32 tests. Of those, 5 are daemon integration tests with real PTYs (`crates/hive-daemon/tests/daemon.rs`): survive disconnect and reattach, resize propagation, input control handover, exit codes and TERM→KILL escalation, and lock contention plus clean shutdown.

## Built beyond the bare list (cheap now, painful later)
- Explicit input ownership (controller and observers, `ControlLost`). Spec §11 requires it, and the Phase 3 work builds on it.
- Responder for terminal queries ([[ADR-0005 Daemon answers terminal queries]]).
- Slow-client resync ([[ADR-0004 Lagging clients resync from snapshot]]).
- Theme tokens with a 256-colour fallback, and the brand constants (spec §21 asks for these early).

## Not done (intentionally deferred)
Scrollback UI, mouse, sidebar and multiple visible sessions (Phase 2), daemon reconnection in the TUI (Phase 3), configurable keys (Phase 5). See [[Risks and Open Questions]].
