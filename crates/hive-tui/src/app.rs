//! Frontend state machine. Pure: handlers take events and return the
//! messages to send to the daemon, which keeps this testable without a
//! terminal or socket.
//!
//! Switching sessions re-attaches (see ADR-0007): the daemon holds every
//! session's screen, so the client only ever mirrors the visible one.

use std::collections::BTreeMap;
use std::num::NonZeroU16;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use hive_core::config::Config;
use hive_core::launch;
use hive_protocol::{
    ClientMessage, CreateSession, ServerMessage, SessionId, SessionInfo, TermSize,
};

use crate::keys;

/// Sidebar width including its 1-column border.
pub const SIDEBAR_WIDTH: u16 = 26;
/// Below this terminal width the sidebar is hidden to keep the agent usable.
pub const MIN_COLS_FOR_SIDEBAR: u16 = 60;

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
pub enum Focus {
    /// Keys go to the attached agent.
    Terminal,
    /// Keys navigate the session list.
    Sidebar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// Leader pressed; the next key is a Hive command.
    Leader,
    ConfirmKill(SessionId),
    Help,
    Rename {
        session: SessionId,
        input: String,
    },
    /// Agent picker for a new session; `selected` indexes `App::agents`.
    NewSession {
        selected: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChoice {
    pub name: String,
    pub command: String,
    pub installed: bool,
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
    pub focus: Focus,
    /// Sidebar selection. Follows the attached session unless moved.
    pub cursor: Option<SessionId>,
    /// Populated when the agent picker opens.
    pub agents: Vec<AgentChoice>,
    pub panel: TermSize,
    pub sidebar_visible: bool,
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

fn digit(key: &KeyEvent) -> Option<usize> {
    match key.code {
        KeyCode::Char(c @ '1'..='9') if key.modifiers.is_empty() => Some(c as usize - '1' as usize),
        _ => None,
    }
}

impl App {
    pub fn new(config: Config, start: Start, terminal: (u16, u16)) -> App {
        let mut app = App {
            config,
            sessions: BTreeMap::new(),
            view: None,
            mode: Mode::Normal,
            focus: Focus::Terminal,
            cursor: None,
            agents: Vec::new(),
            panel: TermSize::DEFAULT,
            sidebar_visible: false,
            status: None,
            exit: None,
            start: Some(start),
            awaiting_create: false,
        };
        app.set_terminal_size(terminal);
        app
    }

    pub fn header_rows(&self) -> u16 {
        self.config.branding.show_header as u16
    }

    /// Recomputes layout for a `(cols, rows)` terminal.
    fn set_terminal_size(&mut self, (cols, rows): (u16, u16)) {
        self.sidebar_visible = cols >= MIN_COLS_FOR_SIDEBAR;
        let sidebar = if self.sidebar_visible {
            SIDEBAR_WIDTH
        } else {
            0
        };
        self.panel = TermSize {
            rows: rows.saturating_sub(self.header_rows() + 1),
            cols: cols.saturating_sub(sidebar),
        }
        .clamped();
    }

    pub fn current(&self) -> Option<&SessionInfo> {
        self.view.as_ref().and_then(|v| self.sessions.get(&v.id))
    }

    /// Session ids in sidebar order (creation order).
    pub fn order(&self) -> Vec<SessionId> {
        self.sessions.keys().copied().collect()
    }

    fn set_status(&mut self, tone: Tone, msg: impl Into<String>) {
        self.status = Some((tone, msg.into()));
    }

    /// Initial requests once connected.
    pub fn on_connect(&mut self) -> Vec<ClientMessage> {
        let mut out = vec![ClientMessage::ListSessions];
        match self.start.take() {
            Some(Start::Attach(id)) => out.push(self.attach(id)),
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

    /// Keeps the cursor on an existing session.
    fn fix_cursor(&mut self) {
        if self.cursor.is_some_and(|c| self.sessions.contains_key(&c)) {
            return;
        }
        self.cursor = self
            .view
            .as_ref()
            .map(|v| v.id)
            .filter(|id| self.sessions.contains_key(id))
            .or_else(|| self.sessions.keys().next().copied());
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
                    self.focus = Focus::Terminal;
                    self.status = None;
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
                self.cursor = Some(session.id);
                self.sessions.insert(session.id, session);
                if !matches!(self.status, Some((Tone::Error, _))) {
                    self.status = None;
                }
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
                    } else {
                        self.focus = Focus::Sidebar;
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
                if self.mode == Mode::ConfirmKill(id)
                    || matches!(self.mode, Mode::Rename { session, .. } if session == id)
                {
                    self.mode = Mode::Normal;
                }
            }
            ServerMessage::ShuttingDown => {
                self.exit = Some(Outcome::DaemonGone("the daemon shut down".into()));
            }
        }
        self.fix_cursor();
        out
    }

    pub fn on_terminal_event(&mut self, event: Event) -> Vec<ClientMessage> {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key(key),
            Event::Paste(text) => {
                if let Mode::Rename { input, .. } = &mut self.mode {
                    input.extend(text.chars().filter(|c| !c.is_control()));
                    return Vec::new();
                }
                match &self.view {
                    Some(v) if self.can_type() => vec![ClientMessage::Input(keys::encode_paste(
                        &text,
                        v.parser.screen().bracketed_paste(),
                    ))],
                    _ => Vec::new(),
                }
            }
            Event::Resize(cols, rows) => {
                self.set_terminal_size((cols, rows));
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
            && self.focus == Focus::Terminal
            && self.view.as_ref().is_some_and(|v| v.controller)
            && self.current().is_some_and(|s| s.state.is_alive())
    }

    /// Whether plain keys drive the session list (sidebar focused, or no
    /// live session to type into).
    pub fn sidebar_keys_active(&self) -> bool {
        self.mode == Mode::Normal && !self.can_type()
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
            Mode::Rename { .. } => self.on_rename_key(key),
            Mode::NewSession { .. } => self.on_picker_key(key),
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
            Mode::Normal => self.on_sidebar_key(key),
        }
    }

    fn on_sidebar_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        if let Some(n) = digit(&key) {
            return self.switch_nth(n);
        }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                return match self.cursor {
                    Some(id) => self.switch_to(id),
                    None => Vec::new(),
                };
            }
            KeyCode::Esc | KeyCode::Tab => {
                if self.view.is_some() {
                    self.focus = Focus::Terminal;
                }
            }
            KeyCode::Char('n') => self.open_picker(),
            KeyCode::Char('x') => {
                if let Some(id) = self.cursor {
                    self.mode = Mode::ConfirmKill(id);
                }
            }
            KeyCode::Char('r') => {
                if let Some(id) = self.cursor {
                    self.start_rename(id);
                }
            }
            KeyCode::Char('q') => return self.detach(),
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
        Vec::new()
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
        if let Some(n) = digit(&key) {
            return self.switch_nth(n);
        }
        let current = self.view.as_ref().map(|v| v.id);
        match key.code {
            KeyCode::Char('d' | 'q') => return self.detach(),
            KeyCode::Char('n') => self.open_picker(),
            KeyCode::Char('j') | KeyCode::Down => return self.switch_relative(1),
            KeyCode::Char('k') | KeyCode::Up => return self.switch_relative(-1),
            KeyCode::Char('s') | KeyCode::Tab => {
                self.focus = Focus::Sidebar;
                self.cursor = current.or(self.cursor);
            }
            KeyCode::Char('x') => {
                if let Some(id) = current {
                    self.mode = Mode::ConfirmKill(id);
                }
            }
            KeyCode::Char('r') => {
                if let Some(id) = current {
                    self.start_rename(id);
                }
            }
            KeyCode::Char('a') => {
                if let Some(id) = current {
                    return vec![self.attach(id)];
                }
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
        Vec::new()
    }

    fn move_cursor(&mut self, delta: isize) {
        let order = self.order();
        if order.is_empty() {
            return;
        }
        let idx = self
            .cursor
            .and_then(|c| order.iter().position(|&id| id == c))
            .unwrap_or(0) as isize;
        let next = (idx + delta).clamp(0, order.len() as isize - 1);
        self.cursor = Some(order[next as usize]);
    }

    /// Shows `id` in the terminal panel and focuses it.
    fn switch_to(&mut self, id: SessionId) -> Vec<ClientMessage> {
        self.focus = Focus::Terminal;
        self.cursor = Some(id);
        match &self.view {
            Some(v) if v.id == id && v.controller => Vec::new(),
            _ => vec![self.attach(id)],
        }
    }

    fn switch_nth(&mut self, n: usize) -> Vec<ClientMessage> {
        match self.order().get(n) {
            Some(&id) => self.switch_to(id),
            None => Vec::new(),
        }
    }

    /// Next/previous session, wrapping around.
    fn switch_relative(&mut self, delta: isize) -> Vec<ClientMessage> {
        let order = self.order();
        if order.is_empty() {
            return Vec::new();
        }
        let from = self.view.as_ref().map(|v| v.id).or(self.cursor);
        let idx = from
            .and_then(|c| order.iter().position(|&id| id == c))
            .map(|i| i as isize + delta)
            .unwrap_or(0);
        let next = idx.rem_euclid(order.len() as isize) as usize;
        self.switch_to(order[next])
    }

    fn start_rename(&mut self, id: SessionId) {
        let input = self
            .sessions
            .get(&id)
            .map(|s| s.label.clone())
            .unwrap_or_default();
        self.mode = Mode::Rename { session: id, input };
    }

    fn on_rename_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        let Mode::Rename { session, input } = &mut self.mode else {
            return Vec::new();
        };
        match key.code {
            KeyCode::Enter => {
                let msg = ClientMessage::RenameSession {
                    session: *session,
                    label: std::mem::take(input),
                };
                self.mode = Mode::Normal;
                return vec![msg];
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => input.clear(),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => input.push(c),
            _ => {}
        }
        Vec::new()
    }

    fn open_picker(&mut self) {
        let path = std::env::var_os("PATH").unwrap_or_default();
        self.agents = self
            .config
            .agents()
            .into_iter()
            .map(|(name, def)| AgentChoice {
                installed: launch::find_executable(&def.command, &path).is_some(),
                command: def.command,
                name,
            })
            .collect();
        let selected = self
            .agents
            .iter()
            .position(|a| a.name == self.config.general.default_agent)
            .unwrap_or(0);
        self.mode = Mode::NewSession { selected };
    }

    fn on_picker_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        let Mode::NewSession { selected } = &mut self.mode else {
            return Vec::new();
        };
        let last = self.agents.len().saturating_sub(1);
        let choose = match (digit(&key), key.code) {
            (Some(n), _) if n <= last => Some(n),
            (_, KeyCode::Enter) => Some(*selected),
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                *selected = (*selected + 1).min(last);
                None
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                *selected = selected.saturating_sub(1);
                None
            }
            (_, KeyCode::Esc | KeyCode::Char('q')) => {
                self.mode = Mode::Normal;
                None
            }
            _ => None,
        };
        match choose
            .and_then(|i| self.agents.get(i))
            .map(|a| a.name.clone())
        {
            Some(agent) => {
                self.mode = Mode::Normal;
                self.create_session(&agent)
            }
            None => Vec::new(),
        }
    }

    fn create_session(&mut self, agent: &str) -> Vec<ClientMessage> {
        match launch::create_request(&self.config, agent, None, None, &[], self.panel) {
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

    fn detach(&mut self) -> Vec<ClientMessage> {
        self.exit = Some(Outcome::Detached {
            session: self.view.as_ref().map(|v| v.id),
        });
        vec![ClientMessage::Detach]
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

    fn ch(c: char) -> Event {
        key(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn leader() -> Event {
        key(KeyCode::Char(' '), KeyModifiers::CONTROL)
    }

    fn attached(app: &mut App, id: SessionId) {
        app.on_server(ServerMessage::Attached {
            session: info(id, SessionState::Running),
            snapshot: b"hello".to_vec(),
            controller: true,
        });
    }

    /// Three running sessions, attached to #1.
    fn app() -> App {
        let mut app = App::new(Config::default(), Start::Attach(1), (100, 26));
        app.on_connect();
        app.on_server(ServerMessage::SessionList(
            (1..=3).map(|i| info(i, SessionState::Running)).collect(),
        ));
        attached(&mut app, 1);
        app
    }

    fn attach_msg(app: &App, id: SessionId) -> Vec<ClientMessage> {
        vec![ClientMessage::Attach {
            session: id,
            size: app.panel,
        }]
    }

    #[test]
    fn panel_excludes_chrome_and_sidebar() {
        let app = App::new(Config::default(), Start::Auto, (100, 30));
        assert!(app.sidebar_visible);
        assert_eq!(
            app.panel,
            TermSize {
                rows: 28,
                cols: 100 - SIDEBAR_WIDTH
            }
        );
        let narrow = App::new(Config::default(), Start::Auto, (50, 30));
        assert!(!narrow.sidebar_visible);
        assert_eq!(narrow.panel.cols, 50);
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
        assert_eq!(out, attach_msg(&app, 2));
    }

    #[test]
    fn keys_are_forwarded_and_leader_is_intercepted() {
        let mut app = app();
        assert_eq!(
            app.on_terminal_event(ch('a')),
            vec![ClientMessage::Input(b"a".to_vec())]
        );
        assert!(app.on_terminal_event(leader()).is_empty());
        assert_eq!(app.mode, Mode::Leader);
        assert_eq!(
            app.on_terminal_event(leader()),
            vec![ClientMessage::Input(vec![0])]
        );
    }

    #[test]
    fn leader_d_detaches_without_killing() {
        let mut app = app();
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('d')), vec![ClientMessage::Detach]);
        assert_eq!(app.exit, Some(Outcome::Detached { session: Some(1) }));
    }

    #[test]
    fn leader_j_k_and_digits_switch_sessions() {
        let mut app = app();
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('j')), attach_msg(&app, 2));
        attached(&mut app, 2);
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('k')), attach_msg(&app, 1));
        attached(&mut app, 1);
        // Wraps backwards from the first to the last.
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('k')), attach_msg(&app, 3));
        attached(&mut app, 3);
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('2')), attach_msg(&app, 2));
        // Out-of-range digit does nothing.
        app.on_terminal_event(leader());
        assert!(app.on_terminal_event(ch('9')).is_empty());
    }

    #[test]
    fn sidebar_navigation_selects_then_opens() {
        let mut app = app();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('s'));
        assert_eq!(app.focus, Focus::Sidebar);
        // Plain keys now navigate instead of reaching the agent.
        assert!(app.on_terminal_event(ch('j')).is_empty());
        assert!(
            app.on_terminal_event(key(KeyCode::Down, KeyModifiers::NONE))
                .is_empty()
        );
        assert!(app.on_terminal_event(ch('j')).is_empty()); // clamped at the end
        assert_eq!(app.cursor, Some(3));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE)),
            attach_msg(&app, 3)
        );
        assert_eq!(app.focus, Focus::Terminal);
    }

    #[test]
    fn escape_returns_focus_without_switching() {
        let mut app = app();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('s'));
        app.on_terminal_event(ch('j'));
        assert!(
            app.on_terminal_event(key(KeyCode::Esc, KeyModifiers::NONE))
                .is_empty()
        );
        assert_eq!(app.focus, Focus::Terminal);
        assert_eq!(app.view.as_ref().unwrap().id, 1);
    }

    #[test]
    fn rename_edits_label_and_submits() {
        let mut app = app();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('r'));
        assert_eq!(
            app.mode,
            Mode::Rename {
                session: 1,
                input: "shell".into()
            }
        );
        app.on_terminal_event(key(KeyCode::Char('u'), KeyModifiers::CONTROL));
        for c in "api".chars() {
            app.on_terminal_event(ch(c));
        }
        app.on_terminal_event(key(KeyCode::Backspace, KeyModifiers::NONE));
        app.on_terminal_event(Event::Paste("p\nx".into()));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE)),
            vec![ClientMessage::RenameSession {
                session: 1,
                label: "appx".into()
            }]
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn picker_lists_agents_and_creates_selected_one() {
        let mut config = Config::default();
        config.general.default_agent = "shell".into();
        config.agents.insert(
            "zz-test-sh".into(),
            hive_core::config::AgentDef {
                command: "sh".into(),
                args: vec![],
                env: Default::default(),
            },
        );
        let mut app = App::new(config, Start::Auto, (100, 26));
        app.on_terminal_event(ch('n'));
        let Mode::NewSession { selected } = app.mode else {
            panic!("picker not open")
        };
        assert_eq!(app.agents[selected].name, "shell");
        // Built-ins and config agents are listed, in name order.
        assert_eq!(app.agents.last().unwrap().name, "zz-test-sh");
        assert!(app.agents.last().unwrap().installed);

        app.on_terminal_event(ch('j'));
        let out = app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE));
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("expected CreateSession, got {out:?}")
        };
        assert_eq!(req.agent, "zz-test-sh");
        assert_eq!(req.size, app.panel);
        assert_eq!(app.mode, Mode::Normal);

        // Once created, the new session is attached and focused.
        let out = app.on_server(ServerMessage::SessionCreated(info(
            7,
            SessionState::Running,
        )));
        assert_eq!(out, attach_msg(&app, 7));
    }

    #[test]
    fn picker_reports_missing_agent() {
        let mut config = Config::default();
        config.agents.insert(
            "aaa-missing".into(),
            hive_core::config::AgentDef {
                command: "definitely-not-installed-hive".into(),
                args: vec![],
                env: Default::default(),
            },
        );
        let mut app = App::new(config, Start::Auto, (100, 26));
        app.on_terminal_event(ch('n'));
        assert!(!app.agents[0].installed);
        assert!(app.on_terminal_event(ch('1')).is_empty());
        assert!(matches!(app.status, Some((Tone::Error, ref m)) if m.contains("not found")));
    }

    #[test]
    fn kill_requires_confirmation_and_next_session_is_attached() {
        let mut app = app();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('x'));
        assert_eq!(app.mode, Mode::ConfirmKill(1));
        assert!(app.on_terminal_event(ch('n')).is_empty());
        assert_eq!(app.mode, Mode::Normal);

        app.on_terminal_event(leader());
        app.on_terminal_event(ch('x'));
        assert_eq!(
            app.on_terminal_event(ch('y')),
            vec![ClientMessage::KillSession(1)]
        );
        app.on_server(ServerMessage::SessionRemoved(1));
        let out = app.on_server(ServerMessage::Detached { session: 1 });
        assert_eq!(out, attach_msg(&app, 3));
    }

    #[test]
    fn killing_the_last_session_focuses_the_sidebar() {
        let mut app = App::new(Config::default(), Start::Attach(1), (100, 26));
        app.on_connect();
        attached(&mut app, 1);
        app.on_server(ServerMessage::SessionRemoved(1));
        assert!(
            app.on_server(ServerMessage::Detached { session: 1 })
                .is_empty()
        );
        assert!(app.view.is_none());
        assert_eq!(app.focus, Focus::Sidebar);
        assert_eq!(app.cursor, None);
    }

    #[test]
    fn output_for_other_sessions_is_ignored() {
        let mut app = app();
        app.on_server(ServerMessage::Output {
            session: 2,
            data: b"NOPE".to_vec(),
        });
        app.on_server(ServerMessage::Output {
            session: 1,
            data: b" world".to_vec(),
        });
        let text = app.view.as_ref().unwrap().parser.screen().contents();
        assert!(text.starts_with("hello world") && !text.contains("NOPE"));
    }

    #[test]
    fn resize_updates_panel_and_mirror() {
        let mut app = app();
        let out = app.on_terminal_event(Event::Resize(120, 40));
        let panel = TermSize {
            rows: 38,
            cols: 120 - SIDEBAR_WIDTH,
        };
        assert_eq!(out, vec![ClientMessage::Resize(panel)]);
        app.on_server(ServerMessage::Resized {
            session: 1,
            size: panel,
        });
        let (r, c) = app.view.as_ref().unwrap().parser.screen().size();
        assert_eq!((r.get(), c.get()), (panel.rows, panel.cols));
    }

    #[test]
    fn losing_control_stops_input() {
        let mut app = app();
        app.on_server(ServerMessage::ControlLost { session: 1 });
        assert!(app.on_terminal_event(ch('a')).is_empty());
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('a')), attach_msg(&app, 1));
    }

    #[test]
    fn exited_session_gets_sidebar_keys() {
        let mut app = app();
        app.on_server(ServerMessage::SessionUpdated(info(
            1,
            SessionState::Exited {
                code: Some(0),
                signal: None,
            },
        )));
        assert!(!app.can_type());
        assert!(app.sidebar_keys_active());
        assert_eq!(app.on_terminal_event(ch('2')), attach_msg(&app, 2));
    }
}
