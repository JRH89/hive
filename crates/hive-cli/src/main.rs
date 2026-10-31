//! `hive` — the command-line entry point and TUI launcher.

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use hive_core::brand;
use hive_core::client::{Client, ClientError};
use hive_core::config::Config;
use hive_core::launch;
use hive_core::paths::Paths;
use hive_protocol::{ClientMessage, ServerMessage, SessionId, SessionInfo, SessionState, TermSize};
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
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
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
            detached,
            args,
        }) => {
            let agent = agent.unwrap_or_else(|| config.general.default_agent.clone());
            let size = terminal_size();
            let req = launch::create_request(&config, &agent, label, cwd, &args, size)?;
            let mut client = connect(auto).await?;
            if detached {
                client.send(&ClientMessage::CreateSession(req)).await?;
                let info = client
                    .recv_until(|m| match m {
                        ServerMessage::SessionCreated(i) => Some(i),
                        _ => None,
                    })
                    .await?;
                println!("created session {} ({})", info.id, info.agent);
                Ok(())
            } else {
                tui(&paths, client, config, Start::Create(req)).await
            }
        }
        Some(Command::List) => {
            let sessions = list(&mut connect(false).await.or_else(not_running)?).await?;
            print_sessions(&sessions);
            Ok(())
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

fn print_sessions(sessions: &[SessionInfo]) {
    if sessions.is_empty() {
        println!("no sessions");
        return;
    }
    println!("  ID  AGENT      LABEL            STATE          DIRECTORY");
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
        println!(
            "{:>4}  {:<10} {:<16} {:<14} {}",
            s.id, s.agent, s.label, state, s.cwd
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
