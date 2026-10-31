//! Accepting clients: a 0600 Unix socket whose peers must be our uid, or on
//! Windows a local-only named pipe whose clients must send the token first
//! (ADR-0021). Also the OS's "please stop" signals.

use std::io;
use std::sync::Arc;

use hive_core::paths::Paths;

use crate::{Daemon, server};

#[cfg(unix)]
pub(crate) use unix::*;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(unix)]
mod unix {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tokio::net::{UnixListener, UnixStream};
    use tokio::signal::unix::{Signal, SignalKind, signal};

    pub(crate) struct Listener {
        inner: UnixListener,
        uid: u32,
    }

    pub(crate) type Conn = UnixStream;

    impl Listener {
        /// Callers hold the daemon lock, so any existing socket is stale.
        pub(crate) fn bind(paths: &Paths) -> io::Result<Listener> {
            match fs::remove_file(&paths.socket) {
                Ok(()) => tracing::info!("removed stale socket"),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            let inner = UnixListener::bind(&paths.socket)?;
            fs::set_permissions(&paths.socket, fs::Permissions::from_mode(0o600))?;
            tracing::info!(socket = %paths.socket.display(), "socket bound");
            Ok(Listener {
                inner,
                uid: rustix::process::getuid().as_raw(),
            })
        }

        pub(crate) async fn accept(&mut self) -> io::Result<Conn> {
            let (stream, _) = self.inner.accept().await?;
            let peer = stream.peer_cred()?.uid();
            if peer != self.uid {
                return Err(io::Error::other(format!(
                    "rejected connection from uid {peer}"
                )));
            }
            Ok(stream)
        }

        pub(crate) fn cleanup(&self, paths: &Paths) {
            let _ = fs::remove_file(&paths.socket);
        }
    }

    pub(crate) async fn serve(daemon: Arc<Daemon>, conn: Conn) {
        server::handle_client(daemon, conn).await;
    }

    /// SIGTERM or SIGINT.
    pub(crate) struct Stop(Signal, Signal);

    impl Stop {
        pub(crate) fn new() -> io::Result<Stop> {
            Ok(Stop(
                signal(SignalKind::terminate())?,
                signal(SignalKind::interrupt())?,
            ))
        }

        pub(crate) async fn recv(&mut self) {
            tokio::select! {
                _ = self.0.recv() => {}
                _ = self.1.recv() => {}
            }
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use tokio::signal::windows::{CtrlBreak, CtrlC, ctrl_break, ctrl_c};

    use hive_core::transport::{TOKEN_LEN, new_token, token_matches};

    pub(crate) struct Listener {
        name: String,
        next: NamedPipeServer,
        token: Arc<[u8]>,
    }

    pub(crate) struct Conn {
        pipe: NamedPipeServer,
        token: Arc<[u8]>,
    }

    impl Listener {
        /// A new token per daemon, written before the pipe exists, so a
        /// client that sees the pipe can always read the matching token.
        pub(crate) fn bind(paths: &Paths) -> io::Result<Listener> {
            let token: Arc<[u8]> = new_token().into_bytes().into();
            std::fs::write(paths.token_file(), &token)?;
            let name = paths.pipe_name();
            let next = ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .create(&name)?;
            tracing::info!(pipe = %name, "pipe created");
            Ok(Listener { name, next, token })
        }

        pub(crate) async fn accept(&mut self) -> io::Result<Conn> {
            self.next.connect().await?;
            let fresh = ServerOptions::new()
                .reject_remote_clients(true)
                .create(&self.name)?;
            Ok(Conn {
                pipe: std::mem::replace(&mut self.next, fresh),
                token: self.token.clone(),
            })
        }

        pub(crate) fn cleanup(&self, paths: &Paths) {
            let _ = std::fs::remove_file(paths.token_file());
        }
    }

    pub(crate) async fn serve(daemon: Arc<Daemon>, mut conn: Conn) {
        let mut got = [0u8; TOKEN_LEN];
        let read = tokio::time::timeout(Duration::from_secs(5), conn.pipe.read_exact(&mut got));
        match read.await {
            Ok(Ok(_)) if token_matches(&conn.token, &got) => {
                server::handle_client(daemon, conn.pipe).await;
            }
            _ => tracing::warn!("rejected pipe client without a valid token"),
        }
    }

    /// Ctrl+C or Ctrl+Break (only when started from a console).
    pub(crate) struct Stop(CtrlC, CtrlBreak);

    impl Stop {
        pub(crate) fn new() -> io::Result<Stop> {
            Ok(Stop(ctrl_c()?, ctrl_break()?))
        }

        pub(crate) async fn recv(&mut self) {
            tokio::select! {
                _ = self.0.recv() => {}
                _ = self.1.recv() => {}
            }
        }
    }
}
