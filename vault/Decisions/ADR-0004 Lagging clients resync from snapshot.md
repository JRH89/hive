---
tags: [adr]
status: accepted
date: 2026-10-30
---
# ADR-0004: Slow clients are resynced from a snapshot, never block the PTY

**Problem:** A noisy agent with a slow (or suspended) frontend must not stall the agent or other sessions (spec §14). Output must not be dropped silently either.

**Decision:** Each session publishes to a bounded `tokio::broadcast` (256 chunks of ≤16 KiB). The PTY reader never waits on clients. When a client's receiver lags, the forwarder takes a fresh `state_formatted()` snapshot plus a new receiver (atomically, under the `term` lock) and sends `Resync{size, snapshot}`.

**Retention policy (documented, as the spec requires):** the authoritative screen and scrollback live in the daemon's emulator. A lagging client skips intermediate frames but always converges to the exact current screen.

**Gotcha found during implementation:** `broadcast::Receiver::try_recv()` *repositions* the receiver after returning `Lagged`. The next `recv()` therefore does not report the lag again. Coalescing code must resync right at the `try_recv` lag, otherwise data is silently skipped. See `forward_output` in `server.rs`.
