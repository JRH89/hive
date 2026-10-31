//! `hive` — the command-line entry point and TUI launcher.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use hive_core::brand;
use hive_core::client::{Client, ClientError};
use hive_core::config::Config;
use hive_core::launch;
use hive_core::paths::Paths;
use hive_protocol::{
    ClientMessage, NewWorktree, ServerMessage, SessionId, SessionInfo, SessionState, TermSize,
    WorkspaceInfo, WorktreeInfo,
};
use hive_tui::{Outcome, Start};

#[derive(Parser)]
#[command(
    name = "hive",
    version,
    about = format!("{} {} — {}", brand::MARK, brand::NAME, brand::TAGLINE),
    long_about = "A terminal multiplexer for AI coding agents.\n\n\
        Agents run inside `hived`, a background daemon, so they keep running \
        after you detach or close the terminal. Run `hive` with no arguments \
        to open the interface."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Launch an agent in a new session and attach to it
    New {
        /// Agent name: claude, codex, gemini, opencode, shell, or one from config
        agent: Option<String>,
        /// Session label (defaults to the agent name)
        #[arg(short, long)]
        label: Option<String>,
        /// Working directory (defaults to the current directory)
        #[arg(short = 'C', long)]
        cwd: Option<PathBuf>,
        /// Workspace name or id (defaults to the one containing the directory)
        #[arg(short = 'W', long)]
        workspace: Option<String>,
        /// Run in a new git worktree of the workspace's repository
        #[arg(short, long)]
        worktree: bool,
        /// Branch for the worktree (implies --worktree; created if missing)
        #[arg(short, long)]
        branch: Option<String>,
        /// Create the session without attaching
        #[arg(short, long)]
        detached: bool,
        /// Extra arguments passed to the agent
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// List sessions
    #[command(alias = "ls")]
    List,
    /// Attach to a session
    Attach { session: SessionId },
    /// Terminate a session and remove it
    Kill { session: SessionId },
    /// Show daemon and session summary
    Status,
    /// Manage workspaces (groups of sessions, usually one repository each)
    #[command(alias = "ws")]
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    /// Manage git worktrees created for agents
    #[command(alias = "wt")]
    Worktree {
        #[command(subcommand)]
        action: WorktreeAction,
    },
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
enum WorkspaceAction {
    /// List workspaces
    #[command(alias = "ls")]
    List,
    /// Add a workspace for a directory (its repository root inside git)
    Add {
        path: PathBuf,
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Rename a workspace
    Rename { workspace: String, name: String },
    /// Forget a workspace that has no sessions (nothing on disk changes)
    #[command(alias = "remove")]
    Rm { workspace: String },
}

#[derive(Subcommand)]
enum WorktreeAction {
    /// List a repository's worktrees with their state
    #[command(alias = "ls")]
    List {
        /// Workspace name or id (defaults to the one containing the current directory)
        #[arg(short = 'W', long)]
        workspace: Option<String>,
    },
    /// Remove a worktree. Refused while a session runs in it. The branch is kept
    #[command(alias = "remove")]
    Rm {
        path: PathBuf,
        /// Also discard uncommitted changes
        #[arg(long)]
        force: bool,
        /// Don't ask for confirmation with --force
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon if it is not running
    Start,
    /// Stop the daemon. Terminates all sessions
    Stop,
    /// Show whether the daemon is running
    Status,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let paths = Paths::resolve();
    init_logging(&paths);
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(run(cli, paths)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hive: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Logs go to a file: stdout belongs to the TUI.
fn init_logging(paths: &Paths) {
    let Ok(()) = paths.ensure_state_dir() else {
        return;
    };
    let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.state_dir.join("hive.log"))
    else {
        return;
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("HIVE_LOG")
                .unwrap_or_else(|_| "warn".into()),
        )
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .init();
}

async fn run(cli: Cli, paths: Paths) -> anyhow::Result<()> {
    let config = Config::load(&paths.config_file)?;
    let connect = |auto_start: bool| {
        let paths = paths.clone();
        async move { Client::connect_or_start(&paths, "hive", auto_start).await }
    };
    let auto = config.daemon.auto_start;

    match cli.command {
        None => tui(&paths, connect(auto).await?, config, Start::Auto).await,
        Some(Command::New {
            agent,
            label,
            cwd,
            workspace,
            worktree,
            branch,
            detached,
            args,
        }) => {
            let agent = agent.unwrap_or_else(|| config.general.default_agent.clone());
            let size = terminal_size();
            let mut client = connect(auto).await?;
            let workspace = match workspace {
                Some(w) => Some(find_workspace(&workspaces(&mut client).await?, &w)?),
                None => None,
            };
            // An explicit workspace launches in its root unless -C says otherwise.
            let cwd = cwd.or_else(|| workspace.as_ref().map(|w| PathBuf::from(&w.root)));
            let mut req = launch::create_request(&config, &agent, label, cwd, &args, size)?;
            req.workspace = workspace.map(|w| w.id);
            if worktree || branch.is_some() {
                req.worktree = Some(NewWorktree { branch });
            }
            if detached {
                let shared = sharing(&list(&mut client).await?, &req.cwd);
                client.send(&ClientMessage::CreateSession(req)).await?;
                let info = client
                    .recv_until(|m| match m {
                        ServerMessage::SessionCreated(i) => Some(i),
                        _ => None,
                    })
                    .await?;
                println!(
                    "created session {} ({}) in {}",
                    info.id, info.agent, info.cwd
                );
                if info.worktree.is_none() {
                    warn_shared(&shared, &info.cwd);
                }
                Ok(())
            } else {
                tui(&paths, client, config, Start::Create(req)).await
            }
        }
        Some(Command::List) => {
            let mut client = connect(false).await.or_else(not_running)?;
            let sessions = list(&mut client).await?;
            print_sessions(&sessions, &workspaces(&mut client).await?);
            Ok(())
        }
        Some(Command::Workspace { action }) => {
            workspace(action, connect(false).await.or_else(not_running)?).await
        }
        Some(Command::Worktree { action }) => {
            worktree(action, connect(false).await.or_else(not_running)?).await
        }
        Some(Command::Attach { session }) => {
            let mut client = connect(false).await.or_else(not_running)?;
            if !list(&mut client).await?.iter().any(|s| s.id == session) {
                bail!("no session with id {session} (see `hive list`)");
            }
            tui(&paths, client, config, Start::Attach(session)).await
        }
        Some(Command::Kill { session }) => {
            let mut client = connect(false).await.or_else(not_running)?;
            client.send(&ClientMessage::KillSession(session)).await?;
            client
                .recv_until(|m| {
                    matches!(m, ServerMessage::SessionRemoved(id) if id == session).then_some(())
                })
                .await?;
            println!("session {session} terminated");
            Ok(())
        }
        Some(Command::Status) => {
            match connect(false).await {
                Ok(mut client) => {
                    let sessions = list(&mut client).await?;
                    let active = sessions.iter().filter(|s| s.state.is_alive()).count();
                    println!(
                        "{} {}  daemon running (pid {}) · {} active · {} exited",
                        brand::MARK,
                        brand::NAME,
                        client.daemon.pid,
                        active,
                        sessions.len() - active
                    );
                }
                Err(ClientError::NotRunning(_)) => {
                    println!("{} {}  daemon not running", brand::MARK, brand::NAME)
                }
                Err(e) => return Err(e.into()),
            }
            Ok(())
        }
        Some(Command::Daemon { action }) => daemon(action, &paths, connect).await,
    }
}

async fn daemon<F, Fut>(action: DaemonAction, paths: &Paths, connect: F) -> anyhow::Result<()>
where
    F: Fn(bool) -> Fut,
    Fut: std::future::Future<Output = Result<Client, ClientError>>,
{
    match action {
        DaemonAction::Start => {
            let client = connect(true).await?;
            println!("hived running (pid {})", client.daemon.pid);
        }
        DaemonAction::Status => match connect(false).await {
            Ok(client) => println!(
                "hived {} running (pid {})\nsocket: {}",
                client.daemon.version,
                client.daemon.pid,
                paths.socket.display()
            ),
            Err(ClientError::NotRunning(_)) => println!("hived is not running"),
            Err(e) => return Err(e.into()),
        },
        DaemonAction::Stop => match connect(false).await {
            Ok(mut client) => {
                client.send(&ClientMessage::Shutdown).await?;
                // Wait for the connection to close, i.e. the daemon to exit.
                let _ = tokio::time::timeout(Duration::from_secs(15), async {
                    while let Ok(Some(_)) = client.recv().await {}
                })
                .await;
                println!("hived stopped");
            }
            Err(ClientError::NotRunning(_)) => println!("hived is not running"),
            Err(e) => return Err(e.into()),
        },
    }
    Ok(())
}

async fn workspace(action: WorkspaceAction, mut client: Client) -> anyhow::Result<()> {
    match action {
        WorkspaceAction::List => {
            let ws = workspaces(&mut client).await?;
            let sessions = list(&mut client).await?;
            if ws.is_empty() {
                println!("no workspaces");
            } else {
                println!("  ID  NAME             SESSIONS  ROOT");
            }
            for w in ws {
                let n = sessions.iter().filter(|s| s.workspace == w.id).count();
                let git = if w.is_repo { "" } else { "  (not git)" };
                println!("{:>4}  {:<16} {:>8}  {}{git}", w.id, w.name, n, w.root);
            }
        }
        WorkspaceAction::Add { path, name } => {
            let path = std::path::absolute(&path).unwrap_or(path);
            client
                .send(&ClientMessage::CreateWorkspace {
                    path: path.display().to_string(),
                    name,
                })
                .await?;
            let w = client
                .recv_until(|m| match m {
                    ServerMessage::WorkspaceCreated(w) => Some(w),
                    _ => None,
                })
                .await?;
            println!("workspace {} ({}) at {}", w.id, w.name, w.root);
        }
        WorkspaceAction::Rename { workspace, name } => {
            let w = find_workspace(&workspaces(&mut client).await?, &workspace)?;
            client
                .send(&ClientMessage::RenameWorkspace {
                    workspace: w.id,
                    name,
                })
                .await?;
            let w = client
                .recv_until(|m| match m {
                    ServerMessage::WorkspaceUpdated(u) if u.id == w.id => Some(u),
                    _ => None,
                })
                .await?;
            println!("workspace {} is now {}", w.id, w.name);
        }
        WorkspaceAction::Rm { workspace } => {
            let w = find_workspace(&workspaces(&mut client).await?, &workspace)?;
            client.send(&ClientMessage::RemoveWorkspace(w.id)).await?;
            client
                .recv_until(|m| {
                    matches!(m, ServerMessage::WorkspaceRemoved(id) if id == w.id).then_some(())
                })
                .await?;
            println!("workspace {} ({}) removed", w.id, w.name);
        }
    }
    Ok(())
}

async fn worktree(action: WorktreeAction, mut client: Client) -> anyhow::Result<()> {
    match action {
        WorktreeAction::List { workspace } => {
            let ws = workspaces(&mut client).await?;
            let w = match workspace {
                Some(w) => find_workspace(&ws, &w)?,
                None => current_workspace(&ws)?,
            };
            client.send(&ClientMessage::ListWorktrees(w.id)).await?;
            let trees = client
                .recv_until(|m| match m {
                    ServerMessage::WorktreeList { worktrees, .. } => Some(worktrees),
                    _ => None,
                })
                .await?;
            println!("{} ({})", w.name, w.root);
            for t in trees {
                print_worktree(&t);
            }
        }
        WorktreeAction::Rm { path, force, yes } => {
            let path = std::path::absolute(&path).unwrap_or(path);
            if force
                && !yes
                && !confirm(&format!(
                    "Remove {} and discard any uncommitted changes in it?",
                    path.display()
                ))?
            {
                println!("cancelled");
                return Ok(());
            }
            client
                .send(&ClientMessage::RemoveWorktree {
                    path: path.display().to_string(),
                    force,
                })
                .await?;
            let result = client
                .recv_until(|m| match m {
                    ServerMessage::WorktreeRemoved { path } => Some(path),
                    _ => None,
                })
                .await;
            match result {
                Ok(p) => println!("removed worktree {p} (its branch was kept)"),
                Err(ClientError::Daemon(msg)) if msg.contains("uncommitted changes") => {
                    bail!("{msg}; commit or stash them, or pass --force to discard them")
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}

fn print_worktree(t: &WorktreeInfo) {
    let branch = t.branch.as_deref().unwrap_or("(detached)");
    let state = match t.dirty {
        Some(true) => "modified",
        Some(false) => "clean",
        None => "?",
    };
    let users = if t.sessions.is_empty() {
        String::new()
    } else {
        let ids: Vec<_> = t.sessions.iter().map(|id| format!("#{id}")).collect();
        format!("  sessions {}", ids.join(" "))
    };
    let kind = if t.main { "main" } else { "worktree" };
    println!("  {kind:<9} {branch:<24} {state:<9} {}{users}", t.path);
}

fn confirm(question: &str) -> anyhow::Result<bool> {
    print!("{question} [y/N] ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

/// Matches an id or an exact (case-insensitive) name.
fn find_workspace(list: &[WorkspaceInfo], query: &str) -> anyhow::Result<WorkspaceInfo> {
    if let Ok(id) = query.parse::<u64>()
        && let Some(w) = list.iter().find(|w| w.id == id)
    {
        return Ok(w.clone());
    }
    let named: Vec<_> = list
        .iter()
        .filter(|w| w.name.eq_ignore_ascii_case(query))
        .collect();
    match named.as_slice() {
        [w] => Ok((*w).clone()),
        [] => bail!("no workspace named `{query}` (see `hive workspace list`)"),
        _ => bail!("several workspaces are named `{query}`; use its id"),
    }
}

/// The workspace containing the current directory.
fn current_workspace(list: &[WorkspaceInfo]) -> anyhow::Result<WorkspaceInfo> {
    let cwd = std::env::current_dir()?;
    let key = hive_workspace::workspace_key(&cwd);
    let roots = list.iter().map(|w| (w.id, Path::new(&w.root)));
    let id = hive_workspace::best_match(roots, &key)
        .context("the current directory is in no workspace; pass --workspace")?;
    Ok(list.iter().find(|w| w.id == id).cloned().expect("matched"))
}

/// Running sessions whose directory is exactly `cwd`.
fn sharing(sessions: &[SessionInfo], cwd: &str) -> Vec<SessionInfo> {
    sessions
        .iter()
        .filter(|s| s.state.is_alive() && s.cwd == cwd)
        .cloned()
        .collect()
}

fn warn_shared(shared: &[SessionInfo], cwd: &str) {
    if shared.is_empty() {
        return;
    }
    let ids: Vec<_> = shared
        .iter()
        .map(|s| format!("#{} ({})", s.id, s.label))
        .collect();
    eprintln!(
        "warning: {} also running in {cwd}. Agents sharing a directory can overwrite \
         each other's changes; use --worktree to isolate them.",
        ids.join(", ")
    );
}

async fn workspaces(client: &mut Client) -> anyhow::Result<Vec<WorkspaceInfo>> {
    client.send(&ClientMessage::ListWorkspaces).await?;
    client
        .recv_until(|m| match m {
            ServerMessage::WorkspaceList(l) => Some(l),
            _ => None,
        })
        .await
        .context("listing workspaces")
}

fn not_running(e: ClientError) -> anyhow::Result<Client> {
    match e {
        ClientError::NotRunning(_) => bail!("the daemon is not running; start one with `hive`"),
        e => Err(e.into()),
    }
}

async fn list(client: &mut Client) -> anyhow::Result<Vec<SessionInfo>> {
    client.send(&ClientMessage::ListSessions).await?;
    client
        .recv_until(|m| match m {
            ServerMessage::SessionList(l) => Some(l),
            _ => None,
        })
        .await
        .context("listing sessions")
}

fn print_sessions(sessions: &[SessionInfo], workspaces: &[WorkspaceInfo]) {
    if sessions.is_empty() {
        println!("no sessions");
        return;
    }
    println!(
        "  ID  AGENT      LABEL            STATE          WORKSPACE    BRANCH               DIRECTORY"
    );
    for s in sessions {
        let state = match &s.state {
            SessionState::Starting => "starting".into(),
            SessionState::Running if s.attached_clients > 0 => "running*".into(),
            SessionState::Running => "running".into(),
            SessionState::Exited { code: Some(c), .. } => format!("exited ({c})"),
            SessionState::Exited { signal, .. } => {
                format!("killed ({})", signal.as_deref().unwrap_or("?"))
            }
            SessionState::Failed { .. } => "failed".into(),
            SessionState::Lost => "ended†".into(),
        };
        let workspace = workspaces
            .iter()
            .find(|w| w.id == s.workspace)
            .map_or("?", |w| w.name.as_str());
        let mut branch = s.branch.clone().unwrap_or_else(|| "-".into());
        if s.worktree.is_some() {
            branch.push_str(" (wt)");
        }
        println!(
            "{:>4}  {:<10} {:<16} {:<14} {:<12} {:<20} {}",
            s.id, s.agent, s.label, state, workspace, branch, s.cwd
        );
    }
    if sessions.iter().any(|s| s.state == SessionState::Lost) {
        println!(
            "\n† was running when hived stopped; relaunch it from the UI (enter in the sidebar)"
        );
    }
}

fn terminal_size() -> TermSize {
    crossterm_size().unwrap_or(TermSize::DEFAULT)
}

fn crossterm_size() -> Option<TermSize> {
    let (cols, rows) = ratatui::crossterm::terminal::size().ok()?;
    Some(TermSize { rows, cols })
}

async fn tui(paths: &Paths, client: Client, config: Config, start: Start) -> anyhow::Result<()> {
    match hive_tui::run(paths.clone(), client, config, start).await? {
        Outcome::Detached { session: Some(id) } => {
            println!("detached from session {id} — it keeps running. `hive attach {id}` to return.")
        }
        Outcome::Detached { session: None } => {}
        Outcome::DaemonGone(reason) => bail!("disconnected: {reason}"),
    }
    Ok(())
}
