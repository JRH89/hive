//! One agent process on its own PTY, plus the daemon's authoritative emulator
//! for it.
//!
//! Threads per session (all blocking I/O stays off the async runtime):
//! - reader: PTY output -> emulator -> output broadcast
//! - writer: input channel -> PTY
//! - waiter: blocks on the child's exit and publishes the new state
//!
//! The reader feeds the emulator and publishes to the broadcast while holding
//! the `term` lock. Attaching takes the snapshot and subscribes under the same
//! lock, so a client never misses or duplicates output.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::num::NonZeroU16;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use hive_protocol::{CreateSession, ServerMessage, SessionId, SessionInfo, SessionState, TermSize};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use rustix::process::{Pid, Signal};
use tokio::sync::{broadcast, mpsc, watch};

use crate::responder::Responder;

pub(crate) type ClientId = u64;

/// Chunks of PTY output queued per session. With 16 KiB reads this bounds
/// per-session buffering to ~4 MiB; slower clients are resynced from a
/// snapshot instead of stalling the PTY.
const OUTPUT_QUEUE: usize = 256;
const INPUT_QUEUE: usize = 256;
const READ_BUF: usize = 16 * 1024;

#[derive(Clone, Debug)]
pub(crate) enum Chunk {
    Output(Arc<[u8]>),
    Resized(TermSize),
}

struct Meta {
    label: String,
    state: SessionState,
    size: TermSize,
    controller: Option<ClientId>,
    attached: BTreeSet<ClientId>,
    /// Set once by `remove`. Checked under this lock before publishing so no
    /// `SessionUpdated` can follow the `SessionRemoved` broadcast.
    removed: bool,
}

struct Term {
    parser: vt100::Parser<Responder>,
    master: Box<dyn MasterPty + Send>,
}

pub(crate) struct Session {
    pub id: SessionId,
    agent: String,
    command: Vec<String>,
    cwd: String,
    created_at: u64,
    pid: Option<u32>,
    meta: Mutex<Meta>,
    term: Mutex<Term>,
    output: broadcast::Sender<Chunk>,
    input: mpsc::Sender<Vec<u8>>,
    exited: watch::Sender<bool>,
    removed: watch::Sender<bool>,
    /// Daemon-wide lifecycle broadcast. Only `publish`/`remove` send on it.
    events: broadcast::Sender<ServerMessage>,
}

pub(crate) struct Attachment {
    pub info: SessionInfo,
    pub snapshot: Vec<u8>,
    pub output: broadcast::Receiver<Chunk>,
    /// Whether the attaching client holds input control.
    pub controller: bool,
    pub previous_controller: Option<ClientId>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic while holding a session lock is a bug, but should not take the
    // whole daemon (and every other session) down with it.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn nz(v: u16) -> NonZeroU16 {
    NonZeroU16::new(v.max(1)).expect("clamped to >= 1")
}

fn pty_size(size: TermSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

impl Session {
    pub fn spawn(
        id: SessionId,
        req: CreateSession,
        scrollback: usize,
        events: broadcast::Sender<ServerMessage>,
    ) -> anyhow::Result<Arc<Session>> {
        let Some(program) = req.command.first() else {
            bail!("empty command");
        };
        let size = req.size.clamped();
        let pair = native_pty_system()
            .openpty(pty_size(size))
            .context("failed to open a pseudoterminal")?;

        let mut cmd = CommandBuilder::new(program);
        cmd.args(&req.command[1..]);
        cmd.cwd(&req.cwd);
        cmd.env_clear();
        for (k, v) in &req.env {
            cmd.env(k, v);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("HIVE", "1");
        cmd.env("HIVE_SESSION_ID", id.to_string());

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("failed to launch `{program}`"))?;
        // Our copy of the slave must close so the reader sees EOF once the
        // child (and anything it spawned) exits.
        drop(pair.slave);

        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let (input_tx, input_rx) = mpsc::channel(INPUT_QUEUE);
        let (output_tx, _) = broadcast::channel(OUTPUT_QUEUE);

        let session = Arc::new(Session {
            id,
            agent: req.agent,
            command: req.command.clone(),
            cwd: req.cwd,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            pid: child.process_id(),
            meta: Mutex::new(Meta {
                label: req.label,
                state: SessionState::Running,
                size,
                controller: None,
                attached: BTreeSet::new(),
                removed: false,
            }),
            term: Mutex::new(Term {
                parser: vt100::Parser::new_with_callbacks(
                    nz(size.rows),
                    nz(size.cols),
                    scrollback,
                    Responder::default(),
                ),
                master: pair.master,
            }),
            output: output_tx,
            input: input_tx,
            exited: watch::channel(false).0,
            removed: watch::channel(false).0,
            events,
        });

        let s = session.clone();
        std::thread::Builder::new()
            .name(format!("hive-pty-read-{id}"))
            .spawn(move || s.read_loop(reader))?;
        std::thread::Builder::new()
            .name(format!("hive-pty-write-{id}"))
            .spawn(move || write_loop(writer, input_rx))?;
        let s = session.clone();
        std::thread::Builder::new()
            .name(format!("hive-wait-{id}"))
            .spawn(move || {
                let state = match child.wait() {
                    Ok(status) => SessionState::Exited {
                        code: status.signal().is_none().then(|| status.exit_code()),
                        signal: status.signal().map(str::to_owned),
                    },
                    Err(e) => SessionState::Failed {
                        reason: format!("wait failed: {e}"),
                    },
                };
                tracing::info!(session = id, ?state, "session process ended");
                lock(&s.meta).state = state;
                s.exited.send_replace(true);
                s.publish();
            })?;

        Ok(session)
    }

    fn read_loop(&self, mut reader: Box<dyn Read + Send>) {
        let mut buf = vec![0u8; READ_BUF];
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                // EIO is the normal Linux signal that the slave side closed.
                Err(_) => break,
            };
            let replies = {
                let mut term = lock(&self.term);
                term.parser.process(&buf[..n]);
                // No receivers is fine: background sessions keep their state.
                let _ = self.output.send(Chunk::Output(Arc::from(&buf[..n])));
                term.parser.callbacks_mut().take()
            };
            if !replies.is_empty() && self.input.try_send(replies).is_err() {
                tracing::warn!(
                    session = self.id,
                    "dropped terminal query reply; input queue full"
                );
            }
        }
        tracing::debug!(session = self.id, "pty reader finished");
    }

    pub fn info(&self) -> SessionInfo {
        self.info_locked(&lock(&self.meta))
    }

    fn info_locked(&self, meta: &Meta) -> SessionInfo {
        SessionInfo {
            id: self.id,
            agent: self.agent.clone(),
            label: meta.label.clone(),
            command: self.command.clone(),
            cwd: self.cwd.clone(),
            state: meta.state.clone(),
            size: meta.size,
            created_at: self.created_at,
            attached_clients: meta.attached.len() as u32,
        }
    }

    /// Sets the label, trimmed and capped. Empty resets to the agent name.
    pub fn rename(&self, label: &str) {
        const MAX_LABEL: usize = 64;
        let label: String = label.trim().chars().take(MAX_LABEL).collect();
        lock(&self.meta).label = if label.is_empty() {
            self.agent.clone()
        } else {
            label
        };
    }

    pub fn is_alive(&self) -> bool {
        lock(&self.meta).state.is_alive()
    }

    /// Lock order: `term` before `meta`.
    fn resize_locked(&self, term: &mut Term, size: TermSize) {
        let size = size.clamped();
        let mut meta = lock(&self.meta);
        if meta.size == size || !meta.state.is_alive() {
            return;
        }
        if let Err(e) = term.master.resize(pty_size(size)) {
            tracing::warn!(session = self.id, "pty resize failed: {e}");
            return;
        }
        term.parser.set_size(nz(size.rows), nz(size.cols));
        meta.size = size;
        let _ = self.output.send(Chunk::Resized(size));
    }

    /// Attaches `client`. It takes input control (and the PTY is resized to
    /// its viewport) when control is free or `force` is set; otherwise it
    /// observes at the current size. A displaced controller is returned so
    /// the caller can notify it.
    pub fn attach(&self, client: ClientId, size: TermSize, force: bool) -> Attachment {
        let (snapshot, output, controller, previous_controller) = {
            let mut term = lock(&self.term);
            let (controller, previous) = {
                let mut meta = lock(&self.meta);
                meta.attached.insert(client);
                let free = meta.controller.is_none_or(|c| c == client);
                if force || free {
                    (
                        true,
                        meta.controller.replace(client).filter(|&c| c != client),
                    )
                } else {
                    (false, None)
                }
            };
            if controller {
                self.resize_locked(&mut term, size);
            }
            let snapshot = term.parser.screen().state_formatted().into_bytes();
            (snapshot, self.output.subscribe(), controller, previous)
        };
        Attachment {
            info: self.info(),
            snapshot,
            output,
            controller,
            previous_controller,
        }
    }

    /// Fresh snapshot and receiver for a client that lagged behind.
    pub fn resync(&self) -> (TermSize, Vec<u8>, broadcast::Receiver<Chunk>) {
        let term = lock(&self.term);
        let size = lock(&self.meta).size;
        let snapshot = term.parser.screen().state_formatted().into_bytes();
        (size, snapshot, self.output.subscribe())
    }

    /// Detaches `client`. If it held control, returns the observers that
    /// should be told control is now free.
    pub fn detach(&self, client: ClientId) -> Vec<ClientId> {
        let mut meta = lock(&self.meta);
        meta.attached.remove(&client);
        if meta.controller == Some(client) {
            meta.controller = None;
            meta.attached.iter().copied().collect()
        } else {
            Vec::new()
        }
    }

    fn is_controller(&self, client: ClientId) -> bool {
        lock(&self.meta).controller == Some(client)
    }

    pub fn resize(&self, client: ClientId, size: TermSize) {
        if self.is_controller(client) {
            let mut term = lock(&self.term);
            self.resize_locked(&mut term, size);
        }
    }

    /// Queues input for the PTY. Awaits when the queue is full, which pushes
    /// back on the client rather than buffering without bound.
    pub async fn input(&self, client: ClientId, data: Vec<u8>) {
        if self.is_controller(client) && self.is_alive() {
            let _ = self.input.send(data).await;
        }
    }

    /// SIGTERM to the child's process group, then SIGKILL after `timeout`.
    pub async fn terminate(&self, timeout: Duration) {
        let Some(pid) = self.pid.and_then(|p| Pid::from_raw(p as i32)) else {
            return;
        };
        if !self.is_alive() {
            return;
        }
        // The PTY child is a session leader, so its pid is also its process
        // group id. Signalling the group catches its descendants too.
        signal_group(pid, Signal::TERM);
        let mut exited = self.exited.subscribe();
        let waited = tokio::time::timeout(timeout, wait_true(&mut exited)).await;
        if waited.is_err() && self.is_alive() {
            tracing::warn!(
                session = self.id,
                "did not exit after SIGTERM; sending SIGKILL"
            );
            signal_group(pid, Signal::KILL);
            let _ = tokio::time::timeout(Duration::from_secs(2), wait_true(&mut exited)).await;
        }
    }

    /// Broadcasts the current info, unless the session has been removed.
    pub fn publish(&self) {
        let meta = lock(&self.meta);
        if !meta.removed {
            let _ = self
                .events
                .send(ServerMessage::SessionUpdated(self.info_locked(&meta)));
        }
    }

    /// Marks the session removed and broadcasts `SessionRemoved`. Holding the
    /// meta lock orders this after any in-flight `publish`.
    pub fn remove(&self) {
        let mut meta = lock(&self.meta);
        meta.removed = true;
        let _ = self.events.send(ServerMessage::SessionRemoved(self.id));
        drop(meta);
        self.removed.send_replace(true);
    }

    pub fn removed(&self) -> watch::Receiver<bool> {
        self.removed.subscribe()
    }
}

/// Resolves once the flag is set (or its sender is gone). The guard returned
/// by `wait_for` is dropped here so callers can hold the future across awaits.
pub(crate) async fn wait_true(rx: &mut watch::Receiver<bool>) {
    let _ = rx.wait_for(|&v| v).await;
}

fn signal_group(pid: Pid, sig: Signal) {
    if rustix::process::kill_process_group(pid, sig).is_err() {
        let _ = rustix::process::kill_process(pid, sig);
    }
}

fn write_loop(mut writer: Box<dyn Write + Send>, mut rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(data) = rx.blocking_recv() {
        if writer
            .write_all(&data)
            .and_then(|_| writer.flush())
            .is_err()
        {
            break;
        }
    }
}
