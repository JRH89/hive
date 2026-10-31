---
tags: [adr]
status: accepted
date: 2026-10-30
---
# ADR-0002: `vt100` (0.18) as the terminal emulator

**Options:** `vt100` (doy) vs `avt` (asciinema).

**Why vt100:**
- `Screen::state_formatted()` emits escape bytes that reproduce the whole screen and input modes. That is exactly the attach snapshot we need, so clients just replay bytes into their own parser and the protocol stays "bytes in, bytes out".
- A `Callbacks` trait with `unhandled_csi`/`unhandled_osc` lets the daemon answer terminal queries ([[ADR-0005 Daemon answers terminal queries]]).
- It exposes the modes the client needs: `application_cursor()`, `bracketed_paste()`, `hide_cursor()`, mouse modes.
- Per-cell access (`cell(row,col)`, wide-char flags, colours, attrs) maps straight onto a ratatui `Buffer`.
- `avt` is solid, but it is designed around asciinema's own serialisation. Its snapshot story would need more custom code.

**Notes on 0.18:** Sizes are `NonZeroU16`. `Parser::set_size` (not `Screen::set_size`) fires `on_scroll`. `on_scroll` enables streaming scrollback capture later.

**Revisit if:** we hit rendering bugs with a real agent that vt100 can't parse, or need features like the kitty graphics protocol.
