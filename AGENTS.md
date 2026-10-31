# HIVE
## One terminal. Many agents.

Codex contributors: also read [CODEX.md](CODEX.md) for repository orientation, concurrent-agent boundaries, verification, and required Obsidian updates. The specification below remains authoritative.

### Project specification, architecture, branding, and implementation plan

**Project:** Hive  
**Binary:** `hive`  
**Daemon:** `hived`  
**Language:** Rust  
**Interface:** Terminal User Interface (TUI)  
**Category:** AI coding agent multiplexer  
**Initial platform:** Linux  
**Design philosophy:** Lightweight, reliable, keyboard-first, extensible.

---

# 1. Project Vision

You are a senior Rust systems engineer specializing in terminal applications, pseudoterminals, asynchronous programming, process management, and developer tooling.

Design and implement **Hive**, a production-quality terminal multiplexer for managing multiple AI coding agents from a unified interface.

Hive should provide an experience comparable to Herdr while remaining an independent implementation with its own architecture, interface, branding, and capabilities.

Hive allows developers to launch, organize, monitor, switch between, detach from, and reattach to multiple AI coding agent sessions.

Supported agents initially include:

- Claude Code
- OpenAI Codex CLI
- Gemini CLI
- OpenCode
- Custom terminal commands

Agents must execute as real interactive terminal processes, not simulated conversations or API wrappers.

Hive is primarily a terminal multiplexer, not an autonomous orchestration platform.

Its architecture should accommodate future agent collaboration and orchestration without requiring those features in the initial release.

## Product Philosophy

**One terminal. Many agents.**

A developer should be able to open Hive, launch several coding agents, organize them by project, switch between them instantly, and leave them running without keeping the frontend open.

The experience should feel native to the terminal.

No browser, Electron runtime, cloud account, or external service should be required.

---

# 2. Brand Identity

Branding is a core part of the project, not something added after development.

Hive should have a recognizable identity across its CLI, TUI, documentation, GitHub repository, and eventual website.

## Name

**Hive**

The name represents a centralized environment containing multiple independent workers.

The metaphor maps naturally to the software:

| Hive concept | Software concept |
|---|---|
| Hive | Unified agent management environment |
| Workers | AI coding agents |
| Cells | Individual terminal sessions |
| Colonies | Project workspaces |
| Activity | Concurrent agent execution |
| Queen / coordinator | Potential future orchestration layer |

These concepts are branding inspiration, not mandatory user-facing terminology.

Prefer familiar technical terminology in the actual interface: sessions, workspaces, agents, terminals, and processes.

Do not force bee-related language into every feature.

## Tagline

**One terminal. Many agents.**

Supporting messaging:

- Your agents. One workspace.
- Keep your agents together.
- Manage the swarm.
- Multiple agents, zero chaos.
- A home for your coding agents.

The primary tagline should remain consistent across product materials.

## Visual Identity

Hive should have a modern, restrained developer-tool aesthetic.

Visual influences:

- Modern terminal applications
- Neovim
- Ghostty
- Lazygit
- Minimal Unix utilities
- Professional infrastructure dashboards

### Color Palette

Use a dark-first palette.

Suggested colors:

| Purpose | Color | Hex |
|---|---|---|
| Primary background | Deep charcoal | `#111318` |
| Secondary surface | Slate | `#1C2028` |
| Primary accent | Amber | `#F2B84B` |
| Secondary accent | Honey gold | `#D99A2B` |
| Primary text | Soft white | `#E8EAF0` |
| Muted text | Slate gray | `#8992A3` |
| Success | Muted green | `#79C99E` |
| Warning | Amber | `#E8AE55` |
| Error | Soft red | `#E57373` |

Use amber sparingly for selection, active sessions, important indicators, and branding.

Avoid excessive yellow backgrounds, decorative gradients, or unnecessary visual noise.

Colors must be configurable and degrade gracefully in terminals with limited color support.

## Logo Concept

The primary mark should be a minimal hexagonal symbol inspired by a honeycomb cell.

Potential directions:

1. A single outlined hexagon containing a terminal cursor.
2. Three connected hexagons representing concurrent sessions.
3. A hexagonal arrangement suggesting a network of workers.
4. A stylized H constructed from geometric cells.

Avoid cartoon bees, excessive honeycomb patterns, or childish illustrations.

The mark should remain recognizable at favicon and terminal-icon sizes.

Do not use Unicode glyphs that require Nerd Fonts as the only way to display the branding.

## CLI Identity

The command should be simple:

```bash
hive
```

Examples of the intended command vocabulary:

```bash
hive new claude
hive new codex
hive list
hive attach <session-id>
hive detach
hive status
hive daemon status
```

These are proposed CLI commands. Final command semantics must be defined and documented during implementation.

Do not add commands that are unnecessary for the initial release.

## Startup Experience

Display a compact branded startup header where appropriate:

```text
  ◇ HIVE
  One terminal. Many agents.
```

The startup experience must remain fast.

Avoid lengthy splash screens, animated introductions, or unnecessary delays.

---

# 3. Technology Stack

Use Rust and the following ecosystem:

| Component | Technology |
|---|---|
| Language | Rust, stable |
| TUI framework | Ratatui |
| Terminal input | Crossterm |
| Async runtime | Tokio |
| PTY management | portable-pty |
| Terminal emulation | vt100 or avt |
| Serialization | Serde |
| Configuration | TOML |
| Logging | tracing |
| Local IPC | Unix domain sockets |
| Git integration | Git CLI |
| Testing | Rust unit and integration tests |

Additional dependencies may be introduced when technically justified.

Prefer mature, maintained libraries.

Avoid unnecessary abstractions and dependencies.

Target Linux first, including terminal environments such as Ghostty, Kitty, and WezTerm.

Keep platform-specific code isolated for future Windows and macOS support.

---

# 4. Core Architecture — Mandatory From Day One

Hive must use a **daemon-first architecture from Phase 1**.

Do not build a monolithic TUI that owns child processes and attempt to extract a daemon later.

The architecture consists of three primary components.

## A. Hive TUI

The frontend is responsible for:

- Rendering the interface
- Processing keyboard and mouse events
- Displaying terminal screen buffers
- Managing navigation and focus
- Sending commands to the daemon
- Receiving terminal output and session updates
- Connecting and reconnecting to the daemon

The frontend must not own agent processes or PTYs.

Closing the TUI must not automatically terminate active agents.

## B. Hive Daemon (`hived`)

The daemon is responsible for:

- Launching agent processes
- Owning PTY master handles
- Managing process lifecycles
- Maintaining session metadata
- Reading and buffering terminal output
- Maintaining authoritative terminal screen state
- Processing terminal input
- Handling resize requests
- Tracking connected clients
- Managing session attachment
- Surviving frontend disconnections
- Graceful shutdown and cleanup

The daemon must operate independently of the frontend.

A frontend crash must not terminate agent processes.

## C. Shared IPC Protocol

Use a versioned local protocol over Unix domain sockets.

Support:

- Handshake and protocol negotiation
- Session creation
- Session listing
- Session attachment
- Session detachment
- Terminal input and output
- Terminal resizing
- Session status updates
- Session termination
- Error reporting
- State synchronization
- Reconnection

Use explicit Serde message types.

Implement bounded message framing and backpressure.

Use secure per-user runtime directories and socket permissions.

Handle stale sockets and concurrent startup attempts safely.

## Architectural Rule

**The daemon is the authoritative owner of agent processes and terminal state. The TUI is a client.**

This separation must exist from the first working prototype.

The daemon is not expected to preserve running processes across system reboot or its own termination.

---

# 5. Repository Structure

Use a Cargo workspace.

Suggested structure:

```text
hive/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── crates/
│   ├── hive-cli/
│   ├── hive-tui/
│   ├── hive-daemon/
│   ├── hive-core/
│   ├── hive-pty/
│   ├── hive-protocol/
│   ├── hive-agents/
│   └── hive-workspace/
├── assets/
│   └── branding/
├── docs/
├── examples/
└── tests/
```

Do not create empty crates simply to satisfy the proposed structure.

Each crate should have a meaningful responsibility.

Keep ownership boundaries clear and avoid circular dependencies.

---

# 6. Agent Management

Hive must support multiple interactive AI coding agents simultaneously.

Each agent session must have:

- Unique session identifier
- Agent type
- User-defined label
- Working directory
- Associated workspace
- Process state
- PTY dimensions
- Terminal buffer
- Creation timestamp
- Optional Git branch or worktree
- Attachment state

## Agent Adapters

Implement an extensible adapter system.

Each adapter should define:

- Executable detection
- Launch command
- Arguments
- Environment variables
- Working directory
- Optional native resume behavior
- Optional agent-specific metadata

Do not assume every agent exposes identical command-line arguments or capabilities.

Custom agent commands must be configurable without modifying Rust source code.

Avoid relying on fragile terminal-output parsing to determine agent status.

---

# 7. Real Terminal Emulation

Hive must render actual interactive terminal sessions.

Do not display raw stdout in a basic text widget.

Required functionality:

- ANSI escape sequences
- Cursor movement
- Foreground and background colors
- Text attributes
- Alternate screen buffers
- Screen clearing
- Terminal resizing
- Unicode and wide characters
- Interactive prompts
- Scrollback
- Mouse reporting where supported
- Bracketed paste where supported

Evaluate `vt100` and `avt` before choosing the emulator.

Document the selection.

Each session must maintain an independent terminal state.

Background sessions must continue processing output even when they are not visible.

The daemon should maintain the authoritative terminal state so reattached clients can receive a consistent snapshot.

Use bounded scrollback and output queues.

Define a predictable policy for terminal dimensions when no client is attached.

---

# 8. Hive TUI Design

The interface should resemble a polished terminal workspace manager.

## Primary Layout

```text
┌──────────────────────────────────────────────────────────┐
│ ◇ HIVE                        One terminal. Many agents. │
├──────────────────┬───────────────────────────────────────┤
│ WORKSPACES       │ CLAUDE — frontend                     │
│                  │                                       │
│ ▾ my-project     │ $ claude                              │
│   ● Claude       │                                       │
│   ● Codex        │ Working on authentication...          │
│   ○ Gemini       │                                       │
│                  │                                       │
│ ▾ another-repo   │                                       │
│   ● Codex        │                                       │
│                  │                                       │
├──────────────────┴───────────────────────────────────────┤
│ 3 active · 1 idle        Ctrl+P Commands · ? Help        │
└──────────────────────────────────────────────────────────┘
```

This is a conceptual wireframe, not a pixel-perfect requirement.

## Sidebar

Group sessions by workspace.

Show:

- Workspace name
- Agent name
- Session label
- Running/exited state
- Git branch
- Optional attention indicator

Use subtle status colors and clear selection highlighting.

Avoid displaying speculative statuses such as "thinking" or "waiting" unless reliable detection is available.

## Main Terminal Panel

Render the selected agent's interactive terminal.

The terminal must receive normal input when focused.

Support resizing, scrollback, and correct screen rendering.

## Status Bar

Display:

- Active workspace
- Selected agent
- Session count
- Daemon connection status
- Useful keyboard shortcuts

## Command Palette

Provide a searchable command interface.

Examples:

- New Session
- Switch Session
- Rename Session
- Terminate Session
- New Workspace
- Switch Workspace
- Detach
- Settings
- Help

## Empty State

When no sessions exist, display a simple branded welcome:

```text
          ◇ HIVE

    One terminal. Many agents.

    No active sessions.

    Press N to launch your first agent.
```

Keep branding restrained and functional.

---

# 9. Keyboard Navigation

Hive must be keyboard-first.

Implement configurable keybindings.

Use a leader-key system to prevent conflicts with interactive agent applications.

Support:

- New session
- Next/previous session
- Sidebar navigation
- Terminal focus
- Command palette
- Workspace switching
- Session termination
- Detach
- Help

Ensure that normal terminal input reaches the selected agent.

Allow literal control sequences to be forwarded when they conflict with Hive shortcuts.

Do not rely exclusively on key combinations that terminals cannot consistently distinguish.

---

# 10. Session Lifecycle

Implement explicit lifecycle states:

- Starting
- Running
- Exited
- Failed

Track attachment separately from process state.

A running session may have zero attached clients.

Support:

- Create
- List
- Attach
- Detach
- Rename
- Resize
- Inspect
- Terminate
- Remove exited session metadata

Graceful termination should be attempted before forced termination.

Use configurable termination timeouts.

Avoid accidentally killing unrelated processes.

---

# 11. Persistence and Reconnection

Hive must support persistent sessions while its daemon remains alive.

Expected workflow:

1. Run `hive`.
2. Launch Claude Code.
3. Launch Codex.
4. Switch between sessions.
5. Detach from Hive.
6. Leave both agents running.
7. Reopen Hive.
8. Reattach to the existing sessions.

Handle:

- Frontend crashes
- IPC disconnections
- Stale socket files
- Daemon startup races
- Protocol mismatches
- Slow consumers
- Multiple frontend connections
- Resize conflicts
- Reconnection state synchronization

Initially support one active input controller per session.

Additional clients may observe sessions, but input ownership must be explicit.

Do not claim live-process recovery after daemon termination or system reboot.

---

# 12. Workspace Management

Workspaces organize sessions by repository or project.

Support:

- Creating workspaces
- Naming workspaces
- Associating repository paths
- Grouping sessions
- Switching workspaces
- Remembering workspace metadata
- Displaying Git branches

## Git Worktrees

Provide optional worktree isolation.

Users should be able to launch separate agents in separate worktrees associated with the same repository.

Warn users when multiple agents share the same working directory.

Never silently delete worktrees containing uncommitted changes.

Require explicit confirmation for destructive Git operations.

---

# 13. Configuration

Use TOML configuration.

Default location on Linux:

```text
~/.config/hive/config.toml
```

Example:

```toml
[general]
theme = "hive-dark"
default_agent = "claude"
restore_workspace = true

[branding]
show_header = true
show_tagline = true

[terminal]
scrollback_lines = 10000
mouse_support = true

[daemon]
auto_start = true

[agents.claude]
command = "claude"

[agents.codex]
command = "codex"

[agents.gemini]
command = "gemini"

[agents.opencode]
command = "opencode"
```

Support configurable themes, keybindings, agent commands, and terminal behavior.

Use appropriate platform directories for configuration, runtime state, and logs.

---

# 14. Performance and Reliability

Hive must remain responsive with several concurrently active agents.

Requirements:

- Event-driven architecture
- No busy-loop polling
- Bounded memory usage
- Bounded output queues
- Backpressure handling
- Independent PTY readers
- Efficient terminal redraws
- Graceful client disconnections
- Correct process cleanup
- Structured logging
- Terminal restoration after errors where possible

A noisy agent must not freeze other sessions.

Do not silently discard output without a documented retention policy.

Avoid blocking operations on the TUI rendering path.

---

# 15. Security

Hive executes local processes with the user's permissions.

Requirements:

- No unnecessary privilege escalation
- No public network listener by default
- Restrict IPC access to the current user
- Avoid shell injection
- Prefer direct executable invocation
- Validate IPC messages
- Protect secrets from logs
- Confirm destructive actions
- Avoid unsafe automatic command approval

Do not bypass agent permission systems.

---

# 16. Implementation Roadmap

Build Hive incrementally.

Every phase must result in functional, tested software.

## Phase 1 — Daemon-First Foundation

**Goal:** Prove the complete architecture with one interactive agent session.

Implement:

1. Cargo workspace
2. Shared IPC protocol
3. Minimal daemon
4. PTY process launcher
5. Terminal emulator
6. Minimal Ratatui frontend
7. Keyboard input forwarding
8. Terminal output rendering
9. Terminal resizing
10. Basic detach and reattach

The daemon must own the PTY from the beginning.

### Acceptance Criteria

- Running `hive` starts or connects to the daemon.
- One installed agent CLI can be launched.
- Terminal output renders correctly.
- Interactive prompts work.
- Keyboard input is forwarded.
- Resizing works.
- Closing the TUI does not kill the agent.
- Reopening Hive reconnects to the live session.

Do not proceed until verified.

## Phase 2 — Multiplexing

**Goal:** Support multiple simultaneous agent sessions.

Implement:

- Multiple PTYs
- Independent terminal buffers
- Session switching
- Sidebar navigation
- Session labels
- Background output processing
- Session creation and termination
- Basic Hive visual branding

### Acceptance Criteria

Claude Code and Codex can run simultaneously.

Switching between them must not interrupt execution.

Both remain interactive.

Background output must not freeze the UI.

## Phase 3 — Robust Persistence

**Goal:** Make daemon lifecycle and reconnection reliable.

Implement:

- Automatic daemon startup
- State synchronization
- Reconnection recovery
- Stale socket cleanup
- Multiple client handling
- Explicit input ownership
- Graceful termination
- Session metadata persistence

### Acceptance Criteria

Frontend crashes do not kill agents.

Reconnection restores access to running sessions.

Multiple clients behave predictably.

## Phase 4 — Workspaces and Git

**Goal:** Organize agents by development project.

Implement:

- Workspace creation
- Repository discovery
- Session grouping
- Git branch display
- Optional worktree creation
- Worktree-aware launching
- Safe cleanup

### Acceptance Criteria

Users can manage multiple repositories.

Sessions appear under the correct workspace.

Agents can work in isolated worktrees.

## Phase 5 — Product Experience and Branding

**Goal:** Make Hive polished and recognizable.

Implement:

- Final Hive TUI theme
- Hexagonal brand mark
- Branded startup and empty states
- Command palette
- Configurable keybindings
- Theme support
- Help screen
- Session search
- Custom agent definitions
- README branding
- Terminal screenshots
- CLI help text

### Acceptance Criteria

Hive has a consistent visual identity without sacrificing terminal usability.

The interface must remain usable without Nerd Fonts or advanced terminal color support.

## Phase 6 — Testing and Distribution

**Goal:** Prepare for public release.

Implement:

- Unit tests
- IPC tests
- PTY lifecycle integration tests
- Detach/reattach tests
- Concurrent session stress tests
- Terminal resize tests
- CI pipeline
- Linux release binaries
- Installation instructions
- User documentation
- Troubleshooting guide

Investigate Windows and macOS support after the Linux implementation is stable.

---

# 17. README and Public Presentation

Create a professional README.

Suggested structure:

```text
◇ HIVE
One terminal. Many agents.

A lightweight, Rust-powered terminal
multiplexer for AI coding agents.
```

The README should contain:

1. Product overview
2. Terminal screenshot
3. Key features
4. Installation
5. Quick start
6. Supported agents
7. Keyboard shortcuts
8. Configuration
9. Architecture overview
10. Development instructions
11. Roadmap
12. License

Use the same visual identity as the application.

Do not claim unsupported features are implemented.

Clearly distinguish current capabilities from planned features.

## Website Direction

A future landing page should use:

- Dark background
- Amber accents
- Large terminal screenshot
- Minimal geometric hexagonal branding
- Clear feature explanations
- Fast-loading, developer-focused design

Suggested hero:

**HIVE**

**One terminal. Many agents.**

Run Claude, Codex, Gemini, and more from one lightweight terminal workspace.

Manage sessions, switch instantly, and keep your agents running.

Avoid exaggerated AI marketing language.

Focus on what the tool actually does.

---

# 18. Future Capabilities

Design for future extensibility, but do not implement these in the initial release:

- Agent-to-agent messaging
- Shared task queues
- Agent coordination
- Dependency-aware task scheduling
- Shared context
- Agent activity detection
- Approval dashboards
- Git conflict detection
- Performance metrics
- Remote daemon connections
- Distributed execution
- Plugin system

A future coordination layer could be branded around the hive metaphor, but it must remain separate from the core multiplexer.

The initial product must not depend on autonomous orchestration.

---

# 19. Engineering Standards

Follow these rules throughout development:

1. Inspect the repository before modifying code.
2. Explain major architectural decisions.
3. Implement the smallest functional increment.
4. Prefer straightforward, testable Rust.
5. Avoid unnecessary traits and generic frameworks.
6. Do not generate placeholder features and call them complete.
7. Run `cargo fmt`, `cargo clippy`, and `cargo test` after meaningful changes.
8. Resolve failures before moving forward.
9. Keep documentation synchronized with implementation.
10. Preserve working functionality.
11. Do not fabricate test results.
12. Do not perform destructive operations without authorization.

Use real PTYs and simple test commands for integration testing before relying exclusively on installed AI agent CLIs.

---

# 20. Initial Release Definition of Done

A successful initial release must allow a developer to:

1. Install Hive as a native binary.
2. Run `hive`.
3. Launch Claude Code.
4. Launch Codex in another session.
5. View sessions in a sidebar.
6. Switch between sessions.
7. Interact normally with each agent.
8. Keep background sessions running.
9. Detach from the TUI.
10. Reopen Hive and reattach.
11. Organize sessions by workspace.
12. Optionally isolate agents using Git worktrees.
13. Terminate sessions independently.

The application must use real PTYs and actual agent CLIs.

It must not be a simulated terminal, mock dashboard, or API-based chat interface.

---

# 21. Starting Instructions

Begin by:

1. Reviewing this specification.
2. Proposing the final architecture.
3. Identifying appropriate crate boundaries.
4. Evaluating terminal emulation libraries.
5. Identifying technical risks around PTYs, IPC, backpressure, and persistence.
6. Creating the Cargo workspace.
7. Implementing Phase 1 only.
8. Running tests and verifying acceptance criteria.

Do not implement later phases until Phase 1 is functional.

Do not prioritize visual polish over correctness during the foundational phases.

However, establish Hive's naming conventions, CLI identity, and basic theme tokens from the beginning so branding does not require a major redesign later.

**Non-negotiable architectural requirement: The daemon owns the PTYs and agent processes from day one. The TUI is always a client.**

**Non-negotiable product requirement: Hive must feel like a fast, reliable, native terminal tool—not a web application disguised as a TUI.**

**HIVE — One terminal. Many agents.**
