---
tags: [phase]
status: done
date: 2026-11-02
---
# Phase 11 - Notes and Context

Moved from AGENTS.md §18 ("Shared context") by the owner on 2026-11-02. Design: [[ADR-0027 Notes and context layer]].

| Criterion | Evidence |
|---|---|
| Vault library: parse, resolve, backlinks, search | `hive-notes` tests (7): links/tags/frontmatter/aliases parsed, code ignored; Obsidian resolution (case, shallowest, path, alias, unique prefix); hidden and build folders skipped; ranking (name > tag > text, phrase bonus, plural match) and line numbers; append never rewrites or escapes the vault; snippets cut on char boundaries |
| Search fast without an index | release build: 18 ms on this vault, 0.26 s on a synthetic 2000-note 8 MB vault (only query terms counted; names indexed for resolution) |
| `hive notes` app | `notes::tests` (6) + `notes::md::tests` (4): tree folders-first, starts on Home, folding, preview follows cursor, Tab/Enter follows links, Backspace back, missing link creates the note, search opens the hit and keeps history, new note in the selected folder, daily note in `journal_dir`, edit/Obsidian actions, narrow pane shows one panel; renderer: headings, lists, nesting, tasks, quotes, code, tables, rules, wrapping, link lines, ASCII glyphs |
| Live | tmux on a copy of this vault: tree + rendered Home, 41 links; Tab+Enter → Status; Backspace → Home; `/ dirty worktree` → ADR-0012 first; `?` help; `q` quits |
| Leader N opens it in a split; integrations split by default | isolated daemon with only `[integrations.obsidian] enabled = true`: Shift+N → session `notes · vault` beside the shell in the same tab; `app::tests::notes_open_the_workspace_vault_once`; `config` test: `OpenIn` default `split`, default agent `hive notes .` |
| `hive context` | on this vault: `search worktree cleanup dirty` → ADR-0012:5 first; `get ADR-0012` → note + "Linked from"; `list -n 4`; `context::tests::entries_are_dated_and_signed`; CLI docs drift test covers every subcommand |
| `HIVE_VAULT` in sessions | isolated daemon: `echo $HIVE_VAULT` in a shell of the vault's space printed the vault path |
| Windows | `cargo clippy --target x86_64-pc-windows-gnu --workspace --all-targets -D warnings` clean |

Links: [[Roadmap]], [[Status]].
