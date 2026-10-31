//! `~/.config/hive/config.toml`. Every field has a default, so a missing or
//! partial file is valid.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub general: General,
    pub branding: Branding,
    pub terminal: Terminal,
    pub daemon: Daemon,
    /// User-defined or overridden agents, keyed by name.
    pub agents: BTreeMap<String, AgentDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    pub theme: String,
    pub default_agent: String,
    pub restore_workspace: bool,
}

impl Default for General {
    fn default() -> Self {
        General {
            theme: "hive-dark".into(),
            default_agent: "claude".into(),
            restore_workspace: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Branding {
    pub show_header: bool,
    pub show_tagline: bool,
}

impl Default for Branding {
    fn default() -> Self {
        Branding {
            show_header: true,
            show_tagline: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Terminal {
    pub scrollback_lines: usize,
    pub mouse_support: bool,
}

impl Default for Terminal {
    fn default() -> Self {
        Terminal {
            scrollback_lines: 10_000,
            mouse_support: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Daemon {
    pub auto_start: bool,
    /// Seconds between SIGTERM and SIGKILL when terminating a session.
    pub kill_timeout_secs: u64,
}

impl Default for Daemon {
    fn default() -> Self {
        Daemon {
            auto_start: true,
            kill_timeout_secs: 3,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentDef {
    /// Executable name or path. Invoked directly, never via a shell.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl AgentDef {
    fn simple(command: &str) -> AgentDef {
        AgentDef {
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    pub fn argv(&self, extra_args: &[String]) -> Vec<String> {
        std::iter::once(self.command.clone())
            .chain(self.args.iter().cloned())
            .chain(extra_args.iter().cloned())
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid config {path}: {source}")]
    Parse {
        path: String,
        source: toml::de::Error,
    },
}

impl Config {
    /// Loads `path`, falling back to defaults if it does not exist.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::parse(&text).map_err(|source| ConfigError::Parse {
                path: path.display().to_string(),
                source,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.display().to_string(),
                source,
            }),
        }
    }

    pub fn parse(text: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(text)
    }

    /// Built-in agents overlaid with the user's `[agents.*]` tables.
    pub fn agents(&self) -> BTreeMap<String, AgentDef> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut agents: BTreeMap<String, AgentDef> = [
            ("claude", AgentDef::simple("claude")),
            ("codex", AgentDef::simple("codex")),
            ("gemini", AgentDef::simple("gemini")),
            ("opencode", AgentDef::simple("opencode")),
            ("shell", AgentDef::simple(&shell)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        agents.extend(self.agents.clone());
        agents
    }

    pub fn agent(&self, name: &str) -> Option<AgentDef> {
        self.agents().remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_uses_defaults() {
        let c = Config::parse("").unwrap();
        assert_eq!(c.general.default_agent, "claude");
        assert!(c.daemon.auto_start);
        assert_eq!(c.agent("codex").unwrap().command, "codex");
    }

    #[test]
    fn user_agents_override_and_extend_builtins() {
        let c = Config::parse(
            r#"
            [agents.claude]
            command = "/opt/claude/bin/claude"
            args = ["--verbose"]

            [agents.aider]
            command = "aider"
            env = { AIDER_DARK_MODE = "1" }
            "#,
        )
        .unwrap();
        let claude = c.agent("claude").unwrap();
        assert_eq!(
            claude.argv(&["x".into()]),
            vec!["/opt/claude/bin/claude", "--verbose", "x"]
        );
        assert_eq!(c.agent("aider").unwrap().env["AIDER_DARK_MODE"], "1");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(Config::parse("[general]\nthme = \"x\"").is_err());
    }
}
