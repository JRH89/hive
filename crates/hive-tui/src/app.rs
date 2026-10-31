//! Frontend state machine. Pure: handlers take events and return the
//! messages to send to the daemon, which keeps this testable without a
//! terminal or socket.
//!
//! Switching sessions re-attaches (see ADR-0007): the daemon holds every
//! session's screen, so the client only ever mirrors the visible one.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU16;
use std::path::PathBuf;

const OBSERVING: &str =
    "Observing: another client controls this session. Leader then a to take control.";
const CONTROL_LOST: &str = "Another client took control. Leader then a to take it back.";
const CONTROL_FREE: &str = "Control is free. Leader then a to take it.";

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use hive_core::config::{Config, OpenIn};
use hive_core::launch;
use hive_protocol::{
    AgentStatus, ClientMessage, CreateSession, NewWorktree, ServerMessage, SessionId, SessionInfo,
    SessionState, TermSize, WorkspaceId, WorkspaceInfo,
};
use ratatui::layout::Rect;

use crate::keymap::{Command, Global, Keymap};
use crate::keys;
use crate::layout::{Dir, PaneArea, Tab};

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
    /// Searchable list of commands, sessions and workspaces. `selected`
    /// indexes `App::palette_items()`.
    Palette {
        query: String,
        selected: usize,
        scope: Scope,
    },
}

/// What the palette lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Commands, sessions and workspaces.
    All,
    Sessions,
    /// `TaskList` entries.
    Tasks,
}

/// What a palette entry does when chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Command(Command),
    Session(SessionId),
    Workspace(WorkspaceId),
    /// Index into `App::tasks`.
    Task(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem {
    pub target: Target,
    pub title: String,
    pub detail: String,
    /// Bound key, shown on the right.
    pub key: Option<String>,
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
    /// Display name for known agents, e.g. "Claude Code".
    pub label: Option<&'static str>,
    pub installed: bool,
}

/// What a session's agent is doing, as shown in the UI. Ordered by how much
/// it wants attention, so a workspace shows its most urgent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Activity {
    Idle,
    /// Finished a turn since you last looked at it.
    Done,
    Working,
    /// Waiting for you (a permission prompt or question).
    Blocked,
}

impl Activity {
    pub fn word(self) -> &'static str {
        match self {
            Activity::Idle => "idle",
            Activity::Done => "done",
            Activity::Working => "working",
            Activity::Blocked => "needs input",
        }
    }
}

fn agent_status(s: &SessionInfo) -> Option<AgentStatus> {
    s.presence.as_ref().map(|p| p.status)
}

/// What a sidebar row shows, for mouse clicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarHit {
    Session(SessionId),
    Workspace(WorkspaceId),
    /// The "new" button: new session in the selected space.
    New,
    /// The "menu" button: the command palette.
    Menu,
    /// A tab in the header's tab bar.
    Tab(usize),
    /// The tab bar's `+`: a new terminal tab.
    NewTab,
}

/// Where the next session created from the UI goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// The focused pane (or a new tab if none).
    Replace,
    Split(Dir),
    NewTab,
}

/// Lines a wheel notch scrolls in a full-screen app without mouse support.
const WHEEL_LINES: usize = 3;

pub struct View {
    pub id: SessionId,
    pub parser: vt100::Parser,
    pub controller: bool,
}

pub struct App {
    pub config: Config,
    pub keymap: Keymap,
    /// Shown by the settings command.
    pub config_path: Option<String>,
    pub sessions: BTreeMap<SessionId, SessionInfo>,
    pub workspaces: BTreeMap<WorkspaceId, WorkspaceInfo>,
    /// Mirrors of the sessions in the active tab's panes, by session.
    pub views: BTreeMap<SessionId, View>,
    /// Tabs of split panes (client-side layout). A session is in at most
    /// one pane.
    pub tabs: Vec<Tab>,
    /// Index of the active tab.
    pub tab: usize,
    place: Place,
    pub mode: Mode,
    pub focus: Focus,
    /// Sidebar selection. Follows the attached session unless moved.
    pub cursor: Option<SessionId>,
    /// Populated when the agent picker opens.
    pub agents: Vec<AgentChoice>,
    pub panel: TermSize,
    pub sidebar_visible: bool,
    /// Hidden by the user (toggle_sidebar), regardless of width.
    pub sidebar_hidden: bool,
    /// Whole terminal, `(cols, rows)`.
    terminal: (u16, u16),
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
    /// Sessions whose agent finished working while not viewed.
    pub done: BTreeSet<SessionId>,
    /// Ring the terminal bell after this update.
    pub bell: bool,
    /// Tasks the palette is offering (`run_task`).
    pub tasks: Option<TaskList>,
    /// Side effects for the frontend loop to carry out.
    pub effects: Vec<Effect>,
    /// When each session's agent started its current turn.
    working_since: BTreeMap<SessionId, std::time::Instant>,
    /// Screen rows of sidebar entries as last drawn (render fills this), so
    /// clicks hit what the user saw.
    pub sidebar_hits: RefCell<Vec<(Rect, SidebarHit)>>,
    /// The space (workspace) whose agents the sidebar lists. Follows the
    /// focused session; a click can pick an empty space.
    pub space: Option<WorkspaceId>,
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

/// Case-insensitive subsequence match. Higher is better: a contiguous
/// substring wins, then consecutive letters and word starts; gaps cost a
/// little.
fn fuzzy(query: &str, text: &str) -> Option<i32> {
    let lower = text.to_lowercase();
    let q = query.trim().to_lowercase();
    let text: Vec<char> = lower.chars().collect();
    let mut score = if !q.is_empty() && lower.contains(&q) {
        50
    } else {
        0
    };
    let mut pos = 0;
    let mut prev: Option<usize> = None;
    for q in query.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        let found = (pos..text.len()).find(|&i| text[i] == q)?;
        score += 10;
        if prev.is_some_and(|p| p + 1 == found) {
            score += 15;
        }
        if found == 0 || matches!(text[found - 1], ' ' | '/' | '-' | '_' | '#' | '.') {
            score += 20;
        }
        score -= (found - pos).min(3) as i32;
        prev = Some(found);
        pos = found + 1;
    }
    Some(score)
}

fn digit(key: &KeyEvent) -> Option<usize> {
    match key.code {
        KeyCode::Char(c @ '1'..='9') if key.modifiers.is_empty() => Some(c as usize - '1' as usize),
        _ => None,
    }
}

/// Agent names of sessions opened by integrations.
pub const NOTES_AGENT: &str = "notes";
pub const REVIEW_AGENT: &str = "review";
pub const GITHUB_AGENT: &str = "checks";
pub const TASK_AGENT: &str = "task";

/// Work outside the app state, done by the frontend loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Run `[integrations.desktop_notify] command` with these two arguments.
    Notify { title: String, body: String },
    /// Append `line` to today's daily note in `dir` (the frontend adds the
    /// date and time).
    Journal { dir: PathBuf, line: String },
}

/// Tasks offered by the palette after `run_task`.
#[derive(Debug, Clone)]
pub struct TaskList {
    pub tasks: Vec<hive_workspace::Task>,
    dir: PathBuf,
    workspace: WorkspaceId,
}

fn argv_def(argv: &[String]) -> hive_core::config::AgentDef {
    hive_core::config::AgentDef {
        command: argv.first().cloned().unwrap_or_default(),
        args: argv.iter().skip(1).cloned().collect(),
        env: Default::default(),
    }
}

/// lazygit, else tig, else plain `git diff` in a pager.
fn default_review_command() -> Vec<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let has = |c: &str| launch::find_executable(c, &path).is_some();
    let argv: &[&str] = if has("lazygit") {
        &["lazygit"]
    } else if has("tig") {
        &["tig", "status"]
    } else {
        &["git", "-p", "diff", "HEAD"]
    };
    argv.iter().map(|s| s.to_string()).collect()
}

/// Commands of integrations that are switched off: no keys, no palette
/// entries, no help lines.
pub fn hidden_commands(config: &Config) -> Vec<Command> {
    let mut hidden = Vec::new();
    let i = &config.integrations;
    for (on, cmd) in [
        (i.obsidian.enabled, Command::Notes),
        (i.review.enabled, Command::Review),
        (i.github.enabled, Command::PullRequest),
        (i.tasks.enabled, Command::RunTask),
    ] {
        if !on {
            hidden.push(cmd);
        }
    }
    hidden
}

impl App {
    pub fn new(config: Config, start: Start, terminal: (u16, u16)) -> App {
        let (keymap, key_error) = match Keymap::from_config(&config.keys, &hidden_commands(&config))
        {
            Ok(k) => (k, None),
            Err(e) => (Keymap::default(), Some(e.to_string())),
        };
        let mut app = App {
            config,
            keymap,
            config_path: None,
            sessions: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            views: BTreeMap::new(),
            tabs: Vec::new(),
            tab: 0,
            place: Place::Replace,
            mode: Mode::Normal,
            focus: Focus::Terminal,
            cursor: None,
            agents: Vec::new(),
            panel: TermSize::DEFAULT,
            sidebar_visible: false,
            sidebar_hidden: false,
            terminal,
            status: None,
            exit: None,
            link: Link::Connected,
            daemon_pid: None,
            start: Some(start),
            prefer: None,
            awaiting_create: false,
            remove_worktree_after: BTreeMap::new(),
            done: BTreeSet::new(),
            bell: false,
            tasks: None,
            effects: Vec::new(),
            working_since: BTreeMap::new(),
            sidebar_hits: RefCell::new(Vec::new()),
            space: None,
        };
        app.set_terminal_size(terminal);
        if let Some(e) = key_error {
            app.set_status(Tone::Error, format!("{e}; using default keys"));
        }
        app
    }

    pub fn header_rows(&self) -> u16 {
        self.config.branding.show_header as u16
    }

    /// Recomputes layout for a `(cols, rows)` terminal.
    fn set_terminal_size(&mut self, (cols, rows): (u16, u16)) {
        self.terminal = (cols, rows);
        self.sidebar_visible = cols >= MIN_COLS_FOR_SIDEBAR && !self.sidebar_hidden;
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
        self.focused().and_then(|id| self.sessions.get(&id))
    }

    /// The session keys go to: the active tab's focused pane.
    pub fn focused(&self) -> Option<SessionId> {
        self.tabs.get(self.tab).map(|t| t.focus)
    }

    /// The focused pane's mirror, once attached.
    pub fn view(&self) -> Option<&View> {
        self.focused().and_then(|id| self.views.get(&id))
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.tab)
    }

    /// Sessions in the active tab (attached even when zoomed away).
    fn tab_sessions(&self) -> Vec<SessionId> {
        self.active_tab()
            .map(|t| t.root.panes())
            .unwrap_or_default()
    }

    /// The terminal panel on screen.
    pub fn panel_rect(&self) -> Rect {
        let (x, y) = self.panel_origin();
        Rect::new(x, y, self.panel.cols, self.panel.rows)
    }

    /// Where each drawn pane is, plus the separators between them.
    pub fn pane_areas(&self) -> (Vec<PaneArea>, Vec<(Rect, Dir)>) {
        match self.active_tab() {
            Some(t) => t.areas(self.panel_rect()),
            None => (Vec::new(), Vec::new()),
        }
    }

    /// The size a session's pane gives it, if drawn.
    fn pane_size(&self, id: SessionId) -> Option<TermSize> {
        self.pane_areas()
            .0
            .into_iter()
            .find(|p| p.session == id)
            .map(|p| {
                TermSize {
                    rows: p.body.height,
                    cols: p.body.width,
                }
                .clamped()
            })
    }

    /// Brings the daemon in line with the active tab: detaches what left it,
    /// attaches new panes and resizes the rest to their panes.
    fn sync_panes(&mut self, before: Vec<SessionId>) -> Vec<ClientMessage> {
        let now = self.tab_sessions();
        let mut out = Vec::new();
        for id in before.iter().filter(|id| !now.contains(id)) {
            self.views.remove(id);
            out.push(ClientMessage::Detach { session: Some(*id) });
        }
        for &id in &now {
            if !self.views.contains_key(&id) {
                out.push(self.attach(id));
            }
        }
        out.extend(self.resize_panes());
        out
    }

    /// Resize requests for controlled panes whose size changed.
    fn resize_panes(&self) -> Vec<ClientMessage> {
        self.views
            .values()
            .filter(|v| v.controller)
            .filter_map(|v| {
                let size = self.pane_size(v.id)?;
                let (rows, cols) = v.parser.screen().size();
                (size.rows != rows.get() || size.cols != cols.get()).then_some(
                    ClientMessage::Resize {
                        session: v.id,
                        size,
                    },
                )
            })
            .collect()
    }

    /// Shows `id`: focuses its pane if a tab has it, otherwise puts it in
    /// the focused pane (or a first tab).
    fn show(&mut self, id: SessionId) -> Vec<ClientMessage> {
        let before = self.tab_sessions();
        if let Some(i) = self.tabs.iter().position(|t| t.root.contains(id)) {
            self.tab = i;
            self.tabs[i].focus = id;
        } else if let Some(t) = self.tabs.get_mut(self.tab) {
            t.replace_focused(id);
        } else {
            self.tabs.push(Tab::new(id));
            self.tab = self.tabs.len() - 1;
        }
        self.cursor = Some(id);
        self.space = self.sessions.get(&id).map(|s| s.workspace);
        self.done.remove(&id);
        self.sync_panes(before)
    }

    /// Places a newly created session according to `self.place`.
    fn place_new(&mut self, id: SessionId) -> Vec<ClientMessage> {
        let place = std::mem::replace(&mut self.place, Place::Replace);
        let before = self.tab_sessions();
        match (place, self.tabs.get_mut(self.tab)) {
            (Place::Split(dir), Some(t)) => t.split_focused(id, dir),
            (Place::NewTab, _) | (Place::Split(_), None) => {
                self.tabs.push(Tab::new(id));
                self.tab = self.tabs.len() - 1;
            }
            (Place::Replace, _) => return self.show(id),
        }
        self.cursor = Some(id);
        self.sync_panes(before)
    }

    /// Takes a session out of the layout (it ended or was removed). An
    /// emptied active tab shows another session instead.
    fn drop_pane(&mut self, id: SessionId) -> Vec<ClientMessage> {
        self.views.remove(&id);
        let Some(i) = self.tabs.iter().position(|t| t.root.contains(id)) else {
            return Vec::new();
        };
        let active = i == self.tab;
        let before = self.tab_sessions();
        if self.tabs[i].remove(id) {
            if !active {
                return Vec::new();
            }
            self.cursor = self.focused();
            return self.sync_panes(before);
        }
        self.tabs.remove(i);
        if i < self.tab || self.tab >= self.tabs.len() {
            self.tab = self.tab.saturating_sub(1);
        }
        if !active {
            return Vec::new();
        }
        if self.tabs.is_empty() {
            return match self.pick_session(Some(id)) {
                Some(next) => self.show(next),
                None => {
                    self.focus = Focus::Sidebar;
                    Vec::new()
                }
            };
        }
        self.cursor = self.focused();
        self.sync_panes(Vec::new())
    }

    /// Input for the focused session.
    fn input(&self, data: Vec<u8>) -> Vec<ClientMessage> {
        match self.focused() {
            Some(session) => vec![ClientMessage::Input { session, data }],
            None => Vec::new(),
        }
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
    /// The space the sidebar lists agents for: the sidebar cursor's while
    /// browsing it, else the chosen one, else the focused session's.
    pub fn shown_space(&self) -> Option<WorkspaceId> {
        let of = |id: Option<SessionId>| {
            id.and_then(|id| self.sessions.get(&id))
                .map(|s| s.workspace)
        };
        let browsing = self.focus == Focus::Sidebar || self.sidebar_keys_active();
        browsing
            .then(|| of(self.cursor))
            .flatten()
            .or(self.space.filter(|w| self.workspaces.contains_key(w)))
            .or_else(|| of(self.focused()))
            .or_else(|| self.groups().first().map(|(w, _)| *w))
    }

    /// The agents listed under the shown space, numbered 1-9 for jumps.
    pub fn space_sessions(&self) -> Vec<SessionId> {
        let space = self.shown_space();
        self.groups()
            .into_iter()
            .find(|(w, _)| Some(*w) == space)
            .map(|(_, ids)| ids)
            .unwrap_or_default()
    }

    pub fn order(&self) -> Vec<SessionId> {
        self.groups().into_iter().flat_map(|(_, ids)| ids).collect()
    }

    pub fn activity(&self, id: SessionId) -> Option<Activity> {
        let s = self.sessions.get(&id).filter(|s| s.state.is_alive())?;
        match agent_status(s)? {
            AgentStatus::Blocked => Some(Activity::Blocked),
            AgentStatus::Working => Some(Activity::Working),
            AgentStatus::Idle if self.done.contains(&id) => Some(Activity::Done),
            AgentStatus::Idle => Some(Activity::Idle),
            AgentStatus::Unknown => None,
        }
    }

    /// The most urgent activity among a workspace's sessions.
    pub fn workspace_activity(&self, w: WorkspaceId) -> Option<Activity> {
        self.sessions
            .values()
            .filter(|s| s.workspace == w)
            .filter_map(|s| self.activity(s.id))
            .max()
    }

    fn viewing(&self, id: SessionId) -> bool {
        self.tab_sessions().contains(&id)
    }

    /// Tracks "done" and rings the bell when a session you aren't viewing
    /// starts needing input or finishes working.
    fn note_activity(&mut self, old: Option<AgentStatus>, new: &SessionInfo) {
        let id = new.id;
        let status = agent_status(new).filter(|_| new.state.is_alive());
        let unseen = !self.viewing(id);
        let started = match status {
            Some(AgentStatus::Working) => {
                self.working_since
                    .entry(id)
                    .or_insert_with(std::time::Instant::now);
                None
            }
            _ => self.working_since.remove(&id),
        };
        match status {
            Some(AgentStatus::Idle) if old == Some(AgentStatus::Working) && unseen => {
                self.done.insert(id);
                self.bell |= self.config.notifications.bell;
                self.notify(new, "finished");
                self.journal(new, started.map(|t| t.elapsed()));
            }
            Some(AgentStatus::Idle) => {}
            Some(AgentStatus::Blocked) if old != Some(AgentStatus::Blocked) && unseen => {
                self.done.remove(&id);
                self.bell |= self.config.notifications.bell;
                self.notify(new, "needs input");
            }
            _ => {
                self.done.remove(&id);
            }
        }
    }

    fn notify(&mut self, s: &SessionInfo, what: &str) {
        if self.config.integrations.desktop_notify.enabled {
            let agent = s
                .presence
                .as_ref()
                .map_or(s.agent.as_str(), |p| p.agent.as_str());
            self.effects.push(Effect::Notify {
                title: format!("{} {what}", s.label),
                body: format!("#{} {agent} · {}", s.id, self.workspace_name(s.workspace)),
            });
        }
    }

    /// `[integrations.obsidian] journal`: a line in the vault's daily note
    /// when an agent finishes while you're elsewhere.
    fn journal(&mut self, s: &SessionInfo, took: Option<std::time::Duration>) {
        let obsidian = &self.config.integrations.obsidian;
        if !obsidian.enabled || !obsidian.journal {
            return;
        }
        let cwd = PathBuf::from(&s.cwd);
        let root = self
            .workspaces
            .get(&s.workspace)
            .map_or_else(|| cwd.clone(), |w| PathBuf::from(&w.root));
        let Some(vault) = hive_workspace::find_obsidian_vault(&cwd, &root) else {
            return;
        };
        let agent = s
            .presence
            .as_ref()
            .map_or(s.agent.as_str(), |p| p.agent.as_str());
        let mut line = format!("{agent} finished in #{} {}", s.id, s.label);
        if let Some(b) = &s.branch {
            line.push_str(&format!(" ({b})"));
        }
        if let Some(d) = took.filter(|d| d.as_secs() >= 60) {
            line.push_str(&format!(" after {}m", d.as_secs() / 60));
        }
        self.effects.push(Effect::Journal {
            dir: vault.join(&obsidian.journal_dir),
            line,
        });
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
        self.space
            .filter(|w| self.workspaces.contains_key(w))
            .or_else(|| of(self.focused()))
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
            Some(Start::Attach(id)) => {
                self.tabs = vec![Tab::new(id)];
                self.tab = 0;
                self.cursor = Some(id);
                out.push(self.attach(id));
            }
            Some(Start::Create(mut req)) => {
                req.size = self.panel;
                self.awaiting_create = true;
                self.place = Place::Replace;
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
            size: self.pane_size(id).unwrap_or(self.panel),
            force: false,
        }
    }

    fn take_control(&self, id: SessionId) -> ClientMessage {
        ClientMessage::Attach {
            session: id,
            size: self.pane_size(id).unwrap_or(self.panel),
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
            .focused()
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
                    // After a reconnect the daemon has forgotten our
                    // attachments: keep the tabs whose sessions survived and
                    // attach their panes again.
                    self.views.clear();
                    let gone: Vec<SessionId> = self
                        .tabs
                        .iter()
                        .flat_map(|t| t.root.panes())
                        .filter(|id| !self.sessions.contains_key(id) || self.is_lost(*id))
                        .collect();
                    for id in gone {
                        if let Some(i) = self.tabs.iter().position(|t| t.root.contains(id))
                            && !self.tabs[i].remove(id)
                        {
                            self.tabs.remove(i);
                        }
                    }
                    self.tab = self.tab.min(self.tabs.len().saturating_sub(1));
                    if !self.tabs.is_empty() {
                        out.extend(self.sync_panes(Vec::new()));
                    } else {
                        let preferred = self
                            .prefer
                            .take()
                            .filter(|id| self.sessions.contains_key(id) && !self.is_lost(*id));
                        match preferred.or_else(|| self.pick_session(None)) {
                            Some(id) => out.extend(self.show(id)),
                            None => self.focus = Focus::Sidebar,
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
                    out.extend(self.place_new(id));
                    self.focus = Focus::Terminal;
                    self.status = None;
                    let shared = self.sharing_dir(id);
                    if !shared.is_empty() {
                        let ids: Vec<_> = shared.iter().map(|i| format!("#{i}")).collect();
                        let fix = match self.keymap.hint(Command::NewWorktreeSession) {
                            Some(k) => format!(" Leader {k} starts one in its own worktree."),
                            None => String::new(),
                        };
                        self.set_status(
                            Tone::Warning,
                            format!(
                                "Same directory as {}: agents may overwrite each other.{fix}",
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
                let id = session.id;
                if !self.tab_sessions().contains(&id) {
                    // Arrived after its pane went away.
                    out.push(ClientMessage::Detach { session: Some(id) });
                } else {
                    self.views.insert(
                        id,
                        View {
                            id,
                            parser: new_parser(session.size, &snapshot),
                            controller,
                        },
                    );
                    self.done.remove(&id);
                    if self.focused() == Some(id) {
                        self.cursor = Some(id);
                        if !controller {
                            self.set_status(Tone::Warning, OBSERVING);
                        } else if self.status_is_control_notice() {
                            self.status = None;
                        }
                    }
                    // The pane may have changed size since the request.
                    out.extend(self.resize_panes());
                }
                self.sessions.insert(id, session);
            }
            ServerMessage::Output { session, data } => {
                if let Some(v) = self.views.get_mut(&session) {
                    v.parser.process(&data);
                }
            }
            ServerMessage::Resized { session, size } => {
                if let Some(v) = self.views.get_mut(&session) {
                    v.parser.set_size(nz(size.rows), nz(size.cols));
                }
            }
            ServerMessage::Resync {
                session,
                size,
                snapshot,
            } => {
                if let Some(v) = self.views.get_mut(&session) {
                    v.parser = new_parser(size, &snapshot);
                }
            }
            ServerMessage::Detached { session } => {
                // Unprompted only when the session is removed, which may be
                // announced before or after this. Otherwise it answers our
                // own detach of a hidden pane, which stays in its tab.
                if !self.sessions.contains_key(&session) {
                    out.extend(self.drop_pane(session));
                }
            }
            ServerMessage::ControlLost { session } => {
                if let Some(v) = self.views.get_mut(&session) {
                    v.controller = false;
                    if self.focused() == Some(session) {
                        self.set_status(Tone::Warning, CONTROL_LOST);
                    }
                }
            }
            ServerMessage::ControlReleased { session } => {
                if self.focused() == Some(session) && self.view().is_some_and(|v| !v.controller) {
                    self.set_status(Tone::Info, CONTROL_FREE);
                }
            }
            ServerMessage::SessionUpdated(info) => {
                let old = self.sessions.get(&info.id).and_then(agent_status);
                self.note_activity(old, &info);
                self.sessions.insert(info.id, info);
            }
            ServerMessage::SessionRemoved(id) => {
                self.sessions.remove(&id);
                self.done.remove(&id);
                out.extend(self.drop_pane(id));
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
                | Mode::RenameWorkspace { input, .. }
                | Mode::Palette { query: input, .. } = &mut self.mode
                {
                    input.extend(text.chars().filter(|c| !c.is_control()));
                    return Vec::new();
                }
                match self.view() {
                    Some(v) if self.can_type() => self.input(keys::encode_paste(
                        &text,
                        v.parser.screen().bracketed_paste(),
                    )),
                    _ => Vec::new(),
                }
            }
            Event::Mouse(m) => self.on_mouse(m),
            Event::Resize(cols, rows) => {
                self.set_terminal_size((cols, rows));
                self.resize_panes()
            }
            _ => Vec::new(),
        }
    }

    /// Top-left of the terminal panel on screen.
    fn panel_origin(&self) -> (u16, u16) {
        let x = if self.sidebar_visible {
            SIDEBAR_WIDTH
        } else {
            0
        };
        (x, self.header_rows())
    }

    fn on_mouse(&mut self, m: MouseEvent) -> Vec<ClientMessage> {
        if self.link != Link::Connected || self.mode != Mode::Normal {
            return Vec::new();
        }
        let (px, py) = self.panel_origin();
        let at = ratatui::layout::Position::new(m.column, m.row);
        let hit = self
            .sidebar_hits
            .borrow()
            .iter()
            .find(|(r, _)| r.contains(at))
            .map(|(_, h)| *h);
        let click = matches!(m.kind, MouseEventKind::Down(MouseButton::Left));
        match (click, hit) {
            (true, Some(SidebarHit::Tab(i))) => return self.select_tab(i),
            (true, Some(SidebarHit::NewTab)) => return self.new_terminal_tab(),
            _ => {}
        }
        if self.sidebar_visible && m.column < px && m.row >= py {
            return match (m.kind, hit) {
                (MouseEventKind::Down(MouseButton::Left), Some(SidebarHit::Session(id))) => {
                    self.switch_to(id)
                }
                (MouseEventKind::Down(MouseButton::Left), Some(SidebarHit::Workspace(w))) => {
                    self.open_space(w)
                }
                (MouseEventKind::Down(MouseButton::Left), Some(SidebarHit::New)) => {
                    self.run(Command::NewSession)
                }
                (MouseEventKind::Down(MouseButton::Left), Some(SidebarHit::Menu)) => {
                    self.run(Command::Palette)
                }
                (MouseEventKind::ScrollDown, _) => {
                    self.move_cursor(1);
                    Vec::new()
                }
                (MouseEventKind::ScrollUp, _) => {
                    self.move_cursor(-1);
                    Vec::new()
                }
                _ => Vec::new(),
            };
        }
        let Some(pane) = self
            .pane_areas()
            .0
            .into_iter()
            .find(|p| p.title.contains(at) || p.body.contains(at))
        else {
            return Vec::new();
        };
        if matches!(m.kind, MouseEventKind::Down(_)) {
            self.focus = Focus::Terminal;
            if let Some(t) = self.tabs.get_mut(self.tab) {
                t.focus = pane.session;
            }
            self.cursor = Some(pane.session);
            if pane.title.contains(at) {
                return Vec::new();
            }
        }
        if !self.can_type() || self.focused() != Some(pane.session) || !pane.body.contains(at) {
            return Vec::new();
        }
        let Some(screen) = self.view().map(|v| v.parser.screen()) else {
            return Vec::new();
        };
        let pos = (m.column - pane.body.x, m.row - pane.body.y);
        let mode = screen.mouse_protocol_mode();
        if mode != vt100::MouseProtocolMode::None {
            return keys::encode_mouse(
                m.kind,
                m.modifiers,
                pos,
                mode,
                screen.mouse_protocol_encoding(),
            )
            .map(|b| self.input(b))
            .unwrap_or_default();
        }
        // Like xterm's alternate-scroll mode: a full-screen app without mouse
        // support scrolls with the arrow keys.
        let arrow = match m.kind {
            MouseEventKind::ScrollUp => KeyCode::Up,
            MouseEventKind::ScrollDown => KeyCode::Down,
            _ => return Vec::new(),
        };
        if !screen.alternate_screen() {
            return Vec::new();
        }
        keys::encode(
            &KeyEvent::new(arrow, KeyModifiers::NONE),
            screen.application_cursor(),
        )
        .map(|b| self.input(b.repeat(WHEEL_LINES)))
        .unwrap_or_default()
    }

    /// Whether keystrokes should go to the attached agent.
    pub fn can_type(&self) -> bool {
        self.link == Link::Connected
            && self.mode == Mode::Normal
            && self.focus == Focus::Terminal
            && self.view().is_some_and(|v| v.controller)
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
            Mode::Palette { .. } => self.on_palette_key(key),
            Mode::AddWorkspace { .. } | Mode::RenameWorkspace { .. } => {
                self.on_workspace_prompt_key(key)
            }
            Mode::Leader => {
                self.mode = Mode::Normal;
                self.on_leader_command(key)
            }
            Mode::Normal if self.keymap.is_leader(&key) => {
                self.mode = Mode::Leader;
                Vec::new()
            }
            Mode::Normal if self.link == Link::Connected && self.keymap.global(&key).is_some() => {
                match self.keymap.global(&key) {
                    Some(Global::Jump(n)) => self.switch_nth(n),
                    Some(Global::Run(cmd)) => self.run(cmd),
                    None => Vec::new(),
                }
            }
            Mode::Normal if self.can_type() => {
                let app_cursor = self
                    .view()
                    .is_some_and(|v| v.parser.screen().application_cursor());
                keys::encode(&key, app_cursor)
                    .map(|b| self.input(b))
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
                if self.view().is_some() {
                    self.focus = Focus::Terminal;
                }
            }
            KeyCode::Char('n') => self.open_picker_for(Place::Replace),
            KeyCode::Char('w') => self.open_workspaces(),
            KeyCode::Char('p') => self.open_palette(false),
            KeyCode::Char('/') => self.open_palette(true),
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
        if self.keymap.is_leader(&key) {
            // Leader twice sends the leader key itself to the agent.
            let app_cursor = self
                .view()
                .is_some_and(|v| v.parser.screen().application_cursor());
            return match keys::encode(&self.keymap.leader.event(), app_cursor) {
                Some(bytes) if self.can_type() => self.input(bytes),
                _ => Vec::new(),
            };
        }
        if let Some(n) = digit(&key) {
            return self.switch_nth(n);
        }
        match self.keymap.command(&key) {
            Some(cmd) => self.run(cmd),
            None => Vec::new(),
        }
    }

    /// Runs a command from a leader key or the palette.
    pub fn run(&mut self, cmd: Command) -> Vec<ClientMessage> {
        let current = self.focused();
        match cmd {
            Command::Palette => self.open_palette(false),
            Command::SearchSessions => self.open_palette(true),
            Command::NewSession => self.open_picker_for(Place::Replace),
            Command::SplitRight => self.open_picker_for(Place::Split(Dir::Right)),
            Command::SplitDown => self.open_picker_for(Place::Split(Dir::Down)),
            Command::NewTab => return self.new_terminal_tab(),
            Command::NextTab | Command::PrevTab if self.tabs.len() > 1 => {
                let before = self.tab_sessions();
                let delta = if cmd == Command::NextTab { 1 } else { -1 };
                self.tab =
                    (self.tab as isize + delta).rem_euclid(self.tabs.len() as isize) as usize;
                self.cursor = self.focused();
                self.space = self.current().map(|s| s.workspace);
                self.focus = Focus::Terminal;
                return self.sync_panes(before);
            }
            Command::NextTab | Command::PrevTab => {
                let delta = if cmd == Command::NextTab { 1 } else { -1 };
                return self.switch_in_space(delta);
            }
            Command::FocusNextPane => {
                if let Some(t) = self.tabs.get_mut(self.tab) {
                    t.cycle_focus(1);
                    self.cursor = Some(t.focus);
                    self.focus = Focus::Terminal;
                }
            }
            Command::Zoom => {
                if let Some(t) = self
                    .tabs
                    .get_mut(self.tab)
                    .filter(|t| t.root.panes().len() > 1)
                {
                    t.zoomed = !t.zoomed;
                    return self.resize_panes();
                }
            }
            Command::ClosePane => return self.close_pane(),
            Command::Notes if self.config.integrations.obsidian.enabled => {
                return self.open_notes();
            }
            Command::Notes => {}
            Command::Review if self.config.integrations.review.enabled => {
                return self.open_review(false);
            }
            Command::PullRequest if self.config.integrations.github.enabled => {
                return self.open_review(true);
            }
            Command::RunTask if self.config.integrations.tasks.enabled => {
                return self.pick_task();
            }
            Command::Review | Command::PullRequest | Command::RunTask => {}
            Command::CloseTab => return self.close_tab(),
            Command::NewWorktreeSession => {
                self.open_picker_for(Place::Replace);
                if self.picker_can_worktree() {
                    if let Mode::NewSession { worktree, .. } = &mut self.mode {
                        *worktree = true;
                    }
                } else {
                    self.set_status(
                        Tone::Warning,
                        "This workspace is not a git repository; the session will share its directory.",
                    );
                }
            }
            Command::Workspaces => self.open_workspaces(),
            Command::AddWorkspace => {
                let input = std::env::current_dir()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default();
                self.mode = Mode::AddWorkspace { input };
            }
            Command::RenameWorkspace => {
                if let Some(w) = self.active_workspace() {
                    let input = self.workspace_name(w).to_string();
                    self.mode = Mode::RenameWorkspace {
                        workspace: w,
                        input,
                    };
                }
            }
            Command::NextSession => return self.switch_relative(1),
            Command::PrevSession => return self.switch_relative(-1),
            Command::NextWorkspace => return self.switch_workspace(1),
            Command::PrevWorkspace => return self.switch_workspace(-1),
            Command::ToggleSidebar => {
                self.sidebar_hidden = !self.sidebar_hidden;
                if self.sidebar_hidden && self.focus == Focus::Sidebar && self.view().is_some() {
                    self.focus = Focus::Terminal;
                }
                self.set_terminal_size(self.terminal);
                return self.resize_panes();
            }
            Command::Sidebar => {
                self.focus = Focus::Sidebar;
                self.cursor = current.or(self.cursor);
            }
            Command::Rename => {
                if let Some(id) = current.or(self.cursor) {
                    self.start_rename(id);
                }
            }
            Command::Kill => {
                if let Some(id) = current.or(self.cursor) {
                    self.mode = Mode::ConfirmKill(id);
                }
            }
            Command::TakeControl => {
                if let Some(id) = current {
                    return vec![self.take_control(id)];
                }
            }
            Command::Detach => return self.detach(),
            Command::Help => self.mode = Mode::Help,
            Command::Settings => {
                let path = self
                    .config_path
                    .as_deref()
                    .unwrap_or("~/.config/hive/config.toml");
                self.set_status(
                    Tone::Info,
                    format!("Settings live in {path}. Changes apply the next time hive starts."),
                );
            }
        }
        Vec::new()
    }

    fn open_palette(&mut self, sessions_only: bool) {
        self.mode = Mode::Palette {
            query: String::new(),
            selected: 0,
            scope: if sessions_only {
                Scope::Sessions
            } else {
                Scope::All
            },
        };
    }

    /// Palette entries matching the current query, best first.
    pub fn palette_items(&self) -> Vec<PaletteItem> {
        let Mode::Palette { query, scope, .. } = &self.mode else {
            return Vec::new();
        };
        let mut items: Vec<(PaletteItem, String)> = Vec::new();
        if *scope == Scope::Tasks {
            let tasks = self.tasks.as_ref().map_or(&[][..], |l| &l.tasks[..]);
            for (i, t) in tasks.iter().enumerate() {
                let item = PaletteItem {
                    target: Target::Task(i),
                    title: t.title.clone(),
                    detail: String::new(),
                    key: None,
                };
                items.push((item, t.title.clone()));
            }
        }
        let sessions_only = *scope != Scope::All;
        if !sessions_only {
            let hidden = hidden_commands(&self.config);
            for cmd in Command::ALL {
                if matches!(cmd, Command::Palette | Command::SearchSessions)
                    || hidden.contains(&cmd)
                {
                    continue;
                }
                let item = PaletteItem {
                    target: Target::Command(cmd),
                    title: cmd.title().to_string(),
                    detail: String::new(),
                    key: self.keymap.hint(cmd),
                };
                items.push((item, cmd.title().to_string()));
            }
        }
        for id in self.order().into_iter().filter(|_| *scope != Scope::Tasks) {
            let s = &self.sessions[&id];
            let workspace = self.workspace_name(s.workspace);
            let mut detail = format!("#{} {} · {workspace}", s.id, s.agent);
            if let Some(b) = &s.branch {
                detail = format!("{detail} · {b}");
            }
            if s.state == SessionState::Lost {
                detail.push_str(" · ended");
            }
            let haystack = format!("{} {detail}", s.label);
            items.push((
                PaletteItem {
                    target: Target::Session(id),
                    title: s.label.clone(),
                    detail,
                    key: None,
                },
                haystack,
            ));
        }
        if !sessions_only {
            for w in self.workspace_order() {
                let name = self.workspace_name(w).to_string();
                let root = self
                    .workspaces
                    .get(&w)
                    .map(|w| w.root.clone())
                    .unwrap_or_default();
                items.push((
                    PaletteItem {
                        target: Target::Workspace(w),
                        title: format!("Workspace: {name}"),
                        detail: root.clone(),
                        key: None,
                    },
                    format!("workspace {name} {root}"),
                ));
            }
        }
        if query.trim().is_empty() {
            return items.into_iter().map(|(i, _)| i).collect();
        }
        let mut scored: Vec<(i32, PaletteItem)> = items
            .into_iter()
            .filter_map(|(item, hay)| fuzzy(query, &hay).map(|score| (score, item)))
            .collect();
        // Stable: equal scores keep their natural order.
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(_, i)| i).collect()
    }

    fn on_palette_key(&mut self, key: KeyEvent) -> Vec<ClientMessage> {
        let count = self.palette_items().len();
        let chosen = match &self.mode {
            Mode::Palette { selected, .. } => self.palette_items().get(*selected).map(|i| i.target),
            _ => None,
        };
        let Mode::Palette {
            query, selected, ..
        } = &mut self.mode
        else {
            return Vec::new();
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Down => *selected = (*selected + 1).min(count.saturating_sub(1)),
            KeyCode::Char('n') if ctrl => *selected = (*selected + 1).min(count.saturating_sub(1)),
            KeyCode::Up => *selected = selected.saturating_sub(1),
            KeyCode::Char('p') if ctrl => *selected = selected.saturating_sub(1),
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                return match chosen {
                    Some(Target::Command(cmd)) => self.run(cmd),
                    Some(Target::Session(id)) if self.is_lost(id) => self.relaunch(id),
                    Some(Target::Session(id)) => self.switch_to(id),
                    Some(Target::Workspace(w)) => self.open_workspace(w),
                    Some(Target::Task(i)) => self.run_task(i),
                    None => Vec::new(),
                };
            }
            _ => match edit_line(query, &key) {
                Edit::Cancel => self.mode = Mode::Normal,
                Edit::Editing => *selected = 0,
                Edit::Submit => {}
            },
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
        let mut out = self.show(id);
        // Showing an observed session again tries to take control if free.
        if self.views.get(&id).is_some_and(|v| !v.controller) {
            out.push(self.attach(id));
        }
        out
    }

    /// Takes the focused pane out of its tab; the session keeps running.
    /// The last pane of a tab closes the tab, unless it is the only one.
    fn close_pane(&mut self) -> Vec<ClientMessage> {
        let Some(id) = self.focused() else {
            return Vec::new();
        };
        let before = self.tab_sessions();
        if before.len() > 1 {
            self.tabs[self.tab].remove(id);
            self.cursor = self.focused();
            return self.sync_panes(before);
        }
        self.close_tab()
    }

    /// Closes the active tab; its sessions keep running.
    fn close_tab(&mut self) -> Vec<ClientMessage> {
        if self.tabs.len() < 2 {
            self.set_status(
                Tone::Info,
                "This is the only tab. Sessions keep running; detach to leave.",
            );
            return Vec::new();
        }
        let before = self.tab_sessions();
        self.tabs.remove(self.tab);
        self.tab = self.tab.min(self.tabs.len() - 1);
        self.cursor = self.focused();
        self.sync_panes(before)
    }

    /// Selects a space: shows its focused-last agent, or just lists it
    /// (empty, ready for "new").
    fn open_space(&mut self, w: WorkspaceId) -> Vec<ClientMessage> {
        let ids = self
            .groups()
            .into_iter()
            .find(|(g, _)| *g == w)
            .map(|(_, ids)| ids)
            .unwrap_or_default();
        match ids.into_iter().find(|&id| !self.is_lost(id)) {
            Some(id) => self.switch_to(id),
            None => {
                self.space = Some(w);
                // The sidebar shows the cursor's space while browsing.
                self.cursor = None;
                self.focus = Focus::Terminal;
                let name = self.workspace_name(w).to_string();
                let new = self.keymap.hint(Command::NewSession).unwrap_or_default();
                self.set_status(
                    Tone::Info,
                    format!("No agents in {name} yet. Click new (or leader {new}) to start one."),
                );
                Vec::new()
            }
        }
    }

    /// A shell in a new tab, like herdr: in the focused agent's directory,
    /// or the selected space's root.
    fn new_terminal_tab(&mut self) -> Vec<ClientMessage> {
        let workspace = self.active_workspace();
        let cwd = self
            .current()
            .filter(|s| Some(s.workspace) == workspace)
            .map(|s| PathBuf::from(&s.cwd))
            .or_else(|| {
                workspace
                    .and_then(|w| self.workspaces.get(&w))
                    .map(|w| PathBuf::from(&w.root))
            });
        match launch::create_request(&self.config, "shell", None, cwd, &[], self.panel) {
            Ok(mut req) => {
                req.workspace = workspace;
                self.place = Place::NewTab;
                self.awaiting_create = true;
                self.mode = Mode::Normal;
                self.set_status(Tone::Info, "Opening a terminal…");
                vec![ClientMessage::CreateSession(req)]
            }
            Err(e) => {
                self.set_status(Tone::Error, e.to_string());
                Vec::new()
            }
        }
    }

    fn select_tab(&mut self, i: usize) -> Vec<ClientMessage> {
        if i >= self.tabs.len() || i == self.tab {
            return Vec::new();
        }
        let before = self.tab_sessions();
        self.tab = i;
        self.cursor = self.focused();
        self.space = self.current().map(|s| s.workspace);
        self.focus = Focus::Terminal;
        self.sync_panes(before)
    }

    fn switch_nth(&mut self, n: usize) -> Vec<ClientMessage> {
        match self.space_sessions().get(n) {
            Some(&id) => self.switch_to(id),
            None => Vec::new(),
        }
    }

    /// Selects the next/previous space (empty ones too), wrapping around.
    fn switch_workspace(&mut self, delta: isize) -> Vec<ClientMessage> {
        let spaces: Vec<WorkspaceId> = self.groups().into_iter().map(|(w, _)| w).collect();
        if spaces.is_empty() {
            return Vec::new();
        }
        let idx = self
            .shown_space()
            .and_then(|w| spaces.iter().position(|&s| s == w))
            .map_or(0, |i| i as isize + delta);
        self.open_space(spaces[idx.rem_euclid(spaces.len() as isize) as usize])
    }

    /// Next/previous agent of the shown space, wrapping around.
    fn switch_in_space(&mut self, delta: isize) -> Vec<ClientMessage> {
        let ids: Vec<SessionId> = self
            .space_sessions()
            .into_iter()
            .filter(|&id| !self.is_lost(id) || Some(id) == self.focused())
            .collect();
        if ids.is_empty() {
            return Vec::new();
        }
        let idx = self
            .focused()
            .and_then(|f| ids.iter().position(|&id| id == f))
            .map_or(0, |i| i as isize + delta);
        self.switch_to(ids[idx.rem_euclid(ids.len() as isize) as usize])
    }

    /// Next/previous attachable session, wrapping around.
    fn switch_relative(&mut self, delta: isize) -> Vec<ClientMessage> {
        let order: Vec<SessionId> = self
            .order()
            .into_iter()
            .filter(|&id| !self.is_lost(id) || Some(id) == self.focused())
            .collect();
        if order.is_empty() {
            return Vec::new();
        }
        let from = self.focused().or(self.cursor);
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
        self.prefer = self.focused().or(self.prefer);
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

    fn open_picker_for(&mut self, place: Place) {
        self.place = place;
        self.open_picker(None);
    }

    fn open_picker(&mut self, workspace: Option<WorkspaceId>) {
        let path = std::env::var_os("PATH").unwrap_or_default();
        // Known agents are offered only when installed; configured ones (and
        // the default) always, so a missing one is reported, not hidden.
        self.agents = self
            .config
            .agents()
            .into_iter()
            .map(|(name, def)| AgentChoice {
                installed: launch::find_executable(&def.command, &path).is_some(),
                label: hive_agents::catalog::by_id(&name)
                    .filter(|k| k.command == def.command)
                    .map(|k| k.label),
                command: def.command,
                name,
            })
            .filter(|a| {
                a.installed
                    || self.config.is_configured(&a.name)
                    || a.name == self.config.general.default_agent
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

    /// The focused (or selected) session's directory, workspace and that
    /// workspace's root, for integrations that work "beside" it.
    fn context(&mut self) -> Option<(PathBuf, WorkspaceId, PathBuf)> {
        let Some(s) = self
            .focused()
            .or(self.cursor)
            .and_then(|id| self.sessions.get(&id))
        else {
            self.set_status(Tone::Warning, "Open a session in the workspace first.");
            return None;
        };
        let cwd = PathBuf::from(&s.cwd);
        let workspace = s.workspace;
        let root = self
            .workspaces
            .get(&workspace)
            .map_or_else(|| cwd.clone(), |w| PathBuf::from(&w.root));
        Some((cwd, workspace, root))
    }

    /// Opens the Obsidian vault of the focused session's workspace with the
    /// configured editor, or shows the notes pane already open for it.
    fn open_notes(&mut self) -> Vec<ClientMessage> {
        let Some((cwd, workspace, root)) = self.context() else {
            return Vec::new();
        };
        let Some(vault) = hive_workspace::find_obsidian_vault(&cwd, &root) else {
            let name = self.workspace_name(workspace);
            self.set_status(
                Tone::Warning,
                format!("No Obsidian vault (a folder with .obsidian/) in {name}."),
            );
            return Vec::new();
        };
        let obsidian = &self.config.integrations.obsidian;
        let Some(def) = obsidian.notes_agent(&self.config) else {
            self.set_status(
                Tone::Error,
                format!(
                    "[integrations.obsidian] agent `{}` is not defined.",
                    obsidian.agent
                ),
            );
            return Vec::new();
        };
        let label = format!("notes · {}", hive_workspace::default_name(&vault));
        let open_in = obsidian.open_in;
        self.open_tool(NOTES_AGENT, &def, label, vault, workspace, open_in)
    }

    /// Review (`[integrations.review]`) or pull request checks
    /// (`[integrations.github]`) in the focused session's directory.
    fn open_review(&mut self, github: bool) -> Vec<ClientMessage> {
        let Some((cwd, workspace, _)) = self.context() else {
            return Vec::new();
        };
        let integrations = &self.config.integrations;
        let (tool, agent, label) = if github {
            (&integrations.github, GITHUB_AGENT, "checks")
        } else {
            (&integrations.review, REVIEW_AGENT, "review")
        };
        let argv = if !tool.command.is_empty() {
            tool.command.clone()
        } else if github {
            ["gh", "pr", "checks", "--watch"].map(String::from).to_vec()
        } else {
            default_review_command()
        };
        let open_in = tool.open_in;
        let def = argv_def(&argv);
        let label = format!("{label} · {}", hive_workspace::default_name(&cwd));
        self.open_tool(agent, &def, label, cwd, workspace, open_in)
    }

    /// Lists the project tasks next to the focused session in the palette.
    fn pick_task(&mut self) -> Vec<ClientMessage> {
        let Some((cwd, workspace, root)) = self.context() else {
            return Vec::new();
        };
        let found = [cwd, root]
            .into_iter()
            .map(|d| (hive_workspace::find_tasks(&d), d))
            .find(|(t, _)| !t.is_empty());
        let Some((tasks, dir)) = found else {
            self.set_status(
                Tone::Warning,
                "No tasks found (justfile, package.json, Makefile or Cargo.toml).",
            );
            return Vec::new();
        };
        self.tasks = Some(TaskList {
            tasks,
            dir,
            workspace,
        });
        self.mode = Mode::Palette {
            query: String::new(),
            selected: 0,
            scope: Scope::Tasks,
        };
        Vec::new()
    }

    fn run_task(&mut self, i: usize) -> Vec<ClientMessage> {
        let Some(list) = self.tasks.take() else {
            return Vec::new();
        };
        let Some(task) = list.tasks.get(i) else {
            return Vec::new();
        };
        let def = argv_def(&task.argv);
        let label = format!("task · {}", task.title);
        let open_in = self.config.integrations.tasks.open_in;
        self.open_tool(TASK_AGENT, &def, label, list.dir, list.workspace, open_in)
    }

    /// Shows the running session already doing this, or starts it in a tab
    /// or split.
    fn open_tool(
        &mut self,
        agent: &str,
        def: &hive_core::config::AgentDef,
        label: String,
        cwd: PathBuf,
        workspace: WorkspaceId,
        open_in: OpenIn,
    ) -> Vec<ClientMessage> {
        let mut req = match launch::request_for(def, agent, Some(label), Some(cwd), &[], self.panel)
        {
            Ok(req) => req,
            Err(e) => {
                self.set_status(Tone::Error, e.to_string());
                return Vec::new();
            }
        };
        let open = self.sessions.values().find(|s| {
            s.agent == agent
                && s.state == SessionState::Running
                && s.cwd == req.cwd
                && s.command == req.command
        });
        if let Some(id) = open.map(|s| s.id) {
            return self.show(id);
        }
        req.workspace = Some(workspace);
        self.place = match open_in {
            OpenIn::Tab => Place::NewTab,
            OpenIn::Split => Place::Split(Dir::Right),
        };
        self.awaiting_create = true;
        self.set_status(Tone::Info, format!("Opening {}…", req.label));
        vec![ClientMessage::CreateSession(req)]
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
            session: self.focused(),
        });
        vec![ClientMessage::Detach { session: None }]
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
            presence: None,
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

    /// Shows `id` (as switching would) and delivers its `Attached` reply.
    fn attached(app: &mut App, id: SessionId) {
        let _ = app.show(id);
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

    /// Requests other than detaching the pane being replaced.
    fn attaches(out: Vec<ClientMessage>) -> Vec<ClientMessage> {
        out.into_iter()
            .filter(|m| !matches!(m, ClientMessage::Detach { .. }))
            .collect()
    }

    fn attach_msg(app: &App, id: SessionId) -> Vec<ClientMessage> {
        vec![ClientMessage::Attach {
            session: id,
            size: app.panel,
            force: false,
        }]
    }

    fn with_status(id: SessionId, status: AgentStatus) -> SessionInfo {
        SessionInfo {
            presence: Some(hive_protocol::AgentPresence {
                agent: "claude".into(),
                label: "Claude Code".into(),
                status,
                message: None,
                source: hive_protocol::PresenceSource::Process,
            }),
            ..info(id, SessionState::Running)
        }
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn mouse_clicks_sessions_and_reaches_agents_that_ask_for_it() {
        let mut app = app();
        let (px, py) = app.panel_origin();
        app.sidebar_hits
            .borrow_mut()
            .extend([(Rect::new(0, 3, 26, 2), SidebarHit::Session(2))]);
        let click = MouseEventKind::Down(MouseButton::Left);
        assert_eq!(
            attaches(app.on_terminal_event(mouse(click, 2, 4))),
            attach_msg(&app, 2)
        );

        // No mouse mode and the primary screen: wheel does nothing.
        attached(&mut app, 2);
        assert!(
            app.on_terminal_event(mouse(MouseEventKind::ScrollUp, px + 1, py))
                .is_empty()
        );
        // A full-screen app without mouse support scrolls with arrows.
        app.on_server(ServerMessage::Output {
            session: 2,
            data: b"\x1b[?1049h".to_vec(),
        });
        assert_eq!(
            app.on_terminal_event(mouse(MouseEventKind::ScrollDown, px + 1, py)),
            vec![ClientMessage::Input {
                session: 2,
                data: b"\x1b[B\x1b[B\x1b[B".to_vec()
            }]
        );
        // An app that enables SGR mouse gets panel-relative coordinates.
        app.on_server(ServerMessage::Output {
            session: 2,
            data: b"\x1b[?1000h\x1b[?1006h".to_vec(),
        });
        assert_eq!(
            app.on_terminal_event(mouse(click, px + 4, py + 2)),
            vec![ClientMessage::Input {
                session: 2,
                data: b"\x1b[<0;5;3M".to_vec()
            }]
        );
        // The status bar is outside the panel.
        assert!(
            app.on_terminal_event(mouse(click, px + 4, py + app.panel.rows))
                .is_empty()
        );
    }

    #[test]
    fn global_chords_switch_without_the_leader() {
        let mut app = app();
        let alt = |code| key(code, KeyModifiers::ALT);
        assert_eq!(
            attaches(app.on_terminal_event(alt(KeyCode::Char('3')))),
            attach_msg(&app, 3)
        );
        attached(&mut app, 3);
        assert_eq!(
            attaches(app.on_terminal_event(alt(KeyCode::Right))),
            attach_msg(&app, 1)
        );
        // Plain digits still reach the agent.
        attached(&mut app, 1);
        assert_eq!(
            app.on_terminal_event(ch('3')),
            vec![ClientMessage::Input {
                session: 1,
                data: b"3".to_vec()
            }]
        );
    }

    #[test]
    fn toggling_the_sidebar_gives_the_agent_the_full_width() {
        let mut app = app();
        assert!(app.sidebar_visible);
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('b'));
        assert!(!app.sidebar_visible);
        assert_eq!(app.panel.cols, 100);
        assert_eq!(
            out,
            vec![ClientMessage::Resize {
                session: 1,
                size: app.panel
            }]
        );
        app.on_terminal_event(Event::Resize(120, 30));
        assert!(!app.sidebar_visible, "stays hidden across resizes");
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('b'));
        assert_eq!(app.panel.cols, 120 - SIDEBAR_WIDTH);
    }

    /// Runs a leader command that opens the picker and launches `shell`;
    /// returns the CreateSession size.
    fn launch_via(app: &mut App, key: char) -> TermSize {
        app.config.general.default_agent = "shell".into();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch(key));
        let out = app.on_terminal_event(self::key(KeyCode::Enter, KeyModifiers::NONE));
        match out.as_slice() {
            [ClientMessage::CreateSession(req)] => req.size,
            other => panic!("expected CreateSession, got {other:?}"),
        }
    }

    fn sizes(out: &[ClientMessage]) -> Vec<(SessionId, TermSize)> {
        out.iter()
            .filter_map(|m| match m {
                ClientMessage::Attach { session, size, .. }
                | ClientMessage::Resize { session, size } => Some((*session, *size)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn splits_share_the_panel_and_route_input_to_the_focused_pane() {
        let mut app = app();
        let full = app.panel;
        launch_via(&mut app, 'v');
        let out = app.on_server(ServerMessage::SessionCreated(info(
            4,
            SessionState::Running,
        )));
        // Two columns with a separator, each losing a row to its title.
        let half = TermSize {
            rows: full.rows - 1,
            cols: (full.cols - 1) / 2,
        };
        let right = TermSize {
            rows: full.rows - 1,
            cols: full.cols - 1 - half.cols,
        };
        assert_eq!(sizes(&out), vec![(4, right), (1, half)]);
        assert_eq!(app.focused(), Some(4));
        attached(&mut app, 4);
        app.on_server(ServerMessage::Resized {
            session: 1,
            size: half,
        });
        assert_eq!(
            app.on_terminal_event(ch('a')),
            vec![ClientMessage::Input {
                session: 4,
                data: b"a".to_vec()
            }]
        );

        // Focus moves with leader o and with a click on the other pane.
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('o'));
        assert_eq!(app.focused(), Some(1));
        let (panes, _) = app.pane_areas();
        let right_pane = panes.iter().find(|p| p.session == 4).unwrap();
        app.on_terminal_event(mouse(
            MouseEventKind::Down(MouseButton::Left),
            right_pane.body.x + 1,
            right_pane.body.y + 1,
        ));
        assert_eq!(app.focused(), Some(4));

        // Zoom gives the focused pane everything; unzoom restores it.
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('z'));
        assert_eq!(sizes(&out), vec![(4, full)]);
        app.on_server(ServerMessage::Resized {
            session: 4,
            size: full,
        });
        app.on_terminal_event(leader());
        assert_eq!(sizes(&app.on_terminal_event(ch('z'))), vec![(4, right)]);
        app.on_server(ServerMessage::Resized {
            session: 4,
            size: right,
        });

        // Closing a pane detaches it (the session keeps running) and the
        // other pane takes the whole panel.
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('X'));
        assert_eq!(out[0], ClientMessage::Detach { session: Some(4) });
        assert_eq!(sizes(&out), vec![(1, full)]);
        assert!(app.sessions.contains_key(&4));
    }

    #[test]
    fn tabs_keep_their_own_panes() {
        let mut app = app();
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('t'));
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("leader t opens a terminal tab, got {out:?}");
        };
        assert_eq!(req.agent, "shell");
        let out = app.on_server(ServerMessage::SessionCreated(info(
            5,
            SessionState::Running,
        )));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.tab, 1);
        assert_eq!(out[0], ClientMessage::Detach { session: Some(1) });
        assert_eq!(sizes(&out), vec![(5, app.panel)]);
        attached(&mut app, 5);

        // Back to the first tab: its pane comes back, the other goes. The
        // daemon's reply to that detach leaves the hidden tab alone.
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('['));
        assert_eq!(app.focused(), Some(1));
        assert_eq!(out[0], ClientMessage::Detach { session: Some(5) });
        assert_eq!(attaches(out), attach_msg(&app, 1));
        assert!(
            app.on_server(ServerMessage::Detached { session: 5 })
                .is_empty()
        );
        assert_eq!(app.tabs.len(), 2);

        // Switching to a session shown in another tab activates that tab.
        attached(&mut app, 1);
        app.on_terminal_event(Event::Key(KeyEvent::new(
            KeyCode::Char('2'),
            KeyModifiers::ALT,
        )));
        assert_eq!(
            app.tab, 0,
            "#2 isn't in a tab: it replaces the focused pane"
        );
        assert_eq!(app.tabs[0].root.panes(), vec![2]);
        let out = app.run(Command::NextTab);
        assert_eq!(app.focused(), Some(5));
        assert_eq!(attaches(out), attach_msg(&app, 5));

        // A session that ends leaves its tab; an emptied tab closes.
        attached(&mut app, 5);
        app.on_server(ServerMessage::SessionRemoved(5));
        app.on_server(ServerMessage::Detached { session: 5 });
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.focused(), Some(2));
    }

    #[test]
    fn a_split_pane_that_ends_gives_its_space_back() {
        let mut app = app();
        launch_via(&mut app, '-');
        app.on_server(ServerMessage::SessionCreated(info(
            4,
            SessionState::Running,
        )));
        attached(&mut app, 4);
        // The forwarder's Detached can beat the removal broadcast.
        assert!(
            app.on_server(ServerMessage::Detached { session: 4 })
                .is_empty()
        );
        let out = app.on_server(ServerMessage::SessionRemoved(4));
        assert_eq!(app.tabs[0].root.panes(), vec![1]);
        assert_eq!(sizes(&out), vec![(1, app.panel)]);
    }

    fn palette_has(app: &mut App, cmd: Command) -> bool {
        app.on_terminal_event(leader());
        app.on_terminal_event(ch(':'));
        let found = app
            .palette_items()
            .iter()
            .any(|i| i.target == Target::Command(cmd));
        app.on_terminal_event(key(KeyCode::Esc, KeyModifiers::NONE));
        found
    }

    #[test]
    fn notes_are_invisible_until_enabled() {
        let mut app = app();
        assert!(app.keymap.keys_for(Command::Notes).is_empty());
        assert!(!palette_has(&mut app, Command::Notes));
        app.on_terminal_event(leader());
        assert!(app.on_terminal_event(ch('N')).is_empty());
        assert_eq!(app.status, None);
        // A user binding N to something else doesn't clash with the default.
        let mut config = Config::default();
        config.keys.bind.insert(
            "rename_session".into(),
            hive_core::config::KeyList::One("N".into()),
        );
        let app = App::new(config, Start::Attach(1), (100, 26));
        assert_eq!(app.keymap.keys_for(Command::Rename).len(), 1);
    }

    #[test]
    fn notes_open_the_workspace_vault_once() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let vault = root.join("vault");
        std::fs::create_dir_all(vault.join(".obsidian")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();

        let mut config = Config::default();
        config.integrations.obsidian.enabled = true;
        config.integrations.obsidian.agent = "shell".into();
        let mut app = App::new(config, Start::Attach(1), (100, 26));
        app.on_connect();
        app.on_server(ServerMessage::WorkspaceList(vec![WorkspaceInfo {
            id: 1,
            name: "proj".into(),
            root: root.display().to_string(),
            is_repo: false,
        }]));
        let mut one = info(1, SessionState::Running);
        one.cwd = root.join("src").display().to_string();
        app.on_server(ServerMessage::SessionList(vec![one]));
        attached(&mut app, 1);
        assert!(palette_has(&mut app, Command::Notes));

        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('N'));
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("expected CreateSession, got {out:?}");
        };
        assert_eq!(req.agent, NOTES_AGENT);
        assert_eq!(req.label, "notes · vault");
        assert_eq!(req.cwd, vault.display().to_string());
        assert_eq!(req.workspace, Some(1));

        // Opens in a new tab by default.
        let mut notes = info(2, SessionState::Running);
        notes.agent = NOTES_AGENT.into();
        notes.cwd = req.cwd.clone();
        notes.command = req.command.clone();
        app.on_server(ServerMessage::SessionCreated(notes));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.focused(), Some(2));

        // Asking again from the first tab goes back to that pane.
        app.run(Command::PrevTab);
        assert_eq!(app.focused(), Some(1));
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('N'));
        assert!(
            !out.iter()
                .any(|m| matches!(m, ClientMessage::CreateSession(_)))
        );
        assert_eq!((app.tab, app.focused()), (1, Some(2)));
    }

    #[test]
    fn notes_without_a_vault_say_so() {
        let mut config = Config::default();
        config.integrations.obsidian.enabled = true;
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(config, Start::Attach(1), (100, 26));
        app.on_connect();
        let mut one = info(1, SessionState::Running);
        one.cwd = dir.path().display().to_string();
        app.on_server(ServerMessage::SessionList(vec![one]));
        attached(&mut app, 1);
        assert!(app.run(Command::Notes).is_empty());
        assert!(matches!(&app.status, Some((Tone::Warning, m)) if m.contains("No Obsidian vault")));
    }

    /// An app with every integration on and session 1 in `dir`.
    fn integrated(dir: &std::path::Path) -> App {
        let mut config = Config::default();
        let i = &mut config.integrations;
        i.obsidian.enabled = true;
        i.obsidian.journal = true;
        i.obsidian.journal_dir = "Daily".into();
        i.review.enabled = true;
        i.review.command = vec!["sh".into()];
        i.github.enabled = true;
        i.tasks.enabled = true;
        i.desktop_notify.enabled = true;
        let mut app = App::new(config, Start::Attach(1), (100, 26));
        app.on_connect();
        let mut one = info(1, SessionState::Running);
        one.cwd = dir.display().to_string();
        let mut two = info(2, SessionState::Running);
        two.cwd = dir.display().to_string();
        app.on_server(ServerMessage::SessionList(vec![one.clone(), two]));
        attached(&mut app, 1);
        app.on_server(ServerMessage::SessionUpdated(one));
        app
    }

    #[test]
    fn review_opens_beside_the_session_and_is_reused() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = integrated(dir.path());
        assert!(palette_has(&mut app, Command::Review));
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('G'));
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("expected CreateSession, got {out:?}");
        };
        assert_eq!(req.agent, REVIEW_AGENT);
        assert_eq!(req.cwd, dir.path().display().to_string());
        let mut review = info(3, SessionState::Running);
        review.agent = REVIEW_AGENT.into();
        review.cwd = req.cwd.clone();
        review.command = req.command.clone();
        app.on_server(ServerMessage::SessionCreated(review));
        assert_eq!(app.tabs[0].root.panes(), vec![1, 3], "split by default");
        app.run(Command::FocusNextPane);
        let out = app.run(Command::Review);
        assert!(
            !out.iter()
                .any(|m| matches!(m, ClientMessage::CreateSession(_)))
        );
        assert_eq!(app.focused(), Some(3));
    }

    #[test]
    fn tasks_are_offered_in_the_palette_and_run_in_a_split() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Makefile"),
            "test:\n\ttrue\nlint:\n\ttrue\n",
        )
        .unwrap();
        let mut app = integrated(dir.path());
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('T'));
        let titles: Vec<String> = app.palette_items().into_iter().map(|i| i.title).collect();
        assert_eq!(titles, vec!["make test", "make lint"]);
        for c in "lint".chars() {
            app.on_terminal_event(ch(c));
        }
        let out = app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE));
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("expected CreateSession, got {out:?}");
        };
        assert_eq!(req.label, "task · make lint");
        assert_eq!(req.command.last().map(String::as_str), Some("lint"));
        assert_eq!(app.place, Place::Split(Dir::Right));
    }

    #[test]
    fn finished_agents_notify_and_write_the_journal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".obsidian")).unwrap();
        let mut app = integrated(dir.path());
        let mut s = with_status(2, AgentStatus::Working);
        s.cwd = dir.path().display().to_string();
        app.on_server(ServerMessage::SessionUpdated(s.clone()));
        let mut idle = with_status(2, AgentStatus::Idle);
        idle.cwd = s.cwd.clone();
        app.on_server(ServerMessage::SessionUpdated(idle));
        let effects = std::mem::take(&mut app.effects);
        assert!(matches!(&effects[0], Effect::Notify { title, .. } if title.ends_with("finished")));
        assert!(matches!(&effects[1],
            Effect::Journal { dir: d, line } if *d == dir.path().join("Daily") && line.starts_with("claude finished in #2")));

        // Off by default: nothing queued.
        let mut plain = self::app();
        plain.on_server(ServerMessage::SessionUpdated(with_status(
            2,
            AgentStatus::Working,
        )));
        plain.on_server(ServerMessage::SessionUpdated(with_status(
            2,
            AgentStatus::Idle,
        )));
        assert!(plain.done.contains(&2));
        assert!(plain.effects.is_empty());
    }

    #[test]
    fn spaces_tabs_and_buttons_are_clickable_and_alt_arrows_move_between_them() {
        let mut app = app_with_workspaces();
        let mut three = info(3, SessionState::Running);
        three.workspace = 2;
        app.on_server(ServerMessage::SessionUpdated(three));
        let press = |x, y| mouse(MouseEventKind::Down(MouseButton::Left), x, y);
        let set_hits = |app: &App, hits: Vec<(Rect, SidebarHit)>| {
            *app.sidebar_hits.borrow_mut() = hits;
        };
        let spaces: Vec<WorkspaceId> = app.groups().into_iter().map(|(w, _)| w).collect();
        assert!(spaces.len() >= 2);
        let other = spaces[1];
        set_hits(
            &app,
            vec![
                (Rect::new(0, 3, 26, 2), SidebarHit::Workspace(other)),
                (Rect::new(0, 10, 13, 1), SidebarHit::New),
                (Rect::new(13, 10, 13, 1), SidebarHit::Menu),
            ],
        );
        app.on_terminal_event(press(2, 4));
        assert_eq!(app.shown_space(), Some(other));
        assert_eq!(app.current().map(|s| s.workspace), Some(other));

        app.on_terminal_event(press(15, 10));
        assert!(matches!(
            app.mode,
            Mode::Palette {
                scope: Scope::All,
                ..
            }
        ));
        app.on_terminal_event(key(KeyCode::Esc, KeyModifiers::NONE));
        app.on_terminal_event(press(2, 10));
        assert!(matches!(app.mode, Mode::NewSession { .. }));
        app.on_terminal_event(key(KeyCode::Esc, KeyModifiers::NONE));

        // Alt+Up/Down walk the spaces, wrapping.
        let alt = |code| key(code, KeyModifiers::ALT);
        app.on_terminal_event(alt(KeyCode::Down));
        assert_eq!(app.shown_space(), Some(spaces[2 % spaces.len()]));
        app.on_terminal_event(alt(KeyCode::Up));
        assert_eq!(app.shown_space(), Some(other));

        assert_eq!(app.focused(), Some(3));

        // With one tab, Alt+Right is the next agent of this space.
        app.on_terminal_event(Event::Key(KeyEvent::new(
            KeyCode::Char('1'),
            KeyModifiers::ALT,
        )));
        assert_eq!(app.focused(), Some(3), "Alt+1 is this space's first agent");
        let alt_ = |code| key(code, KeyModifiers::ALT);
        app.on_terminal_event(alt_(KeyCode::Up));
        assert_eq!(app.space_sessions(), vec![1, 2]);
        app.on_terminal_event(alt_(KeyCode::Right));
        assert_eq!(app.focused(), Some(2));
        app.on_terminal_event(alt_(KeyCode::Right));
        assert_eq!(app.focused(), Some(1), "wraps within the space");

        // A tab in the header's tab bar.
        app.on_terminal_event(leader());
        let out = app.on_terminal_event(ch('t'));
        let [ClientMessage::CreateSession(req)] = out.as_slice() else {
            panic!("leader t opens a terminal tab, got {out:?}");
        };
        assert_eq!(req.agent, "shell");
        app.on_server(ServerMessage::SessionCreated(info(
            9,
            SessionState::Running,
        )));
        assert_eq!(app.tab, 1);
        set_hits(&app, vec![(Rect::new(80, 0, 9, 1), SidebarHit::Tab(0))]);
        app.on_terminal_event(press(82, 0));
        assert_eq!(app.tab, 0);
        set_hits(&app, vec![(Rect::new(95, 0, 3, 1), SidebarHit::NewTab)]);
        let out = app.on_terminal_event(press(96, 0));
        assert!(matches!(out.as_slice(), [ClientMessage::CreateSession(r)] if r.agent == "shell"));
        assert_eq!(app.place, Place::NewTab);
        app.place = Place::Replace;
        app.on_terminal_event(alt(KeyCode::Right));
        assert_eq!(app.tab, 1, "with tabs, Alt+Right moves between them");
    }

    #[test]
    fn unseen_sessions_become_done_and_ring_the_bell() {
        let mut app = app();
        let update = |app: &mut App, id, status| {
            app.on_server(ServerMessage::SessionUpdated(with_status(id, status)));
            std::mem::take(&mut app.bell)
        };
        assert!(!update(&mut app, 2, AgentStatus::Working));
        assert_eq!(app.activity(2), Some(Activity::Working));
        assert_eq!(app.activity(3), None, "no agent recognised");

        assert!(update(&mut app, 2, AgentStatus::Idle));
        assert_eq!(app.activity(2), Some(Activity::Done));
        assert_eq!(app.workspace_activity(1), Some(Activity::Done));

        assert!(update(&mut app, 3, AgentStatus::Blocked));
        assert!(!update(&mut app, 3, AgentStatus::Blocked), "rings once");
        assert_eq!(app.workspace_activity(1), Some(Activity::Blocked));

        // The viewed session finishing is not news.
        update(&mut app, 1, AgentStatus::Working);
        assert!(!update(&mut app, 1, AgentStatus::Idle));
        assert_eq!(app.activity(1), Some(Activity::Idle));

        // Looking at a done session clears it.
        let _ = app.show(2);
        app.on_server(ServerMessage::Attached {
            session: with_status(2, AgentStatus::Idle),
            snapshot: Vec::new(),
            controller: true,
        });
        assert_eq!(app.activity(2), Some(Activity::Idle));

        app.config.notifications.bell = false;
        update(&mut app, 3, AgentStatus::Working);
        assert!(!update(&mut app, 3, AgentStatus::Idle), "bell disabled");
        assert_eq!(app.activity(3), Some(Activity::Done));
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
        assert_eq!(attaches(out), attach_msg(&app, 2));
    }

    #[test]
    fn keys_are_forwarded_and_leader_is_intercepted() {
        let mut app = app();
        assert_eq!(
            app.on_terminal_event(ch('a')),
            vec![ClientMessage::Input {
                session: 1,
                data: b"a".to_vec()
            }]
        );
        assert!(app.on_terminal_event(leader()).is_empty());
        assert_eq!(app.mode, Mode::Leader);
        assert_eq!(
            app.on_terminal_event(leader()),
            vec![ClientMessage::Input {
                session: 1,
                data: vec![0]
            }]
        );
    }

    #[test]
    fn leader_d_detaches_without_killing() {
        let mut app = app();
        app.on_terminal_event(leader());
        assert_eq!(
            app.on_terminal_event(ch('d')),
            vec![ClientMessage::Detach { session: None }]
        );
        assert_eq!(app.exit, Some(Outcome::Detached { session: Some(1) }));
    }

    #[test]
    fn leader_j_k_and_digits_switch_sessions() {
        let mut app = app();
        app.on_terminal_event(leader());
        assert_eq!(
            attaches(app.on_terminal_event(ch('j'))),
            attach_msg(&app, 2)
        );
        attached(&mut app, 2);
        app.on_terminal_event(leader());
        assert_eq!(
            attaches(app.on_terminal_event(ch('k'))),
            attach_msg(&app, 1)
        );
        attached(&mut app, 1);
        // Wraps backwards from the first to the last.
        app.on_terminal_event(leader());
        assert_eq!(
            attaches(app.on_terminal_event(ch('k'))),
            attach_msg(&app, 3)
        );
        attached(&mut app, 3);
        app.on_terminal_event(leader());
        assert_eq!(
            attaches(app.on_terminal_event(ch('2'))),
            attach_msg(&app, 2)
        );
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
            attaches(app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE))),
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
        assert_eq!(app.view().unwrap().id, 1);
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

        // Installed agents (machine-dependent) may sort in between.
        for _ in 0..app.agents.len() {
            match app.mode {
                Mode::NewSession { selected, .. } if app.agents[selected].name == "zz-test-sh" => {
                    break;
                }
                _ => app.on_terminal_event(ch('j')),
            };
        }
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
        assert_eq!(attaches(out), attach_msg(&app, 7));
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
        let out = app.on_server(ServerMessage::SessionRemoved(1));
        assert_eq!(attaches(out), attach_msg(&app, 3));
        assert!(
            app.on_server(ServerMessage::Detached { session: 1 })
                .is_empty()
        );
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
        assert!(app.view().is_none());
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
        let text = app.view().unwrap().parser.screen().contents();
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
        assert_eq!(
            out,
            vec![ClientMessage::Resize {
                session: 1,
                size: panel
            }]
        );
        app.on_server(ServerMessage::Resized {
            session: 1,
            size: panel,
        });
        let (r, c) = app.view().unwrap().parser.screen().size();
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
        assert_eq!(
            attaches(app.on_terminal_event(ch('2'))),
            attach_msg(&app, 2)
        );
    }

    #[test]
    fn attaching_as_observer_explains_how_to_take_control() {
        let mut app = app();
        let _ = app.show(2);
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: false,
        });
        assert!(!app.can_type());
        assert!(matches!(&app.status, Some((Tone::Warning, m)) if m.contains("Observing")));
        app.on_server(ServerMessage::ControlReleased { session: 2 });
        assert!(matches!(&app.status, Some((Tone::Info, m)) if m.contains("free")));
        let _ = app.show(2);
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: true,
        });
        assert_eq!(app.status, None, "control notices clear once controller");
        let _ = app.show(2);
        app.on_server(ServerMessage::Attached {
            session: info(2, SessionState::Running),
            snapshot: Vec::new(),
            controller: false,
        });
        let _ = app.show(2);
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
        let _ = app.show(2);
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
            app.view()
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
        assert_eq!(attaches(out), attach_msg(&app, 2));
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
        assert_eq!(attaches(out), attach_msg(&app, 4));
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
        assert_eq!(
            attaches(app.on_terminal_event(ch('j'))),
            attach_msg(&app, 3)
        );
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
        assert_eq!(attaches(out), attach_msg(&app, 9));
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
        app.on_terminal_event(ch('c'));
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
            attaches(app.on_terminal_event(key(KeyCode::Enter, KeyModifiers::NONE))),
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

    fn typed(app: &mut App, text: &str) {
        for c in text.chars() {
            app.on_terminal_event(ch(c));
        }
    }

    fn enter() -> Event {
        key(KeyCode::Enter, KeyModifiers::NONE)
    }

    #[test]
    fn palette_finds_and_runs_commands() {
        let mut app = app_with_workspaces();
        app.on_terminal_event(leader());
        app.on_terminal_event(ch(':'));
        assert!(matches!(
            app.mode,
            Mode::Palette {
                scope: Scope::All,
                ..
            }
        ));
        let all = app.palette_items();
        assert!(
            all.iter()
                .any(|i| i.target == Target::Command(Command::Settings))
        );
        assert!(all.iter().any(|i| i.target == Target::Session(2)));
        assert!(all.iter().any(|i| i.target == Target::Workspace(2)));
        // Navigation keys are typed into the query, not used to move.
        typed(&mut app, "term");
        let items = app.palette_items();
        assert_eq!(items[0].target, Target::Command(Command::Kill));
        assert_eq!(items[0].key.as_deref(), Some("x"));
        assert!(app.on_terminal_event(enter()).is_empty());
        assert_eq!(app.mode, Mode::ConfirmKill(1));
    }

    #[test]
    fn session_search_switches_to_the_match() {
        let mut app = app_with_workspaces();
        for (id, label) in [(2, "frontend"), (3, "billing api")] {
            let mut s = info(id, SessionState::Running);
            s.label = label.into();
            app.on_server(ServerMessage::SessionUpdated(s));
        }
        app.on_terminal_event(leader());
        app.on_terminal_event(ch('/'));
        assert!(
            app.palette_items()
                .iter()
                .all(|i| matches!(i.target, Target::Session(_))),
            "sessions only"
        );
        typed(&mut app, "bil");
        assert_eq!(app.palette_items()[0].target, Target::Session(3));
        // Ctrl+N / arrows move; Esc would close.
        app.on_terminal_event(key(KeyCode::Backspace, KeyModifiers::NONE));
        app.on_terminal_event(key(KeyCode::Backspace, KeyModifiers::NONE));
        app.on_terminal_event(key(KeyCode::Backspace, KeyModifiers::NONE));
        app.on_terminal_event(key(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert_eq!(
            attaches(app.on_terminal_event(enter())),
            attach_msg(&app, 2)
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn fuzzy_prefers_word_starts_and_runs() {
        assert!(fuzzy("ns", "New session") > fuzzy("ns", "Rename session"));
        assert!(fuzzy("api", "billing api") > fuzzy("api", "a pile"));
        assert_eq!(fuzzy("xyz", "New session"), None);
        assert_eq!(fuzzy("", "anything"), Some(0));
    }

    #[test]
    fn custom_leader_and_bindings_from_config() {
        let mut config = Config::default();
        config.keys.leader = "ctrl+a".into();
        config.keys.bind.insert(
            "kill_session".into(),
            hive_core::config::KeyList::One("K".into()),
        );
        let mut app = App::new(config, Start::Attach(1), (100, 26));
        app.on_server(ServerMessage::SessionList(vec![info(
            1,
            SessionState::Running,
        )]));
        attached(&mut app, 1);
        // Ctrl+Space is no longer special: it goes to the agent as NUL.
        assert_eq!(
            app.on_terminal_event(leader()),
            vec![ClientMessage::Input {
                session: 1,
                data: vec![0]
            }]
        );
        let ctrl_a = || key(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(app.on_terminal_event(ctrl_a()).is_empty());
        assert_eq!(app.mode, Mode::Leader);
        // Leader twice sends the leader itself (Ctrl+A is 0x01).
        assert_eq!(
            app.on_terminal_event(ctrl_a()),
            vec![ClientMessage::Input {
                session: 1,
                data: vec![1]
            }]
        );
        app.on_terminal_event(ctrl_a());
        app.on_terminal_event(ch('x'));
        assert_eq!(app.mode, Mode::Normal, "x is unbound now");
        app.on_terminal_event(ctrl_a());
        app.on_terminal_event(key(KeyCode::Char('K'), KeyModifiers::SHIFT));
        assert_eq!(app.mode, Mode::ConfirmKill(1));
    }

    #[test]
    fn invalid_keys_fall_back_to_defaults_with_an_error() {
        let mut config = Config::default();
        config.keys.leader = "a".into();
        let app = App::new(config, Start::Auto, (100, 26));
        assert!(
            app.keymap
                .is_leader(&KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL))
        );
        assert!(matches!(&app.status, Some((Tone::Error, m)) if m.contains("leader")));
    }

    #[test]
    fn settings_shows_the_config_path() {
        let mut app = app();
        app.config_path = Some("/home/u/.config/hive/config.toml".into());
        app.run(Command::Settings);
        assert!(
            matches!(&app.status, Some((Tone::Info, m)) if m.contains("/home/u/.config/hive/config.toml"))
        );
    }
}
