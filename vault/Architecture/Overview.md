---
tags: [architecture]
---
# Architecture Overview

```
 hive (TUI / CLI)  ──unix socket──▶  hived
   vt100 mirror                       ├─ Session 1: PTY + child + vt100 (authoritative)
   ratatui render                     ├─ Session 2: …
   key → bytes                        └─ clients registry, event broadcast
```

The rule (spec §4): **hived owns processes, PTYs and terminal state. The TUI is a client.** See [[ADR-0001 Daemon owns PTYs]].

## Crates
| Crate | Role |
|---|---|
| `hive-protocol` | Serde message enums and the u32-LE + postcard framing. No I/O beyond `AsyncRead`/`AsyncWrite`. See [[IPC Protocol]] |
| `hive-core` | `brand` constants, `paths` (runtime/state/config dirs with 0700 enforcement), `config` (TOML and built-in agents), `launch` (agent → `CreateSession`, PATH check), `client` (connect, handshake, auto-start `hived`) |
| `hive-daemon` | lib: `run()`, `Daemon` registry, `server` (per-connection), `session` (PTY threads), `responder` (terminal query replies). bin: `hived` |
| `hive-workspace` | git discovery from `.git` files (`discover`, `read_branch`, `workspace_key`, `best_match`) and the git subprocess calls for worktrees. No async, no protocol types |
| `hive-tui` | `app` (pure state machine returning `ClientMessage`s), `render`, `keys` (crossterm → PTY bytes), `keymap` (`Command`s and bindings, [[ADR-0014 Configurable keymap of named commands]]), `theme` (palette + glyphs by depth, [[ADR-0013 Themes degrade by colour depth and glyph set]]) |
| `hive-cli` | bin `hive`: clap subcommands, launches the TUI |

No `hive-pty` or `hive-agents` crates yet. Spec §5 says not to create empty crates. `hive-workspace` was split out in Phase 4 once it had a real job.

## Data flow: output
1. A per-session **reader thread** does a blocking `read()` on the PTY master (16 KiB).
2. Under the session's `term` lock it feeds the bytes into the daemon's `vt100::Parser<Responder>` and publishes `Chunk::Output` on a bounded `broadcast` (256 chunks).
3. Any query replies (DSR, DA, OSC 10/11) are `try_send`'d to the writer. See [[ADR-0005 Daemon answers terminal queries]].
4. Per attached client, a **forwarder task** coalesces queued chunks (up to 64 KiB) into one `Output` frame and pushes it to the client's bounded out-queue (64 messages, so ≤ 4 MiB). A writer task then writes the frames to the socket.
5. The TUI's reader task decodes frames into an mpsc. The main loop applies queued messages for up to 12 ms (`DRAIN_BUDGET`), then redraws and polls input again.

The client mirrors **only the attached session**. Switching re-attaches and gets a fresh snapshot ([[ADR-0007 Switching re-attaches]]), so background sessions cost the frontend nothing.

Attach takes the snapshot (`session::snapshot`: `state_formatted()` plus `?1049h` when the alternate screen is active) **and** subscribes under the same `term` lock. That gives no gap and no duplicate. See [[ADR-0004 Lagging clients resync from snapshot]] for the slow-consumer case. vt100 can't expose the primary grid hidden behind the alternate screen, so the reader publishes `Chunk::Snapshot` (sent to clients as `Resync`) when an app leaves the alternate screen. A client that attached mid-app then gets the restored primary screen.

## Data flow: input
Key event → `keys::encode` (honours DECCKM from the client mirror) → `Input` → `Session::input` (only from the controller) → bounded mpsc → **writer thread** → PTY. A full queue makes the client's send await, which is backpressure rather than unbounded buffering.

## Threads vs async
PTY I/O and `child.wait()` are blocking, so each session has 3 OS threads (read, write, wait). Everything socket-facing is tokio. `std::sync::Mutex` is never held across `.await`. Lock order is `term` then `meta`.

## Filesystem
| What | Where | Override |
|---|---|---|
| socket, lock | `$XDG_RUNTIME_DIR/hive/` (0700), socket 0600 | `HIVE_RUNTIME_DIR` |
| logs `hived.log`, `hive.log`; `sessions.json` (0600, metadata only) | `$XDG_STATE_HOME/hive/` | `HIVE_STATE_DIR` |
| config | `~/.config/hive/config.toml` | `HIVE_CONFIG` |
| worktrees | `$XDG_DATA_HOME/hive/worktrees/<repo>-<hash>/<branch>` | `[worktrees] dir` |
| daemon binary | sibling of `hive`, then `$PATH` | `HIVE_DAEMON_BIN` |

Log level is set with `HIVE_LOG` (an EnvFilter string). The runtime dir path must fit a Unix socket path (about 107 bytes), so a deep `HIVE_RUNTIME_DIR` fails with "path must be shorter than SUN_LEN".

## Reconnect
The TUI's main loop runs in phases: `connected` until the link drops, then `reconnect` (backoff 100 ms → 2 s, auto-starting `hived` at most every 2 s). On success it re-lists and re-attaches the preferred session with `force: false`. If the daemon pid changed, it warns that sessions ended (they appear `Lost`).
