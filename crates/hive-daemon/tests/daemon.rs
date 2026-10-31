//! End-to-end tests against an in-process daemon using real PTYs and simple
//! shell commands (no agent CLIs required).

use std::num::NonZeroU16;
use std::time::Duration;

use hive_core::client::Client;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{
    ClientMessage, CreateSession, ServerMessage, SessionId, SessionState, TermSize,
};
use tokio::time::timeout;

const T: Duration = Duration::from_secs(10);

struct Harness {
    paths: Paths,
    _tmp: tempfile::TempDir,
    daemon: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Harness {
    async fn start() -> Harness {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::with_runtime_dir(
            tmp.path().join("rt"),
            tmp.path().join("state"),
            tmp.path().join("config.toml"),
        );
        let mut config = Config::default();
        config.daemon.kill_timeout_secs = 1;
        let daemon = tokio::spawn(hive_daemon::run(paths.clone(), config));
        for _ in 0..200 {
            if paths.socket.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Harness {
            paths,
            _tmp: tmp,
            daemon,
        }
    }

    async fn client(&self) -> Client {
        Client::connect(&self.paths, "test").await.unwrap()
    }
}

fn sh(script: &str, size: TermSize) -> CreateSession {
    CreateSession {
        agent: "shell".into(),
        label: "test".into(),
        command: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: std::env::temp_dir().display().to_string(),
        env: vec![("PATH".into(), std::env::var("PATH").unwrap_or_default())],
        size,
    }
}

async fn create(client: &mut Client, req: CreateSession) -> SessionId {
    client
        .send(&ClientMessage::CreateSession(req))
        .await
        .unwrap();
    timeout(
        T,
        client.recv_until(|m| match m {
            ServerMessage::SessionCreated(info) => Some(info.id),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap()
}

/// Mirrors what the TUI does: an emulator fed with the snapshot and stream.
struct Screen(vt100::Parser);

impl Screen {
    fn new(size: TermSize, snapshot: &[u8]) -> Screen {
        let mut p = vt100::Parser::new(
            NonZeroU16::new(size.rows).unwrap(),
            NonZeroU16::new(size.cols).unwrap(),
            0,
        );
        p.process(snapshot);
        Screen(p)
    }

    fn contents(&self) -> String {
        self.0.screen().contents()
    }
}

async fn attach(client: &mut Client, id: SessionId, size: TermSize) -> Screen {
    client
        .send(&ClientMessage::Attach { session: id, size })
        .await
        .unwrap();
    timeout(
        T,
        client.recv_until(|m| match m {
            ServerMessage::Attached {
                session, snapshot, ..
            } => Some(Screen::new(session.size, &snapshot)),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap()
}

/// Feeds output into `screen` until its contents contain `needle`.
async fn wait_for_text(client: &mut Client, screen: &mut Screen, needle: &str) {
    let fut = async {
        while !screen.contents().contains(needle) {
            match client.recv().await.unwrap().expect("daemon closed") {
                ServerMessage::Output { data, .. } => screen.0.process(&data),
                ServerMessage::Resized { size, .. } => screen.0.set_size(
                    NonZeroU16::new(size.rows).unwrap(),
                    NonZeroU16::new(size.cols).unwrap(),
                ),
                ServerMessage::Resync { size, snapshot, .. } => {
                    *screen = Screen::new(size, &snapshot)
                }
                _ => {}
            }
        }
    };
    if timeout(T, fut).await.is_err() {
        panic!(
            "timed out waiting for {needle:?}; screen:\n{}",
            screen.contents()
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn interactive_session_survives_client_disconnect() {
    let h = Harness::start().await;
    let size = TermSize { rows: 24, cols: 80 };

    let mut c1 = h.client().await;
    let id = create(
        &mut c1,
        sh(r#"echo ready; read line; echo "got:$line"; sleep 60"#, size),
    )
    .await;
    let mut screen = attach(&mut c1, id, size).await;
    wait_for_text(&mut c1, &mut screen, "ready").await;

    c1.send(&ClientMessage::Input(b"hello\r".to_vec()))
        .await
        .unwrap();
    wait_for_text(&mut c1, &mut screen, "got:hello").await;

    // Frontend goes away without detaching (simulates a crash).
    drop(c1);

    let mut c2 = h.client().await;
    c2.send(&ClientMessage::ListSessions).await.unwrap();
    let sessions = timeout(
        T,
        c2.recv_until(|m| match m {
            ServerMessage::SessionList(l) => Some(l),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].state, SessionState::Running);

    let screen = attach(&mut c2, id, size).await;
    let text = screen.contents();
    assert!(
        text.contains("ready") && text.contains("got:hello"),
        "snapshot:\n{text}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_resizes_pty_to_client_viewport() {
    let h = Harness::start().await;
    let mut c = h.client().await;
    let id = create(
        &mut c,
        sh(
            "read x; stty size; read y; stty size; sleep 60",
            TermSize { rows: 24, cols: 80 },
        ),
    )
    .await;
    let big = TermSize {
        rows: 40,
        cols: 120,
    };
    let mut screen = attach(&mut c, id, big).await;
    c.send(&ClientMessage::Input(b"\r".to_vec())).await.unwrap();
    wait_for_text(&mut c, &mut screen, "40 120").await;

    // A later resize from the controller reaches the real PTY too.
    c.send(&ClientMessage::Resize(TermSize {
        rows: 30,
        cols: 100,
    }))
    .await
    .unwrap();
    c.send(&ClientMessage::Input(b"\r".to_vec())).await.unwrap();
    wait_for_text(&mut c, &mut screen, "30 100").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn second_client_takes_input_control() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c1 = h.client().await;
    let id = create(&mut c1, sh("cat", size)).await;
    let _ = attach(&mut c1, id, size).await;

    let mut c2 = h.client().await;
    let mut s2 = attach(&mut c2, id, size).await;
    timeout(
        T,
        c1.recv_until(|m| matches!(m, ServerMessage::ControlLost { .. }).then_some(())),
    )
    .await
    .unwrap()
    .unwrap();

    // Input from the observer is ignored; the controller's arrives.
    c1.send(&ClientMessage::Input(b"from-one\r".to_vec()))
        .await
        .unwrap();
    c2.send(&ClientMessage::Input(b"from-two\r".to_vec()))
        .await
        .unwrap();
    wait_for_text(&mut c2, &mut s2, "from-two").await;
    assert!(!s2.contents().contains("from-one"));
}

#[tokio::test(flavor = "multi_thread")]
async fn exit_is_reported_and_kill_removes_session() {
    let h = Harness::start().await;
    let mut c = h.client().await;
    let id = create(&mut c, sh("exit 3", TermSize::DEFAULT)).await;
    let state = timeout(
        T,
        c.recv_until(|m| match m {
            ServerMessage::SessionUpdated(info) if info.id == id && !info.state.is_alive() => {
                Some(info.state)
            }
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        state,
        SessionState::Exited {
            code: Some(3),
            signal: None
        }
    );

    // A process that ignores SIGTERM is SIGKILLed after the timeout. Wait for
    // the trap to be installed first, or SIGTERM wins the race.
    let stubborn = create(
        &mut c,
        sh("trap '' TERM; echo armed; sleep 60", TermSize::DEFAULT),
    )
    .await;
    let mut screen = attach(&mut c, stubborn, TermSize::DEFAULT).await;
    wait_for_text(&mut c, &mut screen, "armed").await;
    let started = std::time::Instant::now();
    for target in [id, stubborn] {
        c.send(&ClientMessage::KillSession(target)).await.unwrap();
        timeout(
            T,
            c.recv_until(|m| {
                matches!(m, ServerMessage::SessionRemoved(r) if r == target).then_some(())
            }),
        )
        .await
        .unwrap()
        .unwrap();
    }
    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "stubborn session should only die after the SIGKILL timeout"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn second_daemon_refuses_to_start_and_shutdown_cleans_up() {
    let h = Harness::start().await;
    let err = hive_daemon::run(h.paths.clone(), Config::default())
        .await
        .unwrap_err();
    assert!(err.is::<hive_daemon::AlreadyRunning>());

    let mut c = h.client().await;
    c.send(&ClientMessage::Shutdown).await.unwrap();
    timeout(T, h.daemon).await.unwrap().unwrap().unwrap();
    assert!(!h.paths.socket.exists());
}
