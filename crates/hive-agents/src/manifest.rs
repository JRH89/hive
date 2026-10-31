//! Screen-based status detection driven by herdr-compatible TOML manifests.
//!
//! A manifest is a list of rules. Each rule names a screen region and a gate
//! of matchers; the highest-priority matching rule decides the state. With no
//! match, an agent that has a manifest is assumed idle. The region and gate
//! semantics follow herdr's engine so its manifests (see `NOTICE`) work as-is.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

/// Highest manifest engine version this implementation understands.
pub const ENGINE_VERSION: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Unknown,
    Idle,
    Working,
    Blocked,
}

/// What the detector sees: the visible screen as text (rows joined by `\n`,
/// trailing blanks trimmed), the window title and the latest OSC 9 payload
/// without its leading `9;` (e.g. `4;1;50`). Empty strings mean "not seen".
#[derive(Debug, Clone, Copy, Default)]
pub struct Input<'a> {
    pub screen: &'a str,
    pub title: &'a str,
    pub progress: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub state: State,
    /// The rule positively showed this state (e.g. an idle prompt), rather
    /// than it being the no-match fallback.
    pub visible: bool,
    /// The matched rule asks for the previous state to be kept (e.g. the
    /// agent is showing a transcript viewer that hides its real state).
    pub skip_update: bool,
    pub rule: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    id: String,
    #[serde(default)]
    min_engine_version: Option<u32>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    rules: Vec<RawRule>,
}

#[derive(Debug, Deserialize)]
struct RawRule {
    id: String,
    state: Option<State>,
    #[serde(default)]
    priority: i64,
    #[serde(default = "default_region")]
    region: String,
    #[serde(default)]
    visible_idle: bool,
    #[serde(default)]
    visible_blocker: bool,
    #[serde(default)]
    visible_working: bool,
    #[serde(default)]
    skip_state_update: bool,
    #[serde(flatten)]
    gate: RawGate,
}

#[derive(Debug, Default, Deserialize)]
struct RawGate {
    #[serde(default)]
    all: Vec<RawGate>,
    #[serde(default)]
    any: Vec<RawGate>,
    #[serde(default, rename = "not")]
    not_gate: Vec<RawGate>,
    #[serde(default)]
    contains: Vec<String>,
    #[serde(default)]
    regex: Vec<String>,
    #[serde(default)]
    line_regex: Vec<String>,
}

fn default_region() -> String {
    "whole_recent".to_string()
}

#[derive(Debug)]
pub struct Manifest {
    pub id: String,
    pub aliases: Vec<String>,
    rules: Vec<Rule>,
}

#[derive(Debug)]
struct Rule {
    id: String,
    state: State,
    priority: i64,
    region: Region,
    visible: bool,
    skip_update: bool,
    gate: Gate,
}

#[derive(Debug, Default)]
struct Gate {
    all: Vec<Gate>,
    any: Vec<Gate>,
    not_gate: Vec<Gate>,
    /// Lowercased; matched against the lowercased region.
    contains: Vec<String>,
    regex: Vec<Regex>,
    line_regex: Vec<Regex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    Title,
    Progress,
    WholeRecent,
    BottomLines(usize),
    BottomNonEmptyLines(usize),
    TopNonEmptyLines(usize),
    AfterLastPromptMarker,
    BeforeCurrentPromptMarker,
    WholeRecentWithoutCurrentPromptMarker,
    CurrentPromptBlockMarker,
    AfterCurrentPromptBlockMarker,
    PromptBoxBody,
    AbovePromptBox,
    LastNonEmptyAbovePromptBox,
    AfterLastHorizontalRule,
}

impl Region {
    fn parse(spec: &str) -> Result<Self, String> {
        let spec = spec.trim();
        let counted = |name: &str| -> Option<usize> {
            let n = spec
                .strip_prefix(name)?
                .strip_prefix('(')?
                .strip_suffix(')')?;
            if n.starts_with('0') || !n.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            n.parse().ok().filter(|n| *n <= u16::MAX as usize)
        };
        Ok(match spec {
            "osc_title" => Self::Title,
            "osc_progress" => Self::Progress,
            "whole_recent" => Self::WholeRecent,
            "after_last_prompt_marker" => Self::AfterLastPromptMarker,
            "before_current_prompt_marker" => Self::BeforeCurrentPromptMarker,
            "whole_recent_without_current_prompt_marker" => {
                Self::WholeRecentWithoutCurrentPromptMarker
            }
            "current_prompt_block_marker" => Self::CurrentPromptBlockMarker,
            "after_current_prompt_block_marker" => Self::AfterCurrentPromptBlockMarker,
            "prompt_box_body" => Self::PromptBoxBody,
            "above_prompt_box" => Self::AbovePromptBox,
            "last_non_empty_above_prompt_box" => Self::LastNonEmptyAbovePromptBox,
            "after_last_horizontal_rule" => Self::AfterLastHorizontalRule,
            _ => {
                if let Some(n) = counted("bottom_lines") {
                    Self::BottomLines(n)
                } else if let Some(n) = counted("bottom_non_empty_lines") {
                    Self::BottomNonEmptyLines(n)
                } else if let Some(n) = counted("top_non_empty_lines") {
                    Self::TopNonEmptyLines(n)
                } else {
                    return Err(format!("unknown region {spec:?}"));
                }
            }
        })
    }

    fn text<'a>(self, input: &Input<'a>) -> &'a str {
        let s = input.screen;
        match self {
            Self::Title => input.title,
            Self::Progress => input.progress,
            Self::WholeRecent => s,
            Self::BottomLines(n) => bottom_lines(s, n),
            Self::BottomNonEmptyLines(n) => bottom_non_empty_lines(s, n),
            Self::TopNonEmptyLines(n) => top_non_empty_lines(s, n),
            Self::AfterLastPromptMarker => after_last_prompt_marker(s),
            Self::BeforeCurrentPromptMarker => before_current_prompt_marker(s),
            Self::WholeRecentWithoutCurrentPromptMarker => {
                let lines: Vec<&str> = s.lines().collect();
                if current_prompt_index(&lines).is_some() {
                    ""
                } else {
                    s
                }
            }
            Self::CurrentPromptBlockMarker => current_prompt_block_marker(s).unwrap_or(""),
            Self::AfterCurrentPromptBlockMarker => {
                after_current_prompt_block_marker(s).unwrap_or("")
            }
            Self::PromptBoxBody => prompt_box_body(s).unwrap_or(""),
            Self::AbovePromptBox => above_prompt_box(s),
            Self::LastNonEmptyAbovePromptBox => last_non_empty_line(above_prompt_box(s)),
            Self::AfterLastHorizontalRule => after_last_horizontal_rule(s),
        }
    }
}

impl Gate {
    fn compile(raw: &RawGate) -> Result<Self, String> {
        let nested = |gates: &[RawGate]| -> Result<Vec<Gate>, String> {
            gates.iter().map(Gate::compile).collect()
        };
        let regexes = |patterns: &[String]| -> Result<Vec<Regex>, String> {
            patterns
                .iter()
                .map(|p| Regex::new(p).map_err(|e| e.to_string()))
                .collect()
        };
        Ok(Self {
            all: nested(&raw.all)?,
            any: nested(&raw.any)?,
            not_gate: nested(&raw.not_gate)?,
            contains: raw.contains.iter().map(|n| n.to_lowercase()).collect(),
            regex: regexes(&raw.regex)?,
            line_regex: regexes(&raw.line_regex)?,
        })
    }

    fn matches(&self, text: &str, lower: &str) -> bool {
        self.contains.iter().all(|n| lower.contains(n.as_str()))
            && self.regex.iter().all(|r| r.is_match(text))
            && self
                .line_regex
                .iter()
                .all(|r| text.lines().any(|line| r.is_match(line)))
            && self.all.iter().all(|g| g.matches(text, lower))
            && (self.any.is_empty() || self.any.iter().any(|g| g.matches(text, lower)))
            && !self.not_gate.iter().any(|g| g.matches(text, lower))
    }
}

impl Manifest {
    pub fn parse(source: &str) -> Result<Self, String> {
        let raw: RawManifest = toml::from_str(source).map_err(|e| e.to_string())?;
        if let Some(v) = raw.min_engine_version
            && v > ENGINE_VERSION
        {
            return Err(format!(
                "{} needs detection engine {v}, this is {ENGINE_VERSION}",
                raw.id
            ));
        }
        let rules = raw
            .rules
            .iter()
            .map(|r| {
                let state = r.state.unwrap_or(State::Unknown);
                Ok(Rule {
                    id: r.id.clone(),
                    state,
                    priority: r.priority,
                    region: Region::parse(&r.region)?,
                    visible: match state {
                        State::Idle => r.visible_idle,
                        State::Working => r.visible_working,
                        State::Blocked => r.visible_blocker,
                        State::Unknown => false,
                    },
                    skip_update: r.skip_state_update,
                    gate: Gate::compile(&r.gate)?,
                })
                .map_err(|e: String| format!("{} rule {}: {e}", raw.id, r.id))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            id: raw.id,
            aliases: raw.aliases,
            rules,
        })
    }

    pub fn detect(&self, input: &Input<'_>) -> Detection {
        let mut best: Option<&Rule> = None;
        for rule in &self.rules {
            if best.is_some_and(|b| b.priority >= rule.priority) {
                continue;
            }
            let text = rule.region.text(input);
            if rule.gate.matches(text, &text.to_lowercase()) {
                best = Some(rule);
            }
        }
        match best {
            Some(rule) => Detection {
                state: rule.state,
                visible: rule.visible,
                skip_update: rule.skip_update,
                rule: Some(rule.id.clone()),
            },
            None => Detection {
                state: State::Idle,
                visible: false,
                skip_update: false,
                rule: None,
            },
        }
    }
}

macro_rules! bundled {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_str!(concat!("../manifests/", $name, ".toml")))),*]
    };
}

const BUNDLED: &[(&str, &str)] = bundled!(
    "amp",
    "antigravity",
    "claude",
    "cline",
    "codex",
    "cursor",
    "devin",
    "droid",
    "gemini",
    "github-copilot",
    "grok",
    "hermes",
    "kilo",
    "kimi",
    "kiro",
    "letta",
    "maki",
    "muse",
    "opencode",
    "pi",
    "qodercli",
    "qwen",
);

static MANIFESTS: LazyLock<HashMap<&'static str, Manifest>> = LazyLock::new(|| {
    BUNDLED
        .iter()
        .filter_map(|(name, src)| match Manifest::parse(src) {
            Ok(m) => Some((*name, m)),
            // A test guarantees every bundled manifest parses.
            Err(_) => None,
        })
        .collect()
});

/// The bundled manifest with this file name (see `AgentKind::manifest`).
pub fn bundled(name: &str) -> Option<&'static Manifest> {
    MANIFESTS.get(name)
}

// Region helpers. Content uses `\n` line endings, so offsets are line lengths
// plus one.

fn line_start(content: &str, lines: &[&str], index: usize) -> usize {
    lines[..index.min(lines.len())]
        .iter()
        .map(|l| l.len() + 1)
        .sum::<usize>()
        .min(content.len())
}

fn from_line<'a>(content: &'a str, lines: &[&str], index: usize) -> &'a str {
    &content[line_start(content, lines, index)..]
}

fn bottom_lines(content: &str, count: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    from_line(content, &lines, lines.len().saturating_sub(count))
}

fn bottom_non_empty_lines(content: &str, count: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, l)| !l.trim().is_empty())
        .take(count)
        .last()
        .map(|(i, _)| i);
    start.map_or("", |i| from_line(content, &lines, i))
}

fn top_non_empty_lines(content: &str, count: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    let end = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .take(count)
        .last()
        .map(|(i, _)| i);
    end.map_or("", |i| &content[..line_start(content, &lines, i + 1)])
}

/// Codex-style prompt line.
fn prompt_line(line: &str) -> bool {
    line == "›" || line.starts_with("› ")
}

fn block_marker_line(line: &str) -> bool {
    line.starts_with(['•', '■', '✗', '✓'])
}

/// The last prompt line, if no output block starts after it.
fn current_prompt_index(lines: &[&str]) -> Option<usize> {
    let i = lines.iter().rposition(|l| prompt_line(l))?;
    (!lines[i + 1..].iter().any(|l| block_marker_line(l))).then_some(i)
}

fn after_last_prompt_marker(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match lines.iter().rposition(|l| prompt_line(l)) {
        Some(i) => from_line(content, &lines, i + 1),
        None => content,
    }
}

fn before_current_prompt_marker(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match current_prompt_index(&lines) {
        Some(i) => &content[..line_start(content, &lines, i)],
        None => content,
    }
}

fn current_prompt_block_marker(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let prompt = current_prompt_index(&lines)?;
    lines[..prompt]
        .iter()
        .rev()
        .find(|l| block_marker_line(l))
        .copied()
}

fn after_current_prompt_block_marker(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let prompt = current_prompt_index(&lines)?;
    let block = lines[..prompt].iter().rposition(|l| block_marker_line(l))?;
    Some(from_line(content, &lines, block))
}

fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    let rule_chars = trimmed.chars().take_while(|&c| c == '─').count();
    if rule_chars == 0 {
        return false;
    }
    let suffix = trimmed[rule_chars * '─'.len_utf8()..].trim_start();
    suffix.is_empty() || rule_chars >= 3
}

/// The prompt box's top border: the second-last horizontal rule.
fn prompt_box_top(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, l)| is_horizontal_rule(l))
        .nth(1)
        .map(|(i, _)| i)
}

fn prompt_box_body(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let top = prompt_box_top(&lines)?;
    let end = lines[top + 1..]
        .iter()
        .position(|l| is_horizontal_rule(l))
        .map_or(lines.len(), |r| top + 1 + r);
    Some(&content[line_start(content, &lines, top + 1)..line_start(content, &lines, end)])
}

fn above_prompt_box(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match prompt_box_top(&lines) {
        Some(top) => &content[..line_start(content, &lines, top)],
        None => content,
    }
}

fn after_last_horizontal_rule(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match lines.iter().rposition(|l| is_horizontal_rule(l)) {
        Some(i) => from_line(content, &lines, i + 1),
        None => content,
    }
}

fn last_non_empty_line(content: &str) -> &str {
    content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(s: &str) -> Input<'_> {
        Input {
            screen: s,
            ..Default::default()
        }
    }

    #[test]
    fn every_bundled_manifest_parses() {
        for (name, src) in BUNDLED {
            let m = Manifest::parse(src).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!m.rules.is_empty(), "{name} has no rules");
        }
        assert_eq!(MANIFESTS.len(), BUNDLED.len());
    }

    #[test]
    fn gates_follow_herdr_semantics() {
        let m = Manifest::parse(
            r#"
            id = "t"
            [[rules]]
            id = "busy"
            state = "working"
            priority = 10
            region = "bottom_non_empty_lines(2)"
            contains = ["ESC TO INTERRUPT"]
            not = [{ contains = ["allow?"] }]
            [[rules]]
            id = "ask"
            state = "blocked"
            priority = 20
            visible_blocker = true
            any = [{ contains = ["allow?"] }, { line_regex = ['^\s*\[y/n\]'] }]
            "#,
        )
        .unwrap();
        let busy = m.detect(&screen("old\n\n✻ thinking… esc to interrupt\n"));
        assert_eq!((busy.state, busy.visible), (State::Working, false));
        let ask = m.detect(&screen("run rm?\n  [y/n]"));
        assert_eq!((ask.state, ask.visible), (State::Blocked, true));
        // Out of region: the marker is above the bottom two non-empty lines.
        let idle = m.detect(&screen("esc to interrupt\na\nb"));
        assert_eq!((idle.state, idle.rule), (State::Idle, None));
    }

    #[test]
    fn regions_slice_like_herdr() {
        let s = "head\n────\nbody one\nbody two\n────\nfooter";
        assert_eq!(prompt_box_body(s), Some("body one\nbody two\n"));
        assert_eq!(above_prompt_box(s), "head\n");
        assert_eq!(after_last_horizontal_rule(s), "footer");
        assert_eq!(bottom_non_empty_lines("a\n\nb\n\n", 2), "a\n\nb\n\n");
        assert_eq!(top_non_empty_lines("\na\n\nb\nc", 2), "\na\n\nb\n");
        let codex = "• did a thing\n› next\n";
        assert_eq!(current_prompt_block_marker(codex), Some("• did a thing"));
        assert_eq!(before_current_prompt_marker(codex), "• did a thing\n");
        assert!(is_horizontal_rule("─── title ──"));
        assert!(!is_horizontal_rule("─ x"));
        assert_eq!(Region::parse("bottom_lines(0)").ok(), None);
    }

    #[test]
    fn claude_spinner_title_is_working() {
        let claude = bundled("claude").unwrap();
        let d = claude.detect(&Input {
            title: "⠋ Claude Code",
            ..Default::default()
        });
        assert_eq!(d.state, State::Working);
        assert_eq!(claude.detect(&screen("> ")).state, State::Idle);
    }
}
