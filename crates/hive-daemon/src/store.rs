//! Session metadata persisted across daemon restarts.
//!
//! Processes cannot survive the daemon, so this only lets a new daemon show
//! what was running when the previous one died (as `Lost` sessions that can
//! be relaunched) and keeps session ids unique across restarts, so a client
//! reconnecting by id can never land on an unrelated session.
//!
//! Environment variables are never stored: they routinely hold credentials.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use hive_protocol::{SessionId, SessionInfo, SessionState, TermSize};
use serde::{Deserialize, Serialize};

const STORE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Store {
    pub version: u32,
    pub next_session_id: SessionId,
    /// Sessions that would be lost if the daemon died right now: live ones
    /// plus earlier `Lost` records the user has not dismissed.
    pub sessions: Vec<StoredSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredSession {
    pub id: SessionId,
    pub agent: String,
    pub label: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub created_at: u64,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            version: STORE_VERSION,
            next_session_id: 1,
            sessions: Vec::new(),
        }
    }
}

impl StoredSession {
    pub fn from_info(info: &SessionInfo) -> StoredSession {
        StoredSession {
            id: info.id,
            agent: info.agent.clone(),
            label: info.label.clone(),
            command: info.command.clone(),
            cwd: info.cwd.clone(),
            created_at: info.created_at,
        }
    }

    pub fn into_lost_info(self) -> SessionInfo {
        SessionInfo {
            id: self.id,
            agent: self.agent,
            label: self.label,
            command: self.command,
            cwd: self.cwd,
            state: SessionState::Lost,
            size: TermSize::DEFAULT,
            created_at: self.created_at,
            attached_clients: 0,
        }
    }
}

impl Store {
    /// Loads `path`. A missing file is an empty store; an unreadable or
    /// corrupt one is moved aside (`.corrupt`) so it can be inspected, and
    /// an empty store is used rather than refusing to start.
    pub fn load(path: &Path) -> Store {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Store::default(),
            Err(e) => {
                tracing::warn!("cannot read {}: {e}; starting empty", path.display());
                return Store::default();
            }
        };
        match serde_json::from_slice::<Store>(&bytes) {
            Ok(store) if store.version == STORE_VERSION => store,
            Ok(store) => {
                tracing::warn!(
                    version = store.version,
                    "unknown session store version; ignoring"
                );
                Store::default()
            }
            Err(e) => {
                let aside = path.with_extension("json.corrupt");
                tracing::warn!("corrupt session store ({e}); moved to {}", aside.display());
                let _ = fs::rename(path, aside);
                Store::default()
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(self).expect("store serializes");
        bytes.push(b'\n');
        bytes
    }

    /// Writes atomically: a crash mid-write leaves the previous file intact.
    pub fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
        let tmp = path.with_extension("json.tmp");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Store {
        Store {
            version: STORE_VERSION,
            next_session_id: 8,
            sessions: vec![StoredSession {
                id: 7,
                agent: "claude".into(),
                label: "api".into(),
                command: vec!["claude".into()],
                cwd: "/src/api".into(),
                created_at: 1,
            }],
        }
    }

    #[test]
    fn round_trips_through_disk_with_private_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        Store::write_bytes(&path, &sample().to_bytes()).unwrap();
        assert_eq!(Store::load(&path), sample());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn missing_and_corrupt_files_yield_an_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        assert_eq!(Store::load(&path), Store::default());
        fs::write(&path, b"{ not json").unwrap();
        assert_eq!(Store::load(&path), Store::default());
        assert!(dir.path().join("sessions.json.corrupt").exists());
    }
}
