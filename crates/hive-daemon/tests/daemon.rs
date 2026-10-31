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
        // `HIVE_LOG=debug cargo test ... -- --nocapture` shows daemon logs.
        if let Ok(filter) = std::env::var("HIVE_LOG") {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_test_writer()
                .try_init();
        }
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::with_runtime_dir(
            tmp.path().join("rt"),
            tmp.path().join("state"),
            tmp.path().join("config.toml"),
        );
        let daemon = spawn_daemon(&paths).await;
        Harness {
            paths,
            _tmp: tmp,
            daemon,
        }
    }

    /// Simulates a daemon crash: the run future is dropped without its
    /// graceful shutdown path, leaving a stale socket behind.
    async fn crash(&mut self) {
        self.daemon.abort();
        let _ = (&mut self.daemon).await;
        assert!(
            self.paths.socket.exists(),
            "crash should leave a stale socket"
        );
    }

    async fn shutdown(&mut self) {
        let mut c = self.client().await;
        c.send(&ClientMessage::Shutdown).await.unwrap();
        timeout(T, &mut self.daemon)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    async fn restart(&mut self) {
        self.daemon = spawn_daemon(&self.paths).await;
    }

    async fn client(&self) -> Client {
        Client::connect(&self.paths, "test").await.unwrap()
    }
}

/// Starts a daemon on `paths` and waits until it accepts connections.
async fn spawn_daemon(paths: &Paths) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let mut config = Config::default();
    config.daemon.kill_timeout_secs = 1;
    let daemon = tokio::spawn(hive_daemon::run(paths.clone(), config));
    for _ in 0..500 {
        if daemon.is_finished() {
            match daemon.await {
                // All test daemons share one process. A PTY child forked by a
                // concurrent test briefly inherits a crashed daemon's lock fd
                // (until its exec closes it), so retry. A real crash takes the
                // whole process down and cannot hit this.
                Ok(Err(e)) if e.is::<hive_daemon::AlreadyRunning>() => {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    return Box::pin(spawn_daemon(paths)).await;
                }
                other => panic!("daemon exited during startup: {other:?}"),
            }
        }
        if Client::connect(paths, "probe").await.is_ok() {
            return daemon;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("daemon did not start");
}

async fn list(client: &mut Client) -> Vec<hive_protocol::SessionInfo> {
    client.send(&ClientMessage::ListSessions).await.unwrap();
    timeout(
        T,
        client.recv_until(|m| match m {
            ServerMessage::SessionList(l) => Some(l),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap()
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

/// Attaches taking control unconditionally (the pre-v3 behaviour most tests
/// want).
async fn attach(client: &mut Client, id: SessionId, size: TermSize) -> Screen {
    attach_with(client, id, size, true).await.0
}

/// Attaches with an explicit `force`; returns the screen and whether this
/// client got input control.
async fn attach_with(
    client: &mut Client,
    id: SessionId,
    size: TermSize,
    force: bool,
) -> (Screen, bool) {
    client
        .send(&ClientMessage::Attach {
            session: id,
            size,
            force,
        })
        .await
        .unwrap();
    timeout(
        T,
        client.recv_until(|m| match m {
            ServerMessage::Attached {
                session,
                snapshot,
                controller,
            } => Some((Screen::new(session.size, &snapshot), controller)),
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

/// Last `<prefix><number>` on screen, e.g. the counter printed by a ticker.
fn counter(screen: &Screen, prefix: &str) -> u64 {
    let text = screen.contents();
    let start = text.rfind(prefix).expect("counter on screen") + prefix.len();
    text[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn switching_between_sessions_keeps_both_running() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c = h.client().await;
    let ticker = create(
        &mut c,
        sh(
            r#"i=0; while :; do i=$((i+1)); printf '\rtick=%d ' $i; sleep 0.02; done"#,
            size,
        ),
    )
    .await;
    let echo = create(&mut c, sh("cat", size)).await;

    let mut screen = attach(&mut c, ticker, size).await;
    wait_for_text(&mut c, &mut screen, "tick=").await;
    let before = counter(&screen, "tick=");

    // Switch away: the ticker is now unattached, the echo session interactive.
    let mut echo_screen = attach(&mut c, echo, size).await;
    c.send(&ClientMessage::Input(b"first\r".to_vec()))
        .await
        .unwrap();
    wait_for_text(&mut c, &mut echo_screen, "first").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Switch back: the ticker kept counting while it was in the background.
    let screen = attach(&mut c, ticker, size).await;
    let after = counter(&screen, "tick=");
    assert!(after > before + 5, "ticker stalled: {before} -> {after}");

    // And the echo session kept its screen and still takes input.
    let mut echo_screen = attach(&mut c, echo, size).await;
    assert!(echo_screen.contents().contains("first"));
    c.send(&ClientMessage::Input(b"second\r".to_vec()))
        .await
        .unwrap();
    wait_for_text(&mut c, &mut echo_screen, "second").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn noisy_session_does_not_starve_others() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c = h.client().await;
    let noisy = create(&mut c, sh("yes 'noise noise noise noise noise'", size)).await;
    let quiet = create(&mut c, sh("cat", size)).await;

    // A client attached to the flood that never reads its socket: the daemon
    // must resync it rather than block the PTY or other sessions.
    let mut stalled = h.client().await;
    stalled
        .send(&ClientMessage::Attach {
            session: noisy,
            size,
            force: true,
        })
        .await
        .unwrap();

    let mut screen = attach(&mut c, quiet, size).await;
    for i in 0..5 {
        let marker = format!("ping{i}");
        let started = std::time::Instant::now();
        c.send(&ClientMessage::Input(format!("{marker}\r").into_bytes()))
            .await
            .unwrap();
        wait_for_text(&mut c, &mut screen, &marker).await;
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "echo took {:?} while another session flooded",
            started.elapsed()
        );
        // Spread the pings out so the flood overruns the stalled client.
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // Once the stalled client finally reads, it must be told to resync
    // rather than having silently lost output.
    let resynced = timeout(
        T,
        stalled.recv_until(|m| matches!(m, ServerMessage::Resync { .. }).then_some(())),
    )
    .await;
    assert!(resynced.is_ok(), "stalled client was never resynced");

    c.send(&ClientMessage::Shutdown).await.unwrap();
    timeout(T, h.daemon).await.unwrap().unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_updates_label_and_empty_resets_it() {
    let h = Harness::start().await;
    let mut c = h.client().await;
    let id = create(&mut c, sh("sleep 60", TermSize::DEFAULT)).await;
    for (label, expected) in [("  frontend  ", "frontend"), ("", "shell")] {
        c.send(&ClientMessage::RenameSession {
            session: id,
            label: label.into(),
        })
        .await
        .unwrap();
        let got = timeout(
            T,
            c.recv_until(|m| match m {
                ServerMessage::SessionUpdated(info) if info.id == id && info.label == expected => {
                    Some(info.label)
                }
                _ => None,
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(got, expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn removed_session_is_never_resurrected_by_later_updates() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c = h.client().await;
    let doomed = create(&mut c, sh("sleep 60", size)).await;
    let other = create(&mut c, sh("sleep 60", size)).await;
    let _ = attach(&mut c, doomed, size).await;

    // What the TUI does: kill the attached session, then move to another.
    c.send(&ClientMessage::KillSession(doomed)).await.unwrap();
    let mut removed = false;
    let mut stale = Vec::new();
    let mut note = |m: &ServerMessage, removed: &mut bool| match m {
        ServerMessage::SessionRemoved(id) if *id == doomed => *removed = true,
        ServerMessage::SessionUpdated(info) if info.id == doomed && *removed => {
            stale.push(info.clone())
        }
        _ => {}
    };
    timeout(T, async {
        loop {
            let m = c.recv().await.unwrap().unwrap();
            note(&m, &mut removed);
            if matches!(m, ServerMessage::Detached { session } if session == doomed) {
                break;
            }
        }
    })
    .await
    .unwrap();
    c.send(&ClientMessage::Attach {
        session: other,
        size,
        force: false,
    })
    .await
    .unwrap();
    c.send(&ClientMessage::ListSessions).await.unwrap();
    timeout(T, async {
        loop {
            let m = c.recv().await.unwrap().unwrap();
            note(&m, &mut removed);
            if matches!(m, ServerMessage::SessionList(_)) {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(removed, "SessionRemoved never arrived");
    assert!(stale.is_empty(), "updates after removal: {stale:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn attaching_to_a_controlled_session_observes_until_forced() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c1 = h.client().await;
    let id = create(&mut c1, sh("cat", size)).await;
    let (_, c1_controls) = attach_with(&mut c1, id, size, false).await;
    assert!(c1_controls, "a free session is taken on attach");

    // A second client switching to it observes instead of stealing.
    let mut c2 = h.client().await;
    let (mut s2, c2_controls) = attach_with(
        &mut c2,
        id,
        TermSize {
            rows: 40,
            cols: 100,
        },
        false,
    )
    .await;
    assert!(!c2_controls);
    let (r, c) = s2.0.screen().size();
    assert_eq!(
        (r.get(), c.get()),
        (24, 80),
        "observers must not resize the PTY"
    );
    c2.send(&ClientMessage::Input(b"ignored\r".to_vec()))
        .await
        .unwrap();
    c1.send(&ClientMessage::Input(b"typed\r".to_vec()))
        .await
        .unwrap();
    wait_for_text(&mut c2, &mut s2, "typed").await;
    assert!(!s2.contents().contains("ignored"));

    // Forcing takes control explicitly and tells the old controller.
    let (_, forced) = attach_with(&mut c2, id, size, true).await;
    assert!(forced);
    timeout(
        T,
        c1.recv_until(|m| matches!(m, ServerMessage::ControlLost { .. }).then_some(())),
    )
    .await
    .unwrap()
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn observers_are_told_when_control_is_released() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c1 = h.client().await;
    let id = create(&mut c1, sh("sleep 60", size)).await;
    attach_with(&mut c1, id, size, false).await;
    let mut c2 = h.client().await;
    let (_, observing) = attach_with(&mut c2, id, size, false).await;
    assert!(!observing);

    // The controller's frontend disappears (crash or quit).
    drop(c1);
    timeout(
        T,
        c2.recv_until(|m| {
            matches!(m, ServerMessage::ControlReleased { session } if session == id).then_some(())
        }),
    )
    .await
    .unwrap()
    .unwrap();
    let (_, now_controls) = attach_with(&mut c2, id, size, false).await;
    assert!(now_controls, "free control is taken on the next attach");
}

#[tokio::test(flavor = "multi_thread")]
async fn crash_marks_running_sessions_lost_and_ids_keep_increasing() {
    let mut h = Harness::start().await;
    let mut c = h.client().await;
    let running = create(&mut c, sh("sleep 60", TermSize::DEFAULT)).await;
    c.send(&ClientMessage::RenameSession {
        session: running,
        label: "api".into(),
    })
    .await
    .unwrap();
    let exited = create(&mut c, sh("exit 0", TermSize::DEFAULT)).await;
    // Wait until the store reflects the rename and the exit.
    timeout(T, async {
        loop {
            let text = std::fs::read_to_string(h.paths.state_dir.join("sessions.json"))
                .unwrap_or_default();
            // Written after `exited` was created (next id is past it) and after
            // it exited (no longer listed).
            let next = format!("\"next_session_id\": {}", exited + 1);
            if text.contains("\"api\"")
                && text.contains(&next)
                && !text.contains(&format!("\"id\": {exited}"))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("store never caught up");
    drop(c);

    h.crash().await;
    h.restart().await;

    let mut c = h.client().await;
    let sessions = list(&mut c).await;
    assert_eq!(
        sessions.len(),
        1,
        "only the running session is remembered: {sessions:?}"
    );
    assert_eq!(sessions[0].id, running);
    assert_eq!(sessions[0].label, "api");
    assert_eq!(sessions[0].state, SessionState::Lost);

    // Lost sessions cannot be attached...
    c.send(&ClientMessage::Attach {
        session: running,
        size: TermSize::DEFAULT,
        force: false,
    })
    .await
    .unwrap();
    let err = timeout(T, c.recv_until(|_| None::<()>))
        .await
        .unwrap()
        .unwrap_err();
    assert!(err.to_string().contains("previous daemon"), "{err}");

    // ...new ids never collide with old ones...
    let fresh = create(&mut c, sh("sleep 60", TermSize::DEFAULT)).await;
    assert!(fresh > exited, "id {fresh} reused (previous max {exited})");

    // ...and removing a lost session just forgets it.
    c.send(&ClientMessage::KillSession(running)).await.unwrap();
    timeout(
        T,
        c.recv_until(|m| {
            matches!(m, ServerMessage::SessionRemoved(r) if r == running).then_some(())
        }),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(list(&mut c).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn graceful_shutdown_forgets_sessions_but_ids_keep_increasing() {
    let mut h = Harness::start().await;
    let mut c = h.client().await;
    let first = create(&mut c, sh("sleep 60", TermSize::DEFAULT)).await;
    drop(c);
    h.shutdown().await;
    h.restart().await;

    let mut c = h.client().await;
    assert!(
        list(&mut c).await.is_empty(),
        "deliberately stopped sessions are not 'lost'"
    );
    let next = create(&mut c, sh("sleep 60", TermSize::DEFAULT)).await;
    assert!(next > first);
}
