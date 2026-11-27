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
        new_space: false,
        parent_session: None,
        role: None,
        task_id: None,
    })
}

/// Choose a shell from the launching client's environment.
pub(crate) fn default_shell() -> String {
    let path = std::env::var_os("PATH").unwrap_or_default();
    if let Some(shell) = crate::shell::launcher().and_then(Path::to_str)
        && let Some(executable) = find_executable(shell, &path)
        && let Ok(executable) = executable.into_os_string().into_string()
    {
        return executable;
    }
    let shell = std::env::var("SHELL").ok();
    if let Some(shell) =
        shell.filter(|shell| !shell.is_empty() && find_executable(shell, &path).is_some())
    {
        return shell;
    }
    #[cfg(windows)]
    {
        if std::env::var_os("MSYSTEM").is_some()
            && let Some(bash) = git_bash(&path)
        {
            return bash.to_string_lossy().into_owned();
        }
        "powershell.exe".into()
    }
    #[cfg(not(windows))]
    {
        "/bin/sh".into()
    }
}

/// Installed shell executables offered by the terminal picker.
pub fn installed_shells() -> Vec<(&'static str, PathBuf)> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    #[cfg(windows)]
    let candidates = [
        ("Git Bash / MSYS", git_bash(&path)),
        ("PowerShell 7", find_executable("pwsh.exe", &path)),
        (
            "Windows PowerShell",
            find_executable("powershell.exe", &path),
        ),
        ("Command Prompt", find_executable("cmd.exe", &path)),
    ];
    #[cfg(not(windows))]
    let candidates = ["bash", "zsh", "fish", "dash", "ksh", "nu", "pwsh", "sh"]
        .map(|name| (name, find_executable(name, &path)));
    candidates
        .into_iter()
        .filter_map(|(name, command)| command.map(|command| (name, command)))
        .collect()
}

#[cfg(windows)]
fn git_bash(path: &std::ffi::OsStr) -> Option<PathBuf> {
    // The Windows system bash.exe is a WSL launcher. An MSYS bash lives
    // alongside msys-2.0.dll or in Git for Windows' bin directory.
    msys_bash_on_path(path).or_else(|| {
        ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"]
            .into_iter()
            .filter_map(std::env::var_os)
            .map(|root| PathBuf::from(root).join("Git/bin/bash.exe"))
            .find(|bash| bash.is_file())
    })
}

#[cfg(any(windows, test))]
fn msys_bash_on_path(path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|dir| {
        let bash = dir.join("bash.exe");
        if dir.join("msys-2.0.dll").is_file() && bash.is_file() {
            return Some(bash);
        }
        let runtime = dir.join("../usr/bin/msys-2.0.dll");
        (runtime.is_file() && bash.is_file()).then_some(bash)
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

    #[test]
    fn identifies_git_bash_after_the_windows_wsl_launcher() {
        let root = tempfile::tempdir().unwrap();
        let system = root.path().join("System32");
        let git = root.path().join("Git/usr/bin");
        std::fs::create_dir_all(&system).unwrap();
        std::fs::create_dir_all(&git).unwrap();
        for file in [
            system.join("bash.exe"),
            git.join("bash.exe"),
            git.join("msys-2.0.dll"),
        ] {
            std::fs::write(file, "").unwrap();
        }
        let path = std::env::join_paths([system, git.clone()]).unwrap();
        assert_eq!(msys_bash_on_path(&path), Some(git.join("bash.exe")));
    }

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
