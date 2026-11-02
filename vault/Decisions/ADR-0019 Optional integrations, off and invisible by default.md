---
tags: [adr, phase-7]
status: accepted
date: 2026-10-30
---
# ADR-0019: Optional integrations are off and invisible by default

**Context:** The user asked for an Obsidian integration, "optional… not cluttering people's UI by default unless it's something they use". Earlier idea: an MCP server. Rejected: agents can already use Obsidian MCP servers themselves, and Hive only needs to find a vault and open it.

**Decision:**
- Integrations live under `[integrations.<name>]` in the config. `enabled = false` is the default.
- A disabled integration's commands are **hidden**: `app::hidden_commands(config)` drops them from the keymap (so their default keys can't clash with the user's bindings), the palette and help.
- An integration is built from existing parts. Obsidian = vault discovery (`hive_workspace::find_obsidian_vault`) plus a normal session (`agent = "notes"`, cwd = vault) placed in a tab or split. No new protocol, no daemon code, no plugin API. That keeps it clear of the §18 "plugins" non-goal.
- The program is data-driven: `agent = ""` → `$VISUAL`/`$EDITOR` (split on whitespace, no shell) → `vi`. Any agent name also works, e.g. `claude` working in the vault.

| Obsidian behaviour | Rule |
|---|---|
| Vault lookup | `.obsidian/` at the session cwd or an ancestor up to the workspace root, else the first non-hidden direct subdirectory of the root |
| Repeat | a running `notes` session in that vault gets focused instead of a second one |
| Bad config | `open_in` is an enum (parse error); an unknown `agent` fails `check_config`/`hive doctor` |

**Consequences:** Future integrations ([[Phase 7 - herdr parity]] ideas) follow the same shape: config table, hidden commands, built from sessions and panes.

**Amendment (2026-10-31):** a disabled command's leader key is no longer silent. It shows a status hint naming the config table to enable (`Keymap::disabled`, `enable_hint`). The palette and help still hide it. Reason: user pressed `Ctrl+Space U` with no config and thought it was broken.

**Amendment (2026-11-01):** integrations can be switched from the TUI (`Ctrl+Space I`), saved to the config; the hint now points there. See [[ADR-0024 Integrations toggle from the TUI]].
