---
tags: [roadmap, phase-2]
status: done
completed: 2026-10-30
branch: phase-2-multiplexing
---
# Phase 2: Multiplexing ✅

Goal (spec §16): several agent sessions at once, with a sidebar and switching.

## Acceptance criteria and evidence
| Criterion | Evidence |
|---|---|
| Claude Code and Codex run simultaneously | In tmux: `hive new claude`, then Leader `n` → picker → `2` (codex). Sidebar showed `● claude 1` and `▌● codex 2` |
| Switching doesn't interrupt execution | Integration test `switching_between_sessions_keeps_both_running`: a ticker kept counting while unattached (`after > before + 5`) |
| Both remain interactive | Typed `message for claude` at Claude's `❯`, switched, typed `message for codex` at Codex's `›` (no Enter, so no model call). Leader `1`/`2` back and forth: both inputs intact |
| Background output doesn't freeze the UI | Endless `yes` session alongside. Switching took 30–55 ms even *while viewing* the flood. Typing into Claude echoed in 20–48 ms. `hive` used 0% CPU with the flood in the background. Integration test `noisy_session_does_not_starve_others`: quiet-session echo < 1 s while a never-reading client is attached to the flood, and that client is later resynced |

`cargo test --workspace` has **46 tests**, including 9 daemon integration tests with real PTYs.

## What shipped
- **Sidebar** (26 columns, hidden below 60): `SESSIONS` heading, `●`/`○`/`✕` state dots, amber `▌` on the attached session, slate highlight on the cursor, `1`–`9` position hints, and a second line with agent · directory.
- **Switching:** Leader `j`/`k` (wraps around), Leader `1`–`9`, or sidebar focus (Leader `s`, then `↑↓`/`enter`, `esc` to go back). See [[ADR-0007 Switching re-attaches]].
- **New-session agent picker** (Leader `n`): built-in and configured agents, with "(not installed)" marked from a PATH check.
- **Rename:** `RenameSession` message (protocol **v2**), Leader `r` or sidebar `r`, inline prompt in the status bar. An empty label resets to the agent name.
- **Kill from the UI** (confirmed). The view then moves to the next session, or focuses the empty sidebar.
- **Event loop:** queued daemon messages are applied within a 12 ms budget per frame, so a flood can't starve input. Connection errors in the drain are no longer swallowed.

## Bugs found and fixed
- Ghost sidebar entry after killing the attached session. See [[ADR-0008 Session events published under the session lock]].
- The per-client queue allowed up to 16 MiB per client (256 messages × 64 KiB coalesced). Cut to 64 messages (≤ 4 MiB).
- The first version of the flood test never actually exercised the lag path. It now asserts that the stalled client receives `Resync`.

## Not in scope (later phases)
Workspace grouping and git branches (Phase 4), activity/attention indicator, command palette and configurable keys (Phase 5), auto-reconnect (Phase 3).
