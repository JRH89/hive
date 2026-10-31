# Codex project guide

Read `vault/00-Index/Status.md`, then `vault/00-Index/Home.md`, before work. Read relevant sections of `AGENTS.md`, the authoritative product specification. This guide follows `CLAUDE.md`; keep shared engineering guidance consistent.

Codex discovers `AGENTS.md` automatically. This supplementary file is explicitly referenced there; it does not replace the specification.

## Project

Hive is a native Rust terminal multiplexer for interactive coding agents. `hive` is the CLI/TUI client; `hived` owns child processes, PTYs, and authoritative terminal state. Never launch PTYs from the frontend. Live processes survive frontend disconnection, not daemon termination or reboot.

Read `CLAUDE.md` for crate boundaries and cross-file concurrency invariants before touching Rust. Preserve snapshot/subscription atomicity, term-before-meta lock ordering, explicit input control, bounded queues, and isolated non-cancel-safe frame reads.

## Concurrent agents

- Inspect `git status --short` before edits and again before staging.
- Another Claude agent is working on Rust. Website work belongs in `website/`; do not modify `crates/`, Cargo files, Rust tests, or Claude's guidance for this task.
- Never reset, stash, overwrite, stage, or commit another agent's changes. Stage explicit owned paths only.
- Re-read shared vault notes immediately before small, additive updates. Preserve other agents' entries and phase evidence.
- Ask before overlapping edits if independent files cannot accommodate the work.

## Obsidian memory

- Append a dated session entry to `vault/Log/YYYY-MM-DD.md`; do not replace existing entries.
- Record decisions with Context / Decision / Consequences and status/date frontmatter. Link notes from Home using `[[wikilinks]]`.
- Update Status and Roadmap when verified progress changes. Website progress must not imply a Rust phase is complete.
- Update architecture notes when implemented behavior changes. Record commands and actual results, including incomplete verification.

## Brand and website

- Exact tagline: **One terminal. Many agents.**
- Charcoal `#111318`, slate `#1C2028`, amber `#F2B84B`, honey `#D99A2B`, text `#E8EAF0`, muted `#8992A3`, success `#79C99E`.
- Use amber sparingly, a minimal outlined hexagon/cursor mark, familiar technical language, and no cartoon bees or decorative gradients.
- Website stack: Next.js, React, TypeScript, Tailwind; independent package and lockfile in `website/`. The Rust app must not require Node or a browser.
- Terminal-first landing page approved. Use original copy and assets; Herdr is a research reference, not a template.
- Clearly label illustrative terminal previews and planned capabilities. Never invent downloads, installation endpoints, performance metrics, repository URLs, license, or availability.
- Current behavior comes from code and verified vault evidence, not the future product specification.

## Verification

For Rust changes:
```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run website checks from `website/` once its package exists. Verify TypeScript, production build, responsive layout, keyboard operation, and real links. Do not claim browser verification unless performed. Do not run formatters over another agent's working files.

Use real PTYs and simple commands for integration tests. Never answer agent trust prompts or change user configuration as a side effect. Invoke processes directly; preserve agent permission systems. Confirm destructive operations and never delete dirty worktrees.
