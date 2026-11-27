//! `hive board` — Hive-native Kanban task board & multi-agent launcher TUI.
//!
//! Built with Hive's `Theme`, rounded borders, mouse support, bottom task
//! inspector, and centered modal dialogs for creating, assigning, and running
//! agent tasks in one smooth flow.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use hive_core::client::Client;
use hive_core::config::{Config, RoleConfig};
use hive_core::paths::Paths;
use hive_protocol::{AgentStatus, ClientMessage, ServerMessage, SessionId, SessionInfo};
use hive_tasks::{NewTask, Task, TaskPatch, TaskStatus, TaskStore};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::theme::{Env, Theme};

const COLUMNS: [TaskStatus; 4] = [
    TaskStatus::Ready,
    TaskStatus::InProgress,
    TaskStatus::InReview,
    TaskStatus::Done,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Info,
    Ok,
    Err,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormField {
    Title,
    Body,
    Role,
}

#[derive(Debug, Clone)]
struct TaskForm {
    editing_id: Option<u32>,
    title: String,
    body: String,
    /// Index into `role_options()` (`0` = `auto`).
    role_idx: usize,
    field: FormField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CardAction {
    Spawn,
    StopAgent,
    CreatePr,
    AssignRole,
    MoveRight,
    MoveLeft,
    MarkDone,
    Edit,
    Delete,
}

impl CardAction {
    const ALL: [CardAction; 9] = [
        CardAction::Spawn,
        CardAction::StopAgent,
        CardAction::CreatePr,
        CardAction::AssignRole,
        CardAction::MoveRight,
        CardAction::MoveLeft,
        CardAction::MarkDone,
        CardAction::Edit,
        CardAction::Delete,
    ];

    fn icon_and_label(self) -> (&'static str, &'static str) {
        match self {
            CardAction::Spawn => ("▶", "Run / Spawn agent on this task"),
            CardAction::StopAgent => ("⏹", "Stop running agent session"),
            CardAction::CreatePr => ("↗", "Create Pull Request (gh pr create)"),
            CardAction::AssignRole => ("●", "Assign role / model…"),
            CardAction::MoveRight => ("→", "Move to next column"),
            CardAction::MoveLeft => ("←", "Move to previous column"),
            CardAction::MarkDone => ("✓", "Mark Done"),
            CardAction::Edit => ("✎", "Edit title & prompt"),
            CardAction::Delete => ("✕", "Delete task"),
        }
    }
}

#[derive(Debug, Clone)]
enum Modal {
    None,
    Form(TaskForm),
    Actions { task_id: u32, selected: usize },
    RolePicker { task_id: u32, selected: usize },
    ConfirmDelete { task_id: u32, title: String },
    Help,
}

struct BoardState {
    workspace_name: String,
    workspace_root: PathBuf,
    store: TaskStore,
    roles: BTreeMap<String, RoleConfig>,
    tasks: Vec<Task>,
    /// Live sessions fetched from `hived` for real-time agent status & swarm tree.
    sessions: Vec<SessionInfo>,
    /// Auto-pipeline: automatically advance finished workers -> IN REVIEW (+ spawn reviewer) -> DONE/PR.
    autopilot: bool,
    /// Sessions observed in `Working` state so we only hand off after real work finishes.
    seen_working: BTreeSet<SessionId>,
    /// When an active session first returned to `Idle` after working (debounce 3s).
    idle_since: BTreeMap<SessionId, Instant>,
    /// Sessions already handled by auto-pipeline so we never double-advance or double-spawn.
    handled_sessions: BTreeSet<SessionId>,
    col: usize,
    row: usize,
    modal: Modal,
    toast: Option<(Tone, String)>,
    /// Last-drawn column rectangles for mouse click hit-testing.
    col_rects: [Rect; 4],
    /// Last-drawn modal list area for mouse click hit-testing.
    modal_list_rect: Option<Rect>,
}

impl BoardState {
    fn new(
        workspace_name: String,
        workspace_root: PathBuf,
        store: TaskStore,
        roles: BTreeMap<String, RoleConfig>,
    ) -> anyhow::Result<Self> {
        let tasks = store.list()?;
        let mut state = Self {
            workspace_name,
            workspace_root,
            store,
            roles,
            tasks,
            sessions: Vec::new(),
            autopilot: true,
            seen_working: BTreeSet::new(),
            idle_since: BTreeMap::new(),
            handled_sessions: BTreeSet::new(),
            col: 0,
            row: 0,
            modal: Modal::None,
            toast: None,
            col_rects: [Rect::default(); 4],
            modal_list_rect: None,
        };
        // Start on the first non-empty column so the user immediately sees a selected card.
        if state.col_tasks(0).is_empty()
            && let Some(first) = (0..COLUMNS.len()).find(|&c| !state.col_tasks(c).is_empty())
        {
            state.col = first;
        }
        Ok(state)
    }

    fn session_for_task(&self, t: &Task) -> Option<&SessionInfo> {
        let tid = t.id.to_string();
        // Prefer any live session tagged with this task_id, else the linked session_id.
        self.sessions
            .iter()
            .find(|s| s.state.is_alive() && s.task_id.as_deref() == Some(tid.as_str()))
            .or_else(|| {
                t.session_id
                    .as_deref()
                    .and_then(|id| id.parse::<SessionId>().ok())
                    .and_then(|sid| self.sessions.iter().find(|s| s.id == sid))
            })
    }

    fn child_count(&self, parent: SessionId) -> usize {
        self.sessions
            .iter()
            .filter(|s| s.parent_session == Some(parent) && s.state.is_alive())
            .count()
    }

    fn reload(&mut self) -> anyhow::Result<()> {
        self.tasks = self.store.list()?;
        self.clamp_row();
        Ok(())
    }

    /// Role picker options: `None` = Auto (smart pick), `Some(name)` = specific role.
    fn role_options(&self) -> Vec<Option<String>> {
        let mut opts = vec![None];
        for name in self.roles.keys() {
            opts.push(Some(name.clone()));
        }
        opts
    }

    fn col_tasks(&self, col_idx: usize) -> Vec<&Task> {
        let status = COLUMNS[col_idx];
        self.tasks
            .iter()
            .filter(|t| match status {
                TaskStatus::Ready => matches!(t.status, TaskStatus::Ready | TaskStatus::Backlog),
                TaskStatus::InProgress => {
                    matches!(t.status, TaskStatus::InProgress | TaskStatus::Blocked)
                }
                other => t.status == other,
            })
            .collect()
    }

    fn clamp_row(&mut self) {
        let len = self.col_tasks(self.col).len();
        if len == 0 {
            self.row = 0;
        } else {
            self.row = self.row.min(len - 1);
        }
    }

    fn focus_task(&mut self, id: u32) {
        for c in 0..COLUMNS.len() {
            let list = self.col_tasks(c);
            if let Some(pos) = list.iter().position(|t| t.id == id) {
                self.col = c;
                self.row = pos;
                return;
            }
        }
    }

    fn selected_task(&self) -> Option<&Task> {
        self.col_tasks(self.col).get(self.row).copied()
    }

    fn selected_id(&self) -> Option<u32> {
        self.selected_task().map(|t| t.id)
    }

    fn set_toast(&mut self, tone: Tone, msg: impl Into<String>) {
        self.toast = Some((tone, msg.into()));
    }

    fn move_col(&mut self, delta: isize) {
        self.col = (self.col as isize + delta).clamp(0, (COLUMNS.len() - 1) as isize) as usize;
        self.clamp_row();
    }

    fn move_row(&mut self, delta: isize) {
        let len = self.col_tasks(self.col).len();
        if len == 0 {
            self.row = 0;
            return;
        }
        self.row = (self.row as isize + delta).clamp(0, (len - 1) as isize) as usize;
    }

    fn open_new_form(&mut self) {
        self.toast = None;
        self.modal = Modal::Form(TaskForm {
            editing_id: None,
            title: String::new(),
            body: String::new(),
            role_idx: 0, // auto
            field: FormField::Title,
        });
    }

    fn open_edit_form(&mut self, id: u32) {
        let Some(t) = self.tasks.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        let opts = self.role_options();
        let role_idx = opts.iter().position(|o| *o == t.role).unwrap_or(0);
        self.modal = Modal::Form(TaskForm {
            editing_id: Some(id),
            title: t.title,
            body: t.body,
            role_idx,
            field: FormField::Title,
        });
    }

    fn open_actions(&mut self) {
        let Some(id) = self.selected_id() else {
            self.open_new_form();
            return;
        };
        self.modal = Modal::Actions {
            task_id: id,
            selected: 0,
        };
    }

    fn open_role_picker(&mut self, task_id: Option<u32>) {
        let Some(id) = task_id.or_else(|| self.selected_id()) else {
            self.set_toast(Tone::Info, "No task selected — press [n] to create one.");
            return;
        };
        let current_role = self
            .tasks
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.role.clone());
        let opts = self.role_options();
        let selected = opts.iter().position(|o| *o == current_role).unwrap_or(0);
        self.modal = Modal::RolePicker {
            task_id: id,
            selected,
        };
    }

    fn open_delete_confirm(&mut self, task_id: Option<u32>) {
        let Some(id) = task_id.or_else(|| self.selected_id()) else {
            return;
        };
        let title = self
            .tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.title.clone())
            .unwrap_or_default();
        self.modal = Modal::ConfirmDelete { task_id: id, title };
    }

    fn move_task_by(&mut self, id: u32, delta: isize) -> anyhow::Result<()> {
        let Some(t) = self.tasks.iter().find(|t| t.id == id) else {
            return Ok(());
        };
        let cur_col = match t.status {
            TaskStatus::Backlog | TaskStatus::Ready => 0usize,
            TaskStatus::InProgress | TaskStatus::Blocked => 1,
            TaskStatus::InReview => 2,
            TaskStatus::Done => 3,
        };
        let target = (cur_col as isize + delta).clamp(0, (COLUMNS.len() - 1) as isize) as usize;
        if target == cur_col {
            return Ok(());
        }
        let next_status = COLUMNS[target];
        self.store.update(
            id,
            TaskPatch {
                status: Some(next_status),
                ..Default::default()
            },
        )?;
        self.reload()?;
        self.focus_task(id);
        self.set_toast(Tone::Ok, format!("Moved #{id} → {}", next_status.label()));
        Ok(())
    }

    fn mark_done(&mut self, id: u32) -> anyhow::Result<()> {
        self.store.update(
            id,
            TaskPatch {
                status: Some(TaskStatus::Done),
                ..Default::default()
            },
        )?;
        self.reload()?;
        self.focus_task(id);
        self.set_toast(Tone::Ok, format!("Marked #{id} Done"));
        Ok(())
    }

    fn apply_role(&mut self, task_id: u32, role_idx: usize) -> anyhow::Result<()> {
        let opts = self.role_options();
        let chosen = opts.get(role_idx).cloned().flatten();
        let label = chosen.as_deref().unwrap_or("auto").to_string();
        self.store.update(
            task_id,
            TaskPatch {
                role: Some(chosen),
                ..Default::default()
            },
        )?;
        self.reload()?;
        self.focus_task(task_id);
        self.set_toast(Tone::Ok, format!("Assigned #{task_id} → {label}"));
        Ok(())
    }

    fn submit_form(&mut self, form: TaskForm, and_spawn: bool) -> anyhow::Result<()> {
        let title = form.title.trim().to_string();
        if title.is_empty() {
            self.set_toast(Tone::Err, "Task title cannot be empty");
            self.modal = Modal::Form(form);
            return Ok(());
        }
        let opts = self.role_options();
        let chosen_role = opts.get(form.role_idx).cloned().flatten();

        let id = if let Some(edit_id) = form.editing_id {
            self.store.update(
                edit_id,
                TaskPatch {
                    title: Some(title),
                    body: Some(form.body),
                    role: Some(chosen_role),
                    ..Default::default()
                },
            )?;
            self.set_toast(Tone::Ok, format!("Updated task #{edit_id}"));
            edit_id
        } else {
            let t = self.store.create(NewTask {
                title,
                role: chosen_role,
                body: form.body,
                ..Default::default()
            })?;
            self.set_toast(Tone::Ok, format!("Created task #{}: {}", t.id, t.title));
            t.id
        };

        self.reload()?;
        self.focus_task(id);
        if and_spawn {
            self.spawn_task(id)?;
        }
        Ok(())
    }

    fn spawn_task(&mut self, id: u32) -> anyhow::Result<()> {
        let task = self.store.get(id)?;
        let role = task.role.clone().unwrap_or_else(|| "auto".to_string());
        let prompt = if task.body.trim().is_empty() {
            format!("Task #{}: {}", task.id, task.title)
        } else {
            format!("Task #{}: {}\n\n{}", task.id, task.title, task.body.trim())
        };
        let out = std::process::Command::new("hive")
            .args([
                "spawn",
                "--role",
                &role,
                "--task",
                &id.to_string(),
                "-C",
                &self.workspace_root.display().to_string(),
                "--prompt",
                &prompt,
                "--tasks-dir",
                &self.store.dir().display().to_string(),
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let msg = String::from_utf8_lossy(&o.stdout).trim().to_string();
                self.reload()?;
                self.focus_task(id);
                self.set_toast(
                    Tone::Ok,
                    if msg.is_empty() {
                        format!("Spawned {role} on #{id} — check the sidebar")
                    } else {
                        msg
                    },
                );
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                self.set_toast(Tone::Err, format!("Spawn failed: {err}"));
            }
            Err(e) => {
                self.set_toast(Tone::Err, format!("Failed to run hive spawn: {e}"));
            }
        }
        Ok(())
    }

    fn stop_task_agent(&mut self, id: u32) -> anyhow::Result<()> {
        let Some(t) = self.tasks.iter().find(|t| t.id == id).cloned() else {
            return Ok(());
        };
        let Some(sess) = self.session_for_task(&t).cloned() else {
            self.set_toast(
                Tone::Info,
                format!("Task #{id} has no running agent session"),
            );
            return Ok(());
        };
        let out = std::process::Command::new("hive")
            .args(["kill", &sess.id.to_string()])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                self.set_toast(
                    Tone::Ok,
                    format!("Stopped session s{} for task #{id}", sess.id),
                );
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                self.set_toast(Tone::Err, format!("Stop failed: {err}"));
            }
            Err(e) => {
                self.set_toast(Tone::Err, format!("Failed to run hive kill: {e}"));
            }
        }
        Ok(())
    }

    fn create_pr_for_task(&mut self, id: u32) -> anyhow::Result<()> {
        let Some(t) = self.tasks.iter().find(|t| t.id == id).cloned() else {
            return Ok(());
        };
        let cwd = self
            .session_for_task(&t)
            .map(|s| PathBuf::from(&s.cwd))
            .unwrap_or_else(|| self.workspace_root.clone());

        let out = std::process::Command::new("gh")
            .current_dir(&cwd)
            .args([
                "pr",
                "create",
                "--title",
                &format!("#{} {}", t.id, t.title),
                "--body",
                if t.body.trim().is_empty() {
                    "Automated PR created from Hive board."
                } else {
                    t.body.trim()
                },
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let _ = self.store.update(
                    id,
                    TaskPatch {
                        note: Some(format!("PR created: {url}")),
                        note_author: Some("hive".into()),
                        ..Default::default()
                    },
                );
                let _ = self.reload();
                self.set_toast(Tone::Ok, format!("PR created: {url}"));
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                self.set_toast(Tone::Err, format!("PR failed: {err}"));
            }
            Err(e) => {
                self.set_toast(Tone::Err, format!("Failed to run gh pr create: {e}"));
            }
        }
        Ok(())
    }

    /// Autonomous pipeline tick: watches running task sessions and automatically
    /// transitions finished workers -> `in_review` (+ spawns `reviewer` in the
    /// worker's exact cwd/worktree) and finished reviewers -> `done` (ready to merge / PR).
    fn tick_autopilot(&mut self) {
        if !self.autopilot {
            return;
        }
        let now = Instant::now();
        let mut finished: Vec<(u32, SessionInfo, TaskStatus)> = Vec::new();

        for t in &self.tasks {
            if !matches!(t.status, TaskStatus::InProgress | TaskStatus::InReview) {
                continue;
            }
            let Some(sess) = self.session_for_task(t).cloned() else {
                continue;
            };
            if self.handled_sessions.contains(&sess.id) {
                continue;
            }
            let status = sess.presence.as_ref().map(|p| p.status);
            if status == Some(AgentStatus::Working) {
                self.seen_working.insert(sess.id);
                self.idle_since.remove(&sess.id);
                continue;
            }

            let is_exited = !sess.state.is_alive();
            let is_settled_idle =
                self.seen_working.contains(&sess.id) && status == Some(AgentStatus::Idle) && {
                    let start = *self.idle_since.entry(sess.id).or_insert(now);
                    now.duration_since(start) >= Duration::from_secs(3)
                };

            if is_exited || is_settled_idle {
                finished.push((t.id, sess, t.status));
            }
        }

        for (task_id, sess, cur_status) in finished {
            let role = sess.role.as_deref().unwrap_or("");
            // Lead stays in control while child subagents are still running.
            if role == "lead" && self.child_count(sess.id) > 0 {
                continue;
            }
            self.handled_sessions.insert(sess.id);
            self.idle_since.remove(&sess.id);

            if cur_status == TaskStatus::InProgress && role != "reviewer" {
                let _ = self.store.update(
                    task_id,
                    TaskPatch {
                        status: Some(TaskStatus::InReview),
                        note: Some(format!(
                            "Agent s{} ({}) finished — auto-moved to IN REVIEW",
                            sess.id,
                            if role.is_empty() { &sess.agent } else { role }
                        )),
                        note_author: Some("autopilot".into()),
                        ..Default::default()
                    },
                );
                let _ = self.reload();
                // Automatically spawn `reviewer` in the worker's exact cwd/worktree!
                if self.roles.contains_key("reviewer") {
                    self.spawn_reviewer_for(task_id, &sess.cwd);
                } else {
                    self.set_toast(
                        Tone::Ok,
                        format!("⚡ #{task_id} finished → IN REVIEW (press [p] for PR)"),
                    );
                }
            } else if cur_status == TaskStatus::InReview && role == "reviewer" {
                let _ = self.store.update(
                    task_id,
                    TaskPatch {
                        status: Some(TaskStatus::Done),
                        note: Some(format!(
                            "Reviewer s{} finished — ready to merge / press [p] to open PR",
                            sess.id
                        )),
                        note_author: Some("autopilot".into()),
                        ..Default::default()
                    },
                );
                let _ = self.reload();
                self.set_toast(
                    Tone::Ok,
                    format!("⚡ #{task_id} reviewed → DONE! Press [p] to open PR"),
                );
            }
        }
    }

    fn spawn_reviewer_for(&mut self, task_id: u32, worker_cwd: &str) {
        let title = self
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| t.title.clone())
            .unwrap_or_default();
        let prompt = format!(
            "Review task #{task_id}: {title}\n\n\
            Inspect `git status` and `git diff` in this directory for bugs, regressions, and test coverage. \
            Run the project's tests if applicable. Leave a concise review summary with \
            `hive task update {task_id} --note \"<review summary>\"`. \
            If everything looks solid and you are on a feature branch, you may commit/push and open a PR with `gh pr create`."
        );
        let out = std::process::Command::new("hive")
            .args([
                "spawn",
                "--role",
                "reviewer",
                "--task",
                &task_id.to_string(),
                "--force",
                "-C",
                worker_cwd,
                "--prompt",
                &prompt,
                "--tasks-dir",
                &self.store.dir().display().to_string(),
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                // Keep card in IN REVIEW (run_spawn moves it to InProgress, so set back to InReview).
                let _ = self.store.update(
                    task_id,
                    TaskPatch {
                        status: Some(TaskStatus::InReview),
                        ..Default::default()
                    },
                );
                let _ = self.reload();
                self.set_toast(
                    Tone::Ok,
                    format!("⚡ #{task_id} → IN REVIEW · auto-spawned reviewer"),
                );
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                self.set_toast(
                    Tone::Err,
                    format!("Auto-review spawn failed on #{task_id}: {err}"),
                );
            }
            Err(e) => {
                self.set_toast(Tone::Err, format!("Auto-review error: {e}"));
            }
        }
    }

    fn run_card_action(&mut self, task_id: u32, action: CardAction) -> anyhow::Result<()> {
        match action {
            CardAction::Spawn => self.spawn_task(task_id)?,
            CardAction::StopAgent => self.stop_task_agent(task_id)?,
            CardAction::CreatePr => self.create_pr_for_task(task_id)?,
            CardAction::AssignRole => self.open_role_picker(Some(task_id)),
            CardAction::MoveRight => self.move_task_by(task_id, 1)?,
            CardAction::MoveLeft => self.move_task_by(task_id, -1)?,
            CardAction::MarkDone => self.mark_done(task_id)?,
            CardAction::Edit => self.open_edit_form(task_id),
            CardAction::Delete => self.open_delete_confirm(Some(task_id)),
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(true);
        }

        let modal = std::mem::replace(&mut self.modal, Modal::None);
        match modal {
            Modal::None => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                KeyCode::Char('h') | KeyCode::Left => self.move_col(-1),
                KeyCode::Char('l') | KeyCode::Right => self.move_col(1),
                KeyCode::Char('j') | KeyCode::Down => self.move_row(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_row(-1),
                KeyCode::Char('n') => self.open_new_form(),
                KeyCode::Enter => self.open_actions(),
                KeyCode::Char('a') => self.open_role_picker(None),
                KeyCode::Char('s') => {
                    if let Some(id) = self.selected_id() {
                        self.spawn_task(id)?;
                    } else {
                        self.set_toast(Tone::Info, "No task selected — press [n] to create one.");
                    }
                }
                KeyCode::Char('x') => {
                    if let Some(id) = self.selected_id() {
                        self.stop_task_agent(id)?;
                    }
                }
                KeyCode::Char('p') => {
                    if let Some(id) = self.selected_id() {
                        self.create_pr_for_task(id)?;
                    }
                }
                KeyCode::Char('P') => {
                    self.autopilot = !self.autopilot;
                    self.set_toast(
                        Tone::Info,
                        if self.autopilot {
                            "⚡ Auto-pipeline ON (worker → auto review → done/PR)"
                        } else {
                            "Auto-pipeline OFF (manual column transitions)"
                        },
                    );
                }
                KeyCode::Char('e') => {
                    if let Some(id) = self.selected_id() {
                        self.open_edit_form(id);
                    }
                }
                KeyCode::Char('d') | KeyCode::Delete => self.open_delete_confirm(None),
                KeyCode::Char(']') | KeyCode::Char('m') => {
                    if let Some(id) = self.selected_id() {
                        self.move_task_by(id, 1)?;
                    }
                }
                KeyCode::Char('[') => {
                    if let Some(id) = self.selected_id() {
                        self.move_task_by(id, -1)?;
                    }
                }
                KeyCode::Char('r') => {
                    self.reload()?;
                    self.set_toast(Tone::Info, "Reloaded board");
                }
                KeyCode::Char('?') => self.modal = Modal::Help,
                _ => {}
            },
            Modal::Form(mut form) => {
                let role_count = self.role_options().len();
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                match key.code {
                    KeyCode::Esc => {}
                    KeyCode::Char('s') if ctrl => {
                        self.submit_form(form, true)?;
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        form.field = match form.field {
                            FormField::Title => FormField::Body,
                            FormField::Body => FormField::Role,
                            FormField::Role => FormField::Title,
                        };
                        self.modal = Modal::Form(form);
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        form.field = match form.field {
                            FormField::Title => FormField::Role,
                            FormField::Body => FormField::Title,
                            FormField::Role => FormField::Body,
                        };
                        self.modal = Modal::Form(form);
                    }
                    KeyCode::Enter => {
                        self.submit_form(form, false)?;
                    }
                    KeyCode::Left if form.field == FormField::Role => {
                        form.role_idx = if form.role_idx == 0 {
                            role_count.saturating_sub(1)
                        } else {
                            form.role_idx - 1
                        };
                        self.modal = Modal::Form(form);
                    }
                    KeyCode::Right | KeyCode::Char(' ') if form.field == FormField::Role => {
                        form.role_idx = (form.role_idx + 1) % role_count.max(1);
                        self.modal = Modal::Form(form);
                    }
                    KeyCode::Backspace => {
                        match form.field {
                            FormField::Title => {
                                form.title.pop();
                            }
                            FormField::Body => {
                                form.body.pop();
                            }
                            FormField::Role => {}
                        }
                        self.modal = Modal::Form(form);
                    }
                    KeyCode::Char(c) if !ctrl => {
                        match form.field {
                            FormField::Title => form.title.push(c),
                            FormField::Body => form.body.push(c),
                            FormField::Role => {}
                        }
                        self.modal = Modal::Form(form);
                    }
                    _ => {
                        self.modal = Modal::Form(form);
                    }
                }
            }
            Modal::Actions {
                task_id,
                mut selected,
            } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {}
                KeyCode::Char('j') | KeyCode::Down => {
                    selected = (selected + 1) % CardAction::ALL.len();
                    self.modal = Modal::Actions { task_id, selected };
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    selected = if selected == 0 {
                        CardAction::ALL.len() - 1
                    } else {
                        selected - 1
                    };
                    self.modal = Modal::Actions { task_id, selected };
                }
                KeyCode::Enter => {
                    self.run_card_action(task_id, CardAction::ALL[selected])?;
                }
                _ => self.modal = Modal::Actions { task_id, selected },
            },
            Modal::RolePicker {
                task_id,
                mut selected,
            } => {
                let len = self.role_options().len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {}
                    KeyCode::Char('j') | KeyCode::Down => {
                        selected = (selected + 1) % len.max(1);
                        self.modal = Modal::RolePicker { task_id, selected };
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        selected = if selected == 0 {
                            len.saturating_sub(1)
                        } else {
                            selected - 1
                        };
                        self.modal = Modal::RolePicker { task_id, selected };
                    }
                    KeyCode::Enter => {
                        self.apply_role(task_id, selected)?;
                    }
                    _ => self.modal = Modal::RolePicker { task_id, selected },
                }
            }
            Modal::ConfirmDelete { task_id, title } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.store.delete(task_id)?;
                    self.reload()?;
                    self.set_toast(Tone::Ok, format!("Deleted #{task_id} \"{title}\""));
                }
                _ => {}
            },
            Modal::Help => {}
        }
        Ok(false)
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> anyhow::Result<()> {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return Ok(());
        }
        let x = mouse.column;
        let y = mouse.row;

        // Clicks inside modal lists
        let modal = std::mem::replace(&mut self.modal, Modal::None);
        match modal {
            Modal::Actions { task_id, selected } => {
                if let Some(r) = self.modal_list_rect
                    && x >= r.x
                    && x < r.x + r.width
                    && y >= r.y
                    && y < r.y + r.height
                {
                    let idx = (y - r.y) as usize;
                    if idx < CardAction::ALL.len() {
                        self.run_card_action(task_id, CardAction::ALL[idx])?;
                        return Ok(());
                    }
                }
                // Click outside dismisses
                let _ = selected;
                return Ok(());
            }
            Modal::RolePicker { task_id, selected } => {
                if let Some(r) = self.modal_list_rect
                    && x >= r.x
                    && x < r.x + r.width
                    && y >= r.y
                    && y < r.y + r.height
                {
                    let idx = (y - r.y) as usize;
                    if idx < self.role_options().len() {
                        self.apply_role(task_id, idx)?;
                        return Ok(());
                    }
                }
                let _ = selected;
                return Ok(());
            }
            Modal::None => {}
            other => {
                self.modal = other;
                return Ok(());
            }
        }

        // Click on a Kanban column / card (each card is 3 rows tall: title, meta, spacer)
        for (c, rect) in self.col_rects.iter().enumerate() {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                let was_col = self.col;
                let was_row = self.row;
                self.col = c;
                let inner_y = y.saturating_sub(rect.y + 1);
                let clicked_row = (inner_y / 3) as usize;
                let count = self.col_tasks(c).len();
                if count > 0 {
                    self.row = clicked_row.min(count - 1);
                    if was_col == self.col && was_row == self.row {
                        self.open_actions();
                    }
                } else {
                    self.row = 0;
                }
                break;
            }
        }
        Ok(())
    }
}

pub async fn run(dir: Option<PathBuf>, config: &Config) -> anyhow::Result<()> {
    let base = match &dir {
        Some(d) => d.clone(),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    let root = hive_workspace::discover(&base)
        .map(|r| r.main_root)
        .unwrap_or(base);
    let workspace_name = hive_workspace::default_name(&root);
    let dir = dir.unwrap_or_else(|| TaskStore::default_dir(&root));
    let store = TaskStore::open(dir)?;
    let roles = config.all_roles();
    let theme = Theme::from_config(&config.general.theme, &config.theme, &Env::current())
        .unwrap_or_else(|_| {
            Theme::from_config("hive-dark", &Default::default(), &Env::current()).unwrap()
        });

    let mut state = BoardState::new(workspace_name, root, store, roles)?;

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let res = run_loop(&mut terminal, &mut state, &theme).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    res
}

async fn fetch_sessions(paths: &Paths) -> Vec<SessionInfo> {
    let fut = async {
        let mut client = Client::connect_or_start(paths, "hive-board", false)
            .await
            .ok()?;
        client.send(&ClientMessage::ListSessions).await.ok()?;
        client
            .recv_until(|m| match m {
                ServerMessage::SessionList(s) => Some(s),
                _ => None,
            })
            .await
            .ok()
    };
    tokio::time::timeout(Duration::from_millis(250), fut)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut BoardState,
    theme: &Theme,
) -> anyhow::Result<()> {
    let paths = Paths::resolve();
    state.sessions = fetch_sessions(&paths).await;
    let mut last_sync = Instant::now();

    loop {
        if matches!(state.modal, Modal::None) && last_sync.elapsed() >= Duration::from_secs(1) {
            let _ = state.reload();
            state.sessions = fetch_sessions(&paths).await;
            state.tick_autopilot();
            last_sync = Instant::now();
        }
        terminal.draw(|f| draw(f, state, theme))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) => {
                if state.handle_key(key)? {
                    break;
                }
                if matches!(state.modal, Modal::None) {
                    state.sessions = fetch_sessions(&paths).await;
                    last_sync = Instant::now();
                }
            }
            Event::Mouse(mouse) => {
                state.handle_mouse(mouse)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn draw(f: &mut Frame<'_>, state: &mut BoardState, theme: &Theme) {
    state.modal_list_rect = None;
    let area = f.area();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top header bar
            Constraint::Min(8),    // 4 Kanban columns
            Constraint::Length(9), // Bottom split: Selected Task + Live Agents swarm tree
            Constraint::Length(1), // Bottom keybind / toast bar
        ])
        .split(area);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(rows[2]);

    draw_header(f, rows[0], state, theme);
    draw_columns(f, rows[1], state, theme);
    draw_inspector(f, bottom[0], state, theme);
    draw_live_agents(f, bottom[1], state, theme);
    draw_footer(f, rows[3], state, theme);

    match &state.modal {
        Modal::None => {}
        Modal::Form(form) => draw_form_modal(f, area, form, state, theme),
        Modal::Actions { task_id, selected } => {
            state.modal_list_rect = Some(draw_actions_modal(
                f, area, *task_id, *selected, state, theme,
            ));
        }
        Modal::RolePicker { task_id, selected } => {
            state.modal_list_rect =
                Some(draw_role_modal(f, area, *task_id, *selected, state, theme));
        }
        Modal::ConfirmDelete { task_id, title } => {
            draw_delete_modal(f, area, *task_id, title, theme);
        }
        Modal::Help => draw_help_modal(f, area, theme),
    }
}

fn draw_header(f: &mut Frame<'_>, area: Rect, state: &BoardState, theme: &Theme) {
    let bar = theme.bar();
    let total = state.tasks.len();
    let running = state
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();

    let left = Line::from(vec![
        Span::styled(
            format!(" {} HIVE BOARD ", theme.glyphs.mark),
            bar.fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", bar.fg(theme.muted)),
        Span::styled(
            state.workspace_name.clone(),
            bar.fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   {total} tasks · {running} in progress"),
            bar.fg(theme.muted),
        ),
    ]);
    let auto_badge = if state.autopilot {
        Span::styled(
            " ⚡ AUTO:ON ",
            bar.fg(theme.success).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" AUTO:OFF ", bar.fg(theme.muted))
    };
    let right = Line::from(vec![
        auto_badge,
        Span::styled(" │ ", bar.fg(theme.muted)),
        Span::styled("P", bar.fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" Auto  ", bar.fg(theme.text)),
        Span::styled("n", bar.fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" New  ", bar.fg(theme.text)),
        Span::styled("?", bar.fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" Help ", bar.fg(theme.text)),
    ]);

    f.render_widget(Paragraph::new(left).style(bar), area);
    f.render_widget(Paragraph::new(right).alignment(Alignment::Right), area);
}

fn draw_columns(f: &mut Frame<'_>, area: Rect, state: &mut BoardState, theme: &Theme) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

    for (idx, status) in COLUMNS.iter().enumerate() {
        state.col_rects[idx] = cols[idx];
        let tasks = state.col_tasks(idx);
        let active_col = idx == state.col;
        let col_accent = match status {
            TaskStatus::Ready => theme.accent,
            TaskStatus::InProgress => theme.warning,
            TaskStatus::InReview => Color::Cyan,
            TaskStatus::Done => theme.success,
            _ => theme.muted,
        };
        let border_style = if active_col {
            Style::new().fg(theme.accent)
        } else {
            Style::new().fg(theme.muted)
        };
        let title_style = if active_col {
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(col_accent)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(theme.glyphs.border)
            .border_style(border_style)
            .title(Span::styled(
                format!(" {} ({}) ", status.label(), tasks.len()),
                title_style,
            ));
        let inner = block.inner(cols[idx]);
        f.render_widget(block, cols[idx]);

        if tasks.is_empty() {
            let hint = if idx == 0 {
                "Empty — press [n]\nto add a task"
            } else {
                "Empty"
            };
            f.render_widget(
                Paragraph::new(hint)
                    .style(Style::new().fg(theme.muted))
                    .alignment(Alignment::Center),
                Rect {
                    x: inner.x,
                    y: inner.y + inner.height / 3,
                    width: inner.width,
                    height: 2.min(inner.height),
                },
            );
            continue;
        }

        let max_w = inner.width as usize;
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (r, t) in tasks.iter().enumerate() {
            let selected = active_col && r == state.row;
            let row_bg = if selected {
                theme.cursor_row()
            } else {
                Style::new()
            };
            let prefix = if selected { theme.glyphs.current } else { " " };
            let title_text = fit(&format!("#{} {}", t.id, t.title), max_w.saturating_sub(3));
            let title_style = if selected {
                row_bg.fg(theme.accent).add_modifier(Modifier::BOLD)
            } else {
                row_bg.fg(theme.text)
            };

            let role_name = t.role.as_deref().unwrap_or("auto");
            let agent_tag = t
                .role
                .as_deref()
                .and_then(|rn| state.roles.get(rn))
                .map(|rc| format!("{role_name}·{}", rc.agent))
                .unwrap_or_else(|| role_name.to_string());
            let (sess_tag, meta_color) = match state.session_for_task(t) {
                Some(s) => {
                    let (glyph, label, col) = live_session_badge(s, theme);
                    let subs = state.child_count(s.id);
                    let sub_txt = if subs > 0 {
                        format!(" +{subs}sub")
                    } else {
                        String::new()
                    };
                    (format!(" {glyph} {label} s{}{sub_txt}", s.id), col)
                }
                None => {
                    let col = if t.role.is_some() {
                        theme.accent_dim
                    } else {
                        theme.muted
                    };
                    (String::new(), col)
                }
            };
            let meta = fit(&format!("[{agent_tag}]{sess_tag}"), max_w.saturating_sub(3));

            // Pad lines to full width so the cursor_row background spans the whole card cleanly.
            lines.push(
                Line::from(vec![
                    Span::styled(format!("{prefix} "), row_bg.fg(theme.accent)),
                    Span::styled(pad(&title_text, max_w.saturating_sub(2)), title_style),
                ])
                .style(row_bg),
            );
            lines.push(
                Line::from(vec![
                    Span::styled(format!("{prefix} "), row_bg.fg(theme.accent)),
                    Span::styled(pad(&meta, max_w.saturating_sub(2)), row_bg.fg(meta_color)),
                ])
                .style(row_bg),
            );
            lines.push(Line::from(""));
        }
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn draw_inspector(f: &mut Frame<'_>, area: Rect, state: &BoardState, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.muted))
        .title(Span::styled(
            " Selected Task ",
            Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(t) = state.selected_task() else {
        let lines = vec![
            Line::from(Span::styled(
                "No task selected in this column.",
                Style::new().fg(theme.muted),
            )),
            Line::from(vec![
                Span::styled("Press ", Style::new().fg(theme.muted)),
                Span::styled(
                    "n",
                    Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to create a task and launch an agent, or ",
                    Style::new().fg(theme.muted),
                ),
                Span::styled(
                    "h/l",
                    Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " (or click) to switch columns.",
                    Style::new().fg(theme.muted),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    };

    let role_desc = match t.role.as_deref() {
        Some(rn) => match state.roles.get(rn) {
            Some(rc) => format!(
                "{rn} ({}{})",
                rc.agent,
                if rc.worktree { ", worktree" } else { "" }
            ),
            None => rn.to_string(),
        },
        None => "auto (picks best role on run)".to_string(),
    };
    let (sess_desc, sess_color) = match state.session_for_task(t) {
        Some(s) => {
            let (glyph, label, col) = live_session_badge(s, theme);
            let note = s
                .presence
                .as_ref()
                .and_then(|p| p.message.as_deref())
                .map(|m| format!(" — {m}"))
                .unwrap_or_default();
            (format!("{glyph} s{} ({label}){note}", s.id), col)
        }
        None => ("not running".into(), theme.muted),
    };
    let branch_desc = t
        .branch
        .as_deref()
        .map(|b| format!(" · branch {b}"))
        .unwrap_or_default();

    let desc_line = if !t.body.trim().is_empty() {
        t.body.lines().next().unwrap_or("").to_string()
    } else if let Some(last_log) = t.log.last() {
        format!("Log: {last_log}")
    } else {
        "No prompt/description yet — press [e] to add instructions.".into()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("#{} {} ", t.id, t.title),
                Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}]", t.status.label()),
                Style::new().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("Role: ", Style::new().fg(theme.muted)),
            Span::styled(role_desc, Style::new().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Agent: ", Style::new().fg(theme.muted)),
            Span::styled(
                format!("{sess_desc}{branch_desc}"),
                Style::new().fg(sess_color),
            ),
        ]),
        Line::from(Span::styled(desc_line, Style::new().fg(theme.muted))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "⏎",
                Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Actions  ", Style::new().fg(theme.text)),
            Span::styled(
                "s",
                Style::new().fg(theme.success).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Run  ", Style::new().fg(theme.text)),
            Span::styled(
                "x",
                Style::new().fg(theme.error).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Stop  ", Style::new().fg(theme.text)),
            Span::styled(
                "a",
                Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Role  ", Style::new().fg(theme.text)),
            Span::styled(
                "e",
                Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Edit  ", Style::new().fg(theme.text)),
            Span::styled(
                "d",
                Style::new().fg(theme.error).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Del", Style::new().fg(theme.text)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn live_session_badge(s: &SessionInfo, theme: &Theme) -> (&'static str, &'static str, Color) {
    if !s.state.is_alive() {
        return (theme.glyphs.stopped, "exited", theme.muted);
    }
    match s.presence.as_ref().map(|p| p.status) {
        Some(AgentStatus::Working) => (theme.glyphs.working, "working", theme.accent),
        Some(AgentStatus::Blocked) => (theme.glyphs.blocked, "blocked", theme.warning),
        Some(AgentStatus::Idle) => (theme.glyphs.running, "idle", theme.success),
        _ => (theme.glyphs.running, "running", theme.success),
    }
}

fn draw_live_agents(f: &mut Frame<'_>, area: Rect, state: &BoardState, theme: &Theme) {
    let active: Vec<&SessionInfo> = state
        .sessions
        .iter()
        .filter(|s| {
            s.state.is_alive()
                && (s.role.is_some() || s.task_id.is_some() || s.parent_session.is_some())
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(if active.is_empty() {
            theme.muted
        } else {
            theme.success
        }))
        .title(Span::styled(
            format!(" Live Agents ({}) ", active.len()),
            Style::new()
                .fg(if active.is_empty() {
                    theme.muted
                } else {
                    theme.success
                })
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if active.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "No orchestrated agents running.",
                    Style::new().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "Press [s] on a task to spawn one.",
                    Style::new().fg(theme.muted),
                )),
            ]),
            inner,
        );
        return;
    }

    let max_w = inner.width as usize;
    let active_ids: Vec<SessionId> = active.iter().map(|s| s.id).collect();
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Render top-level agents first, with their spawned child subagents nested underneath.
    for s in active
        .iter()
        .filter(|s| s.parent_session.is_none_or(|p| !active_ids.contains(&p)))
    {
        lines.push(format_agent_line(s, false, max_w, theme));
        for child in active.iter().filter(|c| c.parent_session == Some(s.id)) {
            lines.push(format_agent_line(child, true, max_w, theme));
        }
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn format_agent_line<'a>(s: &SessionInfo, is_child: bool, max_w: usize, theme: &Theme) -> Line<'a> {
    let (glyph, status_label, col) = live_session_badge(s, theme);
    let indent = if is_child { "  └ " } else { "" };
    let role = s.role.as_deref().unwrap_or(&s.agent);
    let task_part = s
        .task_id
        .as_deref()
        .map(|tid| format!(" #{tid}"))
        .unwrap_or_default();
    let text = fit(
        &format!(
            "{indent}{glyph} s{} {role}{task_part} ({}) · {status_label}",
            s.id, s.agent
        ),
        max_w,
    );
    Line::from(Span::styled(text, Style::new().fg(col)))
}

fn draw_footer(f: &mut Frame<'_>, area: Rect, state: &BoardState, theme: &Theme) {
    let bar = theme.bar();
    if let Some((tone, msg)) = &state.toast {
        let fg = match tone {
            Tone::Info => theme.accent,
            Tone::Ok => theme.success,
            Tone::Err => theme.error,
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {msg}"),
                bar.fg(fg).add_modifier(Modifier::BOLD),
            )))
            .style(bar),
            area,
        );
        return;
    }

    let hints = [
        ("h/l", "column"),
        ("j/k", "card"),
        ("n", "new"),
        ("⏎", "actions"),
        ("s", "run agent"),
        ("a", "role"),
        ("[/]", "move"),
        ("q", "quit"),
    ];
    let mut spans = vec![Span::styled(" ", bar)];
    for (i, (k, v)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", bar));
        }
        spans.push(Span::styled(
            *k,
            bar.fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {v}"), bar.fg(theme.muted)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(bar), area);
}

fn draw_form_modal(
    f: &mut Frame<'_>,
    area: Rect,
    form: &TaskForm,
    state: &BoardState,
    theme: &Theme,
) {
    let popup = centered_rect(area, 66, 13);
    f.render_widget(Clear, popup);
    let title = if let Some(id) = form.editing_id {
        format!(" Edit Task #{id} ")
    } else {
        " New Task ".into()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.accent))
        .style(Style::new().bg(theme.surface).fg(theme.text))
        .title(Span::styled(
            title,
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let opts = state.role_options();
    let role_label = match opts.get(form.role_idx).and_then(|o| o.as_deref()) {
        None => "★ auto (pick best model for this task)".to_string(),
        Some(rn) => match state.roles.get(rn) {
            Some(rc) => format!(
                "{rn} ({}{})",
                rc.agent,
                if rc.worktree { ", worktree" } else { "" }
            ),
            None => rn.to_string(),
        },
    };

    let caret = theme.glyphs.caret;
    let field_style = |active: bool| {
        if active {
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.muted)
        }
    };

    let title_val = if form.field == FormField::Title {
        format!("{}{caret}", form.title)
    } else {
        form.title.clone()
    };
    let body_val = if form.field == FormField::Body {
        format!("{}{caret}", form.body)
    } else if form.body.is_empty() {
        "(optional instructions sent to the agent)".into()
    } else {
        form.body.clone()
    };

    let lines = vec![
        Line::from(Span::styled(
            "Title:",
            field_style(form.field == FormField::Title),
        )),
        Line::from(Span::styled(
            format!("  {title_val}"),
            Style::new().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Prompt / Details:",
            field_style(form.field == FormField::Body),
        )),
        Line::from(Span::styled(
            format!("  {body_val}"),
            Style::new().fg(if form.body.is_empty() && form.field != FormField::Body {
                theme.muted
            } else {
                theme.text
            }),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Role / Model ", field_style(form.field == FormField::Role)),
            Span::styled("(←/→ to change):", Style::new().fg(theme.muted)),
        ]),
        Line::from(Span::styled(
            format!("  ◀ {role_label} ▶"),
            if form.field == FormField::Role {
                theme.highlight()
            } else {
                Style::new().fg(theme.text)
            },
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Tab",
                Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" field   ", Style::new().fg(theme.muted)),
            Span::styled(
                "⏎",
                Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save   ", Style::new().fg(theme.text)),
            Span::styled(
                "Ctrl+S",
                Style::new().fg(theme.success).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save & Run Agent   ", Style::new().fg(theme.text)),
            Span::styled("Esc", Style::new().fg(theme.muted)),
            Span::styled(" cancel", Style::new().fg(theme.muted)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_actions_modal(
    f: &mut Frame<'_>,
    area: Rect,
    task_id: u32,
    selected: usize,
    state: &BoardState,
    theme: &Theme,
) -> Rect {
    let popup = centered_rect(area, 48, (CardAction::ALL.len() + 4) as u16);
    f.render_widget(Clear, popup);
    let task_title = state
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .map(|t| fit(&t.title, 28))
        .unwrap_or_default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.accent))
        .style(Style::new().bg(theme.surface).fg(theme.text))
        .title(Span::styled(
            format!(" #{task_id} {task_title} "),
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let w = inner.width as usize;
    let lines: Vec<Line<'_>> = CardAction::ALL
        .iter()
        .enumerate()
        .map(|(i, act)| {
            let (icon, label) = act.icon_and_label();
            let text = pad(&format!(" {icon}  {label}"), w);
            let style = if i == selected {
                theme.highlight()
            } else if *act == CardAction::Delete {
                Style::new().fg(theme.error)
            } else if *act == CardAction::Spawn {
                Style::new().fg(theme.success)
            } else {
                Style::new().fg(theme.text)
            };
            Line::from(Span::styled(text, style))
        })
        .collect();
    let list_rect = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: CardAction::ALL.len() as u16,
    };
    f.render_widget(Paragraph::new(lines), list_rect);
    list_rect
}

fn draw_role_modal(
    f: &mut Frame<'_>,
    area: Rect,
    task_id: u32,
    selected: usize,
    state: &BoardState,
    theme: &Theme,
) -> Rect {
    let opts = state.role_options();
    let popup = centered_rect(area, 64, (opts.len() + 4).min(18) as u16);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.accent))
        .style(Style::new().bg(theme.surface).fg(theme.text))
        .title(Span::styled(
            format!(" Assign Role / Model for #{task_id} "),
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let w = inner.width as usize;
    let lines: Vec<Line<'_>> = opts
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let text = match opt {
                None => " ★ auto — automatically pick best role for this task".to_string(),
                Some(name) => match state.roles.get(name) {
                    Some(rc) => {
                        let wt = if rc.worktree { "worktree" } else { "shared" };
                        let s = if rc.strengths.is_empty() {
                            String::new()
                        } else {
                            format!(" · {}", rc.strengths.join(", "))
                        };
                        format!(" ● {name:<10} ({}) · {wt}{s}", rc.agent)
                    }
                    None => format!(" ● {name}"),
                },
            };
            let style = if i == selected {
                theme.highlight()
            } else {
                Style::new().fg(theme.text)
            };
            Line::from(Span::styled(pad(&fit(&text, w), w), style))
        })
        .collect();

    let list_rect = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: opts.len().min(inner.height as usize) as u16,
    };
    f.render_widget(Paragraph::new(lines), list_rect);
    list_rect
}

fn draw_delete_modal(f: &mut Frame<'_>, area: Rect, task_id: u32, title: &str, theme: &Theme) {
    let popup = centered_rect(area, 52, 6);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.error))
        .style(Style::new().bg(theme.surface).fg(theme.text))
        .title(Span::styled(
            " Delete Task ",
            Style::new().fg(theme.error).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::from(format!("Delete #{task_id} \"{}\"?", fit(title, 34))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "y / ⏎",
                Style::new().fg(theme.error).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Delete    ", Style::new().fg(theme.text)),
            Span::styled("n / Esc", Style::new().fg(theme.muted)),
            Span::styled(" Cancel", Style::new().fg(theme.muted)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn draw_help_modal(f: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let popup = centered_rect(area, 56, 15);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.accent))
        .style(Style::new().bg(theme.surface).fg(theme.text))
        .title(Span::styled(
            " Hive Board Shortcuts ",
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = [
        ("n", "Create new task (Ctrl+S to create & run immediately)"),
        ("Enter / click", "Open Action Menu on selected task"),
        ("s", "Spawn / run assigned (or auto) agent on task"),
        ("a", "Assign role / model to task"),
        ("e", "Edit task title, prompt, and role"),
        ("[ / ]", "Move task backward / forward a column"),
        ("d", "Delete selected task"),
        ("h / l, j / k", "Move between columns and cards"),
        ("r", "Reload tasks from disk"),
        ("q / Esc", "Quit board"),
    ];
    let lines: Vec<Line<'_>> = rows
        .iter()
        .map(|(k, desc)| {
            Line::from(vec![
                Span::styled(
                    format!(" {k:<15}"),
                    Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, Style::new().fg(theme.text)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(area: Rect, w: u16, h: u16) -> Rect {
    let width = w.min(area.width.saturating_sub(2)).max(1);
    let height = h.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn fit(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max <= 1 {
        s.chars().take(max).collect()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_navigation_and_status_transition() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TaskStore::open(tmp.path()).unwrap();
        store
            .create(NewTask {
                title: "Build board".into(),
                role: Some("coder".into()),
                ..Default::default()
            })
            .unwrap();
        let roles = Config::default().all_roles();
        let mut state =
            BoardState::new("demo".into(), tmp.path().to_path_buf(), store, roles).unwrap();
        assert_eq!(state.col, 0);
        assert_eq!(state.selected_id(), Some(1));

        // Assign role via picker index 0 -> auto (None)
        state.apply_role(1, 0).unwrap();
        assert_eq!(state.selected_task().unwrap().role, None);

        // Move right -> cursor follows task to column 1
        state.move_task_by(1, 1).unwrap();
        assert_eq!(state.col, 1);
        assert_eq!(state.selected_id(), Some(1));

        // Move left -> cursor follows task back to column 0
        state.move_task_by(1, -1).unwrap();
        assert_eq!(state.col, 0);
        assert_eq!(state.selected_id(), Some(1));
    }
}
