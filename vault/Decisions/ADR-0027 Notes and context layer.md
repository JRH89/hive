---
tags: [adr, phase-11]
status: accepted
date: 2026-11-02
---
# ADR-0027: Notes and context layer (Phase 11)

**Context:** The Obsidian integration only ran `$EDITOR` on the vault. On the user's machine that opened VS Code outside Hive and left an empty tab. The user asked for a lazygit-style notes app inside Hive that works with Obsidian, "and also our own context retrieval thing built straight into hive". AGENTS.md §18 had "Shared context" as a future feature; the owner moved it into Phase 11.

**Decision:**
- A new crate, `hive-notes`, holds vault scanning, parsing (title, tags, wikilinks), Obsidian-style link resolution, backlinks and BM25-style search. The CLI and the TUI share it. No daemon involvement: the markdown files are the only state, read fresh on each run.
- `hive notes` is a program, like lazygit, run in a pane, so the PTY rule still holds: the daemon owns the process, and the TUI mirrors it. Leader `N` launches it via `current_exe() notes`; `[integrations.obsidian] agent` still overrides it.
- `hive context` is the agent interface. Agents already run shell commands (`hive report-agent`), so retrieval is a command with plain output, not a protocol or server.
- Writes only append or create (`context add`, new or daily notes). No index file, so there's nothing to go stale.

**Consequences:**
- Works with no Obsidian installed; Obsidian sees the same files.
- Search is lexical. Embeddings can come later behind the same commands.
- `OpenIn` defaults to `split` for every integration, notes included.
