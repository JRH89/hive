---
status: accepted
date: 2026-10-30
---
# ADR-0016: Agent detection from the foreground process and herdr manifests

Supersedes the convention "don't infer agent status from output" (CLAUDE.md, AGENTS.md conventions).

## Context
The user asked Hive to match herdr: recognise more agents (truss wasn't seen at all) and show working/blocked/done. Previously Hive knew only 5 hard-coded agents, and only by the name used at launch. herdr (Apache-2.0) has a maintained, tested set of per-agent screen rules ("manifests"), and a self-report CLI that agents such as truss already call.

## Decision
| Part | Choice |
|---|---|
| Crate | New `hive-agents`: `catalog` (27 agents: id, label, command, aliases, npm package hints, manifest), `process` (argv → agent, looking through node/bun/sh -c/env wrappers; `/proc` group scan), `manifest` (herdr-compatible engine, regions + gates, engine v3), `Tracker` (working→plain-idle needs 1 s to confirm) |
| Manifests | herdr v0.9.5 manifests vendored unmodified in `crates/hive-agents/manifests/` with `LICENSE-APACHE` and `NOTICE`; engine semantics reimplemented |
| Where | Daemon. An agent loop every 500 ms (`spawn_blocking`): foreground pgid from `MasterPty::process_group_leader()`, identify (cached per pgid, retried every 2 s while unknown), rules evaluated only when the screen changed or the tracker is settling. Responder captures OSC 0/2 title and OSC 9;4 progress |
| Protocol v5 | `SessionInfo.presence: Option<AgentPresence {agent, label, status, message, source}>`; `ReportAgent{session, agent, status, message, seq}` / `ReleaseAgent`. Presence is not persisted |
| Self-report | `hive report-agent` / `release-agent` (herdr-style flags accepted). Sessions get `HIVE_SESSION_ID` and `HIVE_BIN_PATH`. A report wins over detection and lapses when the foreground group changes |
| Agents without a manifest | Recognised, status `Unknown` (no guessing) |
| "Done" | Client-side: working → idle while not viewed; cleared on attach. Bell on done/blocked (`[notifications] bell`) |
| Discovery | `Config::agents()` = catalog + shell + `[agents.*]`. Picker and `hive doctor` show installed or configured ones; `hive agents [--all]` |

## Consequences
- Status is only as good as the manifests. Unknown agents show no status rather than a wrong one.
- Detection reads `/proc`, so it is Linux-only for now (macOS: no presence, everything else works).
- The TUI still mirrors one session (ADR-0007); status for background sessions comes from `SessionUpdated`, not mirroring.
- Manifests can be refreshed from herdr releases; `every_bundled_manifest_parses` guards engine compatibility.
