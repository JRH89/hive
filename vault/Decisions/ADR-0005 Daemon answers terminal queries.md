---
tags: [adr]
status: accepted
date: 2026-10-30
---
# ADR-0005: The daemon replies to terminal queries

**Problem:** TUI agents query the terminal at startup: DSR `CSI 6 n` (cursor position), DA `CSI c`, and OSC 10/11 (fg/bg colour, used for theme detection by Codex and others). In a multiplexer the child's output never reaches a real terminal directly. If nobody answers, apps hang or fall back after timeouts. Clients can't answer reliably either: there may be zero clients, or several.

**Decision:** The authoritative emulator's `Responder` (a vt100 `Callbacks` impl) answers DSR 5/6, primary and secondary DA, and OSC 10/11 `?` with hive-dark colours. Replies are `try_send`'d to the PTY writer. Client mirrors use `()` callbacks and never respond.

**Why `try_send`:** The reader thread must never block on the writer. If it did, a child blocked on output would deadlock against a full input queue.

**Verified:** Codex starts and renders its trust prompt under Hive ([[Phase 1 - Foundation]]).
