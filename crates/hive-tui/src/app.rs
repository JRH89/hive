//! Frontend state machine. Pure: handlers take events and return the
//! messages to send to the daemon, which keeps this testable without a
//! terminal or socket.

use std::collections::BTreeMap;
use std::num::NonZeroU16;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use hive_core::config::Config;
use hive_core::launch;
use hive_protocol::{
    ClientMessage, CreateSession, ServerMessage, SessionId, SessionInfo, TermSize,
};

use crate::keys;

/// What the frontend does once connected.
#[derive(Debug, Clone)]
pub enum Start {
    /// Attach to the most recent live session, or show the empty state.
    Auto,
    Attach(SessionId),
    Create(CreateSession),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// User detached; sessions keep running.
    Detached { session: Option<SessionId> },
    /// Daemon went away or shut down.
    DaemonGone(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// Leader pressed; the next key is a Hive command.
    Leader,
    ConfirmKill(SessionId),
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Warning,
    Error,
}

pub struct View {
    pub id: SessionId,
    pub parser: vt100::Parser,
    pub controller: bool,
}

pub struct App {
    pub config: Config,
    pub sessions: BTreeMap<SessionId, SessionInfo>,
    pub view: Option<View>,
    pub mode: Mode,
    pub panel: TermSize,
    pub status: Option<(Tone, String)>,
    pub exit: Option<Outcome>,
    start: Option<Start>,
    /// A session we created and should attach to once it is confirmed.
    awaiting_create: bool,
}

fn nz(v: u16) -> NonZeroU16 {
    NonZeroU16::new(v.max(1)).expect("clamped")
}

fn new_parser(size: TermSize, snapshot: &[u8]) -> vt100::Parser {
    // Scrollback lives in the daemon; the client mirrors the visible screen.
    let mut p = vt100::Parser::new(nz(size.rows), nz(size.cols), 0);
    p.process(snapshot);
    p
}

/// The leader key. Ctrl+Space is NUL on the wire, which agents essentially
/// never need and terminals report consistently.
pub fn is_leader(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char(' ') && key.modifiers == KeyModifiers::CONTROL
}

impl App {
    pub fn new(config: Config, start: Start, terminal: (u16, u16)) -> App {
        let mut app = App {
            config,
            sessions: BTreeMap::new(),
            view: None,
            mode: Mode::Normal,
            panel: TermSize::DEFAULT,
            status: None,
            exit: None,
            start: Some(start),
            awaiting_create: false,
        };
        app.panel = app.panel_for(terminal);
        app
    }

    pub fn header_rows(&self) -> u16 {
        self.config.branding.show_header as u16
    }

    /// Terminal panel size for a `(cols, rows)` terminal.
    pub fn panel_for(&self, (cols, rows): (u16, u16)) -> TermSize {
        TermSize {
            rows: rows.saturating_sub(self.header_rows() + 1),
            cols,
        }
        .clamped()
    }

    pub fn current(&self) -> Option<&SessionInfo> {
        self.view.as_ref().and_then(|v| self.sessions.get(&v.id))
    }

    fn set_status(&mut self, tone: Tone, msg: impl Into<String>) {
        self.status = Some((tone, msg.into()));
    }

    /// Initial requests once connected.
    pub fn on_connect(&mut self) -> Vec<ClientMessage> {
        let mut out = vec![ClientMessage::ListSessions];
        match self.start.take() {
            Some(Start::Attach(id)) => out.push(ClientMessage::Attach {
                session: id,
                size: self.panel,
            }),
            Some(Start::Create(mut req)) => {
                req.size = self.panel;
                self.awaiting_create = true;
                out.push(ClientMessage::CreateSession(req));
            }
            Some(Start::Auto) => self.start = Some(Start::Auto),
            None => {}
        }
        out
    }

    fn attach(&self, id: SessionId) -> ClientMessage {
        ClientMessage::Attach {
            session: id,
            size: self.panel,
        }
    }

    /// Most recently created session, preferring live ones.
    fn pick_session(&self, exclude: Option<SessionId>) -> Option<SessionId> {
        let candidates = || self.sessions.values().filter(|s| Some(s.id) != exclude);
        candidates()
            .filter(|s| s.state.is_alive())
            .map(|s| s.id)
            .max()
            .or_else(|| candidates().map(|s| s.id).max())
    }

    pub fn on_server(&mut self, msg: ServerMessage) -> Vec<ClientMessage> {
        let mut out = Vec::new();
        match msg {
            ServerMessage::Welcome { .. } => {}
            ServerMessage::Error { message } => {
                self.awaiting_create = false;
                self.set_status(Tone::Error, message);
            }
            ServerMessage::SessionList(list) => {
                self.sessions = list.into_iter().map(|s| (s.id, s)).collect();
                if matches!(self.start, Some(Start::Auto)) {
                    self.start = None;
                    if self.view.is_none()
                        && let Some(id) = self.pick_session(None)
                    {
                        out.push(self.attach(id));
                    }
                }
            }
            ServerMessage::SessionCreated(info) => {
                let id = info.id;
                self.sessions.insert(id, info);
                if std::mem::take(&mut self.awaiting_create) {
                    out.push(self.attach(id));
                }
            }
            ServerMessage::Attached {
                session,
                snapshot,
                controller,
            } => {
                self.view = Some(View {
                    id: session.id,
                    parser: new_parser(session.size, &snapshot),
                    controller,
                });
                self.sessions.insert(session.id, session);
                self.status = None;
            }
            ServerMessage::Output { session, data } => {
                if let Some(v) = self.view.as_mut().filter(|v| v.id == session) {
                    v.parser.process(&data);
                }
            }
            ServerMessage::Resized { session, size } => {
                if let Some(v) = self.view.as_mut().filter(|v| v.id == session) {
                    v.parser.set_size(nz(size.rows), nz(size.cols));
                }
            }
            ServerMessage::Resync {
                session,
                size,
                snapshot,
            } => {
                if let Some(v) = self.view.as_mut().filter(|v| v.id == session) {
                    v.parser = new_parser(size, &snapshot);
                }
            }
            ServerMessage::Detached { session } => {
                if self.view.as_ref().is_some_and(|v| v.id == session) {
                    self.view = None;
                    if let Some(next) = self.pick_session(Some(session)) {
                        out.push(self.attach(next));
                    }
                }
            }
            ServerMessage::ControlLost { session } => {
                if let Some(v) = self.view.as_mut().filter(|v| v.id == session) {
                    v.controller = false;
                    self.set_status(
                        Tone::Warning,
                        "Another client took control. Leader then a to take it back.",
                    );
                }
            }
            ServerMessage::SessionUpdated(info) => {
                self.sessions.insert(info.id, info);
            }
            ServerMessage::SessionRemoved(id) => {
                self.sessions.remove(&id);
            }
            ServerMessage::ShuttingDown => {
                self.exit = Some(Outcome::DaemonGone("the daemon shut down".into()));
            }
        }
        out
    }

    pub fn on_terminal_event(&mut self, event: Event) -> Vec<ClientMessage> {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key(key),
            Event::Paste(text) => match &self.view {
                Some(v) if self.can_type() => vec![ClientMessage::Input(keys::encode_paste(
                    &text,
                    v.parser.screen().bracketed_paste(),
                ))],
                _ => Vec::new(),
            },
            Event::Resize(cols, rows) => {
                self.panel = self.panel_for((cols, rows));
                match &self.view {
                    Some(v) if v.controller => vec![ClientMessage::Resize(self.panel)],
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Whether keystrokes should go to the attached agent.
    pub fn can_type(&self) -> bool {
        self.mode == Mode::Normal
            && self.view.as_ref().is_some_and(|v| v.controller)
            && self.current().is_some_and(|s| s.state.is_alive())
    }

    fn on_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        match self.mode {
            Mode::Help => {
                self.mode = Mode::Normal;
                Vec::new()
            }
            Mode::ConfirmKill(id) => {
                self.mode = Mode::Normal;
                if matches!(key.code, KeyCode::Char('y' | 'Y')) {
                    self.set_status(Tone::Info, format!("Terminating session {id}…"));
                    vec![ClientMessage::KillSession(id)]
                } else {
                    self.status = None;
                    Vec::new()
                }
            }
            Mode::Leader => {
                self.mode = Mode::Normal;
                self.on_leader_command(key)
            }
            Mode::Normal if is_leader(&key) => {
                self.mode = Mode::Leader;
                Vec::new()
            }
            Mode::Normal if self.can_type() => {
                let app_cursor = self
                    .view
                    .as_ref()
                    .is_some_and(|v| v.parser.screen().application_cursor());
                keys::encode(&key, app_cursor)
                    .map(|b| vec![ClientMessage::Input(b)])
                    .unwrap_or_default()
            }
            // No live, controlled session: plain keys drive the empty state.
            Mode::Normal => match key.code {
                KeyCode::Char('n') => self.new_session(),
                KeyCode::Char('q') => self.detach(),
                KeyCode::Char('?') => {
                    self.mode = Mode::Help;
                    Vec::new()
                }
                _ => Vec::new(),
            },
        }
    }

    fn on_leader_command(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        if is_leader(&key) {
            // Leader twice sends a literal Ctrl+Space (NUL) to the agent.
            return if self.can_type() {
                vec![ClientMessage::Input(vec![0])]
            } else {
                Vec::new()
            };
        }
        match key.code {
            KeyCode::Char('d' | 'q') => self.detach(),
            KeyCode::Char('n') => self.new_session(),
            KeyCode::Char('x') => {
                if let Some(v) = &self.view {
                    self.mode = Mode::ConfirmKill(v.id);
                }
                Vec::new()
            }
            KeyCode::Char('a') => match &self.view {
                Some(v) => vec![self.attach(v.id)],
                None => Vec::new(),
            },
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn detach(&mut self) -> Vec<ClientMessage> {
        self.exit = Some(Outcome::Detached {
            session: self.view.as_ref().map(|v| v.id),
        });
        vec![ClientMessage::Detach]
    }

    fn new_session(&mut self) -> Vec<ClientMessage> {
        let agent = self.config.general.default_agent.clone();
        match launch::create_request(&self.config, &agent, None, None, &[], self.panel) {
            Ok(req) => {
                self.awaiting_create = true;
                self.set_status(Tone::Info, format!("Launching {agent}…"));
                vec![ClientMessage::CreateSession(req)]
            }
            Err(e) => {
                self.set_status(Tone::Error, e.to_string());
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_protocol::SessionState;

    fn info(id: SessionId, state: SessionState) -> SessionInfo {
        SessionInfo {
            id,
            agent: "shell".into(),
            label: "shell".into(),
            command: vec!["sh".into()],
            cwd: "/".into(),
            state,
            size: TermSize::DEFAULT,
            created_at: 0,
            attached_clients: 0,
        }
    }

    fn key(code: KeyCode, mods: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, mods))
    }

    fn attached_app() -> App {
        let mut app = App::new(Config::default(), Start::Attach(1), (80, 26));
        app.on_connect();
        app.on_server(ServerMessage::Attached {
            session: info(1, SessionState::Running),
            snapshot: b"hello".to_vec(),
            controller: true,
        });
        app
    }

    #[test]
    fn panel_excludes_header_and_status_rows() {
        let app = App::new(Config::default(), Start::Auto, (100, 30));
        assert_eq!(
            app.panel,
            TermSize {
                rows: 28,
                cols: 100
            }
        );
    }

    #[test]
    fn auto_start_attaches_newest_live_session() {
        let mut app = App::new(Config::default(), Start::Auto, (80, 26));
        assert_eq!(app.on_connect(), vec![ClientMessage::ListSessions]);
        let out = app.on_server(ServerMessage::SessionList(vec![
            info(1, SessionState::Running),
            info(2, SessionState::Running),
            info(
                3,
                SessionState::Exited {
                    code: Some(0),
                    signal: None,
                },
            ),
        ]));
        assert_eq!(
            out,
            vec![ClientMessage::Attach {
                session: 2,
                size: app.panel
            }]
        );
    }

    #[test]
    fn keys_are_forwarded_and_leader_is_intercepted() {
        let mut app = attached_app();
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            vec![ClientMessage::Input(b"a".to_vec())]
        );
        assert!(
            app.on_terminal_event(key(KeyCode::Char(' '), KeyModifiers::CONTROL))
                .is_empty()
        );
        assert_eq!(app.mode, Mode::Leader);
        // Leader, leader -> literal NUL.
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Char(' '), KeyModifiers::CONTROL)),
            vec![ClientMessage::Input(vec![0])]
        );
    }

    #[test]
    fn leader_d_detaches_without_killing() {
        let mut app = attached_app();
        app.on_terminal_event(key(KeyCode::Char(' '), KeyModifiers::CONTROL));
        let out = app.on_terminal_event(key(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(out, vec![ClientMessage::Detach]);
        assert_eq!(app.exit, Some(Outcome::Detached { session: Some(1) }));
    }

    #[test]
    fn kill_requires_confirmation() {
        let mut app = attached_app();
        app.on_terminal_event(key(KeyCode::Char(' '), KeyModifiers::CONTROL));
        assert!(
            app.on_terminal_event(key(KeyCode::Char('x'), KeyModifiers::NONE))
                .is_empty()
        );
        assert_eq!(app.mode, Mode::ConfirmKill(1));
        assert!(
            app.on_terminal_event(key(KeyCode::Char('n'), KeyModifiers::NONE))
                .is_empty()
        );
        assert_eq!(app.mode, Mode::Normal);

        app.on_terminal_event(key(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_terminal_event(key(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Char('y'), KeyModifiers::NONE)),
            vec![ClientMessage::KillSession(1)]
        );
    }

    #[test]
    fn output_and_resize_update_the_mirror() {
        let mut app = attached_app();
        app.on_server(ServerMessage::Output {
            session: 1,
            data: b" world".to_vec(),
        });
        assert!(
            app.view
                .as_ref()
                .unwrap()
                .parser
                .screen()
                .contents()
                .starts_with("hello world")
        );
        let out = app.on_terminal_event(Event::Resize(120, 40));
        assert_eq!(
            out,
            vec![ClientMessage::Resize(TermSize {
                rows: 38,
                cols: 120
            })]
        );
        app.on_server(ServerMessage::Resized {
            session: 1,
            size: TermSize {
                rows: 38,
                cols: 120,
            },
        });
        let (r, c) = app.view.as_ref().unwrap().parser.screen().size();
        assert_eq!((r.get(), c.get()), (38, 120));
    }

    #[test]
    fn losing_control_stops_input() {
        let mut app = attached_app();
        app.on_server(ServerMessage::ControlLost { session: 1 });
        assert!(
            app.on_terminal_event(key(KeyCode::Char('a'), KeyModifiers::NONE))
                .is_empty()
        );
        app.on_terminal_event(key(KeyCode::Char(' '), KeyModifiers::CONTROL));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            vec![ClientMessage::Attach {
                session: 1,
                size: app.panel
            }]
        );
    }

    #[test]
    fn exited_session_does_not_receive_keys() {
        let mut app = attached_app();
        app.on_server(ServerMessage::SessionUpdated(info(
            1,
            SessionState::Exited {
                code: Some(0),
                signal: None,
            },
        )));
        assert!(!app.can_type());
        assert!(
            app.on_terminal_event(key(KeyCode::Char('a'), KeyModifiers::NONE))
                .is_empty()
        );
    }
}
