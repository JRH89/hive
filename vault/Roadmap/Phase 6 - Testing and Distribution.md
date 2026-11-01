---
tags: [roadmap, phase-6]
status: done
completed: 2026-10-30
branch: phase-6-distribution
---
# Phase 6: Testing and Distribution ✅

Goal (spec §16): prepare for public release. The spec gives no explicit acceptance criteria, so each "Implement" item is checked below.

## Items and evidence
| Item | Evidence |
|---|---|
| Unit tests | 55 `hive-tui`, 7 `hive-core`, 7 `hive-workspace` (real git), 3 protocol, plus daemon unit tests. Total `cargo test --workspace` **104** |
| IPC tests | Protocol: round-trip, oversized prefix, handshake at variant 0. New daemon test `malformed_clients_are_dropped_without_affecting_the_daemon`: garbage, an 8 MiB+1 prefix, non-Hello first frame (→ `Error`), old protocol version (→ `Welcome` with ours, then close), garbage after Attach. The attached session keeps running and other clients are unaffected |
| PTY lifecycle | Existing 17 tests (exit codes, kill, crash → Lost, graceful shutdown, ids never reused) |
| Detach/reattach | `interactive_session_survives_client_disconnect`. New `reattach_restores_a_full_screen_app_exactly`: alternate screen, cell positions and colours identical after reattach, and the primary screen is restored when the app quits (found a bug, see below) |
| Concurrent stress | New `many_concurrent_sessions_and_clients_stay_correct`: 12 sessions × 400-line bursts, 4 clients attaching and switching concurrently; every session exits with its own code. `noisy_session_does_not_starve_others` (echo < 1 s during a flood) |
| Terminal resize | `attach_resizes_pty_to_client_viewport` plus new `an_observers_resize_does_not_change_the_pty` (resize conflict: an observer's 50×150 and 55×160 are ignored; `stty size` stays 30 100) |
| Flakiness | Daemon suite: 100-run soak under heavy load gave 15 failures in 3 tests. Two were test races and one a daemon race, all fixed (below). After the fixes: **75/75** runs with 3 suites in parallel (each ~13.6 s instead of 2 s), 0 failures |
| CLI end-to-end | New `crates/hive-cli/tests/cli.rs` (4 tests) runs the real `hive` binary against an auto-started private `hived`: version/help, new/list/status/kill/daemon stop, error exits (unknown agent, bad id, bad config), `doctor` |
| CI pipeline | `.github/workflows/ci.yml`: fmt, clippy `-D warnings`, tests, MSRV 1.89 check, static musl builds for x86_64 and aarch64 + `scripts/smoke.sh`. **Not yet run on GitHub** (no remote configured) |
| Linux release binaries | `cargo build --release --locked --target x86_64-unknown-linux-musl` built locally: `static-pie`, hive 5.0 MB, hived 4.3 MB. `scripts/smoke.sh` passed on them. `release.yml`: tag → version check → test → build → smoke → package → **draft** release ([[ADR-0015 Static musl binaries via draft GitHub releases]]). aarch64 is built only in CI |
| Installation instructions | README Install section. `scripts/install.sh` tested against a local `file://` release: installed both binaries, warned about PATH. A tampered archive gave "checksum mismatch", exit 1, nothing installed |
| User documentation | `docs/user-guide.md` (concepts, interface, keys, worktrees, config, themes, upgrade, uninstall). README: supported agents, docs, development |
| Troubleshooting guide | `docs/troubleshooting.md` plus new `hive doctor` (config validity, socket length, daemon/protocol, logs, terminal depth/glyphs, agents on PATH; read-only, exit 1 on problems) |
| Windows/macOS investigation | `cargo check --workspace --all-targets --target aarch64-apple-darwin`: **clean**. No `/proc` or other Linux-only code; `dirs::runtime_dir()` is None on macOS, so the runtime dir falls back to `$TMPDIR/hive-<uid>`. Needs a real Mac run (PTY, `peer_cred`, kill). `x86_64-pc-windows-gnu`: fails at `hive-workspace` (`std::os::unix`); Unix sockets, `setsid` and signals also need replacing (named pipes, ConPTY via portable-pty, job objects). Windows is a port, not a fix |

Gate: fmt, clippy `--locked -D warnings`, `cargo test --workspace --locked` → 104 passed.

## Bugs found and fixed
- **Reattach lost the alternate screen.** `state_formatted()` omits it, so a client attaching while vim/htop/OpenCode ran thought it was on the primary screen, and stayed stale when the app quit. The snapshot now prepends `?1049h`. The reader publishes a fresh snapshot (`Chunk::Snapshot` → `Resync`) when an app leaves the alternate screen, because vt100 can't expose the hidden primary grid.
- **Graceful shutdown could leave sessions "lost".** Aborting the persist task doesn't stop a synchronous `persist(true)` already running on another worker. A live snapshot taken before `terminate_all` could be written after the final store. `persist` now holds `saved` for its whole duration, and the final write closes the store.
- Test race: `SessionRemoved` (events task) vs `SessionList` (reply) are unordered. The test now fences on the events channel.
- Test assumption: the noisy-flood test assumed the stalled client would be overrun in 1.5 s. A loaded debug build manages ~3.5 MiB/s, below the ~8 MiB of buffering. It now meters 16 MiB through a reading observer.
- `hive --help` (long help) lacked the brand line.

## Not done / carried forward
- CI and release workflows are untested until the repository has a GitHub remote. Tag-to-draft-release needs one dry run.
- License: MIT (user decision, 2026-10-30).
- No PNG/GIF screenshots; the README uses text captures.
- macOS untested at runtime; Windows unsupported.
- `[terminal] mouse_support` is implemented (Phase 7). `[general] restore_workspace` is accepted and ignored (2026-10-31), dropped from the spec.
