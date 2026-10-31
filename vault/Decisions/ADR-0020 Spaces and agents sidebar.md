---
tags: [adr, phase-8]
status: accepted
date: 2026-10-30
---
# ADR-0020: herdr-style sidebar of spaces and agents

**Context:** User feedback (2026-10-30): clicking a different workspace "doesn't work" (they were on v0.1.0, which had no mouse). They asked for Hive to be "as easy to use as herdr", shared a screenshot of herdr's sidebar (spaces list with branches, new/menu, agents of the space), and asked for Alt+arrows to select tabs or spaces.

**Decision:**
| Element | Behaviour |
|---|---|
| SPACES | one 2-row entry per workspace: status dot (rollup, or `·`), name, agent count; branch below (main checkout's session). Selected space highlighted |
| `+ new` / `menu` | clickable: picker for the selected space / command palette |
| AGENTS | sessions of the selected space; numbers 1–9 count within the space (`Alt+digit`) |
| Selected space | `App::space`, set by `show()` (follows the focused session), a click on an empty space, or Alt+↑/↓. While browsing the sidebar it follows the cursor |
| Alt+↑/↓ | previous/next space, empty ones included (`open_space`) |
| Alt+←/→ | previous/next tab; with one tab, previous/next agent of the space. Default global chords moved from next/prev_session to next/prev_tab |
| Tab bar | clickable (`SidebarHit::Tab`) |
| Hits | `sidebar_hits` are rectangles recorded while drawing, cleared once per frame |

`hive update` runs the release installer into the running binary's directory, so upgrading is one command.

**Consequences:** The sidebar shows one space's agents at a time, so leader `n`/`p` still walk every session in order. Supersedes the grouped list from Phase 2.
