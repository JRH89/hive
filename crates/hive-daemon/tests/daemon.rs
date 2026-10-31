//! End-to-end tests against an in-process daemon using real PTYs and simple
//! shell commands (no agent CLIs required).

use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::time::Duration;

use hive_core::client::Client;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{
    AgentPresence, AgentStatus, ClientMessage, CreateSession, NewWorktree, PresenceSource,
    ServerMessage, SessionId, SessionInfo, SessionState, TermSize, WorkspaceInfo,
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
    config.worktrees.dir = Some(paths.state_dir.join("worktrees"));
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
        workspace: None,
        worktree: None,
    }
}

fn sh_in(script: &str, cwd: &Path) -> CreateSession {
    CreateSession {
        cwd: cwd.display().to_string(),
        ..sh(script, TermSize::DEFAULT)
    }
}

fn git(dir: &Path, args: &[&str]) {
    let st = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "git {args:?}");
}

/// A repository with one commit on `main`, at `<parent>/proj`.
fn git_repo(parent: &Path) -> PathBuf {
    let root = parent.join("proj");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    std::fs::write(root.join("a.txt"), "a").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root.canonicalize().unwrap()
}

async fn workspaces(client: &mut Client) -> Vec<WorkspaceInfo> {
    client.send(&ClientMessage::ListWorkspaces).await.unwrap();
    timeout(
        T,
        client.recv_until(|m| match m {
            ServerMessage::WorkspaceList(l) => Some(l),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap()
}

async fn create_info(client: &mut Client, req: CreateSession) -> SessionInfo {
    client
        .send(&ClientMessage::CreateSession(req))
        .await
        .unwrap();
    timeout(
        T,
        client.recv_until(|m| match m {
            ServerMessage::SessionCreated(info) => Some(info),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap()
}

/// Sends `msg` and returns the daemon's error text.
async fn expect_error(client: &mut Client, msg: ClientMessage) -> String {
    client.send(&msg).await.unwrap();
    timeout(T, client.recv_until(|_| None::<()>))
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
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

/// Mirrors what the TUI does for one pane: an emulator fed with the
/// snapshot and that session's stream.
struct Screen(vt100::Parser, SessionId);

impl Screen {
    fn new(session: SessionId, size: TermSize, snapshot: &[u8]) -> Screen {
        let mut p = vt100::Parser::new(
            NonZeroU16::new(size.rows).unwrap(),
            NonZeroU16::new(size.cols).unwrap(),
            0,
        );
        p.process(snapshot);
        Screen(p, session)
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
            } => Some((Screen::new(session.id, session.size, &snapshot), controller)),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap()
}

/// Feeds the screen's session output into it until its contents contain
/// `needle`. Output for other attached sessions is skipped.
async fn wait_for_text(client: &mut Client, screen: &mut Screen, needle: &str) {
    let id = screen.1;
    let fut = async {
        while !screen.contents().contains(needle) {
            match client.recv().await.unwrap().expect("daemon closed") {
                ServerMessage::Output { session, data } if session == id => screen.0.process(&data),
                ServerMessage::Resized { session, size } if session == id => screen.0.set_size(
                    NonZeroU16::new(size.rows).unwrap(),
                    NonZeroU16::new(size.cols).unwrap(),
                ),
                ServerMessage::Resync {
                    session,
                    size,
                    snapshot,
                } if session == id => *screen = Screen::new(id, size, &snapshot),
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

    c1.send(&ClientMessage::Input {
        session: id,
        data: b"hello\r".to_vec(),
    })
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
    c.send(&ClientMessage::Input {
        session: id,
        data: b"\r".to_vec(),
    })
    .await
    .unwrap();
    wait_for_text(&mut c, &mut screen, "40 120").await;

    // A later resize from the controller reaches the real PTY too.
    c.send(&ClientMessage::Resize {
        session: id,
        size: TermSize {
            rows: 30,
            cols: 100,
        },
    })
    .await
    .unwrap();
    c.send(&ClientMessage::Input {
        session: id,
        data: b"\r".to_vec(),
    })
    .await
    .unwrap();
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
    c1.send(&ClientMessage::Input {
        session: id,
        data: b"from-one\r".to_vec(),
    })
    .await
    .unwrap();
    c2.send(&ClientMessage::Input {
        session: id,
        data: b"from-two\r".to_vec(),
    })
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
    // Poll rather than wait for the broadcast: it can arrive before the
    // `SessionCreated` reply, which `create` would skip past.
    let state = timeout(T, async {
        loop {
            let s = list(&mut c).await.into_iter().find(|s| s.id == id).unwrap();
            if !s.state.is_alive() {
                break s.state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("exit reported");
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

    // Switch away like the TUI: detach the ticker, attach the echo session.
    c.send(&ClientMessage::Detach {
        session: Some(ticker),
    })
    .await
    .unwrap();
    let mut echo_screen = attach(&mut c, echo, size).await;
    c.send(&ClientMessage::Input {
        session: echo,
        data: b"first\r".to_vec(),
    })
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
    c.send(&ClientMessage::Input {
        session: echo,
        data: b"second\r".to_vec(),
    })
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
        c.send(&ClientMessage::Input {
            session: quiet,
            data: format!("{marker}\r").into_bytes(),
        })
        .await
        .unwrap();
        wait_for_text(&mut c, &mut screen, &marker).await;
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "echo took {:?} while another session flooded",
            started.elapsed()
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // The stalled client lags only once ~8 MiB backs up (socket buffer,
    // client queue, broadcast). Measure the flood with a reading observer
    // instead of assuming a rate, so a slow machine can't skip the overrun.
    let mut meter = h.client().await;
    let _ = attach_with(&mut meter, noisy, size, false).await;
    let mut seen = 0usize;
    timeout(Duration::from_secs(60), async {
        while seen < 16 << 20 {
            match meter.recv().await.unwrap().unwrap() {
                ServerMessage::Output { data, .. } => seen += data.len(),
                // The meter lagged too: at least a broadcast queue went by.
                ServerMessage::Resync { .. } => seen += 4 << 20,
                _ => {}
            }
        }
    })
    .await
    .expect("the flood never produced 16 MiB");

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
    // Lifecycle events are ordered among themselves (ADR-0008), but not
    // against replies, so fence on the events channel: a rename of `other`
    // is broadcast after anything already queued about `doomed`.
    c.send(&ClientMessage::RenameSession {
        session: other,
        label: "fence".into(),
    })
    .await
    .unwrap();
    timeout(T, async {
        loop {
            let m = c.recv().await.unwrap().unwrap();
            note(&m, &mut removed);
            if matches!(&m, ServerMessage::SessionUpdated(i) if i.id == other && i.label == "fence")
            {
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
    c2.send(&ClientMessage::Input {
        session: id,
        data: b"ignored\r".to_vec(),
    })
    .await
    .unwrap();
    c1.send(&ClientMessage::Input {
        session: id,
        data: b"typed\r".to_vec(),
    })
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
    // The exit must have happened first: a write from before `exited` was
    // even registered would otherwise satisfy the store check below.
    timeout(T, async {
        while list(&mut c)
            .await
            .iter()
            .any(|s| s.id == exited && s.state.is_alive())
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("exit reported");
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

#[tokio::test]
async fn sessions_are_grouped_into_workspaces_by_repository() {
    let h = Harness::start().await;
    let tmp = tempfile::tempdir().unwrap();
    let repo = git_repo(tmp.path());
    let plain = tmp.path().join("notes");
    std::fs::create_dir(&plain).unwrap();
    let mut c = h.client().await;

    let a = create_info(&mut c, sh_in("sleep 60", &repo.join("sub"))).await;
    let b = create_info(&mut c, sh_in("sleep 60", &repo)).await;
    let n = create_info(&mut c, sh_in("sleep 60", &plain)).await;

    assert_eq!(a.workspace, b.workspace, "same repository, same workspace");
    assert_ne!(a.workspace, n.workspace);
    assert_eq!(a.branch.as_deref(), Some("main"));
    assert_eq!(a.worktree, None);
    assert_eq!(n.branch, None);

    let ws = workspaces(&mut c).await;
    let repo_ws = ws.iter().find(|w| w.id == a.workspace).unwrap();
    assert_eq!(repo_ws.root, repo.display().to_string());
    assert_eq!(repo_ws.name, "proj");
    assert!(repo_ws.is_repo);
    let plain_ws = ws.iter().find(|w| w.id == n.workspace).unwrap();
    assert!(!plain_ws.is_repo);
    assert_eq!(plain_ws.name, "notes");
}

#[tokio::test]
async fn worktree_sessions_are_isolated_and_cleanup_is_safe() {
    let h = Harness::start().await;
    let tmp = tempfile::tempdir().unwrap();
    let repo = git_repo(tmp.path());
    let mut c = h.client().await;
    let main = create_info(&mut c, sh_in("sleep 60", &repo)).await;

    let wt = create_info(
        &mut c,
        CreateSession {
            workspace: Some(main.workspace),
            worktree: Some(NewWorktree { branch: None }),
            ..sh_in("sleep 60", &repo)
        },
    )
    .await;
    let path = PathBuf::from(wt.worktree.clone().expect("runs in a linked worktree"));
    assert_eq!(wt.workspace, main.workspace);
    assert_eq!(wt.branch, Some(format!("hive/shell-{}", wt.id)));
    assert!(path.starts_with(h.paths.state_dir.join("worktrees")));
    assert_eq!(PathBuf::from(&wt.cwd), path);

    c.send(&ClientMessage::ListWorktrees(main.workspace))
        .await
        .unwrap();
    let trees = timeout(
        T,
        c.recv_until(|m| match m {
            ServerMessage::WorktreeList { worktrees, .. } => Some(worktrees),
            _ => None,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(trees.len(), 2);
    assert!(trees[0].main);
    assert_eq!(trees[0].sessions, vec![main.id]);
    assert_eq!(trees[1].sessions, vec![wt.id]);
    assert_eq!(trees[1].dirty, Some(false));

    let remove = |force| ClientMessage::RemoveWorktree {
        path: path.display().to_string(),
        force,
    };
    let err = expect_error(&mut c, remove(true)).await;
    assert!(err.contains("terminate it first"), "{err}");

    c.send(&ClientMessage::KillSession(wt.id)).await.unwrap();
    timeout(
        T,
        c.recv_until(|m| {
            matches!(m, ServerMessage::SessionRemoved(id) if id == wt.id).then_some(())
        }),
    )
    .await
    .unwrap()
    .unwrap();

    std::fs::write(path.join("work.txt"), "unsaved").unwrap();
    let err = expect_error(&mut c, remove(false)).await;
    assert!(err.contains("uncommitted changes"), "{err}");
    assert!(path.join("work.txt").exists(), "nothing was deleted");

    c.send(&remove(true)).await.unwrap();
    timeout(
        T,
        c.recv_until(|m| matches!(m, ServerMessage::WorktreeRemoved { .. }).then_some(())),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!path.exists());
    // The branch survives the worktree.
    git(
        &repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/hive/shell-{}", wt.id),
        ],
    );

    // The main working tree is never removed.
    let err = expect_error(
        &mut c,
        ClientMessage::RemoveWorktree {
            path: repo.display().to_string(),
            force: true,
        },
    )
    .await;
    assert!(
        err.contains("not a linked worktree") || err.contains("terminate"),
        "{err}"
    );
}

#[tokio::test]
async fn branch_changes_are_published() {
    let h = Harness::start().await;
    let tmp = tempfile::tempdir().unwrap();
    let repo = git_repo(tmp.path());
    let mut c = h.client().await;
    let s = create_info(&mut c, sh_in("sleep 60", &repo)).await;
    assert_eq!(s.branch.as_deref(), Some("main"));

    git(&repo, &["checkout", "-q", "-b", "feature"]);
    timeout(
        T,
        c.recv_until(|m| match m {
            ServerMessage::SessionUpdated(i)
                if i.id == s.id && i.branch.as_deref() == Some("feature") =>
            {
                Some(())
            }
            _ => None,
        }),
    )
    .await
    .expect("branch change published")
    .unwrap();
}

#[tokio::test]
async fn workspaces_persist_and_are_removed_only_when_empty() {
    let mut h = Harness::start().await;
    let tmp = tempfile::tempdir().unwrap();
    let repo = git_repo(tmp.path());
    let mut c = h.client().await;

    let add = ClientMessage::CreateWorkspace {
        path: repo.join("sub").display().to_string(),
        name: Some("api".into()),
    };
    c.send(&add).await.unwrap();
    let pick = |m| match m {
        ServerMessage::WorkspaceCreated(w) => Some(w),
        _ => None,
    };
    let ws = timeout(T, c.recv_until(pick)).await.unwrap().unwrap();
    assert_eq!(ws.name, "api");
    assert_eq!(
        ws.root,
        repo.display().to_string(),
        "the repository root, not the subdirectory"
    );
    c.send(&add).await.unwrap();
    let again = timeout(T, c.recv_until(pick)).await.unwrap().unwrap();
    assert_eq!(again.id, ws.id, "adding the same root twice is idempotent");

    c.send(&ClientMessage::RenameWorkspace {
        workspace: ws.id,
        name: "backend".into(),
    })
    .await
    .unwrap();
    create(&mut c, sh_in("sleep 60", &repo)).await;
    let err = expect_error(&mut c, ClientMessage::RemoveWorkspace(ws.id)).await;
    assert!(err.contains("still has 1 session"), "{err}");
    drop(c);

    h.shutdown().await;
    h.restart().await;
    let mut c = h.client().await;
    let ws_list = workspaces(&mut c).await;
    let kept = ws_list
        .iter()
        .find(|w| w.id == ws.id)
        .expect("workspace remembered");
    assert_eq!(kept.name, "backend");
    assert!(
        list(&mut c).await.is_empty(),
        "sessions were stopped on purpose"
    );

    c.send(&ClientMessage::RemoveWorkspace(ws.id))
        .await
        .unwrap();
    timeout(
        T,
        c.recv_until(|m| {
            matches!(m, ServerMessage::WorkspaceRemoved(id) if id == ws.id).then_some(())
        }),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(workspaces(&mut c).await.iter().all(|w| w.id != ws.id));
}

/// Opens a raw socket connection, sends `bytes`, and returns what the daemon
/// wrote back before closing (or `None` if it is still open after a second).
async fn raw_exchange(paths: &Paths, bytes: &[u8]) -> Option<Vec<u8>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::UnixStream::connect(&paths.socket)
        .await
        .unwrap();
    s.write_all(bytes).await.unwrap();
    let mut reply = Vec::new();
    timeout(Duration::from_secs(1), s.read_to_end(&mut reply))
        .await
        .ok()
        .map(|_| reply)
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_clients_are_dropped_without_affecting_the_daemon() {
    let h = Harness::start().await;
    let mut c = h.client().await;
    let id = create(&mut c, sh("echo alive; sleep 60", TermSize::DEFAULT)).await;

    // Garbage, an oversized length prefix, and a valid frame that isn't Hello
    // are all refused during the handshake.
    let garbage = raw_exchange(&h.paths, b"\x05\x00\x00\x00\xff\xff\xff\xff\xff").await;
    assert!(garbage.is_some(), "daemon kept a garbage connection open");
    let huge = ((hive_protocol::MAX_FRAME_LEN + 1) as u32).to_le_bytes();
    assert!(raw_exchange(&h.paths, &huge).await.is_some());
    let not_hello = hive_protocol::encode_frame(&ClientMessage::ListSessions).unwrap();
    let reply = raw_exchange(&h.paths, &not_hello).await.unwrap();
    let msg: ServerMessage = hive_protocol::read_frame(&mut reply.as_slice())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(msg, ServerMessage::Error { .. }), "{msg:?}");

    // A client speaking another protocol version is told ours, then closed.
    let old = hive_protocol::encode_frame(&ClientMessage::Hello {
        protocol_version: hive_protocol::PROTOCOL_VERSION - 1,
        client: "old".into(),
    })
    .unwrap();
    let reply = raw_exchange(&h.paths, &old).await.expect("not closed");
    let msg: ServerMessage = hive_protocol::read_frame(&mut reply.as_slice())
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(msg, ServerMessage::Welcome { protocol_version, .. } if protocol_version == hive_protocol::PROTOCOL_VERSION)
    );

    // A handshaken, attached client that then sends garbage is dropped; its
    // session keeps running and other clients are unaffected.
    let mut hello = hive_protocol::encode_frame(&ClientMessage::Hello {
        protocol_version: hive_protocol::PROTOCOL_VERSION,
        client: "bad".into(),
    })
    .unwrap();
    hello.extend(
        hive_protocol::encode_frame(&ClientMessage::Attach {
            session: id,
            size: TermSize::DEFAULT,
            force: true,
        })
        .unwrap(),
    );
    hello.extend(b"\x03\x00\x00\x00\xff\xff\xff");
    assert!(raw_exchange(&h.paths, &hello).await.is_some());

    let sessions = list(&mut c).await;
    let s = sessions.iter().find(|s| s.id == id).unwrap();
    assert_eq!(s.state, SessionState::Running);
    let mut screen = attach(&mut c, id, TermSize::DEFAULT).await;
    wait_for_text(&mut c, &mut screen, "alive").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_observers_resize_does_not_change_the_pty() {
    let h = Harness::start().await;
    let mut c1 = h.client().await;
    let id = create(
        &mut c1,
        sh("while read x; do stty size; done", TermSize::DEFAULT),
    )
    .await;
    let small = TermSize {
        rows: 30,
        cols: 100,
    };
    let mut screen = attach(&mut c1, id, small).await;

    let mut c2 = h.client().await;
    let (_, controller) = attach_with(
        &mut c2,
        id,
        TermSize {
            rows: 50,
            cols: 150,
        },
        false,
    )
    .await;
    assert!(!controller);
    c2.send(&ClientMessage::Resize {
        session: id,
        size: TermSize {
            rows: 55,
            cols: 160,
        },
    })
    .await
    .unwrap();

    c1.send(&ClientMessage::Input {
        session: id,
        data: b"\r".to_vec(),
    })
    .await
    .unwrap();
    wait_for_text(&mut c1, &mut screen, "30 100").await;
    assert!(!screen.contents().contains("50 150") && !screen.contents().contains("55 160"));
}

#[tokio::test(flavor = "multi_thread")]
async fn reattach_restores_a_full_screen_app_exactly() {
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c1 = h.client().await;
    // Alternate screen, a positioned line, and a coloured word, like a TUI.
    let id = create(
        &mut c1,
        sh(
            r"echo primary-before; printf '\033[?1049h\033[2J\033[5;10Hfull-screen app\033[7;1H\033[31mred\033[0m'; read x; printf '\033[?1049l'; echo primary-after; sleep 60",
            size,
        ),
    )
    .await;
    let mut screen = attach(&mut c1, id, size).await;
    wait_for_text(&mut c1, &mut screen, "red").await;
    c1.send(&ClientMessage::Detach { session: None })
        .await
        .unwrap();
    drop(c1);

    let mut c2 = h.client().await;
    let again = attach(&mut c2, id, size).await;
    let s = again.0.screen();
    assert!(s.alternate_screen());
    assert_eq!(s.contents(), screen.contents());
    assert_eq!(s.cell(4, 9).unwrap().contents(), "f");
    assert_eq!(s.cell(6, 0).unwrap().fgcolor(), vt100::Color::Idx(1));

    // When the app quits, the primary screen it hid comes back, even for a
    // client that only ever saw the alternate one.
    let mut again = again;
    c2.send(&ClientMessage::Input {
        session: id,
        data: b"\r".to_vec(),
    })
    .await
    .unwrap();
    wait_for_text(&mut c2, &mut again, "primary-after").await;
    wait_for_text(&mut c2, &mut again, "primary-before").await;
    assert!(!again.0.screen().alternate_screen());
    assert!(
        again.contents().contains("primary-before"),
        "{}",
        again.contents()
    );
    assert!(!again.contents().contains("full-screen app"));
}

#[tokio::test(flavor = "multi_thread")]
async fn many_concurrent_sessions_and_clients_stay_correct() {
    const SESSIONS: u32 = 12;
    const CLIENTS: u32 = 4;
    let h = Harness::start().await;
    let size = TermSize::DEFAULT;
    let mut c = h.client().await;
    let mut ids = Vec::new();
    for n in 1..=SESSIONS {
        // A burst of output, then wait for input and exit with a distinct code.
        let script = format!(
            r#"i=0; while [ $i -lt 400 ]; do echo "s{n} line $i"; i=$((i+1)); done; echo "done-{n}"; read x; exit {n}"#
        );
        ids.push((n, create(&mut c, sh(&script, size)).await));
    }

    // Several clients attach to their share of the sessions concurrently,
    // switching between them, and answer each one.
    let mut tasks = Vec::new();
    for k in 0..CLIENTS {
        let mine: Vec<_> = ids
            .iter()
            .copied()
            .filter(|(n, _)| n % CLIENTS == k)
            .collect();
        let mut client = h.client().await;
        tasks.push(tokio::spawn(async move {
            for (n, id) in mine {
                let mut screen = attach(&mut client, id, size).await;
                wait_for_text(&mut client, &mut screen, &format!("done-{n}")).await;
                client
                    .send(&ClientMessage::Input {
                        session: id,
                        data: b"\r".to_vec(),
                    })
                    .await
                    .unwrap();
            }
        }));
    }
    for t in tasks {
        timeout(T, t).await.unwrap().unwrap();
    }

    let deadline = tokio::time::Instant::now() + T;
    loop {
        let sessions = list(&mut c).await;
        let codes: Vec<_> = ids
            .iter()
            .map(|(n, id)| {
                let s = sessions.iter().find(|s| s.id == *id).unwrap();
                match &s.state {
                    SessionState::Exited { code, .. } => *code == Some(*n),
                    _ => false,
                }
            })
            .collect();
        if codes.iter().all(|ok| *ok) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "not all sessions exited with their own code: {sessions:?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Waits for a `SessionUpdated` for `id` whose presence satisfies `want`.
async fn wait_presence(
    c: &mut Client,
    id: SessionId,
    what: &str,
    want: impl Fn(Option<&AgentPresence>) -> bool,
) -> Option<AgentPresence> {
    timeout(
        T,
        c.recv_until(|m| match m {
            ServerMessage::SessionUpdated(i) if i.id == id && want(i.presence.as_ref()) => {
                Some(i.presence)
            }
            _ => None,
        }),
    )
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_are_recognised_and_their_status_tracked() {
    let h = Harness::start().await;
    let tmp = tempfile::tempdir().unwrap();
    // A stand-in `claude`: a busy title spinner until input, then idle.
    let fake = tmp.path().join("claude");
    std::fs::write(
        &fake,
        "#!/bin/sh\nprintf '\\033]0;\\342\\240\\213 Claude Code\\007busy\\n'\n\
         read x\nprintf '\\033]0;Claude Code\\007done\\n'\nsleep 60\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();

    let mut c = h.client().await;
    let s = create_info(&mut c, sh(&fake.display().to_string(), TermSize::DEFAULT)).await;
    let p = wait_presence(&mut c, s.id, "working claude", |p| {
        p.is_some_and(|p| p.agent == "claude" && p.status == AgentStatus::Working)
    })
    .await
    .unwrap();
    assert_eq!(p.label, "Claude Code");
    assert_eq!(p.source, PresenceSource::Process, "launched as `shell`");

    let mut screen = attach(&mut c, s.id, TermSize::DEFAULT).await;
    wait_for_text(&mut c, &mut screen, "busy").await;
    c.send(&ClientMessage::Input {
        session: s.id,
        data: b"go\r".to_vec(),
    })
    .await
    .unwrap();
    wait_presence(&mut c, s.id, "idle claude", |p| {
        p.is_some_and(|p| p.status == AgentStatus::Idle)
    })
    .await;

    // A self-report overrides detection until released; stale reports are
    // ignored.
    let report = |status, seq| ClientMessage::ReportAgent {
        session: s.id,
        agent: "truss".into(),
        status,
        message: Some("approve edit?".into()),
        seq,
    };
    c.send(&report(AgentStatus::Blocked, 2)).await.unwrap();
    let p = wait_presence(&mut c, s.id, "reported truss", |p| {
        p.is_some_and(|p| p.source == PresenceSource::Reported)
    })
    .await
    .unwrap();
    assert_eq!(
        (
            p.agent.as_str(),
            p.label.as_str(),
            p.status,
            p.message.as_deref()
        ),
        (
            "truss",
            "Truss",
            AgentStatus::Blocked,
            Some("approve edit?")
        )
    );
    c.send(&report(AgentStatus::Working, 1)).await.unwrap();
    c.send(&ClientMessage::ReleaseAgent {
        session: s.id,
        agent: "truss".into(),
    })
    .await
    .unwrap();
    let p = wait_presence(&mut c, s.id, "release", |p| {
        p.is_some_and(|p| p.source != PresenceSource::Reported)
    })
    .await
    .unwrap();
    assert_eq!((p.agent.as_str(), p.status), ("claude", AgentStatus::Idle));
    assert!(
        list(&mut c).await[0].presence.is_some(),
        "listed sessions carry presence"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plain_shell_has_no_agent_presence() {
    let h = Harness::start().await;
    let mut c = h.client().await;
    let s = create_info(&mut c, sh("echo ready; sleep 60", TermSize::DEFAULT)).await;
    let mut screen = attach(&mut c, s.id, TermSize::DEFAULT).await;
    wait_for_text(&mut c, &mut screen, "ready").await;
    // Several detection rounds pass; nothing is ever reported.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(list(&mut c).await[0].presence, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn one_client_mirrors_several_sessions_at_their_own_sizes() {
    let h = Harness::start().await;
    let mut c = h.client().await;
    let left = TermSize { rows: 20, cols: 40 };
    let right = TermSize { rows: 10, cols: 30 };
    let a = create(&mut c, sh("cat", left)).await;
    let b = create(&mut c, sh("cat", right)).await;
    let mut sa = attach(&mut c, a, left).await;
    let mut sb = attach(&mut c, b, right).await;

    // Input is routed by session and each pane sees only its own output.
    c.send(&ClientMessage::Input {
        session: a,
        data: b"to-a\r".to_vec(),
    })
    .await
    .unwrap();
    c.send(&ClientMessage::Input {
        session: b,
        data: b"to-b\r".to_vec(),
    })
    .await
    .unwrap();
    timeout(T, async {
        while !(sa.contents().contains("to-a") && sb.contents().contains("to-b")) {
            if let ServerMessage::Output { session, data } = c.recv().await.unwrap().unwrap() {
                for s in [&mut sa, &mut sb] {
                    if s.1 == session {
                        s.0.process(&data);
                    }
                }
            }
        }
    })
    .await
    .expect("both panes received their own output");
    assert!(!sa.contents().contains("to-b") && !sb.contents().contains("to-a"));

    let sizes: Vec<_> = list(&mut c)
        .await
        .into_iter()
        .map(|s| (s.id, s.size))
        .collect();
    assert!(sizes.contains(&(a, left)) && sizes.contains(&(b, right)));

    // Detaching one pane leaves the other attached.
    c.send(&ClientMessage::Detach { session: Some(a) })
        .await
        .unwrap();
    timeout(
        T,
        c.recv_until(|m| {
            matches!(m, ServerMessage::Detached { session } if session == a).then_some(())
        }),
    )
    .await
    .unwrap()
    .unwrap();
    c.send(&ClientMessage::Input {
        session: b,
        data: b"still-b\r".to_vec(),
    })
    .await
    .unwrap();
    wait_for_text(&mut c, &mut sb, "still-b").await;
    c.send(&ClientMessage::Input {
        session: a,
        data: b"ignored\r".to_vec(),
    })
    .await
    .unwrap();
    let mut fresh = attach(&mut c, a, left).await;
    wait_for_text(&mut c, &mut fresh, "to-a").await;
    assert!(
        !fresh.contents().contains("ignored"),
        "input after detach is dropped"
    );
}
