//! `hived`: owns every agent process, PTY and authoritative terminal state.
//! Frontends are clients; they can come and go without affecting sessions.

mod responder;
mod server;
mod session;
mod store;

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, TryLockError};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{CreateSession, ServerMessage, SessionId, SessionInfo};
use tokio::net::UnixListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

use session::{ClientId, Session};
use store::{Store, StoredSession};

#[derive(Debug, thiserror::Error)]
#[error("another hived is already running")]
pub struct AlreadyRunning;

pub(crate) struct Daemon {
    config: Config,
    sessions: Mutex<BTreeMap<SessionId, Arc<Session>>>,
    /// Metadata of sessions that were running when a previous daemon died.
    /// Kept until the user relaunches or removes them.
    lost: Mutex<BTreeMap<SessionId, SessionInfo>>,
    next_session: AtomicU64,
    clients: Mutex<HashMap<ClientId, mpsc::Sender<ServerMessage>>>,
    next_client: AtomicU64,
    /// Session lifecycle broadcasts delivered to every connected client.
    events: broadcast::Sender<ServerMessage>,
    shutdown: watch::Sender<bool>,
    store_path: PathBuf,
    /// Last bytes written to `store_path`, to skip redundant writes.
    saved: Mutex<Vec<u8>>,
}

impl Daemon {
    fn new(config: Config, store_path: PathBuf) -> Daemon {
        let store = Store::load(&store_path);
        let next = store
            .sessions
            .iter()
            .map(|s| s.id + 1)
            .chain([store.next_session_id, 1])
            .max()
            .unwrap_or(1);
        let lost: BTreeMap<_, _> = store
            .sessions
            .into_iter()
            .map(|s| (s.id, s.into_lost_info()))
            .collect();
        if !lost.is_empty() {
            tracing::warn!(
                count = lost.len(),
                "previous daemon stopped unexpectedly; its sessions are marked lost"
            );
        }
        Daemon {
            config,
            sessions: Mutex::new(BTreeMap::new()),
            lost: Mutex::new(lost),
            next_session: AtomicU64::new(next),
            clients: Mutex::new(HashMap::new()),
            next_client: AtomicU64::new(1),
            events: broadcast::channel(1024).0,
            shutdown: watch::channel(false).0,
            store_path,
            saved: Mutex::new(Vec::new()),
        }
    }

    fn get(&self, id: SessionId) -> Option<Arc<Session>> {
        self.sessions.lock().unwrap().get(&id).cloned()
    }

    fn is_lost(&self, id: SessionId) -> bool {
        self.lost.lock().unwrap().contains_key(&id)
    }

    fn list(&self) -> Vec<SessionInfo> {
        let live: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        let mut all: Vec<SessionInfo> = live.iter().map(|s| s.info()).collect();
        all.extend(self.lost.lock().unwrap().values().cloned());
        all.sort_by_key(|s| s.id);
        all
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
        self.sessions.lock().unwrap().insert(id, session.clone());
        session.publish();
        Ok(info)
    }

    async fn kill(&self, id: SessionId) -> anyhow::Result<()> {
        let Some(session) = self.get(id) else {
            // Lost records have no process; removing them is just forgetting.
            // The broadcast is sent under the lock that guards the record so it
            // cannot interleave with a concurrent rename's update.
            let mut lost = self.lost.lock().unwrap();
            if lost.remove(&id).is_some() {
                let _ = self.events.send(ServerMessage::SessionRemoved(id));
                return Ok(());
            }
            anyhow::bail!("no session with id {id}");
        };
        session.terminate(self.kill_timeout()).await;
        self.sessions.lock().unwrap().remove(&id);
        session.remove();
        tracing::info!(session = id, "session removed");
        Ok(())
    }

    fn rename(&self, id: SessionId, label: &str) -> anyhow::Result<()> {
        if let Some(session) = self.get(id) {
            session.rename(label);
            session.publish();
            return Ok(());
        }
        let mut lost = self.lost.lock().unwrap();
        let info = lost
            .get_mut(&id)
            .with_context(|| format!("no session with id {id}"))?;
        let label = label.trim();
        info.label = if label.is_empty() {
            info.agent.clone()
        } else {
            label.chars().take(64).collect()
        };
        let _ = self
            .events
            .send(ServerMessage::SessionUpdated(info.clone()));
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

    /// Writes the session store if its contents changed. `include_live` is
    /// false on graceful shutdown: sessions we terminated on purpose are not
    /// "lost".
    fn persist(&self, include_live: bool) {
        let mut sessions: Vec<StoredSession> = if include_live {
            self.list()
                .iter()
                .filter(|s| s.state.is_alive() || self.is_lost(s.id))
                .map(StoredSession::from_info)
                .collect()
        } else {
            self.lost
                .lock()
                .unwrap()
                .values()
                .map(StoredSession::from_info)
                .collect()
        };
        sessions.sort_by_key(|s| s.id);
        let bytes = Store {
            next_session_id: self.next_session.load(Ordering::Relaxed),
            sessions,
            ..Store::default()
        }
        .to_bytes();
        let mut saved = self.saved.lock().unwrap();
        if *saved == bytes {
            return;
        }
        match Store::write_bytes(&self.store_path, &bytes) {
            Ok(()) => *saved = bytes,
            Err(e) => tracing::warn!("cannot write {}: {e}", self.store_path.display()),
        }
    }
}

/// Persists the store after every burst of lifecycle events.
async fn persist_loop(daemon: Arc<Daemon>) {
    let mut rx = daemon.events.subscribe();
    daemon.persist(true);
    loop {
        if let Err(RecvError::Closed) = rx.recv().await {
            break;
        }
        // Coalesce whatever else is queued into one write.
        while let Ok(_) | Err(TryRecvError::Lagged(_)) = rx.try_recv() {}
        daemon.persist(true);
    }
}

/// Aborts the wrapped task when dropped, so background work dies with `run`
/// even if `run` itself is cancelled.
struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Runs the daemon until it is asked to shut down or receives SIGTERM/SIGINT.
/// Returns [`AlreadyRunning`] (wrapped) if another daemon holds the lock.
pub async fn run(paths: Paths, config: Config) -> anyhow::Result<()> {
    paths
        .ensure_runtime_dir()
        .context("preparing runtime directory")?;
    paths
        .ensure_state_dir()
        .context("preparing state directory")?;

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

    let daemon = Arc::new(Daemon::new(config, paths.state_dir.join("sessions.json")));
    let persister = AbortOnDrop(tokio::spawn(persist_loop(daemon.clone())));
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
    drop(persister);
    daemon.persist(false);
    let _ = fs::remove_file(&paths.socket);
    // Give client writer tasks a moment to flush ShuttingDown.
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(lock_file);
    Ok(())
}
