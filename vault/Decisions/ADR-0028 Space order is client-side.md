---
tags: [adr]
status: accepted
date: 2026-11-04
---
# ADR-0028: Space order is client-side

**Context:** User: "drag a space to where we want it in the list and reorder them easily". Spaces were listed by workspace id (creation order). Workspaces are daemon-owned ([[ADR-0011 Daemon-owned workspaces grouped by repository]]).

**Decision:**
- The order is a UI preference, kept by the TUI like layouts ([[ADR-0023 Layouts are client-side]]): `$HIVE_STATE_DIR/space-order`, one workspace root per line. Read at start; written (temp file + rename) via `Effect::SaveSpaceOrder`.
- `App::workspace_order` sorts by position in that list, then by id; spaces not in it follow in creation order.
- Reorder by dragging a space row in the sidebar (saved on release) or `move_space_up`/`move_space_down`, global `Alt+Shift+↑/↓` (not leader keys: a default leader key would clash with existing user bindings of `K`/`J`, which `Keymap::from_config` rejects).

**Consequences:**
- No protocol change. Keyed by root, so it survives daemon restarts and id changes.
- Two open windows don't see each other's reorder until restarted; acceptable for a preference.
