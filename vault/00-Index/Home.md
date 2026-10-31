# Hive

Knowledge base for building **Hive**, a terminal multiplexer for AI coding agents (one terminal, many agents). The authoritative product spec is `../AGENTS.md`. This vault records what was actually built, why, and what's next.

## Start here
- [[Status]]: where the project is right now (read first)
- [[Roadmap]]: phases and their acceptance criteria
- [[Phase 1 - Foundation]]: what shipped and how it was verified
- [[Phase 2 - Multiplexing]]: sidebar, switching, picker, rename

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

## Ongoing
- [[Risks and Open Questions]]
- `Log/`: dated work log, one note per session
