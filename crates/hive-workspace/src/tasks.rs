//! Project tasks a user may want to run beside an agent: `just` recipes,
//! package.json scripts, make targets and the usual cargo commands. Read
//! from the files directly; nothing is executed to list them.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Shown in the picker, e.g. `just test`.
    pub title: String,
    /// What runs, invoked directly (never through a shell).
    pub argv: Vec<String>,
}

impl Task {
    fn new(argv: &[&str]) -> Task {
        Task {
            title: argv.join(" "),
            argv: argv.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Tasks defined in `dir`, in a stable order: just, package scripts, make,
/// cargo.
pub fn find_tasks(dir: &Path) -> Vec<Task> {
    let mut tasks = Vec::new();
    let read = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| std::fs::read_to_string(dir.join(n)).ok())
    };
    if let Some(text) = read(&["justfile", "Justfile", ".justfile"]) {
        tasks.extend(just_recipes(&text).iter().map(|r| Task::new(&["just", r])));
    }
    if let Some(text) = read(&["package.json"]) {
        let runner = package_runner(dir);
        tasks.extend(
            package_scripts(&text)
                .iter()
                .map(|s| Task::new(&[runner, "run", s])),
        );
    }
    if let Some(text) = read(&["GNUmakefile", "makefile", "Makefile"]) {
        tasks.extend(make_targets(&text).iter().map(|t| Task::new(&["make", t])));
    }
    if dir.join("Cargo.toml").is_file() {
        for sub in ["build", "test", "run", "clippy"] {
            tasks.push(Task::new(&["cargo", sub]));
        }
    }
    tasks
}

fn package_runner(dir: &Path) -> &'static str {
    let has = |f: &str| dir.join(f).exists();
    if has("bun.lock") || has("bun.lockb") {
        "bun"
    } else if has("pnpm-lock.yaml") {
        "pnpm"
    } else if has("yarn.lock") {
        "yarn"
    } else {
        "npm"
    }
}

fn package_scripts(text: &str) -> Vec<String> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    match json.get("scripts").and_then(|s| s.as_object()) {
        Some(scripts) => {
            let mut names: Vec<String> = scripts.keys().cloned().collect();
            names.sort();
            names
        }
        None => Vec::new(),
    }
}

/// Recipe names: unindented `name args...:` lines, skipping settings,
/// assignments (`:=`) and private recipes (`_name`).
fn just_recipes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t', '#', '[']) || line.contains(":=") {
            continue;
        }
        let Some((head, _)) = line.split_once(':') else {
            continue;
        };
        let name = head.trim_start_matches('@');
        let name = name.split_whitespace().next().unwrap_or("");
        let keyword = ["set", "alias", "export", "import", "mod"].contains(&name);
        let valid = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !name.is_empty()
            && valid
            && !keyword
            && !name.starts_with('_')
            && !out.contains(&name.to_string())
        {
            out.push(name.to_string());
        }
    }
    out
}

/// Explicit targets: unindented `name:` lines, skipping special (`.PHONY`),
/// pattern (`%`) and variable targets, and `:=`/`::=` assignments.
fn make_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t', '#', '.']) {
            continue;
        }
        let Some((head, rest)) = line.split_once(':') else {
            continue;
        };
        if rest.starts_with('=') || rest.starts_with(":=") || head.contains('=') {
            continue;
        }
        for name in head.split_whitespace() {
            let valid = name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c));
            if valid && !out.contains(&name.to_string()) {
                out.push(name.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_recipes_skip_settings_assignments_and_private_ones() {
        let text = "set shell := [\"bash\"]\nversion := \"1\"\nalias t := test\n\n# Build it\n@build:\n    cargo build\n\ntest filter='': build\n    cargo test {{filter}}\n_helper:\n    true\n[private]\nhidden:\n";
        assert_eq!(just_recipes(text), vec!["build", "test", "hidden"]);
    }

    #[test]
    fn make_targets_skip_special_pattern_and_variable_lines() {
        let text = ".PHONY: all test\nCC := gcc\nall: build\nbuild test:\n\t$(CC) x.c\n%.o: %.c\n\tcc\nout/bin: x\n";
        assert_eq!(make_targets(text), vec!["all", "build", "test", "out/bin"]);
    }

    #[test]
    fn tasks_come_from_every_kind_of_project_file() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        std::fs::write(d.join("justfile"), "check:\n    true\n").unwrap();
        std::fs::write(
            d.join("package.json"),
            r#"{"scripts":{"dev":"vite","build":"vite build"}}"#,
        )
        .unwrap();
        std::fs::write(d.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(d.join("Makefile"), "install:\n\tcp a b\n").unwrap();
        let titles: Vec<String> = find_tasks(d).into_iter().map(|t| t.title).collect();
        assert_eq!(
            titles,
            vec![
                "just check",
                "pnpm run build",
                "pnpm run dev",
                "make install"
            ]
        );
        std::fs::write(d.join("Cargo.toml"), "").unwrap();
        let tasks = find_tasks(d);
        assert_eq!(tasks.last().unwrap().argv, vec!["cargo", "clippy"]);
        assert!(find_tasks(&d.join("missing")).is_empty());
    }
}
