//! Drives the real full-screen TUI inside a pseudo-terminal (a Unix PTY, or
//! ConPTY on Windows: what Windows Terminal itself uses), against a private
//! daemon. The screen is read back through a vt100 emulator and the TUI is
//! driven only with the bytes a terminal would send for real keystrokes.
//! Runs on Linux, macOS and Windows in CI.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

const LEADER: &[u8] = b"\x00"; // Ctrl+Space
const TIMEOUT: Duration = Duration::from_secs(60);

fn nz(n: u16) -> std::num::NonZeroU16 {
    std::num::NonZeroU16::new(n).unwrap()
}

fn exe(name: &str) -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_hive"))
        .with_file_name(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

fn hived() -> PathBuf {
    static BUILD: Once = Once::new();
    let path = exe("hived");
    BUILD.call_once(|| {
        if !path.is_file() {
            let st = Command::new(env!("CARGO"))
                .args(["build", "-p", "hive-daemon", "--bin", "hived"])
                .status()
                .unwrap();
            assert!(st.success(), "building hived failed");
        }
    });
    path
}

struct Env {
    tmp: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("work")).unwrap();
        Env { tmp }
    }

    fn vars(&self) -> Vec<(String, String)> {
        let t = self.tmp.path();
        let mut v = vec![
            ("HIVE_RUNTIME_DIR", t.join("rt").display().to_string()),
            ("HIVE_STATE_DIR", t.join("state").display().to_string()),
            ("HIVE_CONFIG", t.join("config.toml").display().to_string()),
            ("HIVE_DAEMON_BIN", hived().display().to_string()),
        ];
        if cfg!(unix) {
            v.push(("HOME", t.display().to_string()));
            v.push(("SHELL", "/bin/sh".into()));
        }
        v.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    fn cli(&self, args: &[&str]) -> String {
        let mut c = Command::new(env!("CARGO_BIN_EXE_hive"));
        c.args(args)
            .current_dir(self.tmp.path().join("work"))
            .env_remove("HIVE_LOG");
        for (k, v) in self.vars() {
            c.env(k, v);
        }
        let out = c.output().unwrap();
        assert!(
            out.status.success(),
            "hive {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn tui(&self, cols: u16, rows: u16, extra: &[(&str, &str)]) -> Tui {
        let pty = NativePtySystem::default()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_hive"));
        cmd.cwd(self.tmp.path().join("work"));
        cmd.env_remove("HIVE_LOG");
        cmd.env_remove("NO_COLOR");
        for (k, v) in self.vars() {
            cmd.env(k, v);
        }
        for (k, v) in extra {
            cmd.env(k, v);
        }
        let child = pty.slave.spawn_command(cmd).unwrap();
        drop(pty.slave);
        let parser = Arc::new(Mutex::new(vt100::Parser::new(nz(rows), nz(cols), 0)));
        let mut reader = pty.master.try_clone_reader().unwrap();
        let writer = Arc::new(Mutex::new(pty.master.take_writer().unwrap()));
        let (p, w) = (parser.clone(), writer.clone());
        std::thread::spawn(move || {
            let mut buf = [0u8; 16384];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                // ConPTY asks for the cursor position on start-up; answer
                // like a terminal would.
                if buf[..n].windows(4).any(|x| x == b"\x1b[6n") {
                    let _ = w.lock().unwrap().write_all(b"\x1b[1;1R");
                }
                p.lock().unwrap().process(&buf[..n]);
            }
        });
        Tui {
            parser,
            writer,
            child,
            master: pty.master,
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let mut c = Command::new(env!("CARGO_BIN_EXE_hive"));
        c.args(["daemon", "stop", "--yes"]);
        for (k, v) in self.vars() {
            c.env(k, v);
        }
        let _ = c.output();
    }
}

struct Tui {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
}

impl Tui {
    fn screen(&self) -> String {
        self.parser.lock().unwrap().screen().contents()
    }

    fn send(&self, bytes: &[u8]) {
        let mut w = self.writer.lock().unwrap();
        w.write_all(bytes).unwrap();
        w.flush().unwrap();
    }

    /// Types text the way a person would: one key event per character.
    fn type_str(&self, s: &str) {
        for b in s.bytes() {
            self.send(&[b]);
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn leader(&self, key: &[u8]) {
        self.send(LEADER);
        std::thread::sleep(Duration::from_millis(50));
        self.send(key);
    }

    fn wait(&self, what: &str, f: impl Fn(&str) -> bool) -> String {
        let start = Instant::now();
        loop {
            let s = self.screen();
            if f(&s) {
                return s;
            }
            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for {what}; screen:\n{s}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn wait_for(&self, text: &str) -> String {
        self.wait(&format!("{text:?}"), |s| s.contains(text))
    }

    fn resize(&self, cols: u16, rows: u16) {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        self.parser.lock().unwrap().set_size(nz(rows), nz(cols));
    }

    fn wait_exit(&mut self) {
        let start = Instant::now();
        while start.elapsed() < TIMEOUT {
            if self.child.try_wait().unwrap().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("hive did not exit; screen:\n{}", self.screen());
    }

    /// Whether any cell uses a 24-bit colour.
    fn has_truecolor(&self) -> bool {
        let p = self.parser.lock().unwrap();
        let s = p.screen();
        let (rows, cols) = s.size();
        (0..rows.get()).any(|r| {
            (0..cols.get()).any(|c| {
                s.cell(r, c).is_some_and(|cell| {
                    matches!(cell.fgcolor(), vt100::Color::Rgb(..))
                        || matches!(cell.bgcolor(), vt100::Color::Rgb(..))
                })
            })
        })
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// A shell command whose output (but not its echo) contains `hive-<tag>`.
fn marker_cmd(tag: &str) -> String {
    if cfg!(windows) {
        format!("Write-Output ('hive-' + '{tag}')\r")
    } else {
        format!("printf 'hive-%s\\n' {tag}\r")
    }
}

#[test]
fn tui_starts_types_splits_detaches_and_reattaches() {
    let env = Env::new();
    let tui = env.tui(120, 36, &[]);

    // Start-up: sidebar and a first terminal.
    tui.wait("the sidebar", |s| {
        s.contains("SPACES") && s.contains("AGENTS")
    });
    tui.wait("a terminal session", |s| s.contains("#1"));

    // Typing reaches the shell and its output is mirrored.
    std::thread::sleep(Duration::from_secs(2)); // let the shell draw its prompt
    tui.type_str(&marker_cmd("one"));
    tui.wait_for("hive-one");

    // Leader c: a second terminal, which becomes the attached one.
    tui.leader(b"c");
    tui.wait("a second session", |s| s.contains("#2"));
    std::thread::sleep(Duration::from_secs(2));
    tui.type_str(&marker_cmd("two"));
    tui.wait_for("hive-two");

    // Leader v: split right with a third terminal; both panes visible.
    tui.leader(b"v");
    tui.wait("a third session", |s| s.contains("#3"));
    std::thread::sleep(Duration::from_secs(2));
    tui.type_str(&marker_cmd("three"));
    let s = tui.wait_for("hive-three");
    assert!(
        s.contains("hive-two"),
        "left pane should still show its output:\n{s}"
    );

    // Help opens and closes.
    tui.leader(b"?");
    tui.wait_for("Command palette");
    tui.send(b"\x1b");
    tui.wait("help to close", |s| !s.contains("Command palette"));

    // A resize re-lays out without losing anything.
    tui.resize(100, 30);
    tui.wait("redraw after resize", |s| {
        s.contains("hive-three") && s.contains("SPACES")
    });

    // Backspace edits the line before Enter sends it.
    let cmd = marker_cmd("four");
    tui.type_str(&cmd[..cmd.len() - 1]);
    tui.type_str("XY");
    tui.send(b"\x7f\x7f");
    std::thread::sleep(Duration::from_millis(300));
    tui.send(b"\r");
    tui.wait_for("hive-four");

    // Leader q detaches; the TUI exits and the sessions keep running.
    let mut tui = tui;
    tui.leader(b"q");
    tui.wait_exit();
    let list = env.cli(&["list"]);
    assert!(
        list.lines().filter(|l| !l.trim().is_empty()).count() >= 4,
        "sessions should survive detach:\n{list}"
    );

    // Reattach: the screen comes back from the daemon.
    let tui = env.tui(120, 36, &[]);
    tui.wait("the restored screen", |s| {
        s.contains("SPACES") && s.contains("hive-")
    });
    tui.leader(b"q");
}

#[test]
fn windows_terminal_gets_truecolor() {
    let env = Env::new();
    let tui = env.tui(
        100,
        30,
        &[
            ("WT_SESSION", "00000000-0000-0000-0000-000000000000"),
            ("TERM", ""),
            ("COLORTERM", ""),
        ],
    );
    tui.wait("the sidebar", |s| s.contains("SPACES"));
    assert!(
        tui.has_truecolor(),
        "Windows Terminal supports 24-bit colour"
    );
    tui.leader(b"q");
}
