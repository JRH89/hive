---
tags: [status]
updated: 2026-10-30
---
# Status

**Phase 3 complete** (2026-10-30, branch `phase-3-persistence`, not yet committed or merged). The TUI auto-reconnects and auto-starts `hived`; sessions from a crashed daemon come back as `◌ ended` and can be relaunched; a second client observes until it takes control. See [[Phase 3 - Robust Persistence]]. Earlier: [[Phase 2 - Multiplexing]] (merged to master), [[Phase 1 - Foundation]].

**Next:** [[Roadmap#Phase 4 — Workspaces and Git]]. Workspaces, repo discovery, branch display, optional worktrees.

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
- Processes don't survive a daemon crash (by design, spec §4). Only metadata does, as `Lost` sessions ([[ADR-0009 Persist session metadata, mark lost sessions]]).
- Killing an interactive shell session waits the full kill timeout (bash ignores SIGTERM).
- New sessions launched from the TUI use the `hive` process's cwd (no directory prompt yet).
- No activity indicator for background sessions (see [[ADR-0007 Switching re-attaches]]).

## Website (separate from Rust phases)
- Terminal-first Next.js/React/TypeScript/Tailwind landing page implemented in `website/` using AGENTS branding. User approved the plan on 2026-10-30.
- Typecheck, production static export, exported anchor/asset checks, and local homepage HTTP 200 passed. Independent static review found no blockers.
- Browser QA remains pending: no browser available to verify 375px/desktop rendering or real interactions. See [[Website Design]] and [[Website Implementation Plan]].
- Run from `website/`: `npm ci`, then `npm run dev` (http://127.0.0.1:4310). No deployment configured. Rust phase status is independent.

- Website polish: reduced hero headline/padding after user screenshots, sparse SVG honeycomb background, self-hosted fonts, illustrative selectable sessions, compact source steps, and upcoming Linux/macOS/Windows release install UI for `jrh89/hive`. Installers remain unpublished with inactive downloads.
