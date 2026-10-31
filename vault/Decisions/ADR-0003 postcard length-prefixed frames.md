---
tags: [adr]
status: accepted
date: 2026-10-30
---
# ADR-0003: u32-LE length prefix + postcard

**Decision:** Each frame is a 4-byte little-endian length followed by a `postcard`-encoded serde enum. Max 8 MiB.

**Why:** Terminal output is binary and high-volume. JSON would base64 or array-encode the bytes, which is slow and bloated. Postcard is compact, serde-native and stable (1.x). bincode 2 changed its API a lot recently. The explicit length bound protects both sides from hostile or corrupt prefixes.

**Trade-off:** Not human-readable on the wire. For debugging, log decoded messages with `HIVE_LOG=debug` rather than sniffing the socket.

**Compat rule:** `Hello`/`Welcome` are variant 0 forever (enforced by a test), so version mismatches always produce a clean error. See [[IPC Protocol]].
