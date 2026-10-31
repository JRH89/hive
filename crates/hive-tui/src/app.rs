//! Frontend state machine. Pure: handlers take events and return the
//! messages to send to the daemon, which keeps this testable without a
//! terminal or socket.
//!
//! Switching sessions re-attaches (see ADR-0007): the daemon holds every
//! session's screen, so the client only ever mirrors the visible one.

use std::collections::BTreeMap;
use std::num::NonZeroU16;

const OBSERVING: &str =
    "Observing: another client controls this session. Leader then a to take control.";
const CONTROL_LOST: &str = "Another client took control. Leader then a to take it back.";
const CONTROL_FREE: &str = "Control is free. Leader then a to take it.";

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use hive_core::config::Config;
use hive_core::launch;
use hive_protocol::{
    ClientMessage, CreateSession, NewWorktree, ServerMessage, SessionId, SessionInfo, SessionState,
    TermSize, WorkspaceId, WorkspaceInfo,
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
    /// Launches in `workspace` (or the current directory's), in a new git
    /// worktree when `worktree` is set.
    NewSession {
        selected: usize,
        workspace: Option<WorkspaceId>,
        worktree: bool,
    },
    /// Workspace picker; `selected` indexes `App::workspace_order()`.
    Workspaces {
        selected: usize,
    },
    AddWorkspace {
        input: String,
    },
    RenameWorkspace {
        workspace: WorkspaceId,
        input: String,
    },
}

/// State of the connection to the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Connected,
    /// The connection dropped; the frontend keeps the last screen and retries.
    Reconnecting {
        attempt: u32,
        reason: String,
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
    pub workspaces: BTreeMap<WorkspaceId, WorkspaceInfo>,
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
    pub link: Link,
    /// Pid of the daemon we are (or were last) connected to, to tell a
    /// reconnect to the same daemon from one to a restarted daemon.
    pub daemon_pid: Option<u32>,
    start: Option<Start>,
    /// Session to re-attach once the session list arrives after reconnecting.
    prefer: Option<SessionId>,
    /// A session we created and should attach to once it is confirmed.
    awaiting_create: bool,
    /// Worktrees to remove once their session is gone (kill with `w`).
    remove_worktree_after: BTreeMap<SessionId, String>,
}

/// Result of a key in a one-line text prompt.
enum Edit {
    Submit,
    Cancel,
    Editing,
}

fn edit_line(input: &mut String, key: &KeyEvent) -> Edit {
    match key.code {
        KeyCode::Enter => return Edit::Submit,
        KeyCode::Esc => return Edit::Cancel,
        KeyCode::Backspace => {
            input.pop();
        }
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => input.clear(),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => input.push(c),
        _ => {}
    }
    Edit::Editing
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
            workspaces: BTreeMap::new(),
            view: None,
            mode: Mode::Normal,
            focus: Focus::Terminal,
            cursor: None,
            agents: Vec::new(),
            panel: TermSize::DEFAULT,
            sidebar_visible: false,
            status: None,
            exit: None,
            link: Link::Connected,
            daemon_pid: None,
            start: Some(start),
            prefer: None,
            awaiting_create: false,
            remove_worktree_after: BTreeMap::new(),
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

    /// Workspaces in sidebar order (creation order), including any that
    /// sessions refer to but whose details haven't arrived yet.
    pub fn workspace_order(&self) -> Vec<WorkspaceId> {
        let mut ids: Vec<WorkspaceId> = self.workspaces.keys().copied().collect();
        ids.extend(self.sessions.values().map(|s| s.workspace));
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Sidebar groups: each workspace with its sessions in creation order.
    pub fn groups(&self) -> Vec<(WorkspaceId, Vec<SessionId>)> {
        self.workspace_order()
            .into_iter()
            .map(|w| {
                let ids = self
                    .sessions
                    .values()
                    .filter(|s| s.workspace == w)
                    .map(|s| s.id)
                    .collect();
                (w, ids)
            })
            .collect()
    }

    /// Session ids in sidebar order: grouped by workspace, then creation.
    pub fn order(&self) -> Vec<SessionId> {
        self.groups().into_iter().flat_map(|(_, ids)| ids).collect()
    }

    pub fn workspace_name(&self, id: WorkspaceId) -> &str {
        self.workspaces.get(&id).map_or("…", |w| w.name.as_str())
    }

    /// The workspace new sessions go to: the attached session's, else the
    /// selected one's, else the one containing the current directory.
    pub fn active_workspace(&self) -> Option<WorkspaceId> {
        let of = |id: Option<SessionId>| {
            id.and_then(|id| self.sessions.get(&id))
                .map(|s| s.workspace)
        };
        of(self.view.as_ref().map(|v| v.id))
            .or_else(|| of(self.cursor))
            .or_else(|| {
                let cwd = std::env::current_dir().ok()?;
                let key = hive_workspace::workspace_key(&cwd);
                let roots = self
                    .workspaces
                    .values()
                    .map(|w| (w.id, std::path::Path::new(&w.root)));
                hive_workspace::best_match(roots, &key)
            })
    }

    /// Other running sessions in exactly the same directory (not isolated
    /// by a worktree).
    pub fn sharing_dir(&self, id: SessionId) -> Vec<SessionId> {
        let Some(s) = self.sessions.get(&id).filter(|s| s.state.is_alive()) else {
            return Vec::new();
        };
        self.sessions
            .values()
            .filter(|o| o.id != id && o.state.is_alive() && o.cwd == s.cwd)
            .map(|o| o.id)
            .collect()
    }

    /// True if the status is a notice about who controls input, which is
    /// stale once this client becomes controller.
    fn status_is_control_notice(&self) -> bool {
        matches!(&self.status, Some((_, m)) if [OBSERVING, CONTROL_LOST, CONTROL_FREE].contains(&m.as_str()))
    }

    fn set_status(&mut self, tone: Tone, msg: impl Into<String>) {
        self.status = Some((tone, msg.into()));
    }

    /// Initial requests once connected.
    pub fn on_connect(&mut self) -> Vec<ClientMessage> {
        let mut out = vec![ClientMessage::ListWorkspaces, ClientMessage::ListSessions];
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

    /// Attaches without stealing: input control is taken only if free.
    fn attach(&self, id: SessionId) -> ClientMessage {
        ClientMessage::Attach {
            session: id,
            size: self.panel,
            force: false,
        }
    }

    fn take_control(&self, id: SessionId) -> ClientMessage {
        ClientMessage::Attach {
            session: id,
            size: self.panel,
            force: true,
        }
    }

    fn is_lost(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .is_some_and(|s| s.state == SessionState::Lost)
    }

    /// Most recently created attachable session, preferring live ones.
    fn pick_session(&self, exclude: Option<SessionId>) -> Option<SessionId> {
        let candidates = || {
            self.sessions
                .values()
                .filter(|s| Some(s.id) != exclude && s.state != SessionState::Lost)
        };
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
                let message = match message.strip_suffix(" has uncommitted changes") {
                    Some(path) => format!(
                        "Kept worktree {}: it has uncommitted changes. `hive worktree rm --force {path}` discards them.",
                        base_name(path)
                    ),
                    None => message,
                };
                self.set_status(Tone::Error, message);
            }
            ServerMessage::SessionList(list) => {
                self.sessions = list.into_iter().map(|s| (s.id, s)).collect();
                if matches!(self.start, Some(Start::Auto)) {
                    self.start = None;
                    let preferred = self
                        .prefer
                        .take()
                        .filter(|id| self.sessions.contains_key(id) && !self.is_lost(*id));
                    match preferred.or_else(|| self.pick_session(None)) {
                        Some(id) => out.push(self.attach(id)),
                        None => {
                            self.view = None;
                            self.focus = Focus::Sidebar;
                        }
                    }
                }
            }
            ServerMessage::SessionCreated(info) => {
                let id = info.id;
                // The reply and the broadcast travel separately: an update
                // (e.g. an instant exit) may already be here and is newer.
                self.sessions.entry(id).or_insert(info);
                if std::mem::take(&mut self.awaiting_create) {
                    out.push(self.attach(id));
                    self.focus = Focus::Terminal;
                    self.status = None;
                    let shared = self.sharing_dir(id);
                    if !shared.is_empty() {
                        let ids: Vec<_> = shared.iter().map(|i| format!("#{i}")).collect();
                        self.set_status(
                            Tone::Warning,
                            format!(
                                "Same directory as {}: agents may overwrite each other. Leader n, then w, isolates one in a worktree.",
                                ids.join(", ")
                            ),
                        );
                    }
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
                if !controller {
                    self.set_status(Tone::Warning, OBSERVING);
                } else if self.status_is_control_notice() {
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
                    self.set_status(Tone::Warning, CONTROL_LOST);
                }
            }
            ServerMessage::ControlReleased { session } => {
                if self
                    .view
                    .as_ref()
                    .is_some_and(|v| v.id == session && !v.controller)
                {
                    self.set_status(Tone::Info, CONTROL_FREE);
                }
            }
            ServerMessage::SessionUpdated(info) => {
                self.sessions.insert(info.id, info);
            }
            ServerMessage::SessionRemoved(id) => {
                self.sessions.remove(&id);
                if matches!(&self.status, Some((Tone::Info, m)) if *m == terminating(id)) {
                    self.status = None;
                }
                if let Some(path) = self.remove_worktree_after.remove(&id) {
                    self.set_status(
                        Tone::Info,
                        format!("Removing worktree {}…", base_name(&path)),
                    );
                    out.push(ClientMessage::RemoveWorktree { path, force: false });
                }
                if self.mode == Mode::ConfirmKill(id)
                    || matches!(self.mode, Mode::Rename { session, .. } if session == id)
                {
                    self.mode = Mode::Normal;
                }
            }
            ServerMessage::ShuttingDown => {
                self.exit = Some(Outcome::DaemonGone("the daemon shut down".into()));
            }
            ServerMessage::WorkspaceList(list) => {
                self.workspaces = list.into_iter().map(|w| (w.id, w)).collect();
            }
            ServerMessage::WorkspaceCreated(w) | ServerMessage::WorkspaceUpdated(w) => {
                self.workspaces.insert(w.id, w);
            }
            ServerMessage::WorkspaceRemoved(id) => {
                self.workspaces.remove(&id);
                if let Mode::Workspaces { selected } = &mut self.mode {
                    *selected = (*selected).min(self.workspaces.len().saturating_sub(1));
                }
            }
            ServerMessage::WorktreeList { .. } => {}
            ServerMessage::WorktreeRemoved { path } => {
                self.set_status(
                    Tone::Info,
                    format!(
                        "Removed worktree {}. Its branch was kept.",
                        base_name(&path)
                    ),
                );
            }
        }
        self.fix_cursor();
        out
    }

    pub fn on_terminal_event(&mut self, event: Event) -> Vec<ClientMessage> {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key(key),
            Event::Paste(text) => {
                if let Mode::Rename { input, .. }
                | Mode::AddWorkspace { input }
                | Mode::RenameWorkspace { input, .. } = &mut self.mode
                {
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
        self.link == Link::Connected
            && self.mode == Mode::Normal
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
        if matches!(self.link, Link::Reconnecting { .. }) {
            // Nothing can be sent; only quitting makes sense.
            let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL;
            if key.code == KeyCode::Char('q') || ctrl_c {
                self.exit = Some(Outcome::Detached { session: None });
            }
            return Vec::new();
        }
        match self.mode {
            Mode::Help => {
                self.mode = Mode::Normal;
                Vec::new()
            }
            Mode::ConfirmKill(id) => {
                self.mode = Mode::Normal;
                let worktree = self.sessions.get(&id).and_then(|s| s.worktree.clone());
                match (key.code, worktree) {
                    (KeyCode::Char('y' | 'Y'), _) => {
                        self.set_status(Tone::Info, terminating(id));
                        vec![ClientMessage::KillSession(id)]
                    }
                    (KeyCode::Char('w'), Some(path)) => {
                        self.set_status(Tone::Info, terminating(id));
                        self.remove_worktree_after.insert(id, path);
                        vec![ClientMessage::KillSession(id)]
                    }
                    _ => {
                        self.status = None;
                        Vec::new()
                    }
                }
            }
            Mode::Rename { .. } => self.on_rename_key(key),
            Mode::NewSession { .. } => self.on_picker_key(key),
            Mode::Workspaces { .. } => self.on_workspaces_key(key),
            Mode::AddWorkspace { .. } | Mode::RenameWorkspace { .. } => {
                self.on_workspace_prompt_key(key)
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
                    Some(id) if self.is_lost(id) && key.code == KeyCode::Enter => self.relaunch(id),
                    Some(id) => self.switch_to(id),
                    None => Vec::new(),
                };
            }
            KeyCode::Esc | KeyCode::Tab => {
                if self.view.is_some() {
                    self.focus = Focus::Terminal;
                }
            }
            KeyCode::Char('n') => self.open_picker(None),
            KeyCode::Char('w') => self.open_workspaces(),
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
            KeyCode::Char('n') => self.open_picker(None),
            KeyCode::Char('w') => self.open_workspaces(),
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
                    return vec![self.take_control(id)];
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

    /// Shows `id` in the terminal panel and focuses it. A lost session can't
    /// be shown; the sidebar explains how to relaunch it instead.
    fn switch_to(&mut self, id: SessionId) -> Vec<ClientMessage> {
        self.cursor = Some(id);
        if self.is_lost(id) {
            self.focus = Focus::Sidebar;
            self.set_status(
                Tone::Warning,
                format!(
                    "Session {id} ended when hived stopped. Enter relaunches it, x removes it."
                ),
            );
            return Vec::new();
        }
        self.focus = Focus::Terminal;
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

    /// Next/previous attachable session, wrapping around.
    fn switch_relative(&mut self, delta: isize) -> Vec<ClientMessage> {
        let order: Vec<SessionId> = self
            .order()
            .into_iter()
            .filter(|&id| !self.is_lost(id) || Some(id) == self.view.as_ref().map(|v| v.id))
            .collect();
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

    /// Starts a new session from a lost session's agent, label, command and
    /// directory, then forgets the lost record. The environment comes from
    /// this client because environments are never persisted.
    fn relaunch(&mut self, id: SessionId) -> Vec<ClientMessage> {
        let Some(info) = self.sessions.get(&id) else {
            return Vec::new();
        };
        let mut env: Vec<(String, String)> = std::env::vars().collect();
        if let Some(def) = self.config.agent(&info.agent) {
            env.retain(|(k, _)| !def.env.contains_key(k));
            env.extend(def.env);
        }
        let req = CreateSession {
            agent: info.agent.clone(),
            label: info.label.clone(),
            command: info.command.clone(),
            cwd: info.cwd.clone(),
            env,
            size: self.panel,
            workspace: Some(info.workspace).filter(|w| self.workspaces.contains_key(w)),
            worktree: None,
        };
        self.awaiting_create = true;
        self.set_status(Tone::Info, format!("Relaunching {}…", info.label));
        vec![
            ClientMessage::CreateSession(req),
            ClientMessage::KillSession(id),
        ]
    }

    /// The daemon connection dropped. The last screen stays visible.
    pub fn on_disconnected(&mut self, reason: String) {
        self.link = Link::Reconnecting { attempt: 0, reason };
        self.mode = Mode::Normal;
        self.awaiting_create = false;
        self.prefer = self.view.as_ref().map(|v| v.id).or(self.prefer);
    }

    pub fn on_reconnect_attempt(&mut self) {
        if let Link::Reconnecting { attempt, .. } = &mut self.link {
            *attempt += 1;
        }
    }

    /// Connected again. Re-requests the session list and re-attaches the
    /// previous session if it still exists (ids are never reused, so a
    /// matching id is the same session).
    pub fn on_reconnected(&mut self, daemon_pid: u32) -> Vec<ClientMessage> {
        self.link = Link::Connected;
        if self.daemon_pid.is_some_and(|p| p != daemon_pid) {
            self.set_status(
                Tone::Warning,
                "hived restarted. Sessions that were running are marked ended (◌).",
            );
        } else {
            self.status = None;
        }
        self.daemon_pid = Some(daemon_pid);
        self.start = Some(Start::Auto);
        vec![ClientMessage::ListWorkspaces, ClientMessage::ListSessions]
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
        match edit_line(input, &key) {
            Edit::Submit => {
                let msg = ClientMessage::RenameSession {
                    session: *session,
                    label: std::mem::take(input),
                };
                self.mode = Mode::Normal;
                vec![msg]
            }
            Edit::Cancel => {
                self.mode = Mode::Normal;
                Vec::new()
            }
            Edit::Editing => Vec::new(),
        }
    }

    fn open_workspaces(&mut self) {
        let order = self.workspace_order();
        let selected = self
            .active_workspace()
            .and_then(|w| order.iter().position(|&o| o == w))
            .unwrap_or(0);
        self.mode = Mode::Workspaces { selected };
    }

    fn on_workspaces_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        let order = self.workspace_order();
        let Mode::Workspaces { selected } = &mut self.mode else {
            return Vec::new();
        };
        let last = order.len().saturating_sub(1);
        if let Some(n) = digit(&key).filter(|&n| n <= last) {
            *selected = n;
        }
        let chosen = order.get(*selected).copied();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1),
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Normal,
            KeyCode::Char('a') => {
                let input = std::env::current_dir()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default();
                self.mode = Mode::AddWorkspace { input };
            }
            KeyCode::Char('n') => {
                if let Some(w) = chosen {
                    self.open_picker(Some(w));
                }
            }
            KeyCode::Char('r') => {
                if let Some(w) = chosen {
                    let input = self.workspace_name(w).to_string();
                    self.mode = Mode::RenameWorkspace {
                        workspace: w,
                        input,
                    };
                }
            }
            KeyCode::Char('x') => {
                if let Some(w) = chosen {
                    return vec![ClientMessage::RemoveWorkspace(w)];
                }
            }
            KeyCode::Enter | KeyCode::Char('1'..='9') => {
                if let Some(w) = chosen {
                    self.mode = Mode::Normal;
                    return self.open_workspace(w);
                }
            }
            _ => {}
        }
        Vec::new()
    }

    /// Shows the workspace's most recent session, or offers to launch one.
    fn open_workspace(&mut self, w: WorkspaceId) -> Vec<ClientMessage> {
        let candidates = || {
            self.sessions
                .values()
                .filter(|s| s.workspace == w && s.state != SessionState::Lost)
        };
        let pick = candidates()
            .filter(|s| s.state.is_alive())
            .map(|s| s.id)
            .max()
            .or_else(|| candidates().map(|s| s.id).max());
        match pick {
            Some(id) => self.switch_to(id),
            None => {
                self.open_picker(Some(w));
                Vec::new()
            }
        }
    }

    fn on_workspace_prompt_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        let (Mode::AddWorkspace { input } | Mode::RenameWorkspace { input, .. }) = &mut self.mode
        else {
            return Vec::new();
        };
        match edit_line(input, &key) {
            Edit::Editing => Vec::new(),
            Edit::Cancel => {
                self.mode = Mode::Normal;
                Vec::new()
            }
            Edit::Submit => {
                let msg = match std::mem::replace(&mut self.mode, Mode::Normal) {
                    Mode::AddWorkspace { input } => {
                        let path = expand_home(input.trim());
                        ClientMessage::CreateWorkspace { path, name: None }
                    }
                    Mode::RenameWorkspace { workspace, input } => ClientMessage::RenameWorkspace {
                        workspace,
                        name: input,
                    },
                    _ => unreachable!("matched above"),
                };
                vec![msg]
            }
        }
    }

    fn open_picker(&mut self, workspace: Option<WorkspaceId>) {
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
        self.mode = Mode::NewSession {
            selected,
            workspace: workspace.or_else(|| self.active_workspace()),
            worktree: false,
        };
    }

    /// Whether the picker's target workspace can host worktrees.
    pub fn picker_can_worktree(&self) -> bool {
        match &self.mode {
            Mode::NewSession {
                workspace: Some(w), ..
            } => self.workspaces.get(w).is_some_and(|w| w.is_repo),
            _ => false,
        }
    }

    fn on_picker_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        let can_worktree = self.picker_can_worktree();
        let Mode::NewSession {
            selected,
            workspace,
            worktree,
        } = &mut self.mode
        else {
            return Vec::new();
        };
        let (workspace, use_worktree) = (*workspace, *worktree);
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
            (_, KeyCode::Char('w')) if can_worktree => {
                *worktree = !*worktree;
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
                self.create_session(&agent, workspace, use_worktree)
            }
            None => Vec::new(),
        }
    }

    fn create_session(
        &mut self,
        agent: &str,
        workspace: Option<WorkspaceId>,
        worktree: bool,
    ) -> Vec<ClientMessage> {
        let root = workspace
            .and_then(|w| self.workspaces.get(&w))
            .map(|w| std::path::PathBuf::from(&w.root));
        match launch::create_request(&self.config, agent, None, root, &[], self.panel) {
            Ok(mut req) => {
                req.workspace = workspace;
                req.worktree = worktree.then_some(NewWorktree { branch: None });
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

fn terminating(id: SessionId) -> String {
    format!("Terminating session {id}…")
}

fn base_name(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
}

/// `~` and `~/x` to the home directory, as a shell would.
fn expand_home(path: &str) -> String {
    match (path.strip_prefix('~'), std::env::var("HOME")) {
        (Some(rest), Ok(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{home}{rest}")
        }
        _ => path.to_string(),
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
            workspace: 1,
            branch: None,
            worktree: None,
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
            force: false,
        }]
    }

    fn lost(id: SessionId, label: &str) -> SessionInfo {
        SessionInfo {
            label: label.into(),
            cwd: "/src/api".into(),
            command: vec!["claude".into(), "--verbose".into()],
            agent: "claude".into(),
            ..info(id, SessionState::Lost)
        }
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
        assert_eq!(
            app.on_connect(),
            vec![ClientMessage::ListWorkspaces, ClientMessage::ListSessions]
        );
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
        let Mode::NewSession { selected, .. } = app.mode else {
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
        assert_eq!(
            app.on_terminal_event(ch('a')),
            vec![ClientMessage::Attach {
                session: 1,
                size: app.panel,
                force: true
            }]
        );
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

    #[test]
    fn attaching_as_observer_explains_how_to_take_control() {
        let mut app = app();
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: false,
        });
        assert!(!app.can_type());
        assert!(matches!(&app.status, Some((Tone::Warning, m)) if m.contains("Observing")));
        app.on_server(ServerMessage::ControlReleased { session: 2 });
        assert!(matches!(&app.status, Some((Tone::Info, m)) if m.contains("free")));
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: true,
        });
        assert_eq!(app.status, None, "control notices clear once controller");
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: false,
        });
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: true,
        });
        assert_eq!(
            app.status, None,
            "the observing warning clears on take-over"
        );
    }

    #[test]
    fn disconnect_freezes_input_and_reconnect_reattaches_the_same_session() {
        let mut app = app();
        app.daemon_pid = Some(100);
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: b"before".to_vec(),
            controller: true,
        });
        app.on_disconnected("the daemon closed the connection".into());
        assert!(!app.can_type());
        assert!(
            app.on_terminal_event(ch('a')).is_empty(),
            "nothing is sent while down"
        );
        app.on_reconnect_attempt();
        assert_eq!(
            app.link,
            Link::Reconnecting {
                attempt: 1,
                reason: "the daemon closed the connection".into()
            }
        );
        // The last screen stays visible while reconnecting.
        assert!(
            app.view
                .as_ref()
                .unwrap()
                .parser
                .screen()
                .contents()
                .contains("before")
        );

        assert_eq!(
            app.on_reconnected(100),
            vec![ClientMessage::ListWorkspaces, ClientMessage::ListSessions]
        );
        assert_eq!(app.link, Link::Connected);
        assert!(app.status.is_none(), "same daemon: nothing to warn about");
        let out = app.on_server(ServerMessage::SessionList(
            (1..=3).map(|i| info(i, SessionState::Running)).collect(),
        ));
        assert_eq!(out, attach_msg(&app, 2));
    }

    #[test]
    fn reconnecting_to_a_restarted_daemon_warns_and_skips_lost_sessions() {
        let mut app = app();
        app.daemon_pid = Some(100);
        app.on_disconnected("connection lost".into());
        app.on_reconnected(200);
        assert!(matches!(&app.status, Some((Tone::Warning, m)) if m.contains("restarted")));
        // Session 1 (previously attached) is now lost; 4 is new and live.
        let out = app.on_server(ServerMessage::SessionList(vec![
            lost(1, "api"),
            info(4, SessionState::Running),
        ]));
        assert_eq!(out, attach_msg(&app, 4));
    }

    #[test]
    fn only_lost_sessions_leaves_the_sidebar_focused() {
        let mut app = App::new(Config::default(), Start::Auto, (100, 26));
        app.on_connect();
        assert!(
            app.on_server(ServerMessage::SessionList(vec![lost(1, "api")]))
                .is_empty()
        );
        assert_eq!(app.focus, Focus::Sidebar);
        assert_eq!(app.cursor, Some(1));
    }

    #[test]
    fn quitting_while_reconnecting() {
        let mut app = app();
        app.on_disconnected("gone".into());
        app.on_terminal_event(ch('q'));
        assert_eq!(app.exit, Some(Outcome::Detached { session: None }));
    }

    #[test]
    fn lost_sessions_are_skipped_by_switching_and_relaunched_from_the_sidebar() {
        let mut app = app();
        app.on_server(ServerMessage::SessionUpdated(lost(2, "api")));
        // j from #1 skips lost #2 and lands on #3.
        app.on_terminal_event(leader());
        assert_eq!(app.on_terminal_event(ch('j')), attach_msg(&app, 3));
        // Jumping straight to it explains instead of attaching.
        app.on_terminal_event(leader());
        assert!(app.on_terminal_event(ch('2')).is_empty());
        assert_eq!(app.focus, Focus::Sidebar);
        assert!(matches!(&app.status, Some((Tone::Warning, m)) if m.contains("relaunch")));

        // Enter on it relaunches with the remembered command, cwd and label,
        // then forgets the lost record.
        let out = app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE));
        let [
            ClientMessage::CreateSession(req),
            ClientMessage::KillSession(2),
        ] = out.as_slice()
        else {
            panic!("expected relaunch, got {out:?}")
        };
        assert_eq!(req.command, vec!["claude", "--verbose"]);
        assert_eq!(req.cwd, "/src/api");
        assert_eq!(req.label, "api");
        assert!(
            req.env.iter().any(|(k, _)| k == "PATH"),
            "env comes from this client"
        );
        let out = app.on_server(ServerMessage::SessionCreated(info(
            9,
            SessionState::Running,
        )));
        assert_eq!(out, attach_msg(&app, 9));
    }

    fn ws(id: WorkspaceId, name: &str, is_repo: bool) -> WorkspaceInfo {
        WorkspaceInfo {
            id,
            name: name.into(),
            root: format!("/src/{name}"),
            is_repo,
        }
    }

    /// `app()` plus workspaces: 1 `web` (repo, sessions 1-3), 2 `api`
    /// (repo, empty), 3 `notes` (not git, empty).
    fn app_with_workspaces() -> App {
        let mut app = app();
        app.on_server(ServerMessage::WorkspaceList(vec![
            ws(1, "web", true),
            ws(2, "api", true),
            ws(3, "notes", false),
        ]));
        app
    }

    fn choose_shell(app: &mut App) -> Vec<ClientMessage> {
        let shell = app.agents.iter().position(|a| a.name == "shell").unwrap();
        let Mode::NewSession { selected, .. } = &mut app.mode else {
            panic!("picker not open: {:?}", app.mode)
        };
        *selected = shell;
        app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE))
    }

    #[test]
    fn new_session_targets_the_active_workspace_and_can_use_a_worktree() {
        let mut app = app_with_workspaces();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('n'));
        assert!(matches!(
            app.mode,
            Mode::NewSession {
                workspace: Some(1),
                worktree: false,
                ..
            }
        ));
        assert!(app.picker_can_worktree());
        app.on_terminal_event(ch('w'));
        assert!(matches!(app.mode, Mode::NewSession { worktree: true, .. }));
        let out = choose_shell(&mut app);
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("expected CreateSession, got {out:?}")
        };
        assert_eq!(req.workspace, Some(1));
        assert_eq!(req.cwd, "/src/web");
        assert_eq!(req.worktree, Some(NewWorktree { branch: None }));
    }

    #[test]
    fn worktree_toggle_is_ignored_outside_git() {
        let mut app = app_with_workspaces();
        app.open_picker(Some(3));
        assert!(!app.picker_can_worktree());
        app.on_terminal_event(ch('w'));
        assert!(matches!(
            app.mode,
            Mode::NewSession {
                worktree: false,
                ..
            }
        ));
        let out = choose_shell(&mut app);
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("expected CreateSession, got {out:?}")
        };
        assert_eq!((req.workspace, req.worktree.clone()), (Some(3), None));
    }

    #[test]
    fn workspace_picker_opens_sessions_or_offers_to_launch() {
        let mut app = app_with_workspaces();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('w'));
        assert_eq!(
            app.mode,
            Mode::Workspaces { selected: 0 },
            "starts on the active one"
        );
        // Enter on `web` switches to its newest live session.
        app.on_terminal_event(ch('j'));
        app.on_terminal_event(ch('k'));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE)),
            attach_msg(&app, 3)
        );
        // `api` has no sessions: the agent picker opens for it.
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('w'));
        app.on_terminal_event(ch('2'));
        assert!(matches!(
            app.mode,
            Mode::NewSession {
                workspace: Some(2),
                ..
            }
        ));
    }

    #[test]
    fn workspaces_are_added_renamed_and_removed_from_the_picker() {
        let mut app = app_with_workspaces();
        app.open_workspaces();
        app.on_terminal_event(ch('a'));
        let Mode::AddWorkspace { input } = &mut app.mode else {
            panic!("expected the add prompt")
        };
        assert!(!input.is_empty(), "prefilled with the current directory");
        input.clear();
        app.on_terminal_event(Event::Paste("/src/new".into()));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE)),
            vec![ClientMessage::CreateWorkspace {
                path: "/src/new".into(),
                name: None
            }]
        );

        app.open_workspaces();
        app.on_terminal_event(ch('j'));
        app.on_terminal_event(ch('r'));
        assert!(
            matches!(&app.mode, Mode::RenameWorkspace { workspace: 2, input } if input == "api")
        );
        app.on_terminal_event(ch('2'));
        assert_eq!(
            app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE)),
            vec![ClientMessage::RenameWorkspace {
                workspace: 2,
                name: "api2".into()
            }]
        );

        app.open_workspaces();
        app.on_terminal_event(ch('j'));
        app.on_terminal_event(ch('j'));
        assert_eq!(
            app.on_terminal_event(ch('x')),
            vec![ClientMessage::RemoveWorkspace(3)]
        );
        app.on_server(ServerMessage::WorkspaceRemoved(3));
        assert_eq!(app.mode, Mode::Workspaces { selected: 1 });
    }

    #[test]
    fn killing_with_w_removes_the_worktree_after_the_session_is_gone() {
        let mut app = app_with_workspaces();
        let mut s = info(2, SessionState::Running);
        s.worktree = Some("/wt/web/hive-shell-2".into());
        app.on_server(ServerMessage::SessionUpdated(s));
        app.mode = Mode::ConfirmKill(2);
        assert_eq!(
            app.on_terminal_event(ch('w')),
            vec![ClientMessage::KillSession(2)]
        );
        assert_eq!(
            app.on_server(ServerMessage::SessionRemoved(2)),
            vec![ClientMessage::RemoveWorktree {
                path: "/wt/web/hive-shell-2".into(),
                force: false
            }]
        );
        // The notice survives the view moving on to another session.
        app.on_server(ServerMessage::WorktreeRemoved {
            path: "/wt/web/hive-shell-2".into(),
        });
        attached(&mut app, 1);
        assert!(
            matches!(&app.status, Some((Tone::Info, m)) if m.contains("Removed worktree hive-shell-2")),
            "{:?}",
            app.status
        );
        // A dirty worktree is kept, and the error says how to force it.
        app.on_server(ServerMessage::Error {
            message: "/wt/web/hive-shell-2 has uncommitted changes".into(),
        });
        assert!(matches!(&app.status, Some((Tone::Error, m)) if m.contains("--force")));

        // `w` means nothing for a session without a worktree: it cancels.
        app.mode = Mode::ConfirmKill(3);
        assert!(app.on_terminal_event(ch('w')).is_empty());
        assert!(app.on_server(ServerMessage::SessionRemoved(3)).is_empty());
    }

    #[test]
    fn a_late_created_reply_does_not_undo_a_newer_update() {
        let mut app = app();
        let exited = SessionState::Exited {
            code: Some(3),
            signal: None,
        };
        app.on_server(ServerMessage::SessionUpdated(info(9, exited.clone())));
        app.on_server(ServerMessage::SessionCreated(info(
            9,
            SessionState::Running,
        )));
        assert_eq!(app.sessions[&9].state, exited);
    }

    #[test]
    fn launching_into_a_busy_directory_warns() {
        let mut app = app_with_workspaces();
        app.open_picker(None);
        choose_shell(&mut app);
        // `info` sessions all run in "/"; the new one does too.
        app.on_server(ServerMessage::SessionCreated(info(
            4,
            SessionState::Running,
        )));
        assert!(
            matches!(&app.status, Some((Tone::Warning, m)) if m.contains("#1, #2, #3") && m.contains("worktree")),
            "{:?}",
            app.status
        );
        let mut isolated = info(5, SessionState::Running);
        isolated.cwd = "/wt/x".into();
        isolated.worktree = Some("/wt/x".into());
        app.open_picker(None);
        choose_shell(&mut app);
        app.on_server(ServerMessage::SessionCreated(isolated));
        assert_eq!(app.status, None);
    }
}
