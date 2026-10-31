---
tags: [status]
updated: 2026-10-30
---
# Status

**Phase 2 complete** (2026-10-30, branch `phase-2-multiplexing`). Claude Code and Codex run side by side with a sidebar, switching, picker, rename and kill. See [[Phase 2 - Multiplexing]]. Phase 1: [[Phase 1 - Foundation]].

**Next:** [[Roadmap#Phase 3 — Robust Persistence]]. TUI auto-reconnect, metadata persistence, and multi-client polish.

## Gate before every change
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Known gaps carried forward
- No scrollback viewing in the TUI (the daemon keeps scrollback in its emulator but the client only mirrors the visible screen). See [[Risks and Open Questions]].
- Mouse is not captured or forwarded.
- Shift+Enter is not distinguishable (no kitty keyboard protocol).
- Sessions are not persisted across daemon restarts. This is by design (spec §4).
- New sessions launched from the TUI use the `hive` process's cwd (no directory prompt yet).
- No activity indicator for background sessions (see [[ADR-0007 Switching re-attaches]]).
