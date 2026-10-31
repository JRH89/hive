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
| `hive-tui` | `app` (pure state machine returning `ClientMessage`s), `render`, `keys` (crossterm → PTY bytes), `theme` |
| `hive-cli` | bin `hive`: clap subcommands, launches the TUI |

No `hive-pty`, `hive-agents` or `hive-workspace` crates yet. Spec §5 says not to create empty crates. Split them out when Phase 2 and 4 code gives them a real job.

## Data flow: output
1. A per-session **reader thread** does a blocking `read()` on the PTY master (16 KiB).
2. Under the session's `term` lock it feeds the bytes into the daemon's `vt100::Parser<Responder>` and publishes `Chunk::Output` on a bounded `broadcast` (256 chunks).
3. Any query replies (DSR, DA, OSC 10/11) are `try_send`'d to the writer. See [[ADR-0005 Daemon answers terminal queries]].
4. Per attached client, a **forwarder task** coalesces queued chunks (up to 64 KiB) into one `Output` frame and pushes it to the client's bounded out-queue (64 messages, so ≤ 4 MiB). A writer task then writes the frames to the socket.
5. The TUI's reader task decodes frames into an mpsc. The main loop applies queued messages for up to 12 ms (`DRAIN_BUDGET`), then redraws and polls input again.

The client mirrors **only the attached session**. Switching re-attaches and gets a fresh snapshot ([[ADR-0007 Switching re-attaches]]), so background sessions cost the frontend nothing.

Attach takes the `state_formatted()` snapshot **and** subscribes under the same `term` lock. That gives no gap and no duplicate. See [[ADR-0004 Lagging clients resync from snapshot]] for the slow-consumer case.

## Data flow: input
Key event → `keys::encode` (honours DECCKM from the client mirror) → `Input` → `Session::input` (only from the controller) → bounded mpsc → **writer thread** → PTY. A full queue makes the client's send await, which is backpressure rather than unbounded buffering.

## Threads vs async
PTY I/O and `child.wait()` are blocking, so each session has 3 OS threads (read, write, wait). Everything socket-facing is tokio. `std::sync::Mutex` is never held across `.await`. Lock order is `term` then `meta`.

## Filesystem
| What | Where | Override |
|---|---|---|
| socket, lock | `$XDG_RUNTIME_DIR/hive/` (0700), socket 0600 | `HIVE_RUNTIME_DIR` |
| logs `hived.log`, `hive.log` | `$XDG_STATE_HOME/hive/` | `HIVE_STATE_DIR` |
| config | `~/.config/hive/config.toml` | `HIVE_CONFIG` |
| daemon binary | sibling of `hive`, then `$PATH` | `HIVE_DAEMON_BIN` |

Log level is set with `HIVE_LOG` (an EnvFilter string).
