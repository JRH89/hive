//! Hive's own key bindings: the leader key and the commands that follow it.
//!
//! Everything else goes to the agent, so only the leader is intercepted
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
    NewWorktreeSession,
    Workspaces,
    AddWorkspace,
    RenameWorkspace,
    NextSession,
    PrevSession,
    Sidebar,
    Rename,
    Kill,
    TakeControl,
    Detach,
    Help,
    Settings,
}

impl Command {
    pub const ALL: [Command; 16] = [
        Command::Palette,
        Command::SearchSessions,
        Command::NewSession,
        Command::NewWorktreeSession,
        Command::Workspaces,
        Command::AddWorkspace,
        Command::RenameWorkspace,
        Command::NextSession,
        Command::PrevSession,
        Command::Sidebar,
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
            Command::NewWorktreeSession => "new_worktree_session",
            Command::Workspaces => "workspaces",
            Command::AddWorkspace => "add_workspace",
            Command::RenameWorkspace => "rename_workspace",
            Command::NextSession => "next_session",
            Command::PrevSession => "prev_session",
            Command::Sidebar => "sidebar",
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
            Command::NewSession => "New session",
            Command::NewWorktreeSession => "New session in its own worktree",
            Command::Workspaces => "Switch workspace",
            Command::AddWorkspace => "Add workspace",
            Command::RenameWorkspace => "Rename workspace",
            Command::NextSession => "Next session",
            Command::PrevSession => "Previous session",
            Command::Sidebar => "Focus the sidebar",
            Command::Rename => "Rename session",
            Command::Kill => "Terminate session",
            Command::TakeControl => "Take input control",
            Command::Detach => "Detach (agents keep running)",
            Command::Help => "Help",
            Command::Settings => "Settings (config file location)",
        }
    }

    fn default_keys(self) -> &'static [&'static str] {
        match self {
            Command::Palette => &["p"],
            Command::SearchSessions => &["/"],
            Command::NewSession => &["n"],
            Command::Workspaces => &["w"],
            Command::NextSession => &["j", "down"],
            Command::PrevSession => &["k", "up"],
            Command::Sidebar => &["s", "tab"],
            Command::Rename => &["r"],
            Command::Kill => &["x"],
            Command::TakeControl => &["a"],
            Command::Detach => &["d", "q"],
            Command::Help => &["?"],
            Command::NewWorktreeSession
            | Command::AddWorkspace
            | Command::RenameWorkspace
            | Command::Settings => &[],
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
}

#[derive(Debug, Clone)]
pub struct Keymap {
    pub leader: KeySpec,
    bindings: Vec<(KeySpec, Command)>,
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap::from_config(&Keys::default()).expect("default keys are valid")
    }
}

impl Keymap {
    pub fn from_config(cfg: &Keys) -> Result<Keymap, KeymapError> {
        let leader = KeySpec::parse(&cfg.leader).map_err(KeymapError::BadKey)?;
        if leader.is_typing_key() {
            return Err(KeymapError::TypingLeader(cfg.leader.clone()));
        }
        for name in cfg.bind.keys() {
            if !Command::ALL.iter().any(|c| c.name() == name) {
                return Err(KeymapError::UnknownCommand(name.clone()));
            }
        }
        let mut bindings: Vec<(KeySpec, Command)> = Vec::new();
        for cmd in Command::ALL {
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
        Ok(Keymap { leader, bindings })
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
        assert_eq!(cmd('p'), Some(Command::Palette));
        assert_eq!(cmd('/'), Some(Command::SearchSessions));
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
    fn config_rebinds_unbinds_and_validates() {
        let mut keys = Keys {
            leader: "ctrl+a".into(),
            ..Keys::default()
        };
        keys.bind
            .insert("kill_session".into(), KeyList::One("X".into()));
        keys.bind.insert("detach".into(), KeyList::Many(vec![]));
        keys.bind
            .insert("settings".into(), KeyList::One("f2".into()));
        let km = Keymap::from_config(&keys).unwrap();
        assert!(km.is_leader(&ev(KeyCode::Char('a'), KeyModifiers::CONTROL)));
        assert_eq!(
            km.command(&ev(KeyCode::Char('X'), KeyModifiers::SHIFT)),
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

        let err = |keys: Keys| Keymap::from_config(&keys).unwrap_err();
        let typing = Keys {
            leader: "a".into(),
            ..Keys::default()
        };
        assert!(matches!(err(typing), KeymapError::TypingLeader(_)));
        let mut conflict = Keys::default();
        conflict
            .bind
            .insert("help".into(), KeyList::One("p".into()));
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
