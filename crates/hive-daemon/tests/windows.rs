//! Windows end-to-end tests: the named-pipe transport and token (ADR-0021),
//! and a ConPTY session driven through `cmd.exe`.
#![cfg(windows)]

use std::time::Duration;

use hive_core::client::Client;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{ClientMessage, CreateSession, ServerMessage, SessionState, TermSize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

const T: Duration = Duration::from_secs(20);

async fn start(tmp: &tempfile::TempDir) -> (Paths, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let paths = Paths::with_runtime_dir(
        tmp.path().join("rt"),
        tmp.path().join("state"),
        tmp.path().join("config.toml"),
    );
    let mut config = Config::default();
    config.daemon.kill_timeout_secs = 1;
    let daemon = tokio::spawn(hive_daemon::run(paths.clone(), config));
    for _ in 0..500 {
        assert!(!daemon.is_finished(), "daemon exited during startup");
        if Client::connect(&paths, "probe").await.is_ok() {
            return (paths, daemon);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("daemon did not start");
}

fn cmd(args: &[&str]) -> CreateSession {
    CreateSession {
        agent: "shell".into(),
        label: "test".into(),
        command: ["cmd.exe"]
            .iter()
            .chain(args)
            .map(|s| s.to_string())
            .collect(),
        cwd: std::env::temp_dir().display().to_string(),
        env: Vec::new(),
        size: TermSize::DEFAULT,
        workspace: None,
        worktree: None,
    }
}

/// Collects the session's output until it contains `marker`.
async fn wait_for_output(c: &mut Client, marker: &str) {
    let mut seen = String::new();
    timeout(
        T,
        c.recv_until(|m| {
            match m {
                ServerMessage::Output { data, .. } => {
                    seen.push_str(&String::from_utf8_lossy(&data))
                }
                ServerMessage::Attached { snapshot, .. } => {
                    seen.push_str(&String::from_utf8_lossy(&snapshot))
                }
                _ => {}
            }
            seen.contains(marker).then_some(())
        }),
    )
    .await
    .unwrap_or_else(|_| panic!("no {marker:?} in output: {seen:?}"))
    .unwrap();
}

#[tokio::test]
async fn conpty_session_echoes_input_and_terminates() {
    let tmp = tempfile::tempdir().unwrap();
    let (paths, daemon) = start(&tmp).await;
    let mut c = Client::connect(&paths, "test").await.unwrap();

    c.send(&ClientMessage::CreateSession(cmd(&[
        "/q",
        "/k",
        "echo hive-ready",
    ])))
    .await
    .unwrap();
    let info = timeout(
        T,
        c.recv_until(|m| match m {
            ServerMessage::SessionCreated(info) => Some(info),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    c.send(&ClientMessage::Attach {
        session: info.id,
        size: TermSize::DEFAULT,
        force: true,
    })
    .await
    .unwrap();
    wait_for_output(&mut c, "hive-ready").await;

    // %OS% expands only if cmd really ran the typed line.
    c.send(&ClientMessage::Input {
        session: info.id,
        data: b"echo typed-%OS%\r".to_vec(),
    })
    .await
    .unwrap();
    wait_for_output(&mut c, "typed-Windows_NT").await;

    c.send(&ClientMessage::KillSession(info.id)).await.unwrap();
    timeout(
        T,
        c.recv_until(|m| match m {
            ServerMessage::SessionUpdated(s)
                if s.id == info.id && matches!(s.state, SessionState::Exited { .. }) =>
            {
                Some(())
            }
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap();

    c.send(&ClientMessage::Shutdown).await.unwrap();
    timeout(T, daemon).await.unwrap().unwrap().unwrap();
    assert!(!paths.token_file().exists(), "token removed on shutdown");
}

#[tokio::test]
async fn pipe_clients_without_the_token_are_dropped() {
    use tokio::net::windows::named_pipe::ClientOptions;
    let tmp = tempfile::tempdir().unwrap();
    let (paths, daemon) = start(&tmp).await;

    let mut pipe = ClientOptions::new().open(paths.pipe_name()).unwrap();
    pipe.write_all(&[b'0'; hive_core::transport::TOKEN_LEN])
        .await
        .unwrap();
    let mut buf = [0u8; 16];
    // The daemon closes the pipe: EOF or a broken-pipe error, never a reply.
    let read = timeout(T, pipe.read(&mut buf)).await.unwrap();
    assert!(matches!(read, Ok(0) | Err(_)), "got a reply: {read:?}");

    // The real token still works.
    let mut c = Client::connect(&paths, "test").await.unwrap();
    c.send(&ClientMessage::Shutdown).await.unwrap();
    timeout(T, daemon).await.unwrap().unwrap().unwrap();
}
