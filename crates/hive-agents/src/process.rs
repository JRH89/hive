//! Recognising which agent a running process is, from its argv.

use crate::catalog::{self, AgentKind};

/// Interpreters and launchers whose own name says nothing; the agent is one
/// of their arguments (`node /…/bin/codex`, `sh -c 'claude …'`).
const WRAPPERS: &[&str] = &[
    "node", "nodejs", "bun", "deno", "python", "python3", "uv", "uvx", "npx", "bunx", "env", "sh",
    "bash", "zsh", "dash", "fish", "exec",
];

/// Identifies the agent from a process's argv, looking through runtime
/// wrappers. Returns `None` for anything not in the catalog.
pub fn identify(argv: &[String]) -> Option<&'static AgentKind> {
    identify_words(argv.iter().map(String::as_str), 0)
}

fn identify_words<'a>(
    mut words: impl Iterator<Item = &'a str>,
    depth: u8,
) -> Option<&'static AgentKind> {
    let first = words.next()?;
    if let Some(kind) = by_path(first) {
        return Some(kind);
    }
    if depth > 2 || !WRAPPERS.contains(&stem(first)) {
        return None;
    }
    let mut command_string = false;
    let mut operands = 0;
    for word in words {
        if command_string {
            // `sh -c 'agent args'`: identify the command line's first word.
            return identify_words(word.split_whitespace(), depth + 1);
        }
        if word == "-c" {
            command_string = true;
            continue;
        }
        // Skip flags and `VAR=value` (env); the first operand is the script
        // or the next program.
        if word.starts_with('-') || (word.contains('=') && !word.contains('/')) {
            continue;
        }
        if let Some(kind) = by_path(word) {
            return Some(kind);
        }
        // A script path is the program; bare words may be launcher
        // subcommands (`bun run`, `uv tool run`), so look a little further.
        operands += 1;
        if word.contains('/') || stem(word) != word || operands >= 3 {
            return None;
        }
    }
    None
}

fn by_path(word: &str) -> Option<&'static AgentKind> {
    catalog::by_executable(stem(word)).or_else(|| {
        catalog::CATALOG
            .iter()
            .find(|k| k.package_hints.iter().any(|h| word.contains(h)))
    })
}

/// Basename without a script extension.
fn stem(word: &str) -> &str {
    let base = word.rsplit('/').next().unwrap_or(word);
    [".js", ".mjs", ".cjs", ".ts", ".py", ".exe"]
        .iter()
        .find_map(|ext| base.strip_suffix(ext))
        .unwrap_or(base)
}

/// The argv of a process, if it can be read.
#[cfg(target_os = "linux")]
pub fn argv(pid: i32) -> Option<Vec<String>> {
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let argv: Vec<String> = raw
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    (!argv.is_empty()).then_some(argv)
}

#[cfg(not(target_os = "linux"))]
pub fn argv(_pid: i32) -> Option<Vec<String>> {
    None
}

/// Pids of the members of a process group.
#[cfg(target_os = "linux")]
pub fn group_members(pgid: i32) -> Vec<i32> {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    dir.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<i32>().ok())
        .filter(|pid| {
            std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .ok()
                .and_then(|s| {
                    // Fields after the parenthesised comm: state ppid pgrp …
                    let rest = &s[s.rfind(')')? + 2..];
                    rest.split(' ').nth(2)?.parse::<i32>().ok()
                })
                == Some(pgid)
        })
        .collect()
}

#[cfg(not(target_os = "linux"))]
pub fn group_members(_pgid: i32) -> Vec<i32> {
    Vec::new()
}

/// The agent running in the foreground process group `pgid`: its leader if
/// recognisable, otherwise any member (a wrapper script may lead the group).
pub fn identify_group(pgid: i32) -> Option<&'static AgentKind> {
    if let Some(kind) = argv(pgid).as_deref().and_then(identify) {
        return Some(kind);
    }
    let mut members = group_members(pgid);
    members.sort_unstable();
    members
        .into_iter()
        .filter(|pid| *pid != pgid)
        .find_map(|pid| argv(pid).as_deref().and_then(identify))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(argv: &[&str]) -> Option<&'static str> {
        let argv: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
        identify(&argv).map(|k| k.id)
    }

    #[test]
    fn identifies_direct_and_wrapped_agents() {
        assert_eq!(id(&["/usr/local/bin/claude", "--resume"]), Some("claude"));
        assert_eq!(id(&["claude-code"]), Some("claude"));
        assert_eq!(
            id(&[
                "node",
                "/home/u/.mise/node/lib/node_modules/@truss-harness/tui/dist/bin.js"
            ]),
            Some("truss")
        );
        assert_eq!(
            id(&["node", "--no-warnings", "/x/bin/codex"]),
            Some("codex")
        );
        assert_eq!(id(&["/bin/sh", "-c", "gemini -p hi"]), Some("gemini"));
        assert_eq!(id(&["env", "FOO=1", "cursor-agent"]), Some("cursor"));
        assert_eq!(
            id(&["bun", "run", "/x/node_modules/opencode-ai/bin/opencode"]),
            Some("opencode")
        );
        assert_eq!(id(&["python3", "-m", "aider"]), Some("aider"));
    }

    #[test]
    fn ignores_unrelated_processes() {
        assert_eq!(id(&["bash"]), None);
        assert_eq!(id(&["node", "server.js"]), None);
        assert_eq!(id(&["vim", "claude.md"]), None);
        assert_eq!(id(&[]), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reads_own_process() {
        let me = std::process::id() as i32;
        assert!(argv(me).is_some_and(|a| !a.is_empty()));
        let pgid = rustix_pgid();
        assert!(group_members(pgid).contains(&me));
    }

    #[cfg(target_os = "linux")]
    fn rustix_pgid() -> i32 {
        let s = std::fs::read_to_string("/proc/self/stat").unwrap();
        let rest = &s[s.rfind(')').unwrap() + 2..];
        rest.split(' ').nth(2).unwrap().parse().unwrap()
    }
}
