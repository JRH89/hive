//! The Hive frontend. A client of `hived`: it never owns PTYs or processes,
//! so exiting or crashing here leaves every agent running.

mod app;
mod keys;
mod render;
mod theme;

use std::io::stdout;

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, EventStream};
use crossterm::execute;
use futures_util::StreamExt;
use hive_core::client::Client;
use hive_core::config::Config;
use hive_protocol::ServerMessage;
use tokio::sync::mpsc;

use app::App;
pub use app::{Outcome, Start};
use theme::Theme;

/// Restores the user's terminal on every exit path, including errors.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableBracketedPaste);
        ratatui::restore();
    }
}

pub async fn run(client: Client, config: Config, start: Start) -> anyhow::Result<Outcome> {
    let (mut reader, mut writer) = client.into_split();

    // read_frame is not cancel-safe, so it runs in its own task and the main
    // loop selects on a channel instead.
    let (srv_tx, mut srv_rx) = mpsc::channel::<Result<ServerMessage, String>>(256);
    let reader_task = tokio::spawn(async move {
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
    });

    let mut terminal = ratatui::try_init()?;
    let _guard = TerminalGuard;
    execute!(stdout(), EnableBracketedPaste)?;

    let theme = Theme::detect();
    let size = terminal.size()?;
    let mut app = App::new(config, start, (size.width, size.height));
    let mut events = EventStream::new();

    for msg in app.on_connect() {
        writer.send(&msg).await?;
    }

    let outcome = loop {
        terminal.draw(|f| render::draw(f, &app, &theme))?;

        let out = tokio::select! {
            ev = events.next() => match ev {
                Some(Ok(ev)) => app.on_terminal_event(ev),
                Some(Err(e)) => return Err(e.into()),
                None => break Outcome::Detached { session: None },
            },
            msg = srv_rx.recv() => match msg {
                Some(Ok(m)) => {
                    let mut out = app.on_server(m);
                    // Apply everything already queued before redrawing, so a
                    // burst of output costs one frame instead of many.
                    while let Ok(Ok(m)) = srv_rx.try_recv() {
                        out.extend(app.on_server(m));
                    }
                    out
                }
                Some(Err(e)) => break Outcome::DaemonGone(e),
                None => break Outcome::DaemonGone("connection lost".into()),
            },
        };
        for msg in out {
            if let Err(e) = writer.send(&msg).await {
                app.exit.get_or_insert(Outcome::DaemonGone(e.to_string()));
                break;
            }
        }
        if let Some(outcome) = app.exit.take() {
            break outcome;
        }
    };

    reader_task.abort();
    Ok(outcome)
}
