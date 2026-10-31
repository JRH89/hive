//! Agent CLIs Hive knows by name, so they can be offered in the picker when
//! installed and recognised when started by hand inside a session.

#[derive(Debug, PartialEq, Eq)]
pub struct AgentKind {
    /// Stable id, also the default `[agents.<id>]` key.
    pub id: &'static str,
    pub label: &'static str,
    /// Executable looked up on PATH to launch it.
    pub command: &'static str,
    /// Other executable names it runs as.
    pub aliases: &'static [&'static str],
    /// Path fragments that identify it when run through a runtime such as
    /// `node`, e.g. its npm package directory.
    pub package_hints: &'static [&'static str],
    /// Bundled status manifest, if any (see [`crate::manifest::bundled`]).
    pub manifest: Option<&'static str>,
}

macro_rules! kind {
    ($id:literal, $label:literal, $cmd:literal, [$($alias:literal),*], [$($hint:literal),*], $manifest:expr) => {
        AgentKind {
            id: $id,
            label: $label,
            command: $cmd,
            aliases: &[$($alias),*],
            package_hints: &[$($hint),*],
            manifest: $manifest,
        }
    };
}

pub static CATALOG: &[AgentKind] = &[
    kind!(
        "claude",
        "Claude Code",
        "claude",
        ["claude-code"],
        ["@anthropic-ai/claude-code"],
        Some("claude")
    ),
    kind!(
        "codex",
        "Codex",
        "codex",
        [],
        ["@openai/codex"],
        Some("codex")
    ),
    kind!(
        "gemini",
        "Gemini CLI",
        "gemini",
        [],
        ["@google/gemini-cli"],
        Some("gemini")
    ),
    kind!(
        "opencode",
        "OpenCode",
        "opencode",
        ["open-code"],
        ["opencode-ai"],
        Some("opencode")
    ),
    kind!(
        "truss",
        "Truss",
        "truss-tui",
        ["truss", "truss-cli"],
        ["@truss-harness/"],
        None
    ),
    kind!(
        "cursor",
        "Cursor Agent",
        "cursor-agent",
        ["cursor"],
        [],
        Some("cursor")
    ),
    kind!(
        "copilot",
        "GitHub Copilot",
        "copilot",
        ["github-copilot", "ghcs"],
        ["@github/copilot"],
        Some("github-copilot")
    ),
    kind!("crush", "Crush", "crush", [], ["@charmland/crush"], None),
    kind!("grok", "Grok", "grok", ["grok-build"], [], Some("grok")),
    kind!(
        "pi",
        "Pi",
        "pi",
        [],
        ["@mariozechner/pi-coding-agent"],
        Some("pi")
    ),
    kind!("omp", "Oh My Pi", "omp", [], ["@oh-my-pi/"], Some("pi")),
    kind!(
        "muse",
        "Muse",
        "muse",
        ["muse-code", "muse-cli"],
        [],
        Some("muse")
    ),
    kind!(
        "amp",
        "Amp",
        "amp",
        ["amp-local"],
        ["@sourcegraph/amp"],
        Some("amp")
    ),
    kind!("droid", "Droid", "droid", [], [], Some("droid")),
    kind!("kiro", "Kiro", "kiro-cli", ["kiro"], [], Some("kiro")),
    kind!(
        "qwen",
        "Qwen Code",
        "qwen",
        ["qwen-code"],
        ["@qwen-code/"],
        Some("qwen")
    ),
    kind!("kimi", "Kimi", "kimi", ["kimi-code"], [], Some("kimi")),
    kind!(
        "kilo",
        "Kilo Code",
        "kilo",
        ["kilo-code", "kilocode"],
        ["@kilocode/"],
        Some("kilo")
    ),
    kind!("devin", "Devin", "devin", ["devin-cli"], [], Some("devin")),
    kind!("cline", "Cline", "cline", [], [], Some("cline")),
    kind!(
        "hermes",
        "Hermes",
        "hermes",
        ["hermes-agent"],
        [],
        Some("hermes")
    ),
    kind!(
        "qoder",
        "Qoder",
        "qodercli",
        ["qoder", "qoderclicn", "qodercn"],
        [],
        Some("qodercli")
    ),
    kind!(
        "letta",
        "Letta",
        "letta",
        ["letta-code"],
        ["@letta-ai/letta-code"],
        Some("letta")
    ),
    kind!("maki", "Maki", "maki", [], [], Some("maki")),
    kind!(
        "antigravity",
        "Antigravity",
        "agy",
        ["antigravity", "antigravity-cli"],
        [],
        Some("antigravity")
    ),
    kind!("aider", "Aider", "aider", [], [], None),
    kind!("goose", "Goose", "goose", [], [], None),
];

pub fn by_id(id: &str) -> Option<&'static AgentKind> {
    CATALOG.iter().find(|k| k.id == id)
}

/// The agent an executable name (a basename, no extension) belongs to.
pub fn by_executable(name: &str) -> Option<&'static AgentKind> {
    CATALOG
        .iter()
        .find(|k| k.command == name || k.id == name || k.aliases.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_and_names_are_unique_and_manifests_exist() {
        let mut names = HashSet::new();
        for k in CATALOG {
            for n in std::iter::once(k.command).chain(k.aliases.iter().copied()) {
                assert!(names.insert(n), "{n} is claimed twice");
            }
            if let Some(m) = k.manifest {
                assert!(
                    crate::manifest::bundled(m).is_some(),
                    "{}: no manifest {m}",
                    k.id
                );
            }
        }
        let ids: HashSet<_> = CATALOG.iter().map(|k| k.id).collect();
        assert_eq!(ids.len(), CATALOG.len());
    }

    #[test]
    fn truss_is_known_by_its_binaries() {
        assert_eq!(by_executable("truss-tui").unwrap().id, "truss");
        assert_eq!(by_executable("truss").unwrap().id, "truss");
        assert_eq!(by_executable("cursor-agent").unwrap().id, "cursor");
    }
}
