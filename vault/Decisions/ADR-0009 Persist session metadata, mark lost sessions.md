---
status: accepted
date: 2026-10-30
---
# ADR-0009: Persist session metadata, mark lost sessions

## Context
Spec §16 Phase 3 asks for session metadata persistence, "without implying process recovery". PTY children cannot outlive `hived`: a SIGKILLed daemon closes every PTY master, the kernel hangs up the slaves, and the agents exit. A new daemon must not pretend they still exist, and it must not reuse their ids, because a reconnecting client re-attaches by id.

## Decision
- `hived` keeps `$HIVE_STATE_DIR/sessions.json` (mode 0600, written atomically via tmp + rename): `next_session_id` plus metadata for each live session: id, agent, label, argv, cwd, created_at. **No environment**, because it holds secrets.
- A single persister task rewrites the file after each burst of lifecycle broadcasts (coalesced, and skipped when the bytes are unchanged).
- **Graceful shutdown** (`hive daemon stop`, SIGTERM) terminates sessions on purpose and keeps only existing lost records. **Crash** leaves the file as it was, so every session that was alive appears in the next daemon as `SessionState::Lost` (protocol v3).
- Ids continue from `max(stored next, max stored id + 1)`.
- A lost record can be renamed, dismissed (`KillSession`), or relaunched from the TUI sidebar (Enter): `CreateSession` with the same agent, argv, cwd and label using the TUI's env, then removal of the lost record. It cannot be attached.
- A corrupt store is moved aside to `sessions.json.corrupt` and the daemon starts empty rather than refusing to start.

## Consequences
- The user sees what was running before a crash and can restart it in one keystroke. Nothing is silently resurrected.
- A relaunch gets a new id and the relaunching client's environment, not the original one.
- Exited sessions are not persisted. Their final screen lives only in memory.
- One small file write per lifecycle burst. Output never triggers a write.
