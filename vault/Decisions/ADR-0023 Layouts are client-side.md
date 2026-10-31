---
status: accepted
date: 2026-10-31
---
# ADR-0023 Layouts are client-side

## Context
User asked for "customizable layouts per space", optional and off by default. Options: daemon-stored layouts (new protocol, persistence) or config-defined presets the TUI opens.

## Decision
| Aspect | Choice |
|---|---|
| Where | `[layouts.<name>]` in config; no protocol change |
| Shape | `spaces` (names where it opens without asking) + `panes` (`agent` default `shell`, or `run` argv; `split` right/down from the previous pane; `label`) |
| Opening | `Ctrl+Space L` (`Command::Layout`, hidden while no layouts, [[ADR-0019 Optional integrations, off and invisible by default]]): the space's layout, else the only one, else palette `Scope::Layouts` |
| Creation | TUI sends the first `CreateSession` (`Place::NewTab`), queues the rest; each `SessionCreated` pops the next with `Place::Split`. An `Error` clears the queue |
| Validation | `check_config`: empty `panes` or unknown agent stops `hive` before the terminal is taken |

## Consequences
- Built from ordinary sessions and panes: the daemon stays unaware, a layout's panes behave like any other.
- Not restored automatically: opening is explicit. Re-running it opens a second copy.
- `run` panes use agent `layout`; with the other tool panes they are excluded from `is_agent` (AGENTS list, shared-directory warning).
