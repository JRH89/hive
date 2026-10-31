//! The Hive frontend. A client of `hived`: it never owns PTYs or processes,
//! so exiting or crashing here leaves every agent running.

mod app;
mod keys;
mod render;
mod theme;

use std::io::stdout;
use std::time::{Duration, Instant};

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, EventStream};
use crossterm::execute;
use futures_util::StreamExt;
use hive_core::client::{Client, ClientError, spawn_daemon};
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{ClientMessage, ServerMessage};
use tokio::sync::mpsc;

use app::App;
pub use app::{Outcome, Start};
use theme::Theme;

/// Longest the loop spends applying queued daemon messages before it redraws
/// and polls input again (~one frame at 60 Hz).
const DRAIN_BUDGET: Duration = Duration::from_millis(12);

/// Upper bound on the wait between reconnect attempts.
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Aborts the wrapped task when dropped, on every exit path.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Restores the user's terminal on every exit path, including errors.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableBracketedPaste);
        ratatui::restore();
    }
}

/// How a connected phase ended.
enum Phase {
    Exit(Outcome),
    /// The connection dropped unexpectedly; try to reconnect.
    Lost(String),
}

type Term = ratatui::DefaultTerminal;

pub async fn run(
    paths: Paths,
    client: Client,
    config: Config,
    start: Start,
) -> anyhow::Result<Outcome> {
    let mut terminal = ratatui::try_init()?;
    let _guard = TerminalGuard;
    execute!(stdout(), EnableBracketedPaste)?;

    let theme = Theme::detect();
    let size = terminal.size()?;
    let auto_start = config.daemon.auto_start;
    let mut app = App::new(config, start, (size.width, size.height));
    app.daemon_pid = Some(client.daemon.pid);
    let mut events = EventStream::new();

    let mut client = client;
    let mut initial = app.on_connect();
    loop {
        let reason = match connected(
            &mut terminal,
            &theme,
            &mut app,
            &mut events,
            client,
            initial,
        )
        .await?
        {
            Phase::Exit(outcome) => return Ok(outcome),
            Phase::Lost(reason) => reason,
        };
        tracing::warn!("daemon connection lost: {reason}");
        app.on_disconnected(reason);
        match reconnect(
            &mut terminal,
            &theme,
            &mut app,
            &mut events,
            &paths,
            auto_start,
        )
        .await?
        {
            Some(c) => {
                initial = app.on_reconnected(c.daemon.pid);
                client = c;
            }
            None => {
                return Ok(app
                    .exit
                    .take()
                    .unwrap_or(Outcome::Detached { session: None }));
            }
        }
    }
}

/// Runs the UI against one live connection until the user exits or the
/// connection drops.
async fn connected(
    terminal: &mut Term,
    theme: &Theme,
    app: &mut App,
    events: &mut EventStream,
    client: Client,
    initial: Vec<ClientMessage>,
) -> anyhow::Result<Phase> {
    let (mut reader, mut writer) = client.into_split();

    // read_frame is not cancel-safe, so it runs in its own task and the main
    // loop selects on a channel instead.
    let (srv_tx, mut srv_rx) = mpsc::channel::<Result<ServerMessage, String>>(256);
    let reader_task = AbortOnDrop(tokio::spawn(async move {
        loop {
            let msg = match reader.recv().await {
                Ok(Some(m)) => Ok(m),
                Ok(None) => Err("the daemon closed the connection".to_string()),
                Err(e) => Err(e.to_string()),
            };
            let done = msg.is_err();
            if srv_tx.send(msg).await.is_err() || done {
                break;
            }
        }
    }));

    for msg in initial {
        if let Err(e) = writer.send(&msg).await {
            return Ok(Phase::Lost(e.to_string()));
        }
    }

    let phase = loop {
        terminal.draw(|f| render::draw(f, app, theme))?;

        let out = tokio::select! {
            ev = events.next() => match ev {
                Some(Ok(ev)) => app.on_terminal_event(ev),
                Some(Err(e)) => return Err(e.into()),
                None => break Phase::Exit(Outcome::Detached { session: None }),
            },
            msg = srv_rx.recv() => match msg {
                Some(Ok(m)) => {
                    let mut out = app.on_server(m);
                    // Apply what is already queued before redrawing, so a burst
                    // of output costs one frame. The time budget keeps a flood
                    // from starving key handling and redraws.
                    let deadline = Instant::now() + DRAIN_BUDGET;
                    let mut lost = None;
                    while Instant::now() < deadline {
                        match srv_rx.try_recv() {
                            Ok(Ok(m)) => out.extend(app.on_server(m)),
                            Ok(Err(e)) => {
                                lost = Some(e);
                                break;
                            }
                            Err(_) => break,
                        }
                    }
                    if let Some(e) = lost {
                        break Phase::Lost(e);
                    }
                    out
                }
                Some(Err(e)) => break Phase::Lost(e),
                None => break Phase::Lost("connection lost".into()),
            },
        };
        let mut send_failed = None;
        for msg in out {
            if let Err(e) = writer.send(&msg).await {
                send_failed = Some(e.to_string());
                break;
            }
        }
        // A deliberate exit (detach, daemon shutdown) wins over a send error.
        if let Some(outcome) = app.exit.take() {
            break Phase::Exit(outcome);
        }
        if let Some(e) = send_failed {
            break Phase::Lost(e);
        }
    };
    drop(reader_task);
    Ok(phase)
}

/// Retries the connection with backoff, starting `hived` if it is gone and
/// auto-start is enabled. Returns `None` if the user quits meanwhile.
async fn reconnect(
    terminal: &mut Term,
    theme: &Theme,
    app: &mut App,
    events: &mut EventStream,
    paths: &Paths,
    auto_start: bool,
) -> anyhow::Result<Option<Client>> {
    let mut delay = Duration::from_millis(100);
    let mut last_spawn: Option<Instant> = None;
    loop {
        terminal.draw(|f| render::draw(f, app, theme))?;
        tokio::select! {
            ev = events.next() => match ev {
                Some(Ok(ev)) => {
                    app.on_terminal_event(ev);
                    if app.exit.is_some() {
                        return Ok(None);
                    }
                }
                Some(Err(e)) => return Err(e.into()),
                None => return Ok(None),
            },
            _ = tokio::time::sleep(delay) => {
                app.on_reconnect_attempt();
                match Client::connect(paths, "hive").await {
                    Ok(c) => return Ok(Some(c)),
                    Err(ClientError::NotRunning(_)) if auto_start => {
                        // Spawning is idempotent (the daemon's lock decides),
                        // but there is no point doing it on every attempt.
                        if last_spawn.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
                            last_spawn = Some(Instant::now());
                            if let Err(e) = spawn_daemon(paths) {
                                tracing::warn!("cannot start hived: {e}");
                            }
                        }
                    }
                    Err(e @ ClientError::VersionMismatch { .. }) => {
                        app.exit = Some(Outcome::DaemonGone(e.to_string()));
                        return Ok(None);
                    }
                    Err(e) => tracing::debug!("reconnect attempt failed: {e}"),
                }
                delay = (delay * 2).min(MAX_RECONNECT_DELAY);
            }
        }
    }
}
