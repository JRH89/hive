---
tags: [adr, phase-8]
status: accepted
date: 2026-11-02
---
# ADR-0026: Optional tree sidebar

**Context:** The user asked for one list instead of SPACES plus AGENTS: spaces, with their agents indented and collapsible under each. It should be an option, not the default. [[ADR-0020 Spaces and agents sidebar]] stays the default.

**Decision:**
- `[general] sidebar_tree` (bool, default false). `Command::SidebarTree` (palette `sidebar_tree`, no key) flips it and saves it via `Effect::SaveSetting`.
- `render::draw_sidebar_tree`: each space row (arrow, activity dot, name, agent count; branch and git counts below), then its agents indented 3 columns, using `draw_sidebar_session` without the space name. "+ new / menu" sits on the last row. Numbers stay those of `App::agents`, so Alt+1–9 is unchanged.
- Fold state is `App::collapsed` (per window, not saved). `SidebarHit::Collapse(w)` covers the arrow; the rest of the row still opens the space.
- Keyboard needs no new cursor type. In tree mode `move_cursor` walks `tree_stops`: every agent of an open space, and the first agent of a folded one, which stands for the space row. Space toggles, ← folds, Enter or → on a folded stop unfolds. These are sidebar-local keys like j/k/x/r, not Commands.

**Consequences:**
- Spaces without agents can be clicked but not reached by keyboard in the tree.
- Folds reset when `hive` restarts.
