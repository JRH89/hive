//! `hive context`: agents and scripts retrieve and record project knowledge
//! in the space's notes vault (ADR-0027). Plain text by default, grep-like
//! `path:line` so agents can open the spot; `--json` for programs.

use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use clap::Subcommand;
use hive_notes::Vault;
use serde_json::json;

#[derive(Subcommand)]
pub enum ContextAction {
    /// Find notes about something, best first
    #[command(after_help = "Example:\n  hive context search worktree cleanup rules")]
    Search {
        /// Words to look for
        #[arg(required = true)]
        query: Vec<String>,
        /// How many notes to list
        #[arg(short = 'n', long, default_value_t = 5)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Print a note, with what it links to and what links to it
    Get {
        /// A link name ("ADR-0012"), a path in the vault, or a title
        note: String,
        #[arg(long)]
        json: bool,
    },
    /// List notes, most recently changed first
    List {
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Append a dated entry to a note, creating it if needed
    #[command(
        after_help = "The text is the remaining arguments, or standard input.\n\
        Example:\n  hive context add \"Hive/Decisions\" \"Auth tokens now expire after 1h\""
    )]
    Add {
        /// Note path in the vault, e.g. "Hive/Decisions" (.md is added)
        note: String,
        /// Text of the entry (otherwise read from standard input)
        text: Vec<String>,
        /// Who is writing, shown in the entry's heading
        #[arg(long)]
        by: Option<String>,
    },
    /// Print the vault's path
    Path,
}

pub fn run(action: ContextAction, vault: Option<PathBuf>) -> anyhow::Result<()> {
    let root = match vault {
        Some(v) => v,
        None => {
            let cwd = std::env::current_dir()?;
            hive_notes::locate(&cwd).with_context(|| {
                format!(
                    "no notes vault for {}: a folder with .obsidian/ in this repository. \
                     Pass --vault DIR or set HIVE_VAULT.",
                    cwd.display()
                )
            })?
        }
    };
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }
    match action {
        ContextAction::Path => println!("{}", root.display()),
        ContextAction::Search { query, limit, json } => {
            let vault = open(&root)?;
            let hits = vault.search(&query.join(" "), limit);
            if json {
                let out: Vec<_> = hits
                    .iter()
                    .map(|h| {
                        json!({
                            "path": vault.notes[h.note].path,
                            "line": h.line,
                            "score": (h.score * 100.0).round() / 100.0,
                            "snippet": h.snippet,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if hits.is_empty() {
                println!("No notes match. Try fewer or other words.");
            } else {
                for h in &hits {
                    println!("{}:{}", vault.notes[h.note].path, h.line);
                    println!("    {}", h.snippet);
                }
            }
        }
        ContextAction::Get { note, json } => {
            let vault = open(&root)?;
            let Some(i) = vault.find(&note) else {
                let close: Vec<String> = vault
                    .search(&note, 3)
                    .iter()
                    .map(|h| vault.notes[h.note].path.clone())
                    .collect();
                if close.is_empty() {
                    bail!("no note named {note:?}");
                }
                bail!("no note named {note:?}. Close: {}", close.join(", "));
            };
            let n = &vault.notes[i];
            let text = std::fs::read_to_string(vault.root.join(&n.path))?;
            let names = |ids: &[usize]| -> Vec<String> {
                ids.iter().map(|&j| vault.notes[j].path.clone()).collect()
            };
            let links = names(vault.links(i));
            let backlinks = names(&vault.backlinks(i));
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "path": n.path,
                        "title": n.title,
                        "tags": n.tags,
                        "links": links,
                        "backlinks": backlinks,
                        "text": text,
                    }))?
                );
            } else {
                println!("{}", n.path);
                println!("{}", "=".repeat(n.path.chars().count()));
                print!("{text}");
                if !text.ends_with('\n') {
                    println!();
                }
                if !links.is_empty() || !backlinks.is_empty() {
                    println!();
                }
                if !links.is_empty() {
                    println!("Links to: {}", links.join(", "));
                }
                if !backlinks.is_empty() {
                    println!("Linked from: {}", backlinks.join(", "));
                }
            }
        }
        ContextAction::List { limit, json } => {
            let vault = open(&root)?;
            let mut ids: Vec<usize> = (0..vault.notes.len()).collect();
            ids.sort_by_key(|&i| std::cmp::Reverse(vault.notes[i].modified));
            ids.truncate(limit);
            if json {
                let out: Vec<_> = ids
                    .iter()
                    .map(|&i| {
                        let n = &vault.notes[i];
                        json!({ "path": n.path, "title": n.title, "tags": n.tags })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                for i in ids {
                    let n = &vault.notes[i];
                    let tags = if n.tags.is_empty() {
                        String::new()
                    } else {
                        format!("  #{}", n.tags.join(" #"))
                    };
                    println!("{}{tags}", n.path);
                }
            }
        }
        ContextAction::Add { note, text, by } => {
            let mut body = text.join(" ");
            if body.trim().is_empty() {
                if std::io::stdin().is_terminal() {
                    bail!("no text: pass it after the note name, or pipe it in");
                }
                std::io::stdin().read_to_string(&mut body)?;
            }
            if body.trim().is_empty() {
                bail!("no text to add");
            }
            let entry = entry(&body, by.as_deref(), &jiff::Zoned::now());
            let path = hive_notes::append(&root, &note, &entry)?;
            println!("Added to {}", path.display());
        }
    }
    Ok(())
}

fn open(root: &Path) -> anyhow::Result<Vault> {
    Vault::open(root).with_context(|| format!("could not read the vault at {}", root.display()))
}

/// `## 2026-11-02 14:05 · claude (session 51)` then the text, so entries
/// from several agents stay apart and dated.
fn entry(text: &str, by: Option<&str>, now: &jiff::Zoned) -> String {
    let session = std::env::var("HIVE_SESSION_ID")
        .ok()
        .filter(|s| !s.is_empty());
    let who = match (by, session) {
        (Some(b), Some(s)) => format!(" · {b} (session {s})"),
        (Some(b), None) => format!(" · {b}"),
        (None, Some(s)) => format!(" · session {s}"),
        (None, None) => String::new(),
    };
    format!(
        "\n## {}{who}\n\n{}\n",
        now.strftime("%Y-%m-%d %H:%M"),
        text.trim_end()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_dated_and_signed() {
        let now: jiff::Zoned = "2026-11-02T14:05:00-08:00[America/Los_Angeles]"
            .parse()
            .unwrap();
        let e = entry("Tokens expire after 1h.\n\n", Some("claude"), &now);
        assert!(e.starts_with("\n## 2026-11-02 14:05 · claude"));
        assert!(e.ends_with("\n\nTokens expire after 1h.\n"));
    }
}
