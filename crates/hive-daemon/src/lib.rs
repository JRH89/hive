//! `hived`: owns every agent process, PTY and authoritative terminal state.
//! Frontends are clients; they can come and go without affecting sessions.

mod listener;
mod responder;
mod server;
mod session;
mod store;

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{
    CreateSession, ServerMessage, SessionId, SessionInfo, WorkspaceId, WorkspaceInfo, WorktreeInfo,
};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

use session::{ClientId, Placement, Session};
use store::{Store, StoredSession, StoredWorkspace};

/// How often running sessions' branches are re-read (a small file read each).
const BRANCH_REFRESH: Duration = Duration::from_secs(2);

/// How often each session's foreground agent and status are re-checked.
const AGENT_REFRESH: Duration = Duration::from_millis(500);

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
    // Lock order across these maps: saved, workspaces, then sessions, then lost.
    /// Broadcasts about a workspace are sent while holding this lock, so they
    /// reach clients in the order the changes were made.
    workspaces: Mutex<BTreeMap<WorkspaceId, WorkspaceInfo>>,
    next_workspace: AtomicU64,
    clients: Mutex<HashMap<ClientId, mpsc::Sender<ServerMessage>>>,
    next_client: AtomicU64,
    /// Session lifecycle broadcasts delivered to every connected client.
    events: broadcast::Sender<ServerMessage>,
    shutdown: watch::Sender<bool>,
    store_path: PathBuf,
    /// Held for the whole of [`Daemon::persist`], so writes never interleave.
    saved: Mutex<Saved>,
}

#[derive(Default)]
struct Saved {
    /// Last bytes written to `store_path`, to skip redundant writes.
    bytes: Vec<u8>,
    /// The graceful-shutdown write happened; later live snapshots are stale.
    closed: bool,
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
        let next_workspace = store
            .workspaces
            .iter()
            .map(|w| w.id + 1)
            .chain([store.next_workspace_id, 1])
            .max()
            .unwrap_or(1);
        let workspaces: BTreeMap<_, _> = store
            .workspaces
            .into_iter()
            .map(|w| (w.id, w.into_info()))
            .collect();
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
        let daemon = Daemon {
            config,
            sessions: Mutex::new(BTreeMap::new()),
            lost: Mutex::new(lost),
            next_session: AtomicU64::new(next),
            workspaces: Mutex::new(workspaces),
            next_workspace: AtomicU64::new(next_workspace),
            clients: Mutex::new(HashMap::new()),
            next_client: AtomicU64::new(1),
            events: broadcast::channel(1024).0,
            shutdown: watch::channel(false).0,
            store_path,
            saved: Mutex::new(Saved::default()),
        };
        // Records from a version-1 store, or whose workspace was lost, are
        // regrouped by directory.
        // Lock order is workspaces, then sessions, then lost.
        let known: Vec<WorkspaceId> = daemon.workspaces.lock().unwrap().keys().copied().collect();
        let orphans: Vec<(SessionId, String)> = daemon
            .lost
            .lock()
            .unwrap()
            .values()
            .filter(|s| !known.contains(&s.workspace))
            .map(|s| (s.id, s.cwd.clone()))
            .collect();
        for (id, cwd) in orphans {
            let workspace = daemon.ensure_workspace(Path::new(&cwd)).id;
            if let Some(info) = daemon.lost.lock().unwrap().get_mut(&id) {
                info.workspace = workspace;
            }
        }
        daemon
    }

    fn workspace(&self, id: WorkspaceId) -> anyhow::Result<WorkspaceInfo> {
        self.workspaces
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .with_context(|| format!("no workspace with id {id}"))
    }

    fn list_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.workspaces.lock().unwrap().values().cloned().collect()
    }

    /// The workspace `path` belongs to: the one with the deepest root
    /// containing its repository (or the path itself, outside git). Creates
    /// one if none does.
    fn ensure_workspace(&self, path: &Path) -> WorkspaceInfo {
        let key = hive_workspace::workspace_key(path);
        let mut workspaces = self.workspaces.lock().unwrap();
        let roots = workspaces.values().map(|w| (w.id, Path::new(&w.root)));
        if let Some(id) = hive_workspace::best_match(roots, &key) {
            return workspaces[&id].clone();
        }
        self.insert_workspace(&mut workspaces, &key, None)
    }

    fn insert_workspace(
        &self,
        workspaces: &mut BTreeMap<WorkspaceId, WorkspaceInfo>,
        root: &Path,
        name: Option<String>,
    ) -> WorkspaceInfo {
        let id = self.next_workspace.fetch_add(1, Ordering::Relaxed);
        let info = WorkspaceInfo {
            id,
            name: clean_name(name.as_deref()).unwrap_or_else(|| hive_workspace::default_name(root)),
            root: root.display().to_string(),
            is_repo: hive_workspace::discover(root).is_some(),
        };
        tracing::info!(workspace = id, root = %info.root, "workspace created");
        workspaces.insert(id, info.clone());
        let _ = self
            .events
            .send(ServerMessage::WorkspaceUpdated(info.clone()));
        info
    }

    /// Adds a workspace rooted at `path`'s repository (or `path`). Returns
    /// the existing one if a workspace already has exactly that root.
    fn create_workspace(&self, path: &str, name: Option<String>) -> anyhow::Result<WorkspaceInfo> {
        let path = Path::new(path);
        anyhow::ensure!(path.is_dir(), "{} is not a directory", path.display());
        let key = hive_workspace::workspace_key(path);
        let mut workspaces = self.workspaces.lock().unwrap();
        if let Some(w) = workspaces.values().find(|w| Path::new(&w.root) == key) {
            return Ok(w.clone());
        }
        Ok(self.insert_workspace(&mut workspaces, &key, name))
    }

    fn rename_workspace(&self, id: WorkspaceId, name: &str) -> anyhow::Result<()> {
        let mut workspaces = self.workspaces.lock().unwrap();
        let info = workspaces
            .get_mut(&id)
            .with_context(|| format!("no workspace with id {id}"))?;
        info.name = clean_name(Some(name))
            .unwrap_or_else(|| hive_workspace::default_name(Path::new(&info.root)));
        let _ = self
            .events
            .send(ServerMessage::WorkspaceUpdated(info.clone()));
        Ok(())
    }

    /// Forgets a workspace with no sessions (running, exited or lost).
    fn remove_workspace(&self, id: WorkspaceId) -> anyhow::Result<()> {
        // Held across the check and removal so a concurrent create can't
        // pick this workspace in between.
        let mut workspaces = self.workspaces.lock().unwrap();
        anyhow::ensure!(workspaces.contains_key(&id), "no workspace with id {id}");
        let used = self.list().iter().filter(|s| s.workspace == id).count();
        anyhow::ensure!(
            used == 0,
            "workspace {id} still has {used} session(s); remove them first"
        );
        workspaces.remove(&id);
        let _ = self.events.send(ServerMessage::WorkspaceRemoved(id));
        Ok(())
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

    async fn create(&self, mut req: CreateSession) -> anyhow::Result<SessionInfo> {
        let workspace = match req.workspace {
            Some(id) => self.workspace(id)?,
            None => self.ensure_workspace(Path::new(&req.cwd)),
        };
        let id = self.next_session.fetch_add(1, Ordering::Relaxed);
        if let Some(wt) = req.worktree.take() {
            let branch = wt
                .branch
                .filter(|b| !b.trim().is_empty())
                .unwrap_or_else(|| {
                    format!("hive/{}-{id}", hive_workspace::branch_dir_name(&req.agent))
                });
            req.cwd = self.add_worktree(&workspace, branch).await?;
        }
        let placement = Placement {
            workspace: workspace.id,
            repo: hive_workspace::discover(Path::new(&req.cwd)),
        };
        let session = Session::spawn(
            id,
            req,
            placement,
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

    /// Creates a linked worktree of `workspace`'s repository on `branch`
    /// under the configured base directory and returns its path.
    async fn add_worktree(
        &self,
        workspace: &WorkspaceInfo,
        branch: String,
    ) -> anyhow::Result<String> {
        let repo = hive_workspace::discover(Path::new(&workspace.root))
            .with_context(|| format!("workspace `{}` is not a git repository", workspace.name))?;
        let path = self
            .config
            .worktrees
            .base_dir()
            .join(hive_workspace::repo_dir_name(&repo.main_root))
            .join(hive_workspace::branch_dir_name(&branch));
        anyhow::ensure!(
            !path.exists(),
            "{} already exists; pick another branch name",
            path.display()
        );
        let main_root = repo.main_root.clone();
        let target = path.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            hive_workspace::add_worktree(&main_root, &target, &branch)?;
            anyhow::Ok(())
        })
        .await??;
        tracing::info!(path = %path.display(), "worktree created");
        Ok(path.display().to_string())
    }

    async fn list_worktrees(&self, id: WorkspaceId) -> anyhow::Result<Vec<WorktreeInfo>> {
        let workspace = self.workspace(id)?;
        let repo = hive_workspace::discover(Path::new(&workspace.root))
            .with_context(|| format!("workspace `{}` is not a git repository", workspace.name))?;
        let trees = tokio::task::spawn_blocking(move || {
            let list = hive_workspace::list_worktrees(&repo.main_root)?;
            let dirty: Vec<_> = list
                .iter()
                .map(|w| hive_workspace::is_dirty(&w.path).ok())
                .collect();
            anyhow::Ok(list.into_iter().zip(dirty).collect::<Vec<_>>())
        })
        .await??;
        let live: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        Ok(trees
            .into_iter()
            .map(|(w, dirty)| WorktreeInfo {
                sessions: live
                    .iter()
                    .filter(|s| s.is_alive() && s.tree_root().as_deref() == Some(w.path.as_path()))
                    .map(|s| s.id)
                    .collect(),
                path: w.path.display().to_string(),
                branch: w.branch,
                main: w.main,
                dirty,
            })
            .collect())
    }

    /// Removes a linked worktree that no running session uses. A dirty one
    /// needs `force`. The branch is kept.
    async fn remove_worktree(&self, path: &str, force: bool) -> anyhow::Result<()> {
        let path = Path::new(path)
            .canonicalize()
            .with_context(|| format!("no such directory: {path}"))?;
        let users: Vec<_> = self
            .sessions
            .lock()
            .unwrap()
            .values()
            .filter(|s| {
                s.is_alive()
                    && (s.tree_root().as_deref() == Some(path.as_path())
                        || s.cwd().starts_with(&path))
            })
            .map(|s| format!("#{}", s.id))
            .collect();
        anyhow::ensure!(
            users.is_empty(),
            "session {} is running in {}; terminate it first",
            users.join(", "),
            path.display()
        );
        let target = path.clone();
        tokio::task::spawn_blocking(move || hive_workspace::remove_worktree(&target, force))
            .await??;
        tracing::info!(path = %path.display(), force, "worktree removed");
        Ok(())
    }

    /// Republishes sessions whose branch changed (e.g. the agent ran
    /// `git switch`).
    /// Follows each session's process to where it is now (`cd`) and
    /// re-reads branches; republishes what changed.
    fn refresh_places(&self) {
        let live: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        for s in live {
            let mut changed = false;
            if let Some(cwd) = s.live_cwd().filter(|c| *c != s.cwd()) {
                let workspace = self.space_for(&cwd, s.workspace());
                if workspace != s.workspace() {
                    tracing::info!(session = s.id, cwd = %cwd.display(), workspace, "session moved");
                }
                s.move_to(&cwd, workspace);
                changed = true;
            }
            if s.refresh_branch() || changed {
                s.publish();
            }
        }
    }

    /// The workspace a session that moved to `cwd` belongs to: its
    /// repository's (created if new, so spaces are the repos in use), else
    /// the deepest workspace containing it, else where it was.
    fn space_for(&self, cwd: &Path, current: WorkspaceId) -> WorkspaceId {
        let mut workspaces = self.workspaces.lock().unwrap();
        match hive_workspace::discover(cwd) {
            Some(repo) => match workspaces
                .values()
                .find(|w| Path::new(&w.root) == repo.main_root)
            {
                Some(w) => w.id,
                None => {
                    self.insert_workspace(&mut workspaces, &repo.main_root, None)
                        .id
                }
            },
            None => {
                let roots = workspaces.values().map(|w| (w.id, Path::new(&w.root)));
                hive_workspace::best_match(roots, cwd).unwrap_or(current)
            }
        }
    }

    /// Republishes sessions whose agent presence or status changed.
    fn detect_agents(&self) {
        let live: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        let now = std::time::Instant::now();
        for s in live {
            if s.detect_agent(now) {
                s.publish();
            }
        }
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
    ///
    /// The persist task can be mid-write when shutdown aborts it, so the lock
    /// is taken before reading any state: a live snapshot taken before
    /// `terminate_all` must not land after the final write.
    fn persist(&self, include_live: bool) {
        let mut saved = self.saved.lock().unwrap();
        if saved.closed {
            return;
        }
        saved.closed = !include_live;
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
        let workspaces = self
            .workspaces
            .lock()
            .unwrap()
            .values()
            .map(StoredWorkspace::from_info)
            .collect();
        let bytes = Store {
            next_session_id: self.next_session.load(Ordering::Relaxed),
            sessions,
            next_workspace_id: self.next_workspace.load(Ordering::Relaxed),
            workspaces,
            ..Store::default()
        }
        .to_bytes();
        if saved.bytes == bytes {
            return;
        }
        match Store::write_bytes(&self.store_path, &bytes) {
            Ok(()) => saved.bytes = bytes,
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

async fn branch_loop(daemon: Arc<Daemon>) {
    let mut tick = tokio::time::interval(BRANCH_REFRESH);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        let d = daemon.clone();
        // Reads /proc (or runs lsof) and .git files: off the runtime threads.
        let _ = tokio::task::spawn_blocking(move || d.refresh_places()).await;
    }
}

async fn agent_loop(daemon: Arc<Daemon>) {
    let mut tick = tokio::time::interval(AGENT_REFRESH);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        let d = daemon.clone();
        // Reads /proc and runs screen rules: keep it off the runtime threads.
        if tokio::task::spawn_blocking(move || d.detect_agents())
            .await
            .is_err()
        {
            tracing::error!("agent detection panicked");
        }
    }
}

/// Trimmed and capped; `None` when empty.
fn clean_name(name: Option<&str>) -> Option<String> {
    let name: String = name?.trim().chars().take(64).collect();
    (!name.is_empty()).then_some(name)
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

    let mut listener = listener::Listener::bind(&paths).context("listening")?;
    tracing::info!(pid = std::process::id(), "hived listening");

    let daemon = Arc::new(Daemon::new(config, paths.state_dir.join("sessions.json")));
    let persister = AbortOnDrop(tokio::spawn(persist_loop(daemon.clone())));
    let _branches = AbortOnDrop(tokio::spawn(branch_loop(daemon.clone())));
    let _agents = AbortOnDrop(tokio::spawn(agent_loop(daemon.clone())));
    let mut shutdown = daemon.shutdown.subscribe();
    let mut stop = listener::Stop::new()?;

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(conn) => {
                    tokio::spawn(listener::serve(daemon.clone(), conn));
                }
                Err(e) => tracing::warn!("accept failed: {e}"),
            },
            _ = shutdown.wait_for(|&s| s) => break,
            _ = stop.recv() => break,
        }
    }

    tracing::info!("shutting down");
    let _ = daemon.events.send(ServerMessage::ShuttingDown);
    daemon.terminate_all().await;
    drop(persister);
    daemon.persist(false);
    listener.cleanup(&paths);
    // Give client writer tasks a moment to flush ShuttingDown.
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(lock_file);
    Ok(())
}
