//! Wire protocol spoken between Hive clients (`hive`) and the daemon (`hived`).
//!
//! Every message is a length-prefixed frame: a little-endian `u32` byte count
//! followed by a `postcard`-encoded [`ClientMessage`] or [`ServerMessage`].
//!
//! Compatibility rule: `ClientMessage::Hello` and `ServerMessage::Welcome` must
//! stay variant 0 with an unchanged shape, so that mismatched versions can
//! always complete the handshake and report the mismatch cleanly.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Bumped on any incompatible change to the message types below.
pub const PROTOCOL_VERSION: u32 = 6;

/// Upper bound on a single frame. Protects both sides from unbounded
/// allocations caused by a corrupt or hostile length prefix.
pub const MAX_FRAME_LEN: usize = 8 * 1024 * 1024;

pub type SessionId = u64;
pub type WorkspaceId = u64;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermSize {
    pub rows: u16,
    pub cols: u16,
}

impl TermSize {
    pub const DEFAULT: TermSize = TermSize { rows: 24, cols: 80 };

    /// Clamps to at least 1x1, which every PTY and emulator requires.
    pub fn clamped(self) -> TermSize {
        TermSize {
            rows: self.rows.max(1),
            cols: self.cols.max(1),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Starting,
    Running,
    Exited {
        code: Option<u32>,
        signal: Option<String>,
    },
    Failed {
        reason: String,
    },
    /// Was running when a previous daemon stopped unexpectedly. Only the
    /// metadata survives; the process is gone and cannot be attached.
    Lost,
}

impl SessionState {
    pub fn is_alive(&self) -> bool {
        matches!(self, SessionState::Starting | SessionState::Running)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: SessionId,
    /// Adapter name, e.g. `claude` or `shell`.
    pub agent: String,
    pub label: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub state: SessionState,
    pub size: TermSize,
    /// Seconds since the Unix epoch.
    pub created_at: u64,
    pub attached_clients: u32,
    /// Every session belongs to exactly one workspace.
    pub workspace: WorkspaceId,
    /// Checked-out branch (or short commit id when detached) of the working
    /// tree `cwd` is in. Refreshed by the daemon while the session runs.
    pub branch: Option<String>,
    /// Root of the linked git worktree `cwd` is in, if any.
    pub worktree: Option<String>,
    /// The agent running in the session's foreground right now, if Hive
    /// recognises one. Not persisted.
    pub presence: Option<AgentPresence>,
}

/// What an agent is doing, from screen rules or its own report.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentStatus {
    /// Running, but its state can't be told (no rules for it).
    Unknown,
    Idle,
    Working,
    /// Waiting for the user: a permission prompt or question.
    Blocked,
}

/// How Hive knows which agent is running.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceSource {
    /// The foreground process is the agent the session was launched with.
    Launched,
    /// The foreground process was recognised (e.g. started from a shell).
    Process,
    /// The agent reported itself (`hive report-agent`).
    Reported,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AgentPresence {
    /// Catalog id (e.g. `claude`) or the name a reporting agent gave.
    pub agent: String,
    pub label: String,
    pub status: AgentStatus,
    /// Short note from a reporting agent, e.g. what it is waiting on.
    pub message: Option<String>,
    pub source: PresenceSource,
}

/// A named group of sessions, usually one repository.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub id: WorkspaceId,
    pub name: String,
    /// Canonical path. For a repository, the root of its main working tree.
    pub root: String,
    /// Whether `root` is a git repository (worktrees can be created).
    pub is_repo: bool,
}

/// Asks the daemon to create a linked worktree and launch the session in it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct NewWorktree {
    /// Checked out if it exists, created from HEAD otherwise. Defaults to
    /// `hive/<agent>-<session id>`.
    pub branch: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: Option<String>,
    /// The repository's main working tree (never removed by Hive).
    pub main: bool,
    /// `None` if git could not tell.
    pub dirty: Option<bool>,
    /// Live sessions running in this worktree.
    pub sessions: Vec<SessionId>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CreateSession {
    pub agent: String,
    pub label: String,
    /// argv; the first element is the executable. Never passed through a shell.
    pub command: Vec<String>,
    pub cwd: String,
    /// Full environment for the child. Clients send their own environment so
    /// agents see the PATH and credentials of the shell that launched them.
    pub env: Vec<(String, String)>,
    pub size: TermSize,
    /// Explicit workspace. When `None` the daemon picks (or creates) the one
    /// that contains `cwd`.
    pub workspace: Option<WorkspaceId>,
    /// Launch in a new linked worktree of the workspace's repository instead
    /// of `cwd`.
    pub worktree: Option<NewWorktree>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    /// Must be the first message on every connection.
    Hello {
        protocol_version: u32,
        client: String,
    },
    ListSessions,
    CreateSession(CreateSession),
    /// Attach to a session, in addition to any others this connection is
    /// attached to (one per visible pane). Re-attaching to the same session
    /// refreshes it without releasing control. Input control is taken if it
    /// is free, or always when `force` is set (the previous controller then
    /// gets `ControlLost`). Otherwise the client observes.
    Attach {
        session: SessionId,
        size: TermSize,
        force: bool,
    },
    /// Detach from one session, or from all when `None`.
    Detach {
        session: Option<SessionId>,
    },
    /// Raw bytes for an attached session's PTY. Ignored unless this client
    /// holds its input control.
    Input {
        session: SessionId,
        data: Vec<u8>,
    },
    /// New size for an attached session. Ignored unless this client holds
    /// its input control.
    Resize {
        session: SessionId,
        size: TermSize,
    },
    /// Terminate (if running) and remove a session.
    KillSession(SessionId),
    /// Change a session's label. An empty label resets it to the agent name.
    RenameSession {
        session: SessionId,
        label: String,
    },
    /// Terminate all sessions and stop the daemon.
    Shutdown,
    ListWorkspaces,
    /// Adds a workspace for `path` (its repository's main root when inside
    /// git). Replies `WorkspaceCreated`, with the existing workspace if one
    /// already has that root.
    CreateWorkspace {
        path: String,
        name: Option<String>,
    },
    RenameWorkspace {
        workspace: WorkspaceId,
        name: String,
    },
    /// Forgets a workspace that has no sessions. Nothing on disk changes.
    RemoveWorkspace(WorkspaceId),
    ListWorktrees(WorkspaceId),
    /// Removes a linked worktree. Refused while a session runs in it, and
    /// when it has uncommitted changes unless `force`. Branches are kept.
    RemoveWorktree {
        path: String,
        force: bool,
    },
    /// An agent reporting its own status, overriding screen detection until
    /// released. Reports with a `seq` not above the last one are ignored.
    ReportAgent {
        session: SessionId,
        agent: String,
        status: AgentStatus,
        message: Option<String>,
        seq: u64,
    },
    /// Ends a report by `agent`; detection takes over again.
    ReleaseAgent {
        session: SessionId,
        agent: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    Welcome {
        protocol_version: u32,
        daemon_version: String,
        daemon_pid: u32,
    },
    Error {
        message: String,
    },
    SessionList(Vec<SessionInfo>),
    SessionCreated(SessionInfo),
    /// Sent in response to `Attach`. `snapshot` is a byte stream that, fed to
    /// a fresh emulator of `session.size`, reproduces the current screen.
    Attached {
        session: SessionInfo,
        snapshot: Vec<u8>,
        controller: bool,
    },
    /// Sent instead of further `Output` when this client fell too far behind.
    /// The client must reset its emulator to `size` and replay `snapshot`.
    Resync {
        session: SessionId,
        size: TermSize,
        snapshot: Vec<u8>,
    },
    Output {
        session: SessionId,
        data: Vec<u8>,
    },
    /// The attached session's PTY changed size. Ordered with `Output`.
    Resized {
        session: SessionId,
        size: TermSize,
    },
    Detached {
        session: SessionId,
    },
    /// Another client attached and took input control.
    ControlLost {
        session: SessionId,
    },
    /// Broadcast to all clients when a session's metadata or state changes.
    SessionUpdated(SessionInfo),
    SessionRemoved(SessionId),
    ShuttingDown,
    /// The controller of the session this client observes detached; control
    /// can be taken with a forced `Attach`.
    ControlReleased {
        session: SessionId,
    },
    WorkspaceList(Vec<WorkspaceInfo>),
    WorkspaceCreated(WorkspaceInfo),
    /// Broadcast when a workspace is added or renamed.
    WorkspaceUpdated(WorkspaceInfo),
    WorkspaceRemoved(WorkspaceId),
    WorktreeList {
        workspace: WorkspaceId,
        worktrees: Vec<WorktreeInfo>,
    },
    WorktreeRemoved {
        path: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to encode message: {0}")]
    Encode(postcard::Error),
    #[error("failed to decode message: {0}")]
    Decode(postcard::Error),
    #[error("frame of {0} bytes exceeds the {MAX_FRAME_LEN} byte limit")]
    FrameTooLarge(usize),
}

/// Encodes `msg` as a complete frame, length prefix included.
pub fn encode_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = postcard::to_extend(msg, vec![0u8; 4]).map_err(ProtocolError::Encode)?;
    let len = buf.len() - 4;
    if len > MAX_FRAME_LEN {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    buf[..4].copy_from_slice(&(len as u32).to_le_bytes());
    Ok(buf)
}

pub async fn write_frame<W, T>(writer: &mut W, msg: &T) -> Result<(), ProtocolError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let frame = encode_frame(msg)?;
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

/// Reads one frame. Returns `Ok(None)` on a clean EOF at a frame boundary.
///
/// Not cancel-safe: run it in a dedicated task rather than in `select!`.
pub async fn read_frame<R, T>(reader: &mut R) -> Result<Option<T>, ProtocolError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_FRAME_LEN {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    postcard::from_bytes(&payload)
        .map(Some)
        .map_err(ProtocolError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_round_trip() {
        let msgs = vec![
            ClientMessage::Hello {
                protocol_version: PROTOCOL_VERSION,
                client: "test".into(),
            },
            ClientMessage::Input {
                session: 1,
                data: b"\x1b[A hello".to_vec(),
            },
            ClientMessage::Resize {
                session: 1,
                size: TermSize {
                    rows: 40,
                    cols: 120,
                },
            },
        ];
        let mut wire = Vec::new();
        for m in &msgs {
            write_frame(&mut wire, m).await.unwrap();
        }
        let mut rd = wire.as_slice();
        for m in &msgs {
            let got: ClientMessage = read_frame(&mut rd).await.unwrap().unwrap();
            assert_eq!(&got, m);
        }
        assert!(
            read_frame::<_, ClientMessage>(&mut rd)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn oversized_length_prefix_is_rejected() {
        let wire = ((MAX_FRAME_LEN + 1) as u32).to_le_bytes();
        let err = read_frame::<_, ClientMessage>(&mut wire.as_slice())
            .await
            .unwrap_err();
        assert!(matches!(err, ProtocolError::FrameTooLarge(_)));
    }

    #[test]
    fn handshake_variants_are_index_zero() {
        // postcard encodes the enum discriminant as the first varint byte.
        let hello = postcard::to_stdvec(&ClientMessage::Hello {
            protocol_version: 1,
            client: String::new(),
        })
        .unwrap();
        assert_eq!(hello[0], 0);
        let welcome = postcard::to_stdvec(&ServerMessage::Welcome {
            protocol_version: 1,
            daemon_version: String::new(),
            daemon_pid: 0,
        })
        .unwrap();
        assert_eq!(welcome[0], 0);
    }
}
