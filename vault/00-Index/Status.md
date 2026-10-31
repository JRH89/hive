---
tags: [status]
updated: 2026-10-30
---
# Status

**Phase 6 complete** (2026-10-30, branch `phase-6-distribution`, not yet committed). All six roadmap phases are done. Robustness and stress tests, CLI end-to-end tests, CI and draft-release workflows, static musl binaries, installer, user guide, troubleshooting and `hive doctor`. See [[Phase 6 - Testing and Distribution]]. Phases 1–5 are merged to master.

**Before a public release:** choose a license, add a GitHub remote and let CI run, dry-run a tag to a draft release, test on macOS. Spec §18 future features stay out of scope.

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
- New sessions launched from the TUI start in the active workspace's root (or `hive`'s cwd). No free-form directory prompt.
- Worktrees can't be listed from the TUI, and custom branch names are CLI-only (`hive new -b`).
- Sidebar/picker keys aren't configurable; theme is fixed at startup.
- No activity indicator for background sessions (see [[ADR-0007 Switching re-attaches]]).

## Website (separate from Rust phases)
- Terminal-first Next.js/React/TypeScript/Tailwind landing page implemented in `website/` using AGENTS branding. User approved the plan on 2026-10-30.
- Typecheck, production static export, exported anchor/asset checks, and local homepage HTTP 200 passed. Independent static review found no blockers.
- Browser QA remains pending: no browser available to verify 375px/desktop rendering or real interactions. See [[Website Design]] and [[Website Implementation Plan]].
- Run from `website/`: `npm ci`, then `npm run dev` (http://127.0.0.1:4310). No deployment configured. Rust phase status is independent.

- Website polish: reduced hero headline/padding after user screenshots, sparse SVG honeycomb background, self-hosted fonts, illustrative selectable sessions, compact source steps, and upcoming Linux/macOS/Windows release install UI for `jrh89/hive`. Installers remain unpublished with inactive downloads.
