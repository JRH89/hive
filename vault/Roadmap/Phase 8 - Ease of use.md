---
tags: [roadmap, phase-8]
status: done
branch: phase-8-ease
---
# Phase 8: ease of use and optional integrations

Goal (user, 2026-10-30): "make it as easy to use as herdr", plus "the next batch" of optional integrations ([[Phase 7 - herdr parity]] ideas). Design: [[ADR-0020 Spaces and agents sidebar]], [[ADR-0019 Optional integrations, off and invisible by default]].

| Criterion | Evidence |
|---|---|
| Spaces / agents sidebar like herdr | render tests `sidebar_lists_sessions_and_marks_the_current_one`, `sidebar_groups_by_workspace_with_branches_and_worktrees` (branch, count, selected space, agents numbered per space); tmux capture with 3 spaces |
| Everything clickable | app test `spaces_tabs_and_buttons_are_clickable_and_alt_arrows_move_between_them` (space, + new, menu, tab bar); tmux: injected SGR clicks on `wsB`, empty `wsC` (hint shown), `+ new` (picker "New session in wsC"), `menu` (palette); tmux reports mouse any-motion + SGR enabled |
| Alt+↑/↓ spaces, Alt+←/→ tabs/agents | same app test; tmux: M-Down ×3 wsB → wsC → hive, M-Up → wsC; M-Right api → review |
| Integrations (review, checks, tasks, notify, journal) | app tests `review_opens_beside_the_session_and_is_reused`, `tasks_are_offered_in_the_palette_and_run_in_a_split`, `finished_agents_notify_and_write_the_journal`; hive-workspace task-parser tests (just, make, package.json runner) |
| `hive update` | scratch copy: `hive update --tag v0.1.0` installed v0.1.0 into the copy's directory |
| Gate | fmt, clippy `-D warnings`, `cargo test --workspace` green |

Bug found: the user was on v0.1.0, which has no mouse support. `hive update` now exists so that can't silently happen again.
Not reproduced: leftover CLI-test daemons in deleted temp dirs (each test cleans up on its own; likely interrupted runs).
