---
tags: [adr, phase-2]
status: superseded
date: 2026-10-30
---
# ADR-0007: Switching sessions re-attaches; the client mirrors one screen

> Superseded by [[ADR-0018 Tabs of split panes over multi-attach]]: one connection now attaches every pane in the active tab.

**Context:** Phase 2 needs instant switching between several agents. Two options from [[Risks and Open Questions]]:
- **A.** One attachment per connection. Switching sends `Attach`, and the daemon replies with a snapshot.
- **B.** Subscribe to many sessions and mirror all of them in the client.

**Decision:** A.
- The daemon already keeps every session's authoritative screen. A snapshot is one screen of escape codes (KBs) over a local socket, so a switch is effectively instant. Measured end to end in tmux: **30–55 ms, including tmux's own capture-polling overhead**.
- Background output costs the frontend **nothing**. Measured with an endless `yes` session in the background: `hive` at **0% CPU** while attached to Claude. B would make every noisy background session cost CPU in every frontend.
- No new protocol surface. `Attach` already resizes to the panel and takes input control, which is exactly the right behaviour for "switch to this session".

**Consequences:**
- Switching makes this client the controller of the target session. Another client viewing it gets `ControlLost`, so the last attach wins (consistent with spec §11).
- A future "activity/attention" indicator for background sessions needs a lightweight daemon-side signal (e.g. a debounced `SessionActivity` broadcast). It can't come from client mirrors.
