//! `hive setup` — interactive guided wizard to detect installed agents and
//! configure `[roles.*]` in `~/.config/hive/config.toml`.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::Context;
use hive_core::config::Config;
use hive_core::launch;

struct Preset {
    name: &'static str,
    purpose: &'static str,
    worktree: bool,
    strengths: &'static [&'static str],
    instructions: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "lead",
        purpose: "Orchestrator / planner (breaks goals into tasks and delegates)",
        worktree: false,
        strengths: &["planning", "orchestration", "architecture"],
        instructions: "Break goals into tasks on the Hive board and delegate to specialist roles.",
    },
    Preset {
        name: "frontend",
        purpose: "Frontend / UI implementation",
        worktree: true,
        strengths: &["frontend", "ui", "css", "react", "web", "page", "component"],
        instructions: "Implement the assigned frontend task, verify builds/tests, and report back.",
    },
    Preset {
        name: "backend",
        purpose: "Backend / API / systems implementation",
        worktree: true,
        strengths: &["backend", "api", "rust", "database", "auth"],
        instructions: "Implement the assigned backend task, run tests, and report back.",
    },
    Preset {
        name: "reviewer",
        purpose: "Independent code reviewer (ideally a different provider from the coder)",
        worktree: false,
        strengths: &["review", "audit", "pr", "diff"],
        instructions: "Review the task branch diff for bugs, edge cases, and regressions.",
    },
];

pub(crate) fn run(config_path: &Path, config: &Config) -> anyhow::Result<()> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let installed: Vec<String> = config
        .agents()
        .into_iter()
        .filter(|(name, def)| {
            name != "shell" && launch::find_executable(&def.command, &path_var).is_some()
        })
        .map(|(name, _)| name)
        .collect();

    println!("◇ HIVE — Multi-Agent Role Setup\n");
    if installed.is_empty() {
        println!("No known AI coding agents were found on PATH (only `shell`).");
        println!("You can still enter any command name you have installed.\n");
    } else {
        println!("Detected installed agents:");
        for (i, name) in installed.iter().enumerate() {
            println!("  [{}] {}", i + 1, name);
        }
        println!();
    }

    let mut generated = String::new();
    let default_agent = installed
        .first()
        .cloned()
        .unwrap_or_else(|| config.general.default_agent.clone());

    for preset in PRESETS {
        println!("Role `{}` — {}", preset.name, preset.purpose);
        let agent_input = prompt(&format!(
            "  Agent (number, name, or 'skip') [{}]: ",
            default_agent
        ))?;
        if agent_input.eq_ignore_ascii_case("skip") || agent_input == "-" {
            println!("  skipped\n");
            continue;
        }
        let agent = if agent_input.is_empty() {
            default_agent.clone()
        } else if let Ok(idx) = agent_input.parse::<usize>()
            && idx >= 1
            && idx <= installed.len()
        {
            installed[idx - 1].clone()
        } else {
            agent_input
        };

        let model = prompt("  Model override (optional, Enter for provider default): ")?;
        let args = if model.is_empty() {
            Vec::new()
        } else {
            model_args(&agent, &model)
        };

        let wt_default = if preset.worktree { "Y/n" } else { "y/N" };
        let wt_in = prompt(&format!("  Isolate in git worktree? [{wt_default}]: "))?;
        let worktree = match wt_in.to_ascii_lowercase().as_str() {
            "y" | "yes" => true,
            "n" | "no" => false,
            _ => preset.worktree,
        };

        generated.push_str(&format!("\n[roles.{}]\n", preset.name));
        generated.push_str(&format!("agent = {:?}\n", agent));
        if !args.is_empty() {
            generated.push_str(&format!("args = {:?}\n", args));
        }
        generated.push_str(&format!("worktree = {}\n", worktree));
        generated.push_str(&format!("strengths = {:?}\n", preset.strengths));
        generated.push_str(&format!("instructions = {:?}\n", preset.instructions));
        println!();
    }

    if generated.is_empty() {
        println!("No roles configured. Nothing written.");
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(config_path).unwrap_or_default();
    let merged = if existing.trim().is_empty() {
        generated.trim_start().to_string()
    } else {
        format!("{}\n{}", existing.trim_end(), generated)
    };

    // Validate before writing so we never break the user's config.toml.
    Config::parse(&merged).context("generated config failed validation")?;
    fs::write(config_path, merged)?;

    println!("Saved roles to {}", config_path.display());
    println!("Run `hive roles` to verify, or `hive board` (Ctrl+Space B) to start.");
    Ok(())
}

fn model_args(agent: &str, model: &str) -> Vec<String> {
    match agent {
        "codex" => vec!["-m".into(), model.into()],
        _ => vec!["--model".into(), model.into()],
    }
}

fn prompt(label: &str) -> anyhow::Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}
