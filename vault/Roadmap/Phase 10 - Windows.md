---
tags: [phase]
status: done
date: 2026-10-31
---
# Phase 10 - Windows

Design: [[ADR-0021 Windows named pipe with token auth]].

| Criterion | Evidence |
|---|---|
| Builds, lints clean | CI `Windows (clippy, test, smoke)` on windows-2022, PR #3 run 41183288776; locally `cargo clippy --target x86_64-pc-windows-gnu -D warnings` |
| Pipe transport + token | `tests/windows.rs::pipe_clients_without_the_token_are_dropped`: wrong token gets EOF, real client then connects (CI pass) |
| ConPTY session, input, kill | `tests/windows.rs::conpty_session_echoes_input_and_terminates`: `cmd /k`, typed `echo typed-%OS%` → `typed-Windows_NT`, KillSession → Exited, token removed on shutdown (CI pass) |
| CLI end to end | `scripts/smoke.ps1`: doctor, new, list, status, kill, daemon stop |
| Release + install | `release.yml` x86_64-pc-windows-msvc zip via `package.ps1`; `install.ps1` (checksum, renames running exes, adds to user PATH) |

Bugs found by CI: test env lacked PATH (hive sends the client env); `Path` casing in a TUI test; hived inherited the caller's stdout pipe (smoke hung) → std handles marked non-inheritable before spawn.

Gap: no screen-based agent status on Windows (no foreground process group); self-reports work. Links: [[Status]], [[Roadmap]].
