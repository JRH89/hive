//! End-to-end tests of the `hive` binary against a private, auto-started
//! `hived`. Each test gets its own runtime, state, config and HOME.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Once;

/// `hived` lives in another package, so `cargo test -p hive-cli` alone
/// doesn't build it. Build it once if it's missing.
fn hived() -> PathBuf {
    static BUILD: Once = Once::new();
    let path = Path::new(env!("CARGO_BIN_EXE_hive")).with_file_name("hived");
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
        Env {
            tmp: tempfile::tempdir().unwrap(),
        }
    }

    fn hive(&self, args: &[&str]) -> Output {
        let t = self.tmp.path();
        Command::new(env!("CARGO_BIN_EXE_hive"))
            .args(args)
            .current_dir(t)
            .env("HIVE_RUNTIME_DIR", t.join("rt"))
            .env("HIVE_STATE_DIR", t.join("state"))
            .env("HIVE_CONFIG", t.join("config.toml"))
            .env("HIVE_DAEMON_BIN", hived())
            .env("HOME", t)
            .env("SHELL", "/bin/sh")
            .env_remove("HIVE_LOG")
            .output()
            .unwrap()
    }

    /// Runs `hive` and returns stdout, asserting success.
    fn ok(&self, args: &[&str]) -> String {
        let out = self.hive(args);
        assert!(
            out.status.success(),
            "hive {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    /// Runs `hive` and returns stderr, asserting failure.
    fn fails(&self, args: &[&str]) -> String {
        let out = self.hive(args);
        assert!(
            !out.status.success(),
            "hive {args:?} unexpectedly succeeded"
        );
        String::from_utf8(out.stderr).unwrap()
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = self.hive(&["daemon", "stop"]);
    }
}

#[test]
fn version_and_help_carry_the_brand() {
    let env = Env::new();
    assert!(env.ok(&["--version"]).contains(env!("CARGO_PKG_VERSION")));
    let help = env.ok(&["--help"]);
    assert!(help.contains("One terminal. Many agents."), "{help}");
    assert!(help.contains("Examples:"), "{help}");
}

#[test]
fn sessions_are_created_listed_and_killed_through_an_auto_started_daemon() {
    let env = Env::new();
    assert!(env.ok(&["daemon", "status"]).contains("not running"));

    let created = env.ok(&["new", "shell", "-d", "-l", "background"]);
    assert!(created.contains("created session 1"), "{created}");

    let list = env.ok(&["list"]);
    assert!(
        list.contains("background") && list.contains("running"),
        "{list}"
    );
    assert!(env.ok(&["status"]).contains("1 active"));

    assert!(env.ok(&["kill", "1"]).contains("terminated"));
    assert!(env.ok(&["list"]).contains("no sessions"));

    assert!(env.ok(&["daemon", "stop"]).contains("stopped"));
    assert!(env.ok(&["daemon", "status"]).contains("not running"));
}

#[test]
fn mistakes_fail_with_a_message_and_nonzero_exit() {
    let env = Env::new();
    assert!(
        env.fails(&["new", "nosuchagent", "-d"])
            .contains("unknown agent")
    );
    assert!(
        env.fails(&["attach", "99"])
            .contains("no session with id 99")
    );
    assert!(env.fails(&["kill", "99"]).contains("99"));

    std::fs::write(
        env.tmp.path().join("config.toml"),
        "[keys]\nleader = \"x\"\n",
    )
    .unwrap();
    assert!(env.fails(&[]).contains("invalid config"));
    std::fs::write(env.tmp.path().join("config.toml"), "not toml [").unwrap();
    assert!(!env.fails(&["list"]).is_empty());
}

#[test]
fn doctor_reports_the_setup_and_fails_on_a_broken_config() {
    let env = Env::new();
    let report = env.ok(&["doctor"]);
    assert!(report.contains("not found; using defaults"), "{report}");
    assert!(report.contains("not running"), "{report}");
    assert!(report.contains("shell"), "{report}");

    std::fs::write(
        env.tmp.path().join("config.toml"),
        "[theme]\naccent = \"red\"\n",
    )
    .unwrap();
    let out = env.hive(&["doctor"]);
    assert!(!out.status.success());
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(
        report.contains("INVALID") && report.contains("accent"),
        "{report}"
    );
    // Doctor never starts the daemon.
    assert!(!env.tmp.path().join("rt").join("hived.sock").exists());
}

#[test]
fn agents_are_listed_and_can_report_their_status() {
    let env = Env::new();
    let all = env.ok(&["agents", "--all"]);
    assert!(
        all.contains("truss") && all.contains("truss-tui") && all.contains("Cursor Agent"),
        "{all}"
    );

    assert!(
        env.ok(&["new", "shell", "-d"])
            .contains("created session 1")
    );
    env.ok(&[
        "report-agent",
        "1",
        "--agent",
        "truss",
        "--state",
        "blocked",
        "--seq",
        "5",
    ]);
    let list = env.ok(&["list"]);
    assert!(list.contains("truss") && list.contains("blocked"), "{list}");

    env.ok(&["release-agent", "1", "--agent", "truss"]);
    assert!(!env.ok(&["list"]).contains("truss"));

    let err = env.fails(&["report-agent", "9", "--agent", "x", "--state", "idle"]);
    assert!(err.contains("no session with id 9"), "{err}");
    let err = env.fails(&["report-agent", "1", "--agent", "x", "--state", "busy"]);
    assert!(err.contains("expected idle, working"), "{err}");
}
