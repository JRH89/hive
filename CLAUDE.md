# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Orientation

Hive is a Rust TUI multiplexer for interactive AI coding agent CLIs. Binaries: `hive` (CLI and TUI client) and `hived` (daemon).

- `AGENTS.md` is the full product spec. Read the relevant section before non-trivial work.
- `vault/` is an Obsidian knowledge base and the project's working memory. **Start with `vault/00-Index/Status.md`**, then `Home.md`.

## Keeping the vault current (required)

- Every work session gets a dated note in `vault/Log/YYYY-MM-DD.md`: what was done, bugs found, notes. Append if the note already exists.
- An architectural decision gets a new `vault/Decisions/ADR-NNNN <title>.md` (Context / Decision / Consequences, with `status`/`date` frontmatter), linked from `Home.md`.
- Update `vault/00-Index/Status.md` and the checkboxes in `vault/Roadmap/` when phase progress changes.
- When behaviour diverges from `vault/Architecture/*`, update those notes in the same change.
- Use `[[wikilinks]]` between notes and keep notes terse (tables over prose).
- Record verification evidence, not claims. Phase notes list how each acceptance criterion was checked.

## Commands

```bash
cargo build                                            # builds target/debug/hive and hived
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p hive-daemon --test daemon <name>         # one PTY integration test
cargo test -p hive-tui app::tests::<name>              # one unit test
cargo test -p hive-cli --test cli                      # end-to-end tests of the hive binary
scripts/smoke.sh target/debug                          # smoke-test built binaries (isolated daemon)
cargo build --release --target x86_64-unknown-linux-musl   # static release build (ADR-0015)
```

CI is `.github/workflows/ci.yml`. A `v*` tag runs `release.yml`, which only ever creates a **draft** release. `hive doctor` is the first thing to ask a user for.

Run fmt, clippy and test after every meaningful change and fix failures before moving on.

**Running against an isolated daemon** (doesn't touch the user's real one): set `HIVE_RUNTIME_DIR` and `HIVE_STATE_DIR` to scratch dirs (keep the runtime dir path short: the socket path must fit in ~107 bytes). `hive` then auto-starts a private `hived` there. To drive the TUI headlessly, run it inside `tmux new-session -d -x W -y H ...` and inspect it with `tmux capture-pane -p` and `tmux send-keys`. Stop it with `hive daemon stop` afterwards. Logs: `$HIVE_STATE_DIR/hived.log`, verbosity via `HIVE_LOG=debug`.

## Architecture (big picture)

**Non-negotiable:** `hived` owns PTYs, child processes and the authoritative `vt100` emulator. The TUI only mirrors and forwards. Never spawn PTYs in the client.

- `hive-protocol`: Serde enums framed as u32-LE length + postcard (8 MiB cap). `Hello`/`Welcome` must stay enum variant 0 (a test enforces this).
- `hive-core`: `paths` (0700 runtime dir, env overrides), `config` (TOML, built-in agents overlaid by `[agents.*]`), `launch` (agent → argv/env/cwd plus PATH check), `client` (handshake, `recv_until`, daemon auto-start).
- `hive-agents`: agent catalog, argv-based process identification, herdr-compatible screen manifests (`manifests/`, Apache-2.0, see NOTICE) and the status `Tracker`.
- `hive-workspace`: git discovery by reading `.git`/`HEAD` (no subprocess) and the git argv calls for worktrees. Shared by the daemon, CLI and TUI.
- `hive-daemon`: `session.rs` runs 3 threads per session (PTY reader → emulator → `broadcast`, writer, waiter). `server.rs` handles per-connection dispatch and output forwarding. `responder.rs` answers DSR/DA/OSC 10/11 queries for the child.
- `hive-tui`: `app.rs` is a **pure** state machine (events in, `Vec<ClientMessage>` out), so unit-test it there. `render.rs` copies vt100 cells into a ratatui buffer. `keys.rs` encodes crossterm keys to PTY bytes. `layout.rs` holds tabs of split panes (pure geometry shared with render). `keymap.rs` maps keys to `Command`s (every leader action is a `Command`, dispatched by `App::run`; the palette and help are generated from it). `theme.rs` resolves a palette and glyph set from config and the environment (ADR-0013). `check_config` validates both before the terminal is taken over.

Invariants that span files:
- Attach snapshots (`state_formatted()`) and subscribes to the output broadcast **under the same `term` lock** the reader holds while publishing. Breaking this causes lost or duplicated output.
- Lock order is `term` before `meta`. Never hold a `std::sync::Mutex` or a `watch::Ref` across `.await`.
- Slow clients get `Resync` (a fresh snapshot) and never block the PTY. `broadcast::try_recv` repositions after `Lagged`, so resync immediately.
- Per-session broadcasts go only through `Session::publish()`/`remove()`, never `daemon.events.send(SessionUpdated(..))` directly. They are ordered under the meta lock so nothing can resurrect a removed session (ADR-0008).
- One connection attaches several sessions. The TUI mirrors exactly the panes of the active tab (`layout.rs`). Switching tabs detaches hidden panes (ADR-0018), so don't add background mirroring. A pane leaves the layout only when its session is removed, never on the reply to the TUI's own `Detach`.
- Control is per session: only the controller's `Input`/`Resize` (both name their session) are honoured. `Attach` takes control only if it's free or `force` is set (TUI Leader `a`); otherwise the client observes (ADR-0010). Take-over sends `ControlLost`; a controller leaving sends observers `ControlReleased`.
- Workspaces are daemon-owned. Every session gets one, resolved from its cwd's repository main root (ADR-0011). The daemon runs `git worktree add/remove`; dirty or in-use worktrees are never removed without force, and branches are never deleted (ADR-0012).
- `SessionCreated` (reply) can arrive after a `SessionUpdated` (broadcast) for the same session. Clients must not overwrite newer info with it.
- `sessions.json` (state dir) holds metadata only, never env. Sessions alive at a crash return as `SessionState::Lost`; graceful shutdown forgets them. Ids never repeat (ADR-0009).
- The reader thread must never block on the writer (`try_send` for query replies).
- `hive-protocol::read_frame` is not cancel-safe. Keep it in its own task, never in `select!`.

## Roadmap discipline

Phases are in AGENTS.md §16. Status is in `vault/00-Index/Status.md`. Don't start a phase until the previous one's acceptance criteria are verified with evidence. Put correctness before polish.

## Conventions

- Invoke agents directly from argv, never through a shell. Agents are data-driven via TOML.
- Agent status comes only from `hive-agents` manifests (herdr-compatible rules) or the agent's own `hive report-agent`. Agents without rules show `Unknown`, never a guess (ADR-0016). Detection runs in the daemon, not the TUI.
- The UI must work without Nerd Fonts, in 256/16-colour terminals and with `NO_COLOR`. Take colours and glyphs from `Theme`/`Glyphs`, never hard-code them. In ASCII mode `render::asciify` folds Hive's own cells but leaves the agent's output untouched. Amber is for accents only.
- New user actions get a `Command` variant (ADR-0014), not a hard-coded key.
- The sidebar shows spaces, then the selected space's agents (`App::shown_space`, ADR-0020). Clickable areas are rectangles render records in `sidebar_hits` each frame; add new clickable UI there.
- Optional integrations go under `[integrations.*]`, default off, and their commands are hidden via `app::hidden_commands` while disabled (ADR-0019). Build them from sessions and panes, not new daemon machinery.
- UI text uses plain terms (session, workspace, agent). Bee metaphors are for branding only.
- Integration tests use real PTYs with `/bin/sh -c` scripts. Wait for an output marker before sending signals or input, to avoid startup races.
- Tests must hold up on slow, loaded CI machines. Don't assume a throughput or a delay. Wait on an observable condition, e.g. bytes seen by a reading client. Replies and lifecycle broadcasts travel through different tasks and are unordered relative to each other, so fence on the events channel (e.g. a rename) rather than on a reply.
- Don't answer agents' trust or permission prompts during manual testing, because that writes to the user's real agent config.
- Never delete worktrees with uncommitted changes. Confirm destructive operations.
- Don't implement §18 future features (orchestration, plugins, remote daemons).
