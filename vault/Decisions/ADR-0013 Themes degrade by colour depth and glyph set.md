---
status: accepted
date: 2026-10-30
---
# ADR-0013: Themes degrade by colour depth and glyph set

## Context
Spec §5/§16 require a consistent identity that still works without Nerd Fonts, in 256-colour terminals, and on basic consoles. Before Phase 5, colours and glyphs were hard-coded in `render.rs`, and only a 256/truecolor split existed.

## Decision
| Piece | Rule |
|---|---|
| Palettes | Defined once in RGB (`hive-dark`, `hive-light`), plus `ansi` (the terminal's own 16 colours) and `mono` (no colour, emphasis via REVERSED/BOLD) |
| Depth | `NO_COLOR` or `TERM=dumb` → none; `COLORTERM` truecolor/24bit → truecolor; `TERM` containing `256color` → 256; else 16. RGB is mapped to the nearest xterm-256 or ANSI-16 colour. `[theme] colors` overrides |
| Glyphs | `UNICODE` (◇ ● ○ ✕ ◌ ▌ ▾ …, rounded borders) or `ASCII` (`<> * o x ? > - ~`, `+-|` borders). ASCII when `TERM=linux` or the locale isn't UTF-8; `[theme] glyphs` overrides. No Nerd Font glyphs in either |
| Agent cells | In ASCII mode, `render::asciify` folds Hive's own cells after drawing, but skips cells unchanged from the agent snapshot, so the agent's output is never rewritten |
| Overrides | `[theme] surface/accent/…` take `#rrggbb`; bad names or values fail before the UI opens (`check_config`) |

## Consequences
- Render code takes every colour and glyph from `Theme`. A hard-coded `Color::*` or Unicode glyph is a bug.
- Mono relies on reverse video for selection, so the selected row stays visible with no colour at all.
- The agent's own colours pass through unchanged (Hive does not re-map them to the palette).
