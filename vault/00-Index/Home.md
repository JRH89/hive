# Hive

Knowledge base for building **Hive**, a terminal multiplexer for AI coding agents (one terminal, many agents). The authoritative product spec is `../AGENTS.md`. This vault records what was actually built, why, and what's next.

## Start here
- [[Status]]: where the project is right now (read first)
- [[Roadmap]]: phases and their acceptance criteria
- [[Phase 1 - Foundation]]: what shipped and how it was verified
- [[Phase 2 - Multiplexing]]: sidebar, switching, picker, rename
- [[Phase 3 - Robust Persistence]]: auto-reconnect, lost sessions, explicit control
- [[Phase 4 - Workspaces and Git]]: workspaces, branches, worktrees
- [[Phase 5 - Product Experience and Branding]]: themes, palette, keymap, brand

## Architecture
- [[Overview]]: processes, crates, data flow
- [[IPC Protocol]]: wire format and message semantics
- [[Session Lifecycle]]: states, attach/control, termination

## Decisions
- [[ADR-0001 Daemon owns PTYs]]
- [[ADR-0002 vt100 as emulator]]
- [[ADR-0003 postcard length-prefixed frames]]
- [[ADR-0004 Lagging clients resync from snapshot]]
- [[ADR-0005 Daemon answers terminal queries]]
- [[ADR-0006 Leader key Ctrl+Space]]
- [[ADR-0007 Switching re-attaches]]
- [[ADR-0008 Session events published under the session lock]]
- [[ADR-0009 Persist session metadata, mark lost sessions]]
- [[ADR-0010 Attach observes unless control is free or forced]]
- [[ADR-0011 Daemon-owned workspaces grouped by repository]]
- [[ADR-0012 Worktrees created by the daemon, never deleted dirty]]
- [[ADR-0013 Themes degrade by colour depth and glyph set]]
- [[ADR-0014 Configurable keymap of named commands]]

## Ongoing
- [[Website Design]]: approved website direction, branding, scope and acceptance
- [[Website Implementation Plan]]: isolated Next.js website work
- [[Risks and Open Questions]]
- `Log/`: dated work log, one note per session
