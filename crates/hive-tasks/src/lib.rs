//! Plain-markdown task board store for Hive multi-agent orchestration.
//!
//! Tasks are stored as `<NNN>-<slug>.md` files with YAML frontmatter in a
//! workspace directory (default `<workspace>/.hive/tasks/`). Writes are atomic
//! (temp file + rename) so concurrent agents never corrupt task files.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("task #{0} not found")]
    NotFound(u32),
    #[error(
        "invalid status '{0}' (expected backlog, ready, in_progress, in_review, blocked, or done)"
    )]
    InvalidStatus(String),
    #[error("circular dependency detected involving task #{0}")]
    CircularDependency(u32),
    #[error("parse error in {path}: {reason}")]
    Parse { path: PathBuf, reason: String },
}

pub type Result<T> = std::result::Result<T, TaskError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Backlog,
    Ready,
    InProgress,
    InReview,
    Blocked,
    Done,
}

impl TaskStatus {
    pub const ALL: [TaskStatus; 6] = [
        TaskStatus::Backlog,
        TaskStatus::Ready,
        TaskStatus::InProgress,
        TaskStatus::InReview,
        TaskStatus::Blocked,
        TaskStatus::Done,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Backlog => "backlog",
            TaskStatus::Ready => "ready",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::InReview => "in_review",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Done => "done",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TaskStatus::Backlog => "BACKLOG",
            TaskStatus::Ready => "READY",
            TaskStatus::InProgress => "IN PROGRESS",
            TaskStatus::InReview => "IN REVIEW",
            TaskStatus::Blocked => "BLOCKED",
            TaskStatus::Done => "DONE",
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = TaskError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "backlog" => Ok(TaskStatus::Backlog),
            "ready" => Ok(TaskStatus::Ready),
            "in_progress" | "in-progress" | "active" => Ok(TaskStatus::InProgress),
            "in_review" | "in-review" | "review" => Ok(TaskStatus::InReview),
            "blocked" => Ok(TaskStatus::Blocked),
            "done" | "completed" => Ok(TaskStatus::Done),
            other => Err(TaskError::InvalidStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub status: TaskStatus,
    pub role: Option<String>,
    pub session_id: Option<String>,
    pub parent_session: Option<String>,
    pub branch: Option<String>,
    pub depends_on: Vec<u32>,
    pub body: String,
    pub log: Vec<String>,
    pub created: u64,
    pub updated: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NewTask {
    pub title: String,
    pub status: Option<TaskStatus>,
    pub role: Option<String>,
    pub parent_session: Option<String>,
    pub depends_on: Vec<u32>,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub status: Option<TaskStatus>,
    pub role: Option<Option<String>>,
    pub session_id: Option<Option<String>>,
    pub branch: Option<Option<String>>,
    pub depends_on: Option<Vec<u32>>,
    pub note: Option<String>,
    pub note_author: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskStore {
    dir: PathBuf,
}

impl TaskStore {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Default task directory for a workspace root: `<root>/.hive/tasks`.
    pub fn default_dir(workspace_root: &Path) -> PathBuf {
        workspace_root.join(".hive").join("tasks")
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn list(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        if !self.dir.exists() {
            return Ok(tasks);
        }
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            tasks.push(parse_task(&path, &text)?);
        }
        tasks.sort_by_key(|t| t.id);
        Ok(tasks)
    }

    pub fn get(&self, id: u32) -> Result<Task> {
        self.list()?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or(TaskError::NotFound(id))
    }

    pub fn create(&self, new: NewTask) -> Result<Task> {
        let existing = self.list()?;
        let next_id = existing.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let now = unix_now();
        let status = new.status.unwrap_or(if new.depends_on.is_empty() {
            TaskStatus::Ready
        } else {
            TaskStatus::Backlog
        });
        let task = Task {
            id: next_id,
            title: new.title.trim().to_string(),
            status,
            role: new.role,
            session_id: None,
            parent_session: new.parent_session,
            branch: None,
            depends_on: new.depends_on,
            body: new.body,
            log: Vec::new(),
            created: now,
            updated: now,
        };
        check_no_cycle(&existing, &task)?;
        self.write_task(&task, None)?;
        Ok(task)
    }

    pub fn update(&self, id: u32, patch: TaskPatch) -> Result<Task> {
        let existing = self.list()?;
        let old_path = self.find_path(id)?;
        let mut task = existing
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or(TaskError::NotFound(id))?;

        if let Some(title) = patch.title {
            task.title = title.trim().to_string();
        }
        if let Some(body) = patch.body {
            task.body = body.trim().to_string();
        }
        if let Some(status) = patch.status {
            task.status = status;
        }
        if let Some(role) = patch.role {
            task.role = role;
        }
        if let Some(session_id) = patch.session_id {
            task.session_id = session_id;
        }
        if let Some(branch) = patch.branch {
            task.branch = branch;
        }
        if let Some(deps) = patch.depends_on {
            task.depends_on = deps;
        }
        let now = unix_now();
        if let Some(note) = patch.note {
            let author = patch.note_author.as_deref().unwrap_or("hive");
            task.log.push(format!("{now} [{author}]: {}", note.trim()));
        }
        task.updated = now;

        check_no_cycle(&existing, &task)?;
        self.write_task(&task, Some(&old_path))?;
        Ok(task)
    }

    pub fn delete(&self, id: u32) -> Result<()> {
        let path = self.find_path(id)?;
        fs::remove_file(path)?;
        Ok(())
    }

    /// Returns tasks in `Ready` or `Backlog` whose `depends_on` are all `Done`.
    pub fn ready_tasks(&self) -> Result<Vec<Task>> {
        let all = self.list()?;
        let done: BTreeSet<u32> = all
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
            .map(|t| t.id)
            .collect();
        Ok(all
            .into_iter()
            .filter(|t| {
                matches!(t.status, TaskStatus::Ready | TaskStatus::Backlog)
                    && t.depends_on.iter().all(|dep| done.contains(dep))
            })
            .collect())
    }

    fn find_path(&self, id: u32) -> Result<PathBuf> {
        let prefix = format!("{id:03}-");
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix) && name.ends_with(".md") {
                return Ok(entry.path());
            }
        }
        Err(TaskError::NotFound(id))
    }

    fn write_task(&self, task: &Task, old_path: Option<&Path>) -> Result<()> {
        let filename = format!("{:03}-{}.md", task.id, slugify(&task.title));
        let dest = self.dir.join(filename);
        let tmp = self.dir.join(format!(
            ".tmp-{}-{}-{}.md",
            task.id,
            std::process::id(),
            unix_nanos()
        ));
        let content = render_task(task);
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &dest)?;
        if let Some(old) = old_path
            && old != dest
            && old.exists()
        {
            let _ = fs::remove_file(old);
        }
        Ok(())
    }
}

fn check_no_cycle(existing: &[Task], candidate: &Task) -> Result<()> {
    let mut deps: BTreeMap<u32, Vec<u32>> = existing
        .iter()
        .map(|t| (t.id, t.depends_on.clone()))
        .collect();
    deps.insert(candidate.id, candidate.depends_on.clone());

    let mut visited = BTreeSet::new();
    let mut stack = candidate.depends_on.clone();
    while let Some(cur) = stack.pop() {
        if cur == candidate.id {
            return Err(TaskError::CircularDependency(candidate.id));
        }
        if visited.insert(cur)
            && let Some(next) = deps.get(&cur)
        {
            stack.extend(next.iter().copied());
        }
    }
    Ok(())
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn render_task(t: &Task) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("id: {}\n", t.id));
    out.push_str(&format!("title: {}\n", t.title));
    out.push_str(&format!("status: {}\n", t.status));
    if let Some(ref r) = t.role {
        out.push_str(&format!("role: {r}\n"));
    }
    if let Some(ref s) = t.session_id {
        out.push_str(&format!("session_id: {s}\n"));
    }
    if let Some(ref p) = t.parent_session {
        out.push_str(&format!("parent_session: {p}\n"));
    }
    if let Some(ref b) = t.branch {
        out.push_str(&format!("branch: {b}\n"));
    }
    let deps = t
        .depends_on
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("depends_on: [{deps}]\n"));
    out.push_str(&format!("created: {}\n", t.created));
    out.push_str(&format!("updated: {}\n", t.updated));
    out.push_str("---\n\n");
    if !t.body.trim().is_empty() {
        out.push_str(t.body.trim());
        out.push_str("\n\n");
    }
    if !t.log.is_empty() {
        out.push_str("## Log\n");
        for entry in &t.log {
            out.push_str(&format!("- {entry}\n"));
        }
    }
    out
}

fn parse_task(path: &Path, text: &str) -> Result<Task> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .ok_or_else(|| TaskError::Parse {
            path: path.to_path_buf(),
            reason: "missing YAML frontmatter".into(),
        })?;
    let (fm, after) = rest.split_once("\n---").ok_or_else(|| TaskError::Parse {
        path: path.to_path_buf(),
        reason: "unterminated YAML frontmatter".into(),
    })?;

    let mut id: Option<u32> = None;
    let mut title = String::new();
    let mut status = TaskStatus::Backlog;
    let mut role = None;
    let mut session_id = None;
    let mut parent_session = None;
    let mut branch = None;
    let mut depends_on = Vec::new();
    let mut created = 0u64;
    let mut updated = 0u64;

    for line in fm.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        match k {
            "id" => id = v.parse().ok(),
            "title" => title = v.to_string(),
            "status" => status = v.parse()?,
            "role" if !v.is_empty() => role = Some(v.to_string()),
            "session_id" if !v.is_empty() => session_id = Some(v.to_string()),
            "parent_session" if !v.is_empty() => parent_session = Some(v.to_string()),
            "branch" if !v.is_empty() => branch = Some(v.to_string()),
            "depends_on" => {
                let inner = v.trim_start_matches('[').trim_end_matches(']');
                depends_on = inner
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u32>().ok())
                    .collect();
            }
            "created" => created = v.parse().unwrap_or(0),
            "updated" => updated = v.parse().unwrap_or(0),
            _ => {}
        }
    }

    let id = id.ok_or_else(|| TaskError::Parse {
        path: path.to_path_buf(),
        reason: "missing id".into(),
    })?;

    let after = after.trim_start_matches(['\r', '\n']);
    let (body, log) = if let Some((b, l)) = after.split_once("## Log\n") {
        let entries = l
            .lines()
            .filter_map(|line| line.strip_prefix("- ").map(str::to_string))
            .collect();
        (b.trim().to_string(), entries)
    } else {
        (after.trim().to_string(), Vec::new())
    };

    Ok(Task {
        id,
        title,
        status,
        role,
        session_id,
        parent_session,
        branch,
        depends_on,
        body,
        log,
        created,
        updated,
    })
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_reload_markdown_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TaskStore::open(tmp.path()).unwrap();
        let t = store
            .create(NewTask {
                title: "Implement OAuth refresh".into(),
                role: Some("coder".into()),
                body: "Handle expired tokens cleanly.".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(t.id, 1);
        assert_eq!(t.status, TaskStatus::Ready);

        let updated = store
            .update(
                1,
                TaskPatch {
                    status: Some(TaskStatus::InProgress),
                    session_id: Some(Some("42".into())),
                    note: Some("Started implementation".into()),
                    note_author: Some("coder".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert_eq!(updated.session_id.as_deref(), Some("42"));
        assert_eq!(updated.log.len(), 1);

        let reloaded = store.get(1).unwrap();
        assert_eq!(reloaded, updated);
    }

    #[test]
    fn ready_tasks_respects_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TaskStore::open(tmp.path()).unwrap();
        let t1 = store
            .create(NewTask {
                title: "Plan".into(),
                ..Default::default()
            })
            .unwrap();
        let t2 = store
            .create(NewTask {
                title: "Code".into(),
                depends_on: vec![t1.id],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(t2.status, TaskStatus::Backlog);

        let ready = store.ready_tasks().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, t1.id);

        store
            .update(
                t1.id,
                TaskPatch {
                    status: Some(TaskStatus::Done),
                    ..Default::default()
                },
            )
            .unwrap();
        let ready = store.ready_tasks().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, t2.id);
    }

    #[test]
    fn rejects_circular_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TaskStore::open(tmp.path()).unwrap();
        let t1 = store
            .create(NewTask {
                title: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let t2 = store
            .create(NewTask {
                title: "B".into(),
                depends_on: vec![t1.id],
                ..Default::default()
            })
            .unwrap();
        let err = store
            .update(
                t1.id,
                TaskPatch {
                    depends_on: Some(vec![t2.id]),
                    ..Default::default()
                },
            )
            .unwrap_err();
        assert!(matches!(err, TaskError::CircularDependency(1)));
    }

    #[test]
    fn atomic_concurrent_updates_do_not_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TaskStore::open(tmp.path()).unwrap();
        store
            .create(NewTask {
                title: "Concurrent".into(),
                ..Default::default()
            })
            .unwrap();
        std::thread::scope(|s| {
            for i in 0..8 {
                let st = store.clone();
                s.spawn(move || {
                    let _ = st.update(
                        1,
                        TaskPatch {
                            note: Some(format!("note {i}")),
                            ..Default::default()
                        },
                    );
                });
            }
        });
        let loaded = store.get(1).unwrap();
        assert_eq!(loaded.id, 1);
        assert_eq!(loaded.title, "Concurrent");
    }
}
