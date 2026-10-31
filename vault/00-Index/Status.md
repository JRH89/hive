---
tags: [status]
updated: 2026-10-30
---
# Status

**Phase 1 complete** (2026-10-30). All acceptance criteria were verified against real Claude Code and Codex. See [[Phase 1 - Foundation]].

**Next:** [[Roadmap#Phase 2 — Multiplexing]]. Add a sidebar, switch sessions without leaving the TUI, and run several agents side by side. The daemon already supports multiple sessions. The TUI currently shows one at a time.

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
