//! `~/.config/hive/config.toml`. Every field has a default, so a missing or
//! partial file is valid.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub general: General,
    pub branding: Branding,
    pub terminal: Terminal,
    pub daemon: Daemon,
    pub worktrees: Worktrees,
    pub notifications: Notifications,
    pub integrations: Integrations,
    pub theme: ThemeConfig,
    pub keys: Keys,
    /// User-defined or overridden agents, keyed by name.
    pub agents: BTreeMap<String, AgentDef>,
    /// Pane arrangements opened together in a new tab (`Ctrl+Space L`).
    pub layouts: BTreeMap<String, Layout>,
}

/// `[layouts.<name>]`: panes opened together in a new tab of the current
/// space. The first pane fills the tab; each next one splits the one
/// before it.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    /// Space names this layout is for: there it opens without asking.
    pub spaces: Vec<String>,
    pub panes: Vec<LayoutPane>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutPane {
    /// Agent to start (default `shell`, a terminal).
    pub agent: String,
    /// argv to run instead of an agent, e.g. `["npm", "run", "dev"]`.
    pub run: Vec<String>,
    /// Where it goes relative to the previous pane.
    pub split: SplitDir,
    /// Session label; empty uses the agent's or the command's name.
    pub label: String,
}

impl Default for LayoutPane {
    fn default() -> Self {
        LayoutPane {
            agent: "shell".into(),
            run: Vec::new(),
            split: SplitDir::Right,
            label: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SplitDir {
    #[default]
    Right,
    Down,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    pub theme: String,
    pub default_agent: String,
    /// Ignored: from early versions of the spec, accepted so configs that
    /// set it still load.
    #[serde(rename = "restore_workspace")]
    _restore_workspace: Option<bool>,
}

impl Default for General {
    fn default() -> Self {
        General {
            theme: "hive-dark".into(),
            default_agent: "claude".into(),
            _restore_workspace: None,
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

/// `[notifications]`: how Hive gets your attention when a session you are
/// not looking at needs input or finishes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Notifications {
    /// Ring the terminal bell.
    pub bell: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Notifications { bell: true }
    }
}

/// `[integrations.*]`: optional extras. All off by default, and invisible
/// (no keys, palette entries or marks) until enabled.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Integrations {
    pub obsidian: Obsidian,
    pub review: ToolPane,
    pub github: ToolPane,
    pub tasks: Tasks,
    pub desktop_notify: DesktopNotify,
    pub git_status: GitStatus,
    /// `[integrations.usage]`: token usage and cost (default `ccusage`).
    pub usage: ToolPane,
}

/// `[integrations.review]` / `[integrations.github]`: a tool opened in the
/// focused session's directory.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolPane {
    pub enabled: bool,
    /// argv to run; empty picks a default (see the TUI).
    pub command: Vec<String>,
    pub open_in: OpenIn,
}

impl Default for ToolPane {
    fn default() -> Self {
        ToolPane {
            enabled: false,
            command: Vec::new(),
            open_in: OpenIn::Split,
        }
    }
}

/// `[integrations.tasks]`: run project tasks (just, package.json, make,
/// cargo) beside the agent.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tasks {
    pub enabled: bool,
    pub open_in: OpenIn,
}

impl Default for Tasks {
    fn default() -> Self {
        Tasks {
            enabled: false,
            open_in: OpenIn::Split,
        }
    }
}

/// `[integrations.desktop_notify]`: a desktop notification when a session
/// you aren't viewing needs input or finishes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DesktopNotify {
    pub enabled: bool,
    /// argv; the title and body are appended as two arguments.
    pub command: Vec<String>,
}

impl Default for DesktopNotify {
    fn default() -> Self {
        DesktopNotify {
            enabled: false,
            command: vec!["notify-send".into(), "--app-name=Hive".into()],
        }
    }
}

/// `[integrations.git_status]`: changed files and ahead/behind counts under
/// each repository space in the sidebar.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct GitStatus {
    pub enabled: bool,
}

/// `[integrations.obsidian]`: open a workspace's Obsidian vault in a pane.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Obsidian {
    pub enabled: bool,
    /// Agent to open the vault with; empty means `$VISUAL`, `$EDITOR`, `vi`.
    pub agent: String,
    pub open_in: OpenIn,
    /// Append a line to today's note in the vault when an agent finishes.
    pub journal: bool,
    /// Folder of daily notes inside the vault; empty is the vault root.
    pub journal_dir: String,
}

/// Where the notes pane opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OpenIn {
    #[default]
    Tab,
    /// Split to the right of the focused pane.
    Split,
}

impl Obsidian {
    /// The program that opens the vault, as an agent definition.
    pub fn notes_agent(&self, config: &Config) -> Option<AgentDef> {
        if !self.agent.is_empty() {
            return config.agent(&self.agent);
        }
        let editor = ["VISUAL", "EDITOR"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .find(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "vi".into());
        // `EDITOR="code -w"` is common; split on whitespace, no shell.
        let mut words = editor.split_whitespace().map(str::to_string);
        Some(AgentDef {
            command: words.next()?,
            args: words.collect(),
            env: BTreeMap::new(),
        })
    }
}

/// `[theme]`: colour depth, glyph set and per-token colour overrides. The
/// base palette is chosen by `general.theme`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    /// `auto` (default), `truecolor`, `256`, `16` or `none`.
    pub colors: Option<String>,
    /// `auto` (default), `unicode` or `ascii`.
    pub glyphs: Option<String>,
    /// `#rrggbb` overrides for individual colours.
    pub surface: Option<String>,
    pub accent: Option<String>,
    pub accent_dim: Option<String>,
    pub text: Option<String>,
    pub muted: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

/// `[keys]`: the leader key and what each key does after it.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Keys {
    pub leader: String,
    /// Command name to key(s), e.g. `palette = "p"` or
    /// `next_session = ["j", "down"]`. Replaces that command's defaults; an
    /// empty list unbinds it.
    pub bind: BTreeMap<String, KeyList>,
    /// Command name to prefix-free key(s), active even while typing to an
    /// agent, e.g. `next_session = "alt+right"`. Must use Ctrl/Alt.
    pub global: BTreeMap<String, KeyList>,
    /// Modifier that makes 1-9 jump to sessions without the leader:
    /// `alt` (default), `ctrl+alt`, or `""` to turn it off.
    pub jump_modifier: String,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            leader: "ctrl+space".into(),
            bind: BTreeMap::new(),
            global: BTreeMap::new(),
            jump_modifier: "alt".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum KeyList {
    One(String),
    Many(Vec<String>),
}

impl KeyList {
    pub fn keys(&self) -> Vec<&str> {
        match self {
            KeyList::One(k) if k.is_empty() => Vec::new(),
            KeyList::One(k) => vec![k.as_str()],
            KeyList::Many(ks) => ks.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Worktrees {
    /// Where Hive creates linked worktrees. Defaults to
    /// `$XDG_DATA_HOME/hive/worktrees`.
    pub dir: Option<PathBuf>,
}

impl Worktrees {
    pub fn base_dir(&self) -> PathBuf {
        self.dir.clone().unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("hive")
                .join("worktrees")
        })
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
    /// Every launchable agent: the known catalog (see `hive-agents`), the
    /// user's shell, and `[agents.*]`, which override built-ins of the same
    /// name. Built-ins may not be installed; check before offering them.
    pub fn agents(&self) -> BTreeMap<String, AgentDef> {
        let default = if cfg!(windows) {
            "powershell.exe"
        } else {
            "/bin/sh"
        };
        let shell = std::env::var("SHELL").unwrap_or_else(|_| default.into());
        let mut agents: BTreeMap<String, AgentDef> = hive_agents::CATALOG
            .iter()
            .map(|k| (k.id.to_string(), AgentDef::simple(k.command)))
            .chain([("shell".to_string(), AgentDef::simple(&shell))])
            .collect();
        agents.extend(self.agents.clone());
        agents
    }

    /// Whether `name` is defined in the config file (not only built in).
    pub fn is_configured(&self, name: &str) -> bool {
        self.agents.contains_key(name)
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
        assert!(c.is_configured("aider") && !c.is_configured("codex"));
    }

    #[test]
    fn obsidian_is_off_by_default_and_validated() {
        let off = Config::default().integrations.obsidian;
        assert!(!off.enabled);
        assert_eq!(off.open_in, OpenIn::Tab);
        let on = Config::parse(
            "[integrations.obsidian]\nenabled = true\nagent = \"shell\"\nopen_in = \"split\"\n",
        )
        .unwrap();
        let o = &on.integrations.obsidian;
        assert_eq!(o.open_in, OpenIn::Split);
        assert_eq!(
            o.notes_agent(&on).unwrap().command,
            on.agent("shell").unwrap().command
        );
        assert!(Config::parse("[integrations.obsidian]\nopen_in = \"window\"\n").is_err());
        let i = Config::default().integrations;
        assert!(!i.review.enabled && !i.github.enabled && !i.tasks.enabled);
        assert!(!i.desktop_notify.enabled && !i.obsidian.journal);
        assert_eq!(i.review.open_in, OpenIn::Split);
    }

    #[test]
    fn known_agents_are_built_in() {
        let c = Config::default();
        assert_eq!(c.agent("truss").unwrap().command, "truss-tui");
        assert_eq!(c.agent("cursor").unwrap().command, "cursor-agent");
        assert!(c.agent("shell").is_some());
    }

    #[test]
    fn worktree_dir_defaults_to_data_dir_and_can_be_set() {
        let c = Config::parse("").unwrap();
        assert!(c.worktrees.base_dir().ends_with("hive/worktrees"));
        let c = Config::parse("[worktrees]\ndir = \"/x/wt\"").unwrap();
        assert_eq!(c.worktrees.base_dir(), PathBuf::from("/x/wt"));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(Config::parse("[general]\nthme = \"x\"").is_err());
        assert!(Config::parse("[general]\nrestore_workspace = true").is_ok());
    }

    #[test]
    fn layouts_parse_with_terminal_defaults() {
        assert!(Config::parse("").unwrap().layouts.is_empty());
        let c = Config::parse(
            r#"
[layouts.web]
spaces = ["web"]
panes = [{}, { run = ["npm", "run", "dev"], split = "down" }, { agent = "claude" }]
"#,
        )
        .unwrap();
        let panes = &c.layouts["web"].panes;
        assert_eq!(panes[0].agent, "shell");
        assert_eq!(panes[1].split, SplitDir::Down);
        assert_eq!(panes[1].run, ["npm", "run", "dev"]);
        assert_eq!(panes[2].split, SplitDir::Right);
        assert!(Config::parse("[layouts.x]\npanes = [{ split = \"left\" }]").is_err());
    }
}
