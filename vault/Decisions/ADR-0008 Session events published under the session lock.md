---
tags: [adr, phase-2]
status: accepted
date: 2026-10-30
---
# ADR-0008: Session lifecycle events are published only via `Session::publish`/`remove`

**Bug that prompted it:** Killing the attached session in the TUI left a ghost `○` entry in the sidebar. `kill` broadcast `SessionRemoved`. The TUI then re-attached elsewhere, and the server's `detach()` broadcast `SessionUpdated` for the dead session, which re-inserted it client-side. The regression test is `removed_session_is_never_resurrected_by_later_updates`.

**Decision:** Only two methods may emit per-session lifecycle events:
- `Session::publish()` takes the session's `meta` lock and sends `SessionUpdated` **only if `meta.removed` is false**.
- `Session::remove()` takes the same lock, sets `removed = true`, and sends `SessionRemoved`.

Because both hold the `meta` lock across the check and the send, every `SessionUpdated` is ordered strictly before the `SessionRemoved` on the broadcast channel. The waiter thread's exit notification goes through `publish()` too. That closes the same latent race when a kill gives up waiting before the process exits.

**Rule for future code:** never `daemon.events.send(SessionUpdated(..))` directly. Call `session.publish()`.
