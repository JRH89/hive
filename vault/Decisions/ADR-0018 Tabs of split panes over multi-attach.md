---
tags: [adr, phase-7]
status: accepted
date: 2026-10-30
supersedes: "[[ADR-0007 Switching re-attaches]]"
---
# ADR-0018: Tabs of split panes over a multi-attach connection

**Context:** Phase 7 (D) asks for herdr/tmux-style split panes and tabs. [[ADR-0007 Switching re-attaches]] let a connection attach one session at a time, so the TUI could mirror only one screen.

**Decision:**
- One connection can attach **several sessions**. `Attach` adds to the set (re-attaching the same session replaces its forwarder). `Detach{session: Option}` detaches one session or all of them. `Input` and `Resize` name their session. Protocol v6.
- Control stays **per session** ([[ADR-0010 Attach observes unless control is free or forced]]): a client can control one pane and observe another.
- The TUI holds `tabs: Vec<Tab>` (`layout.rs`). A tab is a binary split tree of panes with one focused pane and a zoom flag. A session is shown in at most one pane.
- The TUI mirrors exactly the panes of the **active tab**. Switching tabs detaches the hidden panes and attaches the shown ones (`sync_panes`), so hidden tabs cost nothing. That keeps ADR-0007's "no background mirroring" property.
- Each pane's PTY is sized to its own body. A lone pane has no title row; split panes get a title row and 1-cell separators.
- Sessions put into the layout:

| Way in | Effect |
|---|---|
| Switching (sidebar, `n`/`p`, `1`–`9`, palette) | focuses the pane if a tab has it; otherwise replaces the focused pane |
| `v`/`%` and `-`/`"` | picker, then split right / down |
| `t` | picker, then a new tab |
| `X` | closes the pane; the session keeps running |

- A pane leaves the layout only on `SessionRemoved`, or on an unprompted `Detached` for a session that is already gone. Replies to the TUI's own `Detach` leave the layout alone.

**Consequences:**
- The status of hidden sessions still comes from daemon broadcasts ([[ADR-0016 Agent detection from process and herdr manifests]]), not from their screens.
- The layout is not persisted. Restarting `hive` gives one pane. Possible follow-up.
- Bug found in manual testing: the daemon's reply to "detach hidden panes" was first treated as "session gone" and emptied the hidden tab. Fixed, with a unit test regression and both arrival orders tested.
