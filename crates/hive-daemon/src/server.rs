//! Per-connection handling: handshake, request dispatch, and streaming the
//! attached session's output to the client.

use std::sync::Arc;
use std::time::Duration;

use hive_core::brand;
use hive_protocol::{
    ClientMessage, PROTOCOL_VERSION, ServerMessage, SessionId, read_frame, write_frame,
};
use tokio::net::UnixStream;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::Daemon;
use crate::session::{Chunk, ClientId, Session, wait_true};

/// Outgoing messages queued per client. When full, output forwarding blocks,
/// the session broadcast lags, and the client is resynced from a snapshot.
/// With `MAX_COALESCE` this bounds per-client buffering to ~4 MiB.
const CLIENT_QUEUE: usize = 64;
/// Upper bound on PTY bytes coalesced into a single `Output` frame.
const MAX_COALESCE: usize = 64 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

struct Attached {
    session: Arc<Session>,
    forwarder: JoinHandle<()>,
}

struct Connection {
    id: ClientId,
    daemon: Arc<Daemon>,
    out: mpsc::Sender<ServerMessage>,
    attached: Option<Attached>,
}

pub(crate) async fn handle_client(daemon: Arc<Daemon>, stream: UnixStream) {
    let id = daemon
        .next_client
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (mut rd, mut wr) = stream.into_split();

    let client_name = match tokio::time::timeout(HANDSHAKE_TIMEOUT, read_frame(&mut rd)).await {
        Ok(Ok(Some(ClientMessage::Hello {
            protocol_version,
            client,
        }))) => {
            let welcome = ServerMessage::Welcome {
                protocol_version: PROTOCOL_VERSION,
                daemon_version: brand::VERSION.to_string(),
                daemon_pid: std::process::id(),
            };
            if write_frame(&mut wr, &welcome).await.is_err() || protocol_version != PROTOCOL_VERSION
            {
                return;
            }
            client
        }
        _ => {
            let _ = write_frame(
                &mut wr,
                &ServerMessage::Error {
                    message: "expected Hello".into(),
                },
            )
            .await;
            return;
        }
    };
    tracing::debug!(client = id, name = %client_name, "client connected");

    let (out_tx, mut out_rx) = mpsc::channel::<ServerMessage>(CLIENT_QUEUE);
    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if write_frame(&mut wr, &msg).await.is_err() {
                break;
            }
        }
    });
    daemon.clients.lock().unwrap().insert(id, out_tx.clone());
    let events = tokio::spawn(forward_events(daemon.clone(), out_tx.clone()));

    let mut conn = Connection {
        id,
        daemon: daemon.clone(),
        out: out_tx,
        attached: None,
    };
    loop {
        match read_frame::<_, ClientMessage>(&mut rd).await {
            Ok(Some(msg)) => conn.handle(msg).await,
            Ok(None) => break,
            Err(e) => {
                tracing::warn!(client = id, "dropping client: {e}");
                break;
            }
        }
    }

    conn.detach();
    daemon.clients.lock().unwrap().remove(&id);
    events.abort();
    drop(conn);
    let _ = writer.await;
    tracing::debug!(client = id, "client disconnected");
}

impl Connection {
    async fn send(&self, msg: ServerMessage) {
        let _ = self.out.send(msg).await;
    }

    async fn error(&self, message: impl Into<String>) {
        self.send(ServerMessage::Error {
            message: message.into(),
        })
        .await;
    }

    async fn handle(&mut self, msg: ClientMessage) {
        match msg {
            ClientMessage::Hello { .. } => self.error("already handshaken").await,
            ClientMessage::ListSessions => {
                self.send(ServerMessage::SessionList(self.daemon.list()))
                    .await
            }
            ClientMessage::CreateSession(req) => match self.daemon.create(req) {
                Ok(info) => self.send(ServerMessage::SessionCreated(info)).await,
                Err(e) => self.error(format!("{e:#}")).await,
            },
            ClientMessage::Attach { session, size } => self.attach(session, size).await,
            ClientMessage::Detach => {
                if let Some(id) = self.detach() {
                    self.send(ServerMessage::Detached { session: id }).await;
                }
            }
            ClientMessage::Input(data) => {
                if let Some(a) = &self.attached {
                    a.session.input(self.id, data).await;
                }
            }
            ClientMessage::Resize(size) => {
                if let Some(a) = &self.attached {
                    a.session.resize(self.id, size);
                }
            }
            ClientMessage::KillSession(id) => {
                // Termination may take seconds; don't block this connection.
                let daemon = self.daemon.clone();
                let out = self.out.clone();
                tokio::spawn(async move {
                    if let Err(e) = daemon.kill(id).await {
                        let _ = out
                            .send(ServerMessage::Error {
                                message: e.to_string(),
                            })
                            .await;
                    }
                });
            }
            ClientMessage::RenameSession { session, label } => {
                if let Err(e) = self.daemon.rename(session, &label) {
                    self.error(e.to_string()).await;
                }
            }
            ClientMessage::Shutdown => {
                tracing::info!(client = self.id, "shutdown requested");
                self.daemon.shutdown.send_replace(true);
            }
        }
    }

    async fn attach(&mut self, id: SessionId, size: hive_protocol::TermSize) {
        let Some(session) = self.daemon.get(id) else {
            self.error(format!("no session with id {id}")).await;
            return;
        };
        self.detach();
        let a = session.attach(self.id, size);
        if let Some(prev) = a.previous_controller {
            self.daemon
                .notify(prev, ServerMessage::ControlLost { session: id });
        }
        self.send(ServerMessage::Attached {
            session: a.info.clone(),
            snapshot: a.snapshot,
            controller: true,
        })
        .await;
        session.publish();
        let forwarder = tokio::spawn(forward_output(session.clone(), a.output, self.out.clone()));
        self.attached = Some(Attached { session, forwarder });
    }

    fn detach(&mut self) -> Option<SessionId> {
        let a = self.attached.take()?;
        a.forwarder.abort();
        a.session.detach(self.id);
        a.session.publish();
        Some(a.session.id)
    }
}

async fn forward_events(daemon: Arc<Daemon>, out: mpsc::Sender<ServerMessage>) {
    let mut rx = daemon.events.subscribe();
    loop {
        let msg = match rx.recv().await {
            Ok(msg) => msg,
            // Missed some updates: send the full picture instead.
            Err(RecvError::Lagged(_)) => ServerMessage::SessionList(daemon.list()),
            Err(RecvError::Closed) => break,
        };
        if out.send(msg).await.is_err() {
            break;
        }
    }
}

fn resync(session: &Session, rx: &mut broadcast::Receiver<Chunk>) -> ServerMessage {
    tracing::debug!(
        session = session.id,
        "client lagged; resyncing from snapshot"
    );
    let (size, snapshot, new_rx) = session.resync();
    *rx = new_rx;
    ServerMessage::Resync {
        session: session.id,
        size,
        snapshot,
    }
}

async fn forward_output(
    session: Arc<Session>,
    mut rx: broadcast::Receiver<Chunk>,
    out: mpsc::Sender<ServerMessage>,
) {
    let id = session.id;
    let mut removed = session.removed();
    loop {
        let first = tokio::select! {
            r = rx.recv() => r,
            _ = wait_true(&mut removed) => {
                let _ = out.send(ServerMessage::Detached { session: id }).await;
                return;
            }
        };
        let msg = match first {
            Ok(Chunk::Output(data)) => {
                // Coalesce whatever else is already queued into one frame.
                let mut buf = data.to_vec();
                let mut then = None;
                while buf.len() < MAX_COALESCE {
                    match rx.try_recv() {
                        Ok(Chunk::Output(more)) => buf.extend_from_slice(&more),
                        Ok(Chunk::Resized(size)) => {
                            then = Some(ServerMessage::Resized { session: id, size });
                            break;
                        }
                        // try_recv repositions the receiver after reporting a
                        // lag, so the resync must happen now or data is lost.
                        Err(TryRecvError::Lagged(_)) => {
                            then = Some(resync(&session, &mut rx));
                            break;
                        }
                        Err(_) => break,
                    }
                }
                if out
                    .send(ServerMessage::Output {
                        session: id,
                        data: buf,
                    })
                    .await
                    .is_err()
                {
                    return;
                }
                match then {
                    Some(msg) => msg,
                    None => continue,
                }
            }
            Ok(Chunk::Resized(size)) => ServerMessage::Resized { session: id, size },
            Err(RecvError::Lagged(_)) => resync(&session, &mut rx),
            Err(RecvError::Closed) => return,
        };
        if out.send(msg).await.is_err() {
            return;
        }
    }
}
