//! Hive's own key bindings: the leader key and the commands that follow it.
//!
//! Everything else goes to the agent, so only the leader and a few global
//! chords (Alt+1..9 and Alt+arrows by default, like herdr) are intercepted
//! while typing. Commands are also reachable from the palette, which is
//! where commands without a default key live.

use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use hive_core::config::Keys;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Command {
    Palette,
    SearchSessions,
    NewSession,
    NewAgent,
    NewWorktreeTerminal,
    NewSpace,
    Usage,
    Layout,
    NewWorktreeSession,
    Workspaces,
    AddWorkspace,
    RenameWorkspace,
    NextSession,
    PrevSession,
    NextWorkspace,
    PrevWorkspace,
    Sidebar,
    ToggleSidebar,
    SplitRight,
    SplitDown,
    FocusNextPane,
    Zoom,
    ClosePane,
    NewTab,
    NextTab,
    PrevTab,
    CloseTab,
    Notes,
    Review,
    PullRequest,
    RunTask,
    Rename,
    Kill,
    TakeControl,
    Detach,
    Help,
    Settings,
}

impl Command {
    pub const ALL: [Command; 37] = [
        Command::Palette,
        Command::SearchSessions,
        Command::NewSession,
        Command::NewAgent,
        Command::NewWorktreeTerminal,
        Command::NewSpace,
        Command::NewWorktreeSession,
        Command::Workspaces,
        Command::AddWorkspace,
        Command::RenameWorkspace,
        Command::NextSession,
        Command::PrevSession,
        Command::NextWorkspace,
        Command::PrevWorkspace,
        Command::Sidebar,
        Command::ToggleSidebar,
        Command::SplitRight,
        Command::SplitDown,
        Command::FocusNextPane,
        Command::Zoom,
        Command::ClosePane,
        Command::NewTab,
        Command::NextTab,
        Command::PrevTab,
        Command::CloseTab,
        Command::Notes,
        Command::Review,
        Command::PullRequest,
        Command::RunTask,
        Command::Usage,
        Command::Layout,
        Command::Rename,
        Command::Kill,
        Command::TakeControl,
        Command::Detach,
        Command::Help,
        Command::Settings,
    ];

    /// The name used in `[keys.bind]`.
    pub fn name(self) -> &'static str {
        match self {
            Command::Palette => "palette",
            Command::SearchSessions => "search_sessions",
            Command::NewSession => "new_session",
            Command::NewAgent => "new_agent",
            Command::NewWorktreeTerminal => "new_worktree_terminal",
            Command::NewSpace => "new_space",
            Command::Usage => "usage",
            Command::Layout => "layout",
            Command::NewWorktreeSession => "new_worktree_session",
            Command::Workspaces => "workspaces",
            Command::AddWorkspace => "add_workspace",
            Command::RenameWorkspace => "rename_workspace",
            Command::NextSession => "next_session",
            Command::PrevSession => "prev_session",
            Command::NextWorkspace => "next_workspace",
            Command::PrevWorkspace => "prev_workspace",
            Command::Sidebar => "sidebar",
            Command::ToggleSidebar => "toggle_sidebar",
            Command::SplitRight => "split_right",
            Command::SplitDown => "split_down",
            Command::FocusNextPane => "focus_next_pane",
            Command::Zoom => "zoom",
            Command::ClosePane => "close_pane",
            Command::NewTab => "new_tab",
            Command::NextTab => "next_tab",
            Command::PrevTab => "prev_tab",
            Command::CloseTab => "close_tab",
            Command::Notes => "notes",
            Command::Review => "review",
            Command::PullRequest => "pull_request",
            Command::RunTask => "run_task",
            Command::Rename => "rename_session",
            Command::Kill => "kill_session",
            Command::TakeControl => "take_control",
            Command::Detach => "detach",
            Command::Help => "help",
            Command::Settings => "settings",
        }
    }

    /// Shown in the palette and the help screen.
    pub fn title(self) -> &'static str {
        match self {
            Command::Palette => "Command palette",
            Command::SearchSessions => "Search sessions",
            Command::NewSession => "New terminal",
            Command::NewAgent => "New agent (pick one)",
            Command::NewWorktreeTerminal => "New terminal in its own worktree (own branch)",
            Command::NewSpace => "New space (a terminal; cd anywhere)",
            Command::Usage => "Token usage and cost of your agents",
            Command::Layout => "Open a layout (panes from [layouts])",
            Command::NewWorktreeSession => "New agent in its own worktree (pick one)",
            Command::Workspaces => "Switch workspace",
            Command::AddWorkspace => "Add a folder as a space",
            Command::RenameWorkspace => "Rename workspace",
            Command::NextSession => "Next session",
            Command::PrevSession => "Previous session",
            Command::NextWorkspace => "Next workspace",
            Command::PrevWorkspace => "Previous workspace",
            Command::Sidebar => "Focus the sidebar",
            Command::ToggleSidebar => "Show or hide the sidebar",
            Command::SplitRight => "Split: terminal on the right",
            Command::SplitDown => "Split: terminal below",
            Command::FocusNextPane => "Focus the next pane",
            Command::Zoom => "Zoom the pane (toggle)",
            Command::ClosePane => "Close the pane (session keeps running)",
            Command::NewTab => "New tab (a terminal)",
            Command::NextTab => "Next tab",
            Command::PrevTab => "Previous tab",
            Command::CloseTab => "Close the tab (sessions keep running)",
            Command::Notes => "Open the workspace's notes (Obsidian vault)",
            Command::Review => "Review changes in the session's directory",
            Command::PullRequest => "Pull request checks for the session's branch",
            Command::RunTask => "Run a project task beside the session",
            Command::Rename => "Rename session",
            Command::Kill => "Terminate session",
            Command::TakeControl => "Take input control",
            Command::Detach => "Detach (agents keep running)",
            Command::Help => "Help",
            Command::Settings => "Settings (config file location)",
        }
    }

    /// Prefix-free chords, active even while typing to an agent.
    fn default_global_keys(self) -> &'static [&'static str] {
        match self {
            Command::NextTab => &["alt+right"],
            Command::PrevTab => &["alt+left"],
            Command::NextWorkspace => &["alt+down"],
            Command::PrevWorkspace => &["alt+up"],
            _ => &[],
        }
    }

    /// Keys after the leader. Close to herdr's and tmux's defaults.
    fn default_keys(self) -> &'static [&'static str] {
        match self {
            Command::Palette => &[":", "P"],
            Command::SearchSessions => &["/"],
            Command::NewSession => &["c"],
            Command::NewAgent => &["A"],
            Command::Workspaces => &["w"],
            Command::NextSession => &["n", "j", "down"],
            Command::PrevSession => &["p", "k", "up"],
            Command::NextWorkspace | Command::PrevWorkspace => &[],
            Command::Sidebar => &["s", "tab"],
            Command::ToggleSidebar => &["b"],
            Command::SplitRight => &["v", "%"],
            Command::SplitDown => &["-", "\""],
            Command::FocusNextPane => &["o"],
            Command::Zoom => &["z"],
            Command::ClosePane => &["X"],
            Command::NewTab => &["t"],
            Command::NextTab => &["]"],
            Command::PrevTab => &["["],
            Command::CloseTab => &[],
            Command::Notes => &["N"],
            Command::Review => &["G"],
            Command::PullRequest => &["H"],
            Command::RunTask => &["T"],
            Command::Rename => &["r"],
            Command::Kill => &["x"],
            Command::TakeControl => &["a"],
            Command::Detach => &["d", "q"],
            Command::Help => &["?"],
            Command::NewWorktreeTerminal => &["C"],
            Command::NewSpace => &["S"],
            Command::Usage => &["U"],
            Command::Layout => &["L"],
            Command::NewWorktreeSession => &[],
            Command::AddWorkspace | Command::RenameWorkspace | Command::Settings => &[],
        }
    }
}

/// A key with Ctrl/Alt (and Shift, for non-character keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySpec {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

const MODS: KeyModifiers = KeyModifiers::CONTROL.union(KeyModifiers::ALT);

impl KeySpec {
    /// Parses `ctrl+space`, `alt+x`, `X`, `?`, `tab`, `shift+tab`, `f2`, ...
    pub fn parse(s: &str) -> Result<KeySpec, String> {
        let mut mods = KeyModifiers::NONE;
        let mut parts: Vec<&str> = s.split('+').collect();
        // `+` itself, or a trailing `ctrl++`.
        if s.ends_with("++") || s == "+" {
            parts.retain(|p| !p.is_empty());
            parts.push("+");
        }
        let Some((name, modifiers)) = parts.split_last() else {
            return Err(format!("empty key `{s}`"));
        };
        for m in modifiers {
            mods |= match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "c" => KeyModifiers::CONTROL,
                "alt" | "meta" | "m" => KeyModifiers::ALT,
                "shift" | "s" => KeyModifiers::SHIFT,
                other => return Err(format!("unknown modifier `{other}` in `{s}`")),
            };
        }
        let lower = name.to_ascii_lowercase();
        let code = match lower.as_str() {
            "space" => KeyCode::Char(' '),
            "enter" | "return" => KeyCode::Enter,
            "tab" if mods.contains(KeyModifiers::SHIFT) => {
                mods.remove(KeyModifiers::SHIFT);
                KeyCode::BackTab
            }
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "esc" | "escape" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            f if f.len() > 1 && f.starts_with('f') && f[1..].parse::<u8>().is_ok() => {
                let n: u8 = f[1..].parse().unwrap_or(0);
                if !(1..=12).contains(&n) {
                    return Err(format!("unknown key `{name}` in `{s}`"));
                }
                KeyCode::F(n)
            }
            _ => {
                let mut chars = name.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => {
                        let mut c = c;
                        if mods.contains(KeyModifiers::SHIFT) {
                            c = c.to_ascii_uppercase();
                            mods.remove(KeyModifiers::SHIFT);
                        }
                        if mods.contains(KeyModifiers::CONTROL) {
                            // Terminals can't tell Ctrl+A from Ctrl+Shift+A.
                            c = c.to_ascii_lowercase();
                        }
                        KeyCode::Char(c)
                    }
                    _ => return Err(format!("unknown key `{name}` in `{s}`")),
                }
            }
        };
        Ok(KeySpec { code, mods })
    }

    pub fn matches(&self, key: &KeyEvent) -> bool {
        match (self.code, key.code) {
            // A character already reflects Shift, and terminals disagree on
            // whether they report it as a modifier too.
            (KeyCode::Char(a), KeyCode::Char(b)) => {
                let b = if key.modifiers.contains(KeyModifiers::CONTROL) {
                    b.to_ascii_lowercase()
                } else {
                    b
                };
                a == b && key.modifiers & MODS == self.mods & MODS
            }
            (a, b) => a == b && key.modifiers == self.mods,
        }
    }

    pub fn event(&self) -> KeyEvent {
        KeyEvent::new(self.code, self.mods)
    }

    /// A plain character or Shift-only key would swallow normal typing.
    fn is_typing_key(&self) -> bool {
        matches!(
            self.code,
            KeyCode::Char(_) | KeyCode::Enter | KeyCode::Tab | KeyCode::Backspace
        ) && !self.mods.intersects(MODS)
    }
}

impl fmt::Display for KeySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.contains(KeyModifiers::CONTROL) {
            f.write_str("Ctrl+")?;
        }
        if self.mods.contains(KeyModifiers::ALT) {
            f.write_str("Alt+")?;
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            f.write_str("Shift+")?;
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("Space"),
            KeyCode::Char(c) if self.mods.is_empty() => write!(f, "{c}"),
            KeyCode::Char(c) => write!(f, "{}", c.to_ascii_uppercase()),
            KeyCode::BackTab => f.write_str("Shift+Tab"),
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::PageUp => f.write_str("PgUp"),
            KeyCode::PageDown => f.write_str("PgDn"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeymapError {
    #[error("[keys] {0}")]
    BadKey(String),
    #[error("[keys.bind] unknown command `{0}`; commands: {list}", list = Command::ALL.map(Command::name).join(", "))]
    UnknownCommand(String),
    #[error(
        "[keys] leader `{0}` would swallow normal typing; use Ctrl or Alt (e.g. \"ctrl+space\", \"ctrl+a\") or an F-key"
    )]
    TypingLeader(String),
    #[error("[keys.bind] `{key}` is bound to both {a} and {b}")]
    Conflict {
        key: String,
        a: &'static str,
        b: &'static str,
    },
    #[error("[keys.bind] `{0}` is reserved (1-9 jump to sessions; the leader sends itself)")]
    Reserved(String),
    #[error(
        "[keys.global] `{0}` would swallow normal typing; global keys need Ctrl or Alt (or an F-key)"
    )]
    TypingGlobal(String),
    #[error("[keys] jump_modifier `{0}` must be \"alt\", \"ctrl+alt\" or \"\" (off)")]
    BadJumpModifier(String),
}

/// What a global chord does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Global {
    Run(Command),
    /// Switch to the nth session (0-based), like the leader's 1-9.
    Jump(usize),
}

#[derive(Debug, Clone)]
pub struct Keymap {
    pub leader: KeySpec,
    bindings: Vec<(KeySpec, Command)>,
    globals: Vec<(KeySpec, Command)>,
    /// Modifiers that turn 1-9 into global session jumps; empty when off.
    jump: KeyModifiers,
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap::from_config(&Keys::default(), &[]).expect("default keys are valid")
    }
}

impl Keymap {
    /// `hidden` commands (disabled integrations) get no keys, so their
    /// defaults never clash with the user's bindings.
    pub fn from_config(cfg: &Keys, hidden: &[Command]) -> Result<Keymap, KeymapError> {
        let leader = KeySpec::parse(&cfg.leader).map_err(KeymapError::BadKey)?;
        if leader.is_typing_key() {
            return Err(KeymapError::TypingLeader(cfg.leader.clone()));
        }
        for name in cfg.bind.keys() {
            if !Command::ALL.iter().any(|c| c.name() == name) {
                return Err(KeymapError::UnknownCommand(name.clone()));
            }
        }
        for name in cfg.global.keys() {
            if !Command::ALL.iter().any(|c| c.name() == name) {
                return Err(KeymapError::UnknownCommand(name.clone()));
            }
        }
        let jump = match cfg.jump_modifier.to_ascii_lowercase().as_str() {
            "" | "none" => KeyModifiers::NONE,
            "alt" => KeyModifiers::ALT,
            "ctrl+alt" | "alt+ctrl" => MODS,
            _ => return Err(KeymapError::BadJumpModifier(cfg.jump_modifier.clone())),
        };
        let mut globals: Vec<(KeySpec, Command)> = Vec::new();
        for cmd in Command::ALL.into_iter().filter(|c| !hidden.contains(c)) {
            let keys: Vec<&str> = match cfg.global.get(cmd.name()) {
                Some(list) => list.keys(),
                None => cmd.default_global_keys().to_vec(),
            };
            for k in keys {
                let spec = KeySpec::parse(k).map_err(KeymapError::BadKey)?;
                if spec.is_typing_key()
                    || (spec.mods.is_empty() && !matches!(spec.code, KeyCode::F(_)))
                {
                    return Err(KeymapError::TypingGlobal(k.to_string()));
                }
                let jumps = !jump.is_empty()
                    && spec.mods == jump
                    && matches!(spec.code, KeyCode::Char('1'..='9'));
                if spec == leader || jumps {
                    return Err(KeymapError::Reserved(k.to_string()));
                }
                if let Some((_, other)) = globals.iter().find(|(s, _)| *s == spec) {
                    return Err(KeymapError::Conflict {
                        key: k.to_string(),
                        a: other.name(),
                        b: cmd.name(),
                    });
                }
                globals.push((spec, cmd));
            }
        }
        let mut bindings: Vec<(KeySpec, Command)> = Vec::new();
        for cmd in Command::ALL.into_iter().filter(|c| !hidden.contains(c)) {
            let keys: Vec<&str> = match cfg.bind.get(cmd.name()) {
                Some(list) => list.keys(),
                None => cmd.default_keys().to_vec(),
            };
            for k in keys {
                let spec = KeySpec::parse(k).map_err(KeymapError::BadKey)?;
                let digit = matches!(spec.code, KeyCode::Char('1'..='9')) && spec.mods.is_empty();
                if digit || spec == leader {
                    return Err(KeymapError::Reserved(k.to_string()));
                }
                if let Some((_, other)) = bindings.iter().find(|(s, _)| *s == spec) {
                    return Err(KeymapError::Conflict {
                        key: k.to_string(),
                        a: other.name(),
                        b: cmd.name(),
                    });
                }
                bindings.push((spec, cmd));
            }
        }
        Ok(Keymap {
            leader,
            bindings,
            globals,
            jump,
        })
    }

    /// The global chord `key` triggers, if any.
    pub fn global(&self, key: &KeyEvent) -> Option<Global> {
        if !self.jump.is_empty()
            && let KeyCode::Char(c @ '1'..='9') = key.code
            && key.modifiers & MODS == self.jump
        {
            return Some(Global::Jump(c as usize - '1' as usize));
        }
        self.globals
            .iter()
            .find(|(spec, _)| spec.matches(key))
            .map(|(_, c)| Global::Run(*c))
    }

    pub fn global_keys_for(&self, cmd: Command) -> Vec<KeySpec> {
        self.globals
            .iter()
            .filter(|(_, c)| *c == cmd)
            .map(|(s, _)| *s)
            .collect()
    }

    /// How global session jumps are written, e.g. `Alt+1-9`.
    pub fn jump_hint(&self) -> Option<String> {
        (!self.jump.is_empty()).then(|| {
            let spec = KeySpec {
                code: KeyCode::Char('1'),
                mods: self.jump,
            };
            format!("{spec}-9")
        })
    }

    pub fn is_leader(&self, key: &KeyEvent) -> bool {
        self.leader.matches(key)
    }

    /// The command a key runs after the leader.
    pub fn command(&self, key: &KeyEvent) -> Option<Command> {
        self.bindings
            .iter()
            .find(|(spec, _)| spec.matches(key))
            .map(|(_, c)| *c)
    }

    pub fn keys_for(&self, cmd: Command) -> Vec<KeySpec> {
        self.bindings
            .iter()
            .filter(|(_, c)| *c == cmd)
            .map(|(s, _)| *s)
            .collect()
    }

    /// The first key for `cmd`, as shown in hints (`p`), if bound.
    pub fn hint(&self, cmd: Command) -> Option<String> {
        self.keys_for(cmd).first().map(ToString::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_core::config::KeyList;

    fn ev(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn parses_and_matches_keys() {
        let k = KeySpec::parse("ctrl+space").unwrap();
        assert!(k.matches(&ev(KeyCode::Char(' '), KeyModifiers::CONTROL)));
        assert_eq!(k.to_string(), "Ctrl+Space");
        // Ctrl+Shift+A arrives as Ctrl+A (or uppercase) depending on terminal.
        let k = KeySpec::parse("ctrl+a").unwrap();
        assert!(k.matches(&ev(
            KeyCode::Char('A'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )));
        // Uppercase means Shift; Shift reported as a modifier or not.
        let k = KeySpec::parse("X").unwrap();
        assert!(k.matches(&ev(KeyCode::Char('X'), KeyModifiers::SHIFT)));
        assert!(k.matches(&ev(KeyCode::Char('X'), KeyModifiers::NONE)));
        assert!(!k.matches(&ev(KeyCode::Char('x'), KeyModifiers::NONE)));
        assert_eq!(
            KeySpec::parse("shift+x").unwrap(),
            KeySpec::parse("X").unwrap()
        );
        assert_eq!(KeySpec::parse("shift+tab").unwrap().code, KeyCode::BackTab);
        assert_eq!(KeySpec::parse("F2").unwrap().code, KeyCode::F(2));
        assert_eq!(KeySpec::parse("ctrl++").unwrap().code, KeyCode::Char('+'));
        assert!(KeySpec::parse("hyper+x").is_err());
        assert!(KeySpec::parse("f13").is_err());
        assert!(KeySpec::parse("nope").is_err());
    }

    #[test]
    fn defaults_cover_the_documented_keys() {
        let km = Keymap::default();
        let cmd = |c: char| km.command(&ev(KeyCode::Char(c), KeyModifiers::NONE));
        assert_eq!(cmd(':'), Some(Command::Palette));
        assert_eq!(cmd('/'), Some(Command::SearchSessions));
        assert_eq!(cmd('c'), Some(Command::NewSession));
        assert_eq!(cmd('n'), Some(Command::NextSession));
        assert_eq!(cmd('p'), Some(Command::PrevSession));
        assert_eq!(cmd('b'), Some(Command::ToggleSidebar));
        assert_eq!(cmd('j'), Some(Command::NextSession));
        assert_eq!(cmd('q'), Some(Command::Detach));
        assert_eq!(
            km.command(&ev(KeyCode::Down, KeyModifiers::NONE)),
            Some(Command::NextSession)
        );
        assert!(km.is_leader(&ev(KeyCode::Char(' '), KeyModifiers::CONTROL)));
        assert!(km.keys_for(Command::Settings).is_empty());
    }

    #[test]
    fn global_chords_default_like_herdr_and_validate() {
        let km = Keymap::default();
        assert_eq!(
            km.global(&ev(KeyCode::Char('3'), KeyModifiers::ALT)),
            Some(Global::Jump(2))
        );
        assert_eq!(
            km.global(&ev(KeyCode::Right, KeyModifiers::ALT)),
            Some(Global::Run(Command::NextTab))
        );
        assert_eq!(
            km.global(&ev(KeyCode::Up, KeyModifiers::ALT)),
            Some(Global::Run(Command::PrevWorkspace))
        );
        assert_eq!(km.global(&ev(KeyCode::Char('3'), KeyModifiers::NONE)), None);
        assert_eq!(km.global(&ev(KeyCode::Right, KeyModifiers::NONE)), None);
        assert_eq!(km.jump_hint().as_deref(), Some("Alt+1-9"));

        let mut keys = Keys {
            jump_modifier: String::new(),
            ..Keys::default()
        };
        keys.global.insert("next_tab".into(), KeyList::Many(vec![]));
        keys.global
            .insert("help".into(), KeyList::One("ctrl+alt+h".into()));
        let km = Keymap::from_config(&keys, &[]).unwrap();
        assert_eq!(km.global(&ev(KeyCode::Char('3'), KeyModifiers::ALT)), None);
        assert_eq!(km.global(&ev(KeyCode::Right, KeyModifiers::ALT)), None);
        assert_eq!(
            km.global(&ev(KeyCode::Char('h'), MODS)),
            Some(Global::Run(Command::Help))
        );

        let mut typing = Keys::default();
        typing
            .global
            .insert("help".into(), KeyList::One("h".into()));
        assert!(matches!(
            Keymap::from_config(&typing, &[]).unwrap_err(),
            KeymapError::TypingGlobal(_)
        ));
        let mut jump = Keys::default();
        jump.global
            .insert("help".into(), KeyList::One("alt+4".into()));
        assert!(matches!(
            Keymap::from_config(&jump, &[]).unwrap_err(),
            KeymapError::Reserved(_)
        ));
        let bad = Keys {
            jump_modifier: "shift".into(),
            ..Keys::default()
        };
        assert!(matches!(
            Keymap::from_config(&bad, &[]).unwrap_err(),
            KeymapError::BadJumpModifier(_)
        ));
    }

    #[test]
    fn config_rebinds_unbinds_and_validates() {
        let mut keys = Keys {
            leader: "ctrl+a".into(),
            ..Keys::default()
        };
        keys.bind
            .insert("kill_session".into(), KeyList::One("K".into()));
        keys.bind.insert("detach".into(), KeyList::Many(vec![]));
        keys.bind
            .insert("settings".into(), KeyList::One("f2".into()));
        let km = Keymap::from_config(&keys, &[]).unwrap();
        assert!(km.is_leader(&ev(KeyCode::Char('a'), KeyModifiers::CONTROL)));
        assert_eq!(
            km.command(&ev(KeyCode::Char('K'), KeyModifiers::SHIFT)),
            Some(Command::Kill)
        );
        assert_eq!(
            km.command(&ev(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            km.command(&ev(KeyCode::Char('d'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            km.command(&ev(KeyCode::F(2), KeyModifiers::NONE)),
            Some(Command::Settings)
        );

        let err = |keys: Keys| Keymap::from_config(&keys, &[]).unwrap_err();
        let typing = Keys {
            leader: "a".into(),
            ..Keys::default()
        };
        assert!(matches!(err(typing), KeymapError::TypingLeader(_)));
        let mut conflict = Keys::default();
        conflict
            .bind
            .insert("help".into(), KeyList::One(":".into()));
        assert!(matches!(err(conflict), KeymapError::Conflict { .. }));
        let mut digit = Keys::default();
        digit.bind.insert("help".into(), KeyList::One("3".into()));
        assert!(matches!(err(digit), KeymapError::Reserved(_)));
        let mut unknown = Keys::default();
        unknown
            .bind
            .insert("explode".into(), KeyList::One("e".into()));
        assert!(matches!(err(unknown), KeymapError::UnknownCommand(_)));
    }
}
