---
tags: [roadmap, phase-3]
status: done
completed: 2026-10-30
branch: phase-3-persistence
---
# Phase 3: Robust Persistence ✅

Goal (spec §16): reliable daemon lifecycle and reconnection.

## Acceptance criteria and evidence
| Criterion | Evidence |
|---|---|
| Frontend crashes do not kill agents | Manual (isolated daemon, tmux): a `tick-N` counter session, attached TUI `kill -9`'d. `hive list` showed it `running` with 0 attached; re-attach showed `tick-18` (was `tick-6`). Test `interactive_session_survives_client_disconnect` |
| Reconnection restores access to running sessions | Unit test `disconnect_freezes_input_and_reconnect_reattaches_the_same_session` (input frozen while down, last screen kept, same session re-attached when the daemon pid is unchanged). Manual: `kill -9 hived` under a TUI → "Connection lost / Reconnecting… attempt 1" popup → a new daemon is auto-started in ≤ 2 s → sidebar shows `◌ alpha`, `◌ beta` and "hived restarted. Sessions that were running are marked ended". No orphaned shells afterwards (`ps`) |
| Multiple clients behave predictably | Manual with two TUIs on session 1: B attached → "Observing…", B's typing didn't reach the PTY. B Leader `a` → A shows "Another client took control", B's typing appears in A. B detached → A shows "Control is free". Tests `attaching_to_a_controlled_session_observes_until_forced`, `observers_are_told_when_control_is_released`, `second_client_takes_input_control` |
| Session metadata persistence | `sessions.json` mode 600, contains label/argv/cwd, no env. Crash → sessions come back `Lost`. Sidebar Enter on `◌ alpha` relaunched it as `#3 alpha` and removed the lost record. `hive kill 2` dismissed a lost record. `hive daemon stop` → `"sessions": []`, `next_session_id: 4`, and the next session was id 4. Tests `crash_marks_running_sessions_lost_and_ids_keep_increasing`, `graceful_shutdown_forgets_sessions_but_ids_keep_increasing`, store unit tests |
| Stale socket cleanup | The crash test asserts the stale socket exists after the crash and the restarted daemon binds anyway (the lock proves staleness) |

Gate: `cargo test --workspace` **58 tests** (13 daemon integration tests with real PTYs), stable over 10 consecutive daemon-suite runs and 3 full workspace runs. clippy `-D warnings` and fmt clean.

## What shipped
- **Auto-reconnect** in the TUI: backoff from 100 ms to 2 s, auto-starting `hived` at most every 2 s (if `daemon.auto_start`). Input is frozen while down; only `q`/Ctrl+C work. A version mismatch exits with the usual hint. If the daemon pid changed, a warning explains that sessions ended.
- **Metadata persistence and `Lost` sessions**, see [[ADR-0009 Persist session metadata, mark lost sessions]]. `◌` dot, "ended" in the sidebar, the status bar counts "N ended", and `hive list` shows `ended†` with a footnote.
- **Explicit control**, see [[ADR-0010 Attach observes unless control is free or forced]]. Protocol **v3**: `Attach.force`, `ControlReleased`, `SessionState::Lost`.
- Routine client disconnects (connection reset) log at debug instead of warn.

## Bugs found and fixed
- Under parallel tests, a restarted daemon sometimes got `AlreadyRunning`. A PTY child forked by a concurrent test inherits the aborted daemon's lock fd until its `exec` closes it (CLOEXEC). All test daemons share one process, so this only affects the harness, which now retries. A real daemon crash takes its whole process down.
- The crash test sometimes crashed before the store reflected the exited session. It now waits for `next_session_id` to pass the exited id and for that id to be absent.
- The "Observing" warning survived a successful take-over (the controller branch kept warnings). Control notices are now cleared when the client becomes controller; unit test extended.
- The reconnect popup clipped its last line; `hive list` misaligned the long lost-state text; lost sessions were counted as "exited".

## Not done / carried forward
- Reconnecting to the *same* daemon after a dropped connection was verified by unit test only. Dropping a live Unix socket without killing either end isn't practical to trigger by hand.
- Interactive `bash` ignores SIGTERM, so killing a shell session waits the full `kill_timeout_secs` before SIGKILL. See [[Risks and Open Questions]].
- Scrollback viewing is still not implemented (it wasn't a Phase 3 criterion).
