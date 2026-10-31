//! `hived`: owns every agent process, PTY and authoritative terminal state.
//! Frontends are clients; they can come and go without affecting sessions.

mod responder;
mod server;
mod session;

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, TryLockError};
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{CreateSession, ServerMessage, SessionId, SessionInfo};
use tokio::net::UnixListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::{broadcast, mpsc, watch};

use session::{ClientId, Session};

#[derive(Debug, thiserror::Error)]
#[error("another hived is already running")]
pub struct AlreadyRunning;

pub(crate) struct Daemon {
    config: Config,
    sessions: Mutex<BTreeMap<SessionId, Arc<Session>>>,
    next_session: AtomicU64,
    clients: Mutex<HashMap<ClientId, mpsc::Sender<ServerMessage>>>,
    next_client: AtomicU64,
    /// Session lifecycle broadcasts delivered to every connected client.
    events: broadcast::Sender<ServerMessage>,
    shutdown: watch::Sender<bool>,
}

impl Daemon {
    fn new(config: Config) -> Daemon {
        Daemon {
            config,
            sessions: Mutex::new(BTreeMap::new()),
            next_session: AtomicU64::new(1),
            clients: Mutex::new(HashMap::new()),
            next_client: AtomicU64::new(1),
            events: broadcast::channel(1024).0,
            shutdown: watch::channel(false).0,
        }
    }

    fn get(&self, id: SessionId) -> Option<Arc<Session>> {
        self.sessions.lock().unwrap().get(&id).cloned()
    }

    fn list(&self) -> Vec<SessionInfo> {
        let sessions: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        sessions.iter().map(|s| s.info()).collect()
    }

    fn create(&self, req: CreateSession) -> anyhow::Result<SessionInfo> {
        let id = self.next_session.fetch_add(1, Ordering::Relaxed);
        let session = Session::spawn(
            id,
            req,
            self.config.terminal.scrollback_lines,
            self.events.clone(),
        )?;
        let info = session.info();
        tracing::info!(session = id, command = ?info.command, cwd = %info.cwd, "session created");
        session.publish();
        self.sessions.lock().unwrap().insert(id, session);
        Ok(info)
    }

    async fn kill(&self, id: SessionId) -> anyhow::Result<()> {
        let session = self
            .get(id)
            .with_context(|| format!("no session with id {id}"))?;
        session.terminate(self.kill_timeout()).await;
        self.sessions.lock().unwrap().remove(&id);
        session.remove();
        tracing::info!(session = id, "session removed");
        Ok(())
    }

    fn rename(&self, id: SessionId, label: &str) -> anyhow::Result<()> {
        let session = self
            .get(id)
            .with_context(|| format!("no session with id {id}"))?;
        session.rename(label);
        session.publish();
        Ok(())
    }

    fn kill_timeout(&self) -> Duration {
        Duration::from_secs(self.config.daemon.kill_timeout_secs)
    }

    fn notify(&self, client: ClientId, msg: ServerMessage) {
        if let Some(tx) = self.clients.lock().unwrap().get(&client) {
            let _ = tx.try_send(msg);
        }
    }

    async fn terminate_all(&self) {
        let sessions: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        let timeout = self.kill_timeout();
        let tasks: Vec<_> = sessions
            .into_iter()
            .map(|s| tokio::spawn(async move { s.terminate(timeout).await }))
            .collect();
        for t in tasks {
            let _ = t.await;
        }
    }
}

/// Runs the daemon until it is asked to shut down or receives SIGTERM/SIGINT.
/// Returns [`AlreadyRunning`] (wrapped) if another daemon holds the lock.
pub async fn run(paths: Paths, config: Config) -> anyhow::Result<()> {
    paths
        .ensure_runtime_dir()
        .context("preparing runtime directory")?;

    // The lock serializes concurrent starts; the loser exits without touching
    // the socket. It is released by the OS if we crash.
    let lock_file = File::create(&paths.lock).context("opening lock file")?;
    match lock_file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Err(AlreadyRunning.into()),
        Err(TryLockError::Error(e)) => return Err(e).context("locking"),
    }

    // Holding the lock means any existing socket is stale.
    match fs::remove_file(&paths.socket) {
        Ok(()) => tracing::info!("removed stale socket"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("removing stale socket"),
    }
    let listener = UnixListener::bind(&paths.socket).context("binding socket")?;
    fs::set_permissions(&paths.socket, fs::Permissions::from_mode(0o600))?;
    tracing::info!(socket = %paths.socket.display(), pid = std::process::id(), "hived listening");

    let daemon = Arc::new(Daemon::new(config));
    let mut shutdown = daemon.shutdown.subscribe();
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let uid = rustix::process::getuid().as_raw();

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!("accept failed: {e}");
                        continue;
                    }
                };
                match stream.peer_cred() {
                    Ok(cred) if cred.uid() == uid => {
                        tokio::spawn(server::handle_client(daemon.clone(), stream));
                    }
                    Ok(cred) => tracing::warn!(peer_uid = cred.uid(), "rejected connection from another user"),
                    Err(e) => tracing::warn!("could not read peer credentials: {e}"),
                }
            }
            _ = shutdown.wait_for(|&s| s) => break,
            _ = sigterm.recv() => break,
            _ = sigint.recv() => break,
        }
    }

    tracing::info!("shutting down");
    let _ = daemon.events.send(ServerMessage::ShuttingDown);
    daemon.terminate_all().await;
    let _ = fs::remove_file(&paths.socket);
    // Give client writer tasks a moment to flush ShuttingDown.
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(lock_file);
    Ok(())
}
