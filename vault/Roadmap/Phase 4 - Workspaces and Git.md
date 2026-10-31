---
tags: [roadmap, phase-4]
status: done
completed: 2026-10-30
branch: phase-4-workspaces
---
# Phase 4: Workspaces and Git ✅

Goal (spec §16): organize agents by development project.

## Acceptance criteria and evidence
| Criterion | Evidence |
|---|---|
| Users can manage multiple repositories | Manual (isolated daemon, scratch repos `webapp`, `api`, plus non-git `notes`): `hive workspace list` showed 3 workspaces, with `notes` marked "(not git)". The TUI's Leader `w` picker lists them with counts; `a`/`r`/`x` send add/rename/remove. `hive workspace rm api` succeeded once it was empty. Tests `workspaces_persist_and_are_removed_only_when_empty` (idempotent add, rename, refusal with sessions, survives restart), `workspaces_are_added_renamed_and_removed_from_the_picker` |
| Sessions appear under the correct workspace | `hive new -C webapp/sub` grouped with `webapp`; worktree sessions (outside the repo dir) grouped with `api`; `-C <existing worktree>` joined `api`. TUI sidebar: `▾ webapp` (3), `▾ api` (2, `wt` marked), `▾ notes`. After `kill -9 hived`, lost sessions kept their workspaces. Tests `sessions_are_grouped_into_workspaces_by_repository`, `sidebar_groups_by_workspace_with_branches_and_worktrees` |
| Agents can work in isolated worktrees | `hive new -w` → branch `hive/shell-4` in `$worktrees/api-839431dd/hive-shell-4`; `-b feature/login` used that branch. From the TUI: Leader `n`, `w` (shows "own git worktree: on"), `5` → `#7` on `hive/shell-7` in `api`. Typing `git switch -c renamed-in-session` in it updated the sidebar within 2 s. Test `worktree_sessions_are_isolated_and_cleanup_is_safe` |
| Safe cleanup | Kill with `w` on a clean worktree removed it, and the branch was kept (`git branch` still lists it). Dirty worktree: TUI said "Kept worktree feature-redo: it has uncommitted changes…"; `hive worktree rm` refused with a hint; `--force` asked `[y/N]` (n → cancelled); `--force -y` removed it, branch kept. Removal while a session runs is refused (test). The main tree is never removed (unit and integration tests) |

Gate: `cargo test --workspace` **79 tests** (17 daemon integration tests, 7 `hive-workspace` tests against real git). The daemon suite passed 40/40 consecutive runs; fmt and clippy are clean.

## What shipped
- Crate `hive-workspace`: `discover` (file-based, stops at filesystem boundaries), `read_branch`, `workspace_key`/`best_match`, and git `add_worktree`/`remove_worktree`/`list_worktrees`/`is_dirty`. See [[ADR-0011 Daemon-owned workspaces grouped by repository]] and [[ADR-0012 Worktrees created by the daemon, never deleted dirty]].
- Protocol **v4**: `SessionInfo.{workspace, branch, worktree}`, `CreateSession.{workspace, worktree}`, workspace and worktree messages. Store v2.
- CLI: `hive new -W/-w/-b`, `hive workspace list|add|rename|rm`, `hive worktree list|rm [--force] [-y]`, and WORKSPACE/BRANCH columns in `hive list`.
- TUI: grouped sidebar (`WORKSPACES`, `▾ name`, `wt` + branch, `!` for a shared directory), Leader `w` workspace picker, `w` worktree toggle in the agent picker, kill-with-worktree, and workspace/branch in the header and status bar.
- Config `[worktrees] dir`.

## Bugs found and fixed
- Discovery treated `/tmp/.git` (a stray directory on this machine) as a repository. Git stops at the filesystem boundary and requires `HEAD`; discovery now does both.
- `SessionCreated` (a direct reply) can arrive after a `SessionUpdated` broadcast for the same session, e.g. an instant exit. The TUI overwrote the newer state; `SessionCreated` now only inserts. A test helper raced the same way (flaky `exit_is_reported…`).
- Crash test: a store write between id allocation and session registration satisfied its wait condition. It now waits for the exit first.
- Manual run: branch truncated behind the agent name in the sidebar (branch now first); the header overlapped the tagline with long paths (pre-existing); worktree notices showed only a long path; "Removed worktree" was wiped by the next `Attached`; "1 sessions".

## Not done / carried forward
- No TUI view of a repository's worktrees (CLI `hive worktree list` only), and no prompt for a custom branch name in the TUI (CLI `-b`).
- Workspace "switching" opens the workspace's newest session; there is no per-workspace filtered sidebar or collapse.
- Lost sessions show no branch (it isn't persisted).
