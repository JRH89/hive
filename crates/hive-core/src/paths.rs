//! Filesystem locations for config, runtime state (socket, lock) and logs.
//!
//! Environment overrides, mainly for tests and running several daemons side
//! by side: `HIVE_RUNTIME_DIR`, `HIVE_CONFIG`, `HIVE_STATE_DIR`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    /// Per-user, mode 0700. Holds the socket and lock file.
    pub runtime_dir: PathBuf,
    pub socket: PathBuf,
    pub lock: PathBuf,
    /// Logs live here.
    pub state_dir: PathBuf,
    pub config_file: PathBuf,
}

impl Paths {
    pub fn resolve() -> Paths {
        let runtime_dir = std::env::var_os("HIVE_RUNTIME_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::runtime_dir().map(|d| d.join("hive")))
            .unwrap_or_else(default_runtime_dir);
        let state_dir = std::env::var_os("HIVE_STATE_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::state_dir().map(|d| d.join("hive")))
            .unwrap_or_else(|| runtime_dir.join("state"));
        let config_file = std::env::var_os("HIVE_CONFIG")
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("hive").join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("hive-config.toml"));
        Paths::with_runtime_dir(runtime_dir, state_dir, config_file)
    }

    pub fn with_runtime_dir(
        runtime_dir: PathBuf,
        state_dir: PathBuf,
        config_file: PathBuf,
    ) -> Paths {
        Paths {
            socket: runtime_dir.join("hived.sock"),
            lock: runtime_dir.join("hived.lock"),
            runtime_dir,
            state_dir,
            config_file,
        }
    }

    /// Creates the runtime directory with mode 0700 and refuses to use one
    /// that is owned by another user or readable by group/others.
    pub fn ensure_runtime_dir(&self) -> io::Result<()> {
        ensure_private_dir(&self.runtime_dir)
    }

    pub fn ensure_state_dir(&self) -> io::Result<()> {
        ensure_private_dir(&self.state_dir)
    }
}

#[cfg(unix)]
fn default_runtime_dir() -> PathBuf {
    std::env::temp_dir().join(format!("hive-{}", rustix::process::getuid().as_raw()))
}

/// Under the user's own local app data, which only they can read.
#[cfg(windows)]
fn default_runtime_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("hive")
        .join("run")
}

impl Paths {
    /// Where clients reach the daemon: the socket, or on Windows the pipe.
    pub fn endpoint(&self) -> String {
        if cfg!(windows) {
            self.pipe_name()
        } else {
            self.socket.display().to_string()
        }
    }

    /// The named pipe the daemon listens on (Windows). Derived from the
    /// runtime directory, so isolated daemons (tests) get their own.
    pub fn pipe_name(&self) -> String {
        // FNV-1a: stable across runs and builds, unlike `DefaultHasher`.
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.runtime_dir.to_string_lossy().bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
        }
        format!(r"\\.\pipe\hive-{h:016x}")
    }

    /// Secret a client must send first on Windows, where the pipe's default
    /// ACL lets other users open it read-only (see ADR-0021).
    pub fn token_file(&self) -> PathBuf {
        self.runtime_dir.join("hived.token")
    }
}

#[cfg(windows)]
fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    // Local app data is private to the user by its default ACL.
    fs::create_dir_all(dir)
}

#[cfg(unix)]
fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let meta = fs::metadata(dir)?;
    let uid = rustix::process::getuid().as_raw();
    if meta.uid() != uid {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} is owned by uid {}, not {}",
                dir.display(),
                meta.uid(),
                uid
            ),
        ));
    }
    if meta.mode() & 0o077 != 0 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn runtime_dir_is_created_private() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("rt");
        let paths = Paths::with_runtime_dir(
            dir.clone(),
            tmp.path().join("st"),
            tmp.path().join("c.toml"),
        );
        paths.ensure_runtime_dir().unwrap();
        assert_eq!(fs::metadata(&dir).unwrap().mode() & 0o777, 0o700);

        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        paths.ensure_runtime_dir().unwrap();
        assert_eq!(fs::metadata(&dir).unwrap().mode() & 0o777, 0o700);
    }
}
