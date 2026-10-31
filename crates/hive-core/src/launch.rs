//! Turns an agent name into a concrete `CreateSession` request.

use std::path::{Path, PathBuf};

use hive_protocol::{CreateSession, TermSize};

use crate::config::{AgentDef, Config};

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("unknown agent `{name}`; known agents: {}", known.join(", "))]
    UnknownAgent { name: String, known: Vec<String> },
    #[error("`{command}` (agent `{agent}`) was not found on PATH; is it installed?")]
    NotInstalled { agent: String, command: String },
    #[error("cannot determine working directory: {0}")]
    Cwd(std::io::Error),
}

pub fn create_request(
    config: &Config,
    agent: &str,
    label: Option<String>,
    cwd: Option<PathBuf>,
    extra_args: &[String],
    size: TermSize,
) -> Result<CreateSession, LaunchError> {
    let agents = config.agents();
    let Some(def) = agents.get(agent) else {
        return Err(LaunchError::UnknownAgent {
            name: agent.to_string(),
            known: agents.keys().cloned().collect(),
        });
    };
    request_for(def, agent, label, cwd, extra_args, size)
}

/// Like [`create_request`], for a definition that isn't a named agent.
pub fn request_for(
    def: &AgentDef,
    agent: &str,
    label: Option<String>,
    cwd: Option<PathBuf>,
    extra_args: &[String],
    size: TermSize,
) -> Result<CreateSession, LaunchError> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    if find_executable(&def.command, &path_var).is_none() {
        return Err(LaunchError::NotInstalled {
            agent: agent.to_string(),
            command: def.command.clone(),
        });
    }
    let cwd = match cwd {
        Some(c) => c,
        None => std::env::current_dir().map_err(LaunchError::Cwd)?,
    };
    let mut env: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| !def.env.contains_key(k))
        .collect();
    env.extend(def.env.iter().map(|(k, v)| (k.clone(), v.clone())));
    Ok(CreateSession {
        agent: agent.to_string(),
        label: label.unwrap_or_else(|| agent.to_string()),
        command: def.argv(extra_args),
        cwd: cwd.display().to_string(),
        env,
        size,
        workspace: None,
        worktree: None,
    })
}

/// Resolves `command` the way `execvp` would: paths containing `/` are used
/// as-is, bare names are searched in `path_var`. On Windows, `PATHEXT`
/// extensions (`.exe`, `.cmd`, ...) are tried too.
pub fn find_executable(command: &str, path_var: &std::ffi::OsStr) -> Option<PathBuf> {
    let is_path = command.contains('/') || (cfg!(windows) && command.contains('\\'));
    if is_path {
        return candidates(PathBuf::from(command)).find(|p| is_exec(p));
    }
    std::env::split_paths(path_var)
        .flat_map(|dir| candidates(dir.join(command)))
        .find(|p| is_exec(p))
}

#[cfg(unix)]
fn candidates(p: PathBuf) -> impl Iterator<Item = PathBuf> {
    std::iter::once(p)
}

#[cfg(windows)]
fn candidates(p: PathBuf) -> impl Iterator<Item = PathBuf> {
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let with_ext: Vec<PathBuf> = exts
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| {
            let mut s = p.clone().into_os_string();
            s.push(e);
            PathBuf::from(s)
        })
        .collect();
    std::iter::once(p).chain(with_ext)
}

#[cfg(unix)]
fn is_exec(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_exec(p: &Path) -> bool {
    p.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn finds_sh_on_path_and_rejects_missing() {
        let path = std::ffi::OsString::from("/usr/bin:/bin");
        assert!(find_executable("sh", &path).is_some());
        assert!(find_executable("definitely-not-a-hive-agent", &path).is_none());
        assert!(find_executable("/bin/sh", &path).is_some());
    }

    #[test]
    fn unknown_agent_lists_known_ones() {
        let err = create_request(
            &Config::default(),
            "nope",
            None,
            None,
            &[],
            TermSize::DEFAULT,
        )
        .unwrap_err();
        assert!(err.to_string().contains("claude"));
    }
}
