//! The Hive frontend. A client of `hived`: it never owns PTYs or processes,
//! so exiting or crashing here leaves every agent running.

mod app;
mod keymap;
mod keys;
mod layout;
mod render;
mod theme;

use std::io::{Write, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    EventStream,
};
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
        let _ = execute!(stdout(), DisableBracketedPaste, DisableMouseCapture);
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

/// Carries out an [`app::Effect`]. Failures are logged, never shown: these
/// are conveniences, and the bell and sidebar already carry the news.
fn run_effect(config: &Config, effect: app::Effect) {
    match effect {
        app::Effect::Notify { title, body } => {
            let cmd = &config.integrations.desktop_notify.command;
            let Some((program, args)) = cmd.split_first() else {
                return;
            };
            let spawned = std::process::Command::new(program)
                .args(args)
                .args([title, body])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            match spawned {
                // Reap it off the UI thread.
                Ok(mut child) => drop(std::thread::spawn(move || child.wait())),
                Err(e) => tracing::warn!("desktop notification failed: {e}"),
            }
        }
        app::Effect::Journal { dir, line } => {
            let now = jiff::Zoned::now();
            let path = dir.join(format!("{}.md", now.strftime("%Y-%m-%d")));
            let entry = format!("- {} {line}\n", now.strftime("%H:%M"));
            let written = std::fs::create_dir_all(&dir).and_then(|_| {
                use std::io::Write as _;
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)?
                    .write_all(entry.as_bytes())
            });
            if let Err(e) = written {
                tracing::warn!("journal {}: {e}", path.display());
            }
        }
    }
}

/// Validates the frontend parts of the config (theme, keys), so mistakes are
/// reported before the terminal switches to the alternate screen.
pub fn check_config(config: &Config) -> anyhow::Result<()> {
    theme_for(config)?;
    keymap::Keymap::from_config(&config.keys, &app::hidden_commands(config))?;
    let obsidian = &config.integrations.obsidian;
    if obsidian.enabled && obsidian.notes_agent(config).is_none() {
        anyhow::bail!(
            "[integrations.obsidian] agent `{}` is not defined; leave it empty to use $EDITOR",
            obsidian.agent
        );
    }
    Ok(())
}

/// How the UI will look in this terminal, for `hive doctor`.
pub fn describe_terminal(config: &Config) -> anyhow::Result<String> {
    let env = theme::Env::current();
    let theme = Theme::from_config(&config.general.theme, &config.theme, &env)?;
    let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "unset".into());
    Ok(format!(
        "theme {}, {}, {} glyphs (TERM={}, COLORTERM={}{})",
        config.general.theme,
        theme.depth,
        if theme.glyphs.ascii {
            "ASCII"
        } else {
            "Unicode"
        },
        show(&env.term),
        show(&env.colorterm),
        if env.no_color { ", NO_COLOR" } else { "" },
    ))
}

fn theme_for(config: &Config) -> anyhow::Result<Theme> {
    Ok(Theme::from_config(
        &config.general.theme,
        &config.theme,
        &theme::Env::current(),
    )?)
}

pub async fn run(
    paths: Paths,
    client: Client,
    config: Config,
    start: Start,
) -> anyhow::Result<Outcome> {
    let theme = theme_for(&config)?;
    let mut terminal = ratatui::try_init()?;
    let _guard = TerminalGuard;
    execute!(stdout(), EnableBracketedPaste)?;
    if config.terminal.mouse_support {
        // Most terminals still select text natively with Shift held.
        execute!(stdout(), EnableMouseCapture)?;
    }

    let size = terminal.size()?;
    let auto_start = config.daemon.auto_start;
    let mut app = App::new(config, start, (size.width, size.height));
    app.config_path = Some(paths.config_file.display().to_string());
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
        if std::mem::take(&mut app.bell) {
            let mut out = stdout();
            let _ = out.write_all(b"\x07").and_then(|_| out.flush());
        }
        for effect in std::mem::take(&mut app.effects) {
            run_effect(&app.config, effect);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_is_valid() {
        let config = Config::parse(include_str!("../../../examples/config.toml"))
            .expect("examples/config.toml parses");
        check_config(&config).expect("examples/config.toml validates");
        assert!(config.agents().contains_key("aider"));
    }
}
