---
tags: [phase]
status: done
date: 2026-10-31
---
# Phase 9 - macOS

| Criterion | Evidence |
|---|---|
| Builds on macOS | CI `macOS (clippy, test, smoke)` job on macos-14, PR #1, run 41176908780: pass |
| Full test suite passes | same job; fixes: ticker test polls instead of sleeping, worktree test compares canonical paths (`/var` → `/private/var`) |
| Agent detection without `/proc` | `ps -A -ww -o pid=,pgid=,args=` path; `ps_agrees_with_proc` cross-checks on Linux |
| Binaries released | `release.yml` matrix: aarch64-apple-darwin (tested), x86_64-apple-darwin (cross, untested); `install.sh` maps Darwin |
| Smoke test | `scripts/smoke.sh target/release` in the macOS job |

Not checked by hand on a Mac (no hardware); CI is the evidence. Links: [[Status]], [[Roadmap]].
