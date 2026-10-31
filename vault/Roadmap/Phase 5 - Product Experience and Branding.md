---
tags: [roadmap, phase-5]
status: done
completed: 2026-10-30
branch: phase-5-polish
---
# Phase 5: Product Experience and Branding ✅

Goal (spec §16): a consistent visual identity without sacrificing terminal usability.

## Acceptance criteria and evidence
| Criterion | Evidence |
|---|---|
| Consistent visual identity | Manual tmux run (100×28, isolated daemon, 4 shell sessions in scratch repos `webapp`/`api`): header `◇ HIVE`, amber accents, grouped sidebar, `Ctrl+Space leader · then p commands` hint. The palette, help, workspace picker and session search all use the same popup style. Empty state (fresh daemon) shows the ASCII hex mark, `◇ HIVE`, the tagline and "Press n to launch an agent". Captures in the README |
| Usable without Nerd Fonts | Both glyph sets avoid Nerd Font glyphs. `TERM=linux NO_COLOR=1`: `cut -c1-26` of the capture has no non-ASCII characters, borders are `|`, markers `* > -`. Tests `ascii_glyphs_and_mono_render_without_unicode_or_colour`, `ascii_mode_leaves_the_agents_own_output_alone` |
| Usable without advanced colour | `tmux capture-pane -e`: theme `mono` and `NO_COLOR` emit **no** `38;`/`48;` codes. `hive-light` at `TERM=xterm-256color` emits only indexed `38;5;N`/`48;5;N`. Theme unit tests cover depth detection, nearest_256 (`#1c2028`→235) and nearest_16 (amber→Yellow) |
| Configurable keys | Config `leader="ctrl+a"`, `new_session="c"`, `add_workspace="W"`: status hint reads `Ctrl+A leader`, help lists `c New session`, `W Add workspace`, `Ctrl+A Send Ctrl+A to the agent`. Test `custom_leader_and_bindings_from_config` |
| Config errors are clear | `hive` exits 1 before taking the terminal: `leader = "x"` → "would swallow normal typing…", `accent = "#zzz"` → "is not a #rrggbb colour", `new_session = "p"` → "`p` is bound to both palette and new_session". `examples/config.toml` is validated by test `example_config_is_valid` |
| Palette and search | Leader `p`, typing `bill`, Enter → switched to `#3 billing`. Leader `/` + `mig` lists only sessions. Tests `palette_finds_and_runs_commands`, `session_search_switches_to_the_match`, `fuzzy_prefers_word_starts_and_runs` |

Gate: `cargo test --workspace` **96 tests**, clippy (`-D warnings`) and fmt clean.

## What shipped
- `theme.rs` rewrite: 4 themes, depth and glyph detection, `[theme]` overrides ([[ADR-0013 Themes degrade by colour depth and glyph set]]).
- `keymap.rs`: `Command`, `KeySpec`, `Keymap`, `[keys]` config ([[ADR-0014 Configurable keymap of named commands]]).
- Command palette (Leader `p`), session search (Leader `/`), generated help, Settings command (shows the config path).
- Brand: `brand::MARK_ART` hex mark in the empty state, `assets/branding/hive-mark.svg`.
- `examples/config.toml`, README branding/screenshots/config docs, clap `after_help` examples.
- `hive_tui::check_config`, called by the CLI before the TUI starts.

## Bugs found and fixed
- The ASCII render test caught Unicode in status and detail strings (`·`, `…`, `—`, arrows). Fixed generically with `asciify` instead of per-string branching.
- Fuzzy search ranked "a pile" above "billing api" for `api`. Added a substring bonus and capped the gap penalty.
- Help popup cut off the sidebar row ("esc bac") at 76 columns. Text shortened.

## Not done / carried forward
- Sidebar and picker keys are not configurable.
- No live theme switching (theme is read at startup).
- Long worktree paths outside `$HOME` still truncate in the header.
- Terminal screenshots are text captures. No PNG/GIF recordings yet.
