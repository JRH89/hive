//! `hive task`, `hive spawn`, `hive msg`, and `hive wait` — multi-agent
//! orchestration CLI primitives.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use clap::Subcommand;
use hive_core::client::Client;
use hive_core::config::Config;
use hive_core::launch;
use hive_protocol::{
    AgentStatus, ClientMessage, NewWorktree, ServerMessage, SessionId, SessionInfo, TermSize,
};
use hive_tasks::{NewTask, TaskPatch, TaskStatus, TaskStore};

#[derive(Subcommand)]
pub(crate) enum TaskAction {
    /// Add a new task to the workspace board
    Add {
        /// Short title for the task
        title: String,
        /// Role assigned to this task (e.g. coder, reviewer)
        #[arg(short, long)]
        role: Option<String>,
        /// Task ids that must be done before this task is ready
        #[arg(long = "depends-on")]
        depends_on: Vec<u32>,
        /// Optional description body
        #[arg(short, long, default_value = "")]
        body: String,
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
    /// List tasks on the workspace board
    #[command(alias = "ls")]
    List {
        /// Filter by status (backlog, ready, in_progress, in_review, blocked, done)
        #[arg(short, long)]
        status: Option<String>,
        /// Filter by role
        #[arg(short, long)]
        role: Option<String>,
        /// Show only ready tasks with all dependencies satisfied
        #[arg(long)]
        ready: bool,
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
    /// Show full details of a task
    Show {
        id: u32,
        #[arg(long)]
        json: bool,
    },
    /// Update a task's status, role, linked session, or append a log note
    Update {
        id: u32,
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        role: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        branch: Option<String>,
        #[arg(short, long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn open_store(dir: Option<PathBuf>) -> anyhow::Result<TaskStore> {
    let dir = match dir {
        Some(d) => d,
        None => {
            let cwd = std::env::current_dir().context("cannot determine current directory")?;
            let root = hive_workspace::discover(&cwd)
                .map(|r| r.main_root)
                .unwrap_or(cwd);
            TaskStore::default_dir(&root)
        }
    };
    Ok(TaskStore::open(dir)?)
}

pub(crate) fn run_task(action: TaskAction, dir: Option<PathBuf>) -> anyhow::Result<()> {
    let store = open_store(dir)?;
    match action {
        TaskAction::Add {
            title,
            role,
            depends_on,
            body,
            json,
        } => {
            let parent_session = std::env::var("HIVE_SESSION_ID").ok();
            let task = store.create(NewTask {
                title,
                status: None,
                role,
                parent_session,
                depends_on,
                body,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "created task #{}: {} [{}]",
                    task.id, task.title, task.status
                );
            }
        }
        TaskAction::List {
            status,
            role,
            ready,
            json,
        } => {
            let mut tasks = if ready {
                store.ready_tasks()?
            } else {
                store.list()?
            };
            if let Some(s) = status {
                let want: TaskStatus = s.parse()?;
                tasks.retain(|t| t.status == want);
            }
            if let Some(r) = role {
                tasks.retain(|t| t.role.as_deref() == Some(r.as_str()));
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else if tasks.is_empty() {
                println!("no tasks");
            } else {
                for t in &tasks {
                    let role = t.role.as_deref().unwrap_or("-");
                    let sess = t.session_id.as_deref().unwrap_or("-");
                    println!(
                        "#{:<3} {:<11} role:{:<10} session:{:<4} {}",
                        t.id, t.status, role, sess, t.title
                    );
                }
            }
        }
        TaskAction::Show { id, json } => {
            let task = store.get(id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!("#{} — {} [{}]", task.id, task.title, task.status);
                if let Some(ref r) = task.role {
                    println!("role: {r}");
                }
                if let Some(ref s) = task.session_id {
                    println!("session: {s}");
                }
                if let Some(ref b) = task.branch {
                    println!("branch: {b}");
                }
                if !task.depends_on.is_empty() {
                    println!("depends_on: {:?}", task.depends_on);
                }
                if !task.body.is_empty() {
                    println!("\n{}", task.body);
                }
                if !task.log.is_empty() {
                    println!("\nLog:");
                    for l in &task.log {
                        println!("  - {l}");
                    }
                }
            }
        }
        TaskAction::Update {
            id,
            status,
            role,
            session,
            branch,
            note,
            json,
        } => {
            let parsed_status = match status {
                Some(s) => Some(s.parse()?),
                None => None,
            };
            let author = std::env::var("HIVE_ROLE")
                .or_else(|_| std::env::var("HIVE_SESSION_ID"))
                .ok();
            let task = store.update(
                id,
                TaskPatch {
                    title: None,
                    body: None,
                    status: parsed_status,
                    role: role.map(Some),
                    session_id: session.map(Some),
                    branch: branch.map(Some),
                    depends_on: None,
                    note,
                    note_author: author,
                },
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "updated task #{}: {} [{}]",
                    task.id, task.title, task.status
                );
            }
        }
    }
    Ok(())
}

pub(crate) struct SpawnArgs {
    pub role: String,
    pub task: Option<u32>,
    pub worktree: bool,
    pub branch: Option<String>,
    pub cwd: Option<PathBuf>,
    pub prompt: Option<String>,
    pub size: TermSize,
    pub force: bool,
}

pub(crate) async fn run_spawn(
    client: &mut Client,
    config: &Config,
    args: SpawnArgs,
    tasks_dir: Option<PathBuf>,
) -> anyhow::Result<SessionInfo> {
    // Guard against accidentally spawning duplicate agents on the same active task.
    if let Some(tid) = args.task
        && !args.force
    {
        let tid_str = tid.to_string();
        client.send(&ClientMessage::ListSessions).await?;
        let existing = client
            .recv_until(|m| match m {
                ServerMessage::SessionList(s) => Some(s),
                _ => None,
            })
            .await?;
        if let Some(alive) = existing
            .into_iter()
            .find(|s| s.state.is_alive() && s.task_id.as_deref() == Some(tid_str.as_str()))
        {
            bail!(
                "task #{tid} already has a running session (s{}, role={}) — pass --force to spawn another",
                alive.id,
                alive.role.as_deref().unwrap_or(&alive.agent)
            );
        }
    }

    let (resolved_role, role_cfg) = if args.role == "auto" {
        let mut context_text = args.prompt.clone().unwrap_or_default();
        if let Some(tid) = args.task
            && let Ok(store) = open_store(tasks_dir.clone())
            && let Ok(t) = store.get(tid)
        {
            context_text = format!("{} {} {}", t.title, t.body, context_text);
        }
        config.pick_role(&context_text)
    } else {
        let Some(cfg) = config.role(&args.role) else {
            let known: Vec<String> = config.all_roles().keys().cloned().collect();
            bail!(
                "unknown role `{}` (available roles: {}, or `auto`)",
                args.role,
                known.join(", ")
            );
        };
        (args.role.clone(), cfg)
    };

    let mut label = resolved_role.clone();
    if let Some(tid) = args.task {
        label = format!("{resolved_role} #{tid}");
    }

    let is_orchestrator = resolved_role == "lead"
        || role_cfg
            .strengths
            .iter()
            .any(|s| s.eq_ignore_ascii_case("orchestration"));

    let orchestration_brief = if is_orchestrator {
        let mut lines = vec![
            "You are running inside Hive as the lead orchestrator.".to_string(),
            "Available specialist roles you can delegate to:".to_string(),
        ];
        for (name, r) in config.all_roles() {
            if name == resolved_role {
                continue;
            }
            let strengths = if r.strengths.is_empty() {
                "general".into()
            } else {
                r.strengths.join(", ")
            };
            lines.push(format!(
                "- {name} (agent: {}, worktree: {}, strengths: {strengths})",
                r.agent, r.worktree
            ));
        }
        lines.push("Use Hive CLI commands to plan and delegate visibly on the board:".into());
        lines.push(
            "1. Create subtasks: `hive task add \"<title>\" --role <role> --body \"<details>\"`"
                .into(),
        );
        lines.push("2. Spawn subagents: `hive spawn --role <role> --task <id> --prompt \"<instructions>\"`".into());
        lines.push("3. Wait/check progress: `hive wait <session-id>` and `hive task list`".into());
        lines.push("4. Move tasks as work finishes: `hive task update <id> --status in_review --note \"...\"` or `--status done`".into());
        if let Some(tid) = args.task {
            lines.push(format!(
                "Update your root task #{tid} (`hive task update {tid} --status done --note \"...\"`) when all subtasks are complete."
            ));
        }
        Some(lines.join("\n"))
    } else {
        args.task.map(|tid| {
            format!(
                "When finished with task #{tid}, update the Hive board with: `hive task update {tid} --status in_review --note \"<summary>\"`"
            )
        })
    };

    let mut prompt_parts: Vec<String> = Vec::new();
    if let Some(ref instr) = role_cfg.instructions
        && !instr.trim().is_empty()
    {
        prompt_parts.push(instr.trim().to_string());
    }
    if let Some(brief) = orchestration_brief {
        prompt_parts.push(brief);
    }
    if let Some(ref p) = args.prompt
        && !p.trim().is_empty()
    {
        prompt_parts.push(p.trim().to_string());
    }
    let initial_prompt = if prompt_parts.is_empty() {
        None
    } else {
        Some(prompt_parts.join("\n\n"))
    };

    let mut launch_args = role_cfg.args.clone();
    let passed_via_cli = if let Some(ref prompt) = initial_prompt {
        match role_cfg.agent.as_str() {
            "claude" | "codex" | "cursor" | "pi" | "omp" | "amp" | "opencode" => {
                launch_args.push(prompt.clone());
                true
            }
            "gemini" | "antigravity" | "agy" => {
                launch_args.push("-i".into());
                launch_args.push(prompt.clone());
                true
            }
            "aider" => {
                launch_args.push("--message".into());
                launch_args.push(prompt.clone());
                true
            }
            _ => false,
        }
    } else {
        false
    };

    let mut req = launch::create_request(
        config,
        &role_cfg.agent,
        Some(label),
        args.cwd,
        &launch_args,
        args.size,
    )?;

    if let Some(ref prompt) = initial_prompt {
        req.env.push(("HIVE_PROMPT".into(), prompt.clone()));
    }
    req.parent_session = std::env::var("HIVE_SESSION_ID")
        .ok()
        .and_then(|s| s.parse::<SessionId>().ok());
    req.role = Some(resolved_role.clone());
    req.task_id = args.task.map(|t| t.to_string());

    if args.worktree || role_cfg.worktree || args.branch.is_some() {
        req.worktree = Some(NewWorktree {
            branch: args.branch,
        });
    }

    client.send(&ClientMessage::CreateSession(req)).await?;
    let info = client
        .recv_until(|m| match m {
            ServerMessage::SessionCreated(i) => Some(i),
            _ => None,
        })
        .await?;

    // Link session to task if requested.
    if let Some(tid) = args.task
        && let Ok(store) = open_store(tasks_dir)
    {
        let _ = store.update(
            tid,
            TaskPatch {
                status: Some(TaskStatus::InProgress),
                role: Some(Some(resolved_role.clone())),
                session_id: Some(Some(info.id.to_string())),
                branch: info.branch.clone().map(Some),
                note: Some(format!(
                    "Spawned {resolved_role} ({}) as session {}",
                    info.agent, info.id
                )),
                note_author: Some("hive".into()),
                ..Default::default()
            },
        );
    }

    // Fallback for custom agents without a known CLI prompt flag: wait briefly
    // for the TUI/shell to initialize, then send via PTY input.
    if !passed_via_cli && let Some(body) = initial_prompt {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let from = std::env::var("HIVE_SESSION_ID")
            .ok()
            .and_then(|s| s.parse().ok());
        client
            .send(&ClientMessage::SendMessage {
                session: info.id,
                from,
                body,
            })
            .await?;
        let _ = client
            .recv_until(|m| match m {
                ServerMessage::MessageDelivered { session } if session == info.id => Some(()),
                _ => None,
            })
            .await;
    }

    Ok(info)
}

pub(crate) async fn run_msg(
    client: &mut Client,
    session: SessionId,
    body: String,
) -> anyhow::Result<()> {
    let from = std::env::var("HIVE_SESSION_ID")
        .ok()
        .and_then(|s| s.parse().ok());
    client
        .send(&ClientMessage::SendMessage {
            session,
            from,
            body,
        })
        .await?;
    client
        .recv_until(|m| match m {
            ServerMessage::MessageDelivered { session: id } if id == session => Some(()),
            _ => None,
        })
        .await?;
    Ok(())
}

pub(crate) async fn run_wait(
    client: &mut Client,
    session: SessionId,
    timeout_secs: Option<u64>,
) -> anyhow::Result<SessionInfo> {
    // Check current state first so we return immediately if already finished/blocked.
    client.send(&ClientMessage::ListSessions).await?;
    let sessions = client
        .recv_until(|m| match m {
            ServerMessage::SessionList(s) => Some(s),
            _ => None,
        })
        .await?;
    let Some(cur) = sessions.into_iter().find(|s| s.id == session) else {
        bail!("session {session} not found");
    };
    if is_wait_done(&cur) {
        return Ok(cur);
    }

    let fut = client.recv_until(|m| match m {
        ServerMessage::SessionUpdated(info) if info.id == session && is_wait_done(&info) => {
            Some(info)
        }
        _ => None,
    });

    match timeout_secs {
        Some(secs) => tokio::time::timeout(Duration::from_secs(secs), fut)
            .await
            .map_err(|_| anyhow::anyhow!("timed out waiting for session {session}"))?
            .map_err(Into::into),
        None => Ok(fut.await?),
    }
}

fn is_wait_done(info: &SessionInfo) -> bool {
    if !info.state.is_alive() {
        return true;
    }
    matches!(
        info.presence.as_ref().map(|p| p.status),
        Some(AgentStatus::Blocked)
    )
}

pub(crate) fn workspace_tasks_dir(dir: Option<PathBuf>) -> PathBuf {
    dir.unwrap_or_else(|| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = hive_workspace::discover(&cwd)
            .map(|r| r.main_root)
            .unwrap_or(cwd);
        TaskStore::default_dir(Path::new(&root))
    })
}
