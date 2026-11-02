---
tags: [adr, phase-8]
status: accepted
date: 2026-11-01
---
# ADR-0024: Integrations toggle from the TUI and are saved to config.toml

**Context:** The user enabled Obsidian by hand, pressed `Ctrl+Space n` (next session) instead of `N`, and asked for "a checkbox in the menu or let me do /integrations and toggle them". Until now the TUI never wrote the config file, and integrations ([[ADR-0019 Optional integrations, off and invisible by default]]) applied only at startup.

**Decision:**
- New `Command::Integrations` (leader `I`, palette "integrations"): the palette in `Scope::Integrations` lists every `[integrations.*]` as `[x]`/`[ ]`. Enter toggles and keeps the list open.
- The toggle applies at once in the TUI: `config.integrations.<name>.enabled` flips, the keymap is rebuilt from `hidden_commands`, git counts are cleared when git status goes off.
- Saving is an `Effect::SaveIntegration`, so `app.rs` stays pure. The frontend edits the file text with `hive_core::config::set_integration_enabled` (line-based: replaces `enabled = …` in `[integrations.<name>]`, inserts the line or appends the section; comments and other keys are kept) and writes it via a temp file + rename.
- The edit is verified by re-parsing; a layout it can't edit (e.g. an inline `review = { … }`) is refused, never broken. Failures show as an error status; the toggle still applies until hive exits.
- Only `enabled` is written. Other settings stay hand-edited. No toml_edit dependency.

**Consequences:** The first config write from the TUI. Disabled-key hints now point at the menu instead of the TOML table (amends ADR-0019's amendment). Only the TUI process that toggles sees the change at once; other open `hive` windows pick it up on restart. The daemon reads no integration settings, so it needs nothing.
