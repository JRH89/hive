---
status: accepted
date: 2026-10-30
supersedes: "last attach wins" (Phase 1)
---
# ADR-0010: Attach observes unless control is free or forced

## Context
In Phases 1 and 2 every `Attach` took input control ("last attach wins"). With auto-reconnect and switching, a second TUI that merely looks at a session (or reconnects to it) would silently steal the keyboard from the person typing. Spec §11 requires explicit input ownership, and §16 asks that "multiple clients behave predictably".

## Decision
- `Attach{session, size, force}` (protocol v3). The client becomes controller if control is free **or** `force` is set. Otherwise it observes.
- The TUI sends `force: false` for every ordinary attach (start, switch, reconnect) and `force: true` only on Leader `a` ("take control").
- When a forced attach takes over, the old controller gets `ControlLost`. When the controller detaches or disconnects, observers get `ControlReleased`, so they can see that Leader `a` will now succeed.
- `hive attach <id>` from the CLI behaves the same way (observe if someone else controls).
- Re-attaching to the session you already control keeps control (no release-and-retake in between).
- Only the controller's `Input`/`Resize` are honoured, as before. Observers render the controller's size, clipped.

## Consequences
- Opening a second window never interrupts typing. Taking over is a deliberate keystroke.
- Status notices ("Observing…", "Another client took control…", "Control is free…") are cleared when the client becomes controller.
- Control is not handed to an observer automatically when the controller leaves. That's simpler and more predictable, but needs one keystroke.
