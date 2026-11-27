//! `hive mcp` — local JSON-RPC 2.0 stdio MCP server exposing Hive's task board
//! and multi-agent delegation primitives as native tools for Claude Code,
//! Codex, Gemini CLI, and other MCP-capable agents.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use hive_core::client::Client;
use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_protocol::{ClientMessage, ServerMessage, SessionId, TermSize};
use hive_tasks::{NewTask, TaskPatch, TaskStatus, TaskStore};
use serde_json::{Value, json};

use crate::tasks::{self, SpawnArgs};

pub(crate) async fn run(
    paths: &Paths,
    config: &Config,
    dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let tasks_dir = tasks::workspace_tasks_dir(dir);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("parse error: {e}") }
                });
                writeln!(out, "{err}")?;
                out.flush()?;
                continue;
            }
        };

        if let Some(resp) = handle_request(paths, config, &tasks_dir, &req).await {
            writeln!(out, "{resp}")?;
            out.flush()?;
        }
    }
    Ok(())
}

pub(crate) async fn handle_request(
    paths: &Paths,
    config: &Config,
    tasks_dir: &PathBuf,
    req: &Value,
) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);

    // Notifications (no id) get no response.
    let id = id?;

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "hive",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(paths, config, tasks_dir, &params).await,
        _ => Err((-32601, format!("method not found: {method}"))),
    };

    Some(match result {
        Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    })
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "task_create",
            "description": "Create a task on the workspace Hive task board.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "role": { "type": "string" },
                    "body": { "type": "string" },
                    "depends_on": { "type": "array", "items": { "type": "integer" } }
                },
                "required": ["title"]
            }
        },
        {
            "name": "task_list",
            "description": "List tasks on the workspace Hive task board.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": { "type": "string" },
                    "role": { "type": "string" },
                    "ready": { "type": "boolean" }
                }
            }
        },
        {
            "name": "task_get",
            "description": "Get full details for a task by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }
        },
        {
            "name": "task_update",
            "description": "Update a task's status, role, session, or append a log note.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "status": { "type": "string" },
                    "role": { "type": "string" },
                    "session_id": { "type": "string" },
                    "note": { "type": "string" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "agent_spawn",
            "description": "Spawn a specialist subagent in Hive using a configured role.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": { "type": "string" },
                    "task_id": { "type": "integer" },
                    "worktree": { "type": "boolean" },
                    "branch": { "type": "string" },
                    "prompt": { "type": "string" }
                },
                "required": ["role"]
            }
        },
        {
            "name": "agent_message",
            "description": "Send a message directly to a running Hive session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": { "type": "integer" },
                    "body": { "type": "string" }
                },
                "required": ["session_id", "body"]
            }
        },
        {
            "name": "agent_wait",
            "description": "Wait for a Hive session to finish or become blocked.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": { "type": "integer" },
                    "timeout_secs": { "type": "integer" }
                },
                "required": ["session_id"]
            }
        },
        {
            "name": "agent_list",
            "description": "List active Hive sessions with their role, parent, and status.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "role_list",
            "description": "List available Hive orchestration roles/models and their strengths for routing tasks.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

async fn call_tool(
    paths: &Paths,
    config: &Config,
    tasks_dir: &PathBuf,
    params: &Value,
) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".into()))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let text = match name {
        "task_create" => {
            let store = TaskStore::open(tasks_dir).map_err(|e| (-32000, e.to_string()))?;
            let title = args
                .get("title")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing title".into()))?
                .to_string();
            let role = args.get("role").and_then(Value::as_str).map(str::to_string);
            let body = args
                .get("body")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let depends_on = args
                .get("depends_on")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u32))
                        .collect()
                })
                .unwrap_or_default();
            let task = store
                .create(NewTask {
                    title,
                    status: None,
                    role,
                    parent_session: std::env::var("HIVE_SESSION_ID").ok(),
                    depends_on,
                    body,
                })
                .map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_string_pretty(&task).unwrap_or_default()
        }
        "task_list" => {
            let store = TaskStore::open(tasks_dir).map_err(|e| (-32000, e.to_string()))?;
            let ready = args.get("ready").and_then(Value::as_bool).unwrap_or(false);
            let mut list = if ready {
                store.ready_tasks()
            } else {
                store.list()
            }
            .map_err(|e| (-32000, e.to_string()))?;
            if let Some(s) = args.get("status").and_then(Value::as_str) {
                let want: TaskStatus = s
                    .parse()
                    .map_err(|e: hive_tasks::TaskError| (-32602, e.to_string()))?;
                list.retain(|t| t.status == want);
            }
            if let Some(r) = args.get("role").and_then(Value::as_str) {
                list.retain(|t| t.role.as_deref() == Some(r));
            }
            serde_json::to_string_pretty(&list).unwrap_or_default()
        }
        "task_get" => {
            let store = TaskStore::open(tasks_dir).map_err(|e| (-32000, e.to_string()))?;
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or((-32602, "missing id".into()))? as u32;
            let task = store.get(id).map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_string_pretty(&task).unwrap_or_default()
        }
        "task_update" => {
            let store = TaskStore::open(tasks_dir).map_err(|e| (-32000, e.to_string()))?;
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or((-32602, "missing id".into()))? as u32;
            let status = match args.get("status").and_then(Value::as_str) {
                Some(s) => Some(
                    s.parse()
                        .map_err(|e: hive_tasks::TaskError| (-32602, e.to_string()))?,
                ),
                None => None,
            };
            let task = store
                .update(
                    id,
                    TaskPatch {
                        status,
                        role: args
                            .get("role")
                            .and_then(Value::as_str)
                            .map(|s| Some(s.to_string())),
                        session_id: args
                            .get("session_id")
                            .and_then(Value::as_str)
                            .map(|s| Some(s.to_string())),
                        note: args.get("note").and_then(Value::as_str).map(str::to_string),
                        note_author: std::env::var("HIVE_ROLE").ok(),
                        ..Default::default()
                    },
                )
                .map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_string_pretty(&task).unwrap_or_default()
        }
        "agent_spawn" => {
            let role = args
                .get("role")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing role".into()))?
                .to_string();
            let mut client = Client::connect_or_start(paths, "hive-mcp", config.daemon.auto_start)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            let info = tasks::run_spawn(
                &mut client,
                config,
                SpawnArgs {
                    role,
                    task: args
                        .get("task_id")
                        .and_then(Value::as_u64)
                        .map(|n| n as u32),
                    worktree: args
                        .get("worktree")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    branch: args
                        .get("branch")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    cwd: None,
                    prompt: args
                        .get("prompt")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    size: TermSize::DEFAULT,
                    force: args.get("force").and_then(Value::as_bool).unwrap_or(false),
                },
                Some(tasks_dir.clone()),
            )
            .await
            .map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_string_pretty(&info).unwrap_or_default()
        }
        "agent_message" => {
            let session_id =
                args.get("session_id")
                    .and_then(Value::as_u64)
                    .ok_or((-32602, "missing session_id".into()))? as SessionId;
            let body = args
                .get("body")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing body".into()))?
                .to_string();
            let mut client = Client::connect_or_start(paths, "hive-mcp", false)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            tasks::run_msg(&mut client, session_id, body)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            format!("delivered message to session {session_id}")
        }
        "agent_wait" => {
            let session_id =
                args.get("session_id")
                    .and_then(Value::as_u64)
                    .ok_or((-32602, "missing session_id".into()))? as SessionId;
            let timeout = args.get("timeout_secs").and_then(Value::as_u64);
            let mut client = Client::connect_or_start(paths, "hive-mcp", false)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            let info = tasks::run_wait(&mut client, session_id, timeout)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_string_pretty(&info).unwrap_or_default()
        }
        "agent_list" => {
            let mut client = Client::connect_or_start(paths, "hive-mcp", false)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            client
                .send(&ClientMessage::ListSessions)
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            let sessions = client
                .recv_until(|m| match m {
                    ServerMessage::SessionList(s) => Some(s),
                    _ => None,
                })
                .await
                .map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_string_pretty(&sessions).unwrap_or_default()
        }
        "role_list" => {
            let roles = config.all_roles();
            serde_json::to_string_pretty(&roles).unwrap_or_default()
        }
        other => return Err((-32601, format!("unknown tool: {other}"))),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mcp_initialize_list_and_task_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks_dir = tmp.path().join("tasks");
        let paths =
            Paths::with_runtime_dir(tmp.path().into(), tmp.path().into(), tmp.path().into());
        let config = Config::default();

        let init = handle_request(
            &paths,
            &config,
            &tasks_dir,
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        )
        .await
        .unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "hive");

        let tools = handle_request(
            &paths,
            &config,
            &tasks_dir,
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        )
        .await
        .unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 9);

        let created = handle_request(
            &paths,
            &config,
            &tasks_dir,
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "task_create",
                    "arguments": { "title": "Review PR", "role": "reviewer" }
                }
            }),
        )
        .await
        .unwrap();
        let body = created["result"]["content"][0]["text"].as_str().unwrap();
        assert!(body.contains("Review PR"));

        let listed = handle_request(
            &paths,
            &config,
            &tasks_dir,
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": { "name": "task_list", "arguments": { "role": "reviewer" } }
            }),
        )
        .await
        .unwrap();
        let list_body = listed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(list_body.contains("Review PR"));
    }
}
