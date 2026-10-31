//! Client side of the daemon connection, including daemon auto-start.

use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hive_protocol::{
    ClientMessage, PROTOCOL_VERSION, ProtocolError, ServerMessage, read_frame, write_frame,
};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

use crate::paths::Paths;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("daemon is not running (socket {0})")]
    NotRunning(PathBuf),
    #[error("could not connect to daemon: {0}")]
    Connect(io::Error),
    #[error("failed to start hived: {0}")]
    Spawn(io::Error),
    #[error("hived did not come up within {0:?}; see its log in the state directory")]
    StartTimeout(Duration),
    #[error(
        "daemon speaks protocol v{daemon}, this client speaks v{client}. \
         Run `hive daemon stop` (terminates all sessions) to restart it"
    )]
    VersionMismatch { daemon: u32, client: u32 },
    #[error("daemon closed the connection")]
    Closed,
    #[error("daemon error: {0}")]
    Daemon(String),
    #[error("unexpected message from daemon: {0:?}")]
    Unexpected(Box<ServerMessage>),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}

#[derive(Debug, Clone)]
pub struct DaemonInfo {
    pub version: String,
    pub pid: u32,
}

pub struct Client {
    reader: ClientReader,
    writer: ClientWriter,
    pub daemon: DaemonInfo,
}

pub struct ClientReader(OwnedReadHalf);
pub struct ClientWriter(OwnedWriteHalf);

impl ClientReader {
    /// Not cancel-safe; see [`hive_protocol::read_frame`].
    pub async fn recv(&mut self) -> Result<Option<ServerMessage>, ClientError> {
        Ok(read_frame(&mut self.0).await?)
    }
}

impl ClientWriter {
    pub async fn send(&mut self, msg: &ClientMessage) -> Result<(), ClientError> {
        Ok(write_frame(&mut self.0, msg).await?)
    }
}

impl Client {
    /// Connects to a running daemon and performs the handshake.
    pub async fn connect(paths: &Paths, client_name: &str) -> Result<Client, ClientError> {
        let stream = match UnixStream::connect(&paths.socket).await {
            Ok(s) => s,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                return Err(ClientError::NotRunning(paths.socket.clone()));
            }
            Err(e) => return Err(ClientError::Connect(e)),
        };
        let (rd, wr) = stream.into_split();
        let mut reader = ClientReader(rd);
        let mut writer = ClientWriter(wr);
        writer
            .send(&ClientMessage::Hello {
                protocol_version: PROTOCOL_VERSION,
                client: client_name.to_string(),
            })
            .await?;
        match reader.recv().await? {
            Some(ServerMessage::Welcome {
                protocol_version,
                daemon_version,
                daemon_pid,
            }) => {
                if protocol_version != PROTOCOL_VERSION {
                    return Err(ClientError::VersionMismatch {
                        daemon: protocol_version,
                        client: PROTOCOL_VERSION,
                    });
                }
                Ok(Client {
                    reader,
                    writer,
                    daemon: DaemonInfo {
                        version: daemon_version,
                        pid: daemon_pid,
                    },
                })
            }
            Some(ServerMessage::Error { message }) => Err(ClientError::Daemon(message)),
            Some(other) => Err(ClientError::Unexpected(Box::new(other))),
            None => Err(ClientError::Closed),
        }
    }

    /// Connects, starting `hived` first if it is not running and
    /// `auto_start` is set.
    pub async fn connect_or_start(
        paths: &Paths,
        client_name: &str,
        auto_start: bool,
    ) -> Result<Client, ClientError> {
        match Client::connect(paths, client_name).await {
            Err(ClientError::NotRunning(_)) if auto_start => {}
            other => return other,
        }
        spawn_daemon(paths)?;
        let timeout = Duration::from_secs(5);
        let deadline = Instant::now() + timeout;
        loop {
            tokio::time::sleep(Duration::from_millis(25)).await;
            match Client::connect(paths, client_name).await {
                Err(ClientError::NotRunning(_)) if Instant::now() < deadline => continue,
                Err(ClientError::NotRunning(_)) => return Err(ClientError::StartTimeout(timeout)),
                other => return other,
            }
        }
    }

    pub async fn send(&mut self, msg: &ClientMessage) -> Result<(), ClientError> {
        self.writer.send(msg).await
    }

    pub async fn recv(&mut self) -> Result<Option<ServerMessage>, ClientError> {
        self.reader.recv().await
    }

    /// Receives until a message matching `pick` arrives, skipping broadcasts.
    /// A daemon `Error` is returned as `ClientError::Daemon`.
    pub async fn recv_until<T>(
        &mut self,
        mut pick: impl FnMut(ServerMessage) -> Option<T>,
    ) -> Result<T, ClientError> {
        loop {
            match self.recv().await? {
                Some(ServerMessage::Error { message }) => return Err(ClientError::Daemon(message)),
                Some(msg) => {
                    if let Some(t) = pick(msg) {
                        return Ok(t);
                    }
                }
                None => return Err(ClientError::Closed),
            }
        }
    }

    pub fn into_split(self) -> (ClientReader, ClientWriter) {
        (self.reader, self.writer)
    }
}

/// Locates `hived`: next to the current executable first, then `$PATH`.
/// `HIVE_DAEMON_BIN` overrides both.
pub fn daemon_executable() -> PathBuf {
    if let Some(p) = std::env::var_os("HIVE_DAEMON_BIN") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("hived");
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from("hived")
}

/// Starts `hived` in the background. Concurrent starts are safe: the daemon
/// takes an exclusive lock and losers exit quietly.
pub fn spawn_daemon(paths: &Paths) -> Result<(), ClientError> {
    let mut cmd = Command::new(daemon_executable());
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env("HIVE_RUNTIME_DIR", &paths.runtime_dir)
        .env("HIVE_STATE_DIR", &paths.state_dir)
        .env("HIVE_CONFIG", &paths.config_file);
    // hived calls setsid() itself, detaching from our terminal and session.
    // The child is reaped by init once we exit; we never wait on it.
    cmd.spawn().map(drop).map_err(ClientError::Spawn)
}
