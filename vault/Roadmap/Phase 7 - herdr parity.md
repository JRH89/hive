---
tags: [roadmap, phase-7]
status: in-progress
branch: phase-7-herdr-parity
---
# Phase 7: herdr parity

Goal (user, 2026-10-30): "match the control and stuff of herdr better… it should detect more agents like it didnt even see truss". Scope chosen: all of (A) discovery + detection, (C) status, (B) controls + mouse, (D) panes + tabs. Design: [[ADR-0016 Agent detection from process and herdr manifests]].

## Step A + C: discovery, detection, status ✅
| Criterion | Evidence |
|---|---|
| truss and other installed agents are known | `hive agents` on the dev machine lists 13 installed incl. `truss → truss-tui`; CLI test `agents_are_listed_and_can_report_their_status` |
| Agent started from a plain shell is recognised | tmux, isolated daemon: shell session runs a stand-in `claude` → sidebar `◐ one / working`, `hive list` AGENT `claude`; daemon test `agents_are_recognised_and_their_status_tracked` (source `Process`) |
| Working → idle from herdr rules | same daemon test (title spinner → plain title); `hive-agents` unit tests incl. every bundled manifest parses |
| Done while unseen, rollup, cleared on view | tmux: switched away, agent finished → `✓ one / done`, workspace header `✓`; switching back cleared it. Unit test `unseen_sessions_become_done_and_ring_the_bell` |
| Self-report overrides, stale seq ignored, release | daemon test + CLI test (`report-agent … blocked` → `hive list` shows truss/blocked) |
| Plain shell shows nothing | daemon test `a_plain_shell_has_no_agent_presence` |
| Gate | fmt, clippy `-D warnings`, `cargo test --workspace` all green |

## Step B: controls and mouse ✅
| Criterion | Evidence |
|---|---|
| herdr/tmux leader defaults | keymap test `defaults_cover_the_documented_keys` (`c`, `n`, `p`, `:`, `b`) |
| Prefix-free chords, configurable, validated | `global_chords_default_like_herdr_and_validate`; app test `global_chords_switch_without_the_leader`; tmux: `M-2` switched one → two |
| Mouse: sidebar clicks, forwarding, wheel | app test `mouse_clicks_sessions_and_reaches_agents_that_ask_for_it`, keys test for X10/SGR encodings and mode filtering; tmux: injected SGR click on row of `one` opened it |
| Sidebar toggle resizes the agent | `toggling_the_sidebar_gives_the_agent_the_full_width`; tmux: leader `b` → panel full width |
| Docs | user guide key tables, global chords, mouse, agent status; `examples/config.toml` (validated by test) |

## Step D: split panes and tabs ✅
Design: [[ADR-0018 Tabs of split panes over multi-attach]].
| Criterion | Evidence |
|---|---|
| One connection mirrors several sessions at their own sizes | daemon test `one_client_mirrors_several_sessions_at_their_own_sizes`; all 24 daemon tests green |
| Splits share the panel; input to the focused pane; zoom; close | app test `splits_share_the_panel_and_route_input_to_the_focused_pane` (exact sizes, `Input{session}`, click focus, zoom/unzoom resizes, close detaches but keeps the session); layout tests (3) |
| Tabs keep their panes; hidden tabs detached | app test `tabs_keep_their_own_panes` (incl. the daemon's `Detached` reply leaving the hidden tab intact) |
| Ended sessions give space back, both message orders | `a_split_pane_that_ends_gives_its_space_back` (Detached before SessionRemoved), kill test (after) |
| Live | tmux 120×30, isolated daemon: leader `v` → two titled panes, `echo` lands in the focused one; `z`/`z` zoom and restore; `o` moves focus (header follows); `t` → tab bar ` 1:shell  2:shell `; `[`/`]` switch with panes intact; `X` closes a pane, the other fills the tab |
| Gate | fmt, clippy `-D warnings`, `cargo test --workspace` green; protocol bumped to v6 |

Bugs found in manual testing:
- Replies to the TUI's own detaches emptied hidden tabs (the tab bar never appeared). Fixed and regression-tested.
- The shared-directory warning still said "Leader n, then w". It now names the bound `new_worktree_session` key.

## Next
- Optional Obsidian notes integration (off by default).
- Persist the pane/tab layout across `hive` restarts (not done).
- Offer truss a Hive reporter (mirror of its herdr one, using `HIVE_SESSION_ID`/`HIVE_BIN_PATH`). Ask first: user's repo.
- Website: say the install one-liner alone is enough (Download is optional).

## Ideas (not scheduled)
- Obsidian: detect a vault (`.obsidian/`) in a workspace and offer a notes tab/pane once tabs exist. An MCP server is not needed for this; agents can already use Obsidian MCP servers themselves.
