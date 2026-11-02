//! A markdown vault, Obsidian-compatible, as Hive sees it: notes, their
//! links and tags, backlinks and ranked search (ADR-0027). The files are the
//! only state; a [`Vault`] is a fresh read of them, cheap enough to redo on
//! every command.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

/// Notes bigger than this are skipped: they're data, not notes.
const MAX_NOTE_BYTES: u64 = 2 * 1024 * 1024;
/// Directories never scanned, besides hidden ones (`.obsidian`, `.git`).
const SKIP_DIRS: &[&str] = &["node_modules", "target"];

#[derive(Debug, Clone)]
pub struct Note {
    /// Path relative to the vault root, `/`-separated, with `.md`.
    pub path: String,
    /// The file name without `.md`, which is what links name.
    pub title: String,
    /// The text after any frontmatter.
    pub body: String,
    /// 1-based line of the file where `body` starts.
    pub body_line: usize,
    /// Link targets as written, without `#heading` or `|alias`.
    pub links: Vec<String>,
    /// Tags from frontmatter and `#tags` in the text, without `#`.
    pub tags: Vec<String>,
    /// Frontmatter `aliases`: other names links may use.
    pub aliases: Vec<String>,
    pub modified: Option<SystemTime>,
}

#[derive(Debug)]
pub struct Vault {
    pub root: PathBuf,
    /// Sorted by path.
    pub notes: Vec<Note>,
    /// For each note, the notes its links resolve to.
    outgoing: Vec<Vec<usize>>,
    /// Lowercase file names and aliases → notes, for resolving links.
    names: std::collections::HashMap<String, Vec<usize>>,
    /// Lowercase paths without `.md`, by note.
    paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub note: usize,
    pub score: f64,
    /// 1-based line in the file of the best matching line.
    pub line: usize,
    pub snippet: String,
}

impl Vault {
    /// Reads every `.md` file under `root`, skipping hidden directories.
    pub fn open(root: &Path) -> io::Result<Vault> {
        let root = plain(root.canonicalize()?);
        let mut files = Vec::new();
        walk(&root, &mut files)?;
        let mut notes: Vec<Note> = files
            .into_iter()
            .filter_map(|path| {
                let meta = std::fs::metadata(&path).ok()?;
                if meta.len() > MAX_NOTE_BYTES {
                    return None;
                }
                let text = std::fs::read_to_string(&path).ok()?;
                let rel = rel_path(&root, &path)?;
                Some(Note::parse(&rel, &text, meta.modified().ok()))
            })
            .collect();
        notes.sort_by(|a, b| a.path.cmp(&b.path));
        let mut names: std::collections::HashMap<String, Vec<usize>> = Default::default();
        for (i, n) in notes.iter().enumerate() {
            for name in std::iter::once(&n.title).chain(&n.aliases) {
                names.entry(name.to_lowercase()).or_default().push(i);
            }
        }
        let paths = notes
            .iter()
            .map(|n| strip_md(&n.path).to_lowercase())
            .collect();
        let mut vault = Vault {
            root,
            notes,
            outgoing: Vec::new(),
            names,
            paths,
        };
        vault.outgoing = (0..vault.notes.len())
            .map(|i| {
                let mut to: Vec<usize> = vault.notes[i]
                    .links
                    .iter()
                    .filter_map(|l| vault.resolve(l))
                    .collect();
                to.sort_unstable();
                to.dedup();
                to
            })
            .collect();
        Ok(vault)
    }

    /// The note a link names, as Obsidian resolves it: a path from the root
    /// (with or without `.md`), else a file name or alias anywhere,
    /// ignoring case; the shallowest wins.
    pub fn resolve(&self, link: &str) -> Option<usize> {
        let link = link.trim().replace('\\', "/");
        let link = link.trim_start_matches("./");
        let bare = strip_md(link).to_lowercase();
        if bare.is_empty() {
            return None;
        }
        let matches: Vec<usize> = if bare.contains('/') {
            let tail = format!("/{bare}");
            (0..self.notes.len())
                .filter(|&i| self.paths[i] == bare || self.paths[i].ends_with(&tail))
                .collect()
        } else {
            self.names.get(&bare).cloned().unwrap_or_default()
        };
        matches
            .into_iter()
            .min_by_key(|&i| (self.notes[i].path.matches('/').count(), i))
    }

    /// A note by link, by path relative to the root, or by the start of
    /// its file name when only one note begins that way ("ADR-0012").
    pub fn find(&self, name: &str) -> Option<usize> {
        let name = name.replace('\\', "/");
        let exact = self
            .notes
            .iter()
            .position(|n| n.path == name || strip_md(&n.path) == strip_md(&name))
            .or_else(|| self.resolve(&name));
        exact.or_else(|| {
            let prefix = strip_md(&name).to_lowercase();
            let mut starts = (0..self.notes.len()).filter(|&i| {
                !prefix.is_empty() && self.notes[i].title.to_lowercase().starts_with(&prefix)
            });
            match (starts.next(), starts.next()) {
                (Some(i), None) => Some(i),
                _ => None,
            }
        })
    }

    pub fn links(&self, note: usize) -> &[usize] {
        &self.outgoing[note]
    }

    /// Notes linking to `note`.
    pub fn backlinks(&self, note: usize) -> Vec<usize> {
        (0..self.notes.len())
            .filter(|&i| i != note && self.outgoing[i].contains(&note))
            .collect()
    }

    /// Links in `note` that resolve to nothing.
    pub fn unresolved(&self, note: usize) -> Vec<&str> {
        self.notes[note]
            .links
            .iter()
            .filter(|l| self.resolve(l).is_none())
            .map(String::as_str)
            .collect()
    }

    /// Notes ranked for `query`: BM25 over the text, with the file name,
    /// tags and folders weighted up and the exact phrase rewarded.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let terms: Vec<String> = {
            let mut seen = BTreeSet::new();
            tokens(query).filter(|t| seen.insert(t.clone())).collect()
        };
        if terms.is_empty() || self.notes.is_empty() {
            return Vec::new();
        }
        // Per note: how often each query term occurs, and its length.
        let (docs, lens): (Vec<BTreeMap<String, usize>>, Vec<f64>) = self
            .notes
            .iter()
            .map(|n| {
                let mut counts = BTreeMap::new();
                let mut len = 0usize;
                for t in tokens(&n.body) {
                    len += 1;
                    if terms.contains(&t) {
                        *counts.entry(t).or_insert(0) += 1;
                    }
                }
                (counts, len as f64)
            })
            .unzip();
        let named: Vec<BTreeSet<String>> = self
            .notes
            .iter()
            .map(|n| {
                tokens(&n.title)
                    .chain(n.aliases.iter().flat_map(|a| tokens(a)))
                    .collect()
            })
            .collect();
        let avg = (lens.iter().sum::<f64>() / lens.len() as f64).max(1.0);
        let n = self.notes.len() as f64;
        let phrase = (terms.len() > 1).then(|| query.trim().to_lowercase());
        let (k1, b) = (1.2, 0.75);
        let idfs: Vec<f64> = terms
            .iter()
            .map(|t| {
                let df = (0..self.notes.len())
                    .filter(|&j| docs[j].contains_key(t) || named[j].contains(t))
                    .count() as f64;
                (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
            })
            .collect();

        let mut hits = Vec::new();
        for (i, note) in self.notes.iter().enumerate() {
            let tags: BTreeSet<String> = note.tags.iter().flat_map(|t| tokens(t)).collect();
            let dirs: BTreeSet<String> = note
                .path
                .rsplit_once('/')
                .map(|(d, _)| tokens(d).collect())
                .unwrap_or_default();
            let mut score = 0.0;
            for (t, &idf) in terms.iter().zip(&idfs) {
                if let Some(&tf) = docs[i].get(t) {
                    let tf = tf as f64;
                    score += idf * tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * lens[i] / avg));
                }
                if named[i].contains(t) {
                    score += 2.0 * idf;
                }
                if tags.contains(t) {
                    score += 1.5 * idf;
                }
                if dirs.contains(t) {
                    score += 0.5 * idf;
                }
            }
            if score <= 0.0 {
                continue;
            }
            if let Some(p) = &phrase
                && note.body.to_lowercase().contains(p.as_str())
            {
                score *= 1.5;
            }
            let (line, snippet) = best_line(note, &terms);
            hits.push(Hit {
                note: i,
                score,
                line,
                snippet,
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| self.notes[a.note].path.cmp(&self.notes[b.note].path))
        });
        hits.truncate(limit);
        hits
    }
}

impl Note {
    pub fn parse(path: &str, text: &str, modified: Option<SystemTime>) -> Note {
        let (front, body, body_line) = split_frontmatter(text);
        let mut tags = front_list(front, "tags");
        tags.extend(front_list(front, "tag"));
        let aliases = {
            let mut a = front_list(front, "aliases");
            a.extend(front_list(front, "alias"));
            a
        };
        let mut links = Vec::new();
        for (line, in_code) in lines_with_code(body) {
            if in_code {
                continue;
            }
            let line = strip_inline_code(line);
            links.extend(wikilinks(&line).into_iter().map(|l| l.target));
            links.extend(md_links(&line));
            tags.extend(hashtags(&line));
        }
        let mut seen = BTreeSet::new();
        tags.retain(|t| seen.insert(t.to_lowercase()));
        let title = path
            .rsplit('/')
            .next()
            .map(strip_md)
            .unwrap_or(path)
            .to_string();
        Note {
            path: path.to_string(),
            title,
            body: body.to_string(),
            body_line,
            links,
            tags,
            aliases,
            modified,
        }
    }
}

/// A `[[target#heading|alias]]` (or `![[…]]` embed) in a line, with its
/// byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub start: usize,
    pub end: usize,
    pub target: String,
    pub heading: Option<String>,
    pub alias: Option<String>,
    pub embed: bool,
}

impl WikiLink {
    /// What to show: the alias, else the target (and heading).
    pub fn text(&self) -> String {
        match (&self.alias, &self.heading) {
            (Some(a), _) => a.clone(),
            (None, Some(h)) if self.target.is_empty() => h.clone(),
            (None, Some(h)) => format!("{} › {h}", self.target),
            (None, None) => self.target.clone(),
        }
    }
}

/// Every wikilink in one line.
pub fn wikilinks(line: &str) -> Vec<WikiLink> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(open) = line[from..].find("[[").map(|i| i + from) {
        let Some(close) = line[open + 2..].find("]]").map(|i| i + open + 2) else {
            break;
        };
        let inner = &line[open + 2..close];
        if inner.contains("[[") {
            from = open + 2;
            continue;
        }
        let (rest, alias) = match inner.split_once('|') {
            Some((r, a)) => (r, Some(a.trim().to_string()).filter(|a| !a.is_empty())),
            None => (inner, None),
        };
        let (target, heading) = match rest.split_once('#') {
            Some((t, h)) => (t, Some(h.trim().to_string()).filter(|h| !h.is_empty())),
            None => (rest, None),
        };
        let embed = open > 0 && line.as_bytes()[open - 1] == b'!';
        out.push(WikiLink {
            start: if embed { open - 1 } else { open },
            end: close + 2,
            target: target.trim().to_string(),
            heading,
            alias,
            embed,
        });
        from = close + 2;
    }
    out
}

/// Targets of `[text](note.md)` links to other notes (not URLs).
fn md_links(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = line[from..].find("](").map(|i| i + from) {
        let start = i + 2;
        let Some(end) = line[start..].find(')').map(|e| e + start) else {
            break;
        };
        let target = line[start..end].split_whitespace().next().unwrap_or("");
        let target = target.split('#').next().unwrap_or("");
        if !target.contains("://") && target.to_lowercase().ends_with(".md") {
            out.push(target.replace("%20", " "));
        }
        from = end;
    }
    out
}

/// `#tags` in a line: after whitespace or at the start, letters, digits,
/// `_`, `-` and `/`, not all digits (so `#1` is not a tag).
fn hashtags(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    for (k, &(i, c)) in chars.iter().enumerate() {
        if c != '#' || (k > 0 && !chars[k - 1].1.is_whitespace()) {
            continue;
        }
        let tag: String = line[i + 1..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
            .collect();
        if !tag.is_empty() && !tag.chars().all(|c| c.is_ascii_digit()) {
            out.push(tag);
        }
    }
    out
}

/// Lines of `text`, each flagged when inside a fenced code block (the
/// fences included).
pub fn lines_with_code(text: &str) -> impl Iterator<Item = (&str, bool)> {
    let mut fence: Option<&'static str> = None;
    text.lines().map(move |line| {
        let t = line.trim_start();
        let opener = ["```", "~~~"].into_iter().find(|f| t.starts_with(f));
        match (fence, opener) {
            (None, Some(f)) => {
                fence = Some(f);
                (line, true)
            }
            (Some(f), Some(g)) if f == g => {
                fence = None;
                (line, true)
            }
            (Some(_), _) => (line, true),
            (None, None) => (line, false),
        }
    })
}

/// The line with `code spans` blanked, so links and tags in them are
/// ignored. Byte positions are kept.
fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
            out.push(' ');
        } else if in_code {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        } else {
            out.push(c);
        }
    }
    out
}

/// `(frontmatter, body, first body line)`. Frontmatter is a `---` block
/// at the very top.
pub fn split_frontmatter(text: &str) -> (&str, &str, usize) {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return ("", text, 1);
    };
    let mut offset = 0;
    for (n, line) in rest.split_inclusive('\n').enumerate() {
        let t = line.trim_end();
        if t == "---" || t == "..." {
            let front = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return (front, body, n + 3);
        }
        offset += line.len();
    }
    ("", text, 1)
}

/// A list-valued frontmatter key: `key: [a, b]`, `key: a, b` or `key:`
/// followed by `- a` lines.
fn front_list(front: &str, key: &str) -> Vec<String> {
    let clean = |s: &str| {
        s.trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .trim_start_matches('#')
            .to_string()
    };
    let mut lines = front.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(value) = line
            .strip_prefix(key)
            .and_then(|r| r.trim_start().strip_prefix(':'))
        else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            let mut out = Vec::new();
            while let Some(item) = lines.peek().and_then(|l| l.trim_start().strip_prefix("- ")) {
                out.push(clean(item));
                lines.next();
            }
            return out.into_iter().filter(|s| !s.is_empty()).collect();
        }
        let value = value.trim_start_matches('[').trim_end_matches(']');
        return value
            .split([',', ' '])
            .map(clean)
            .filter(|s| !s.is_empty())
            .collect();
    }
    Vec::new()
}

/// Lowercase search tokens, with a trailing plural `s` dropped so
/// "worktrees" finds "worktree".
pub fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(|w| {
            let w = w.to_lowercase();
            match w.strip_suffix('s') {
                Some(stem) if stem.chars().count() >= 3 && !stem.ends_with('s') => stem.to_string(),
                _ => w,
            }
        })
}

/// The line matching the most terms (earliest on ties), trimmed to about
/// 160 characters around the first match.
fn best_line(note: &Note, terms: &[String]) -> (usize, String) {
    let mut best = (0, 0usize, "");
    for (n, (line, _)) in lines_with_code(&note.body).enumerate() {
        let words: BTreeSet<String> = tokens(line).collect();
        let found = terms.iter().filter(|t| words.contains(*t)).count();
        if found > best.0 {
            best = (found, n, line);
        }
    }
    if best.0 == 0 {
        // Matched by name or tag only: the first non-empty line.
        if let Some((n, line)) = note
            .body
            .lines()
            .enumerate()
            .find(|(_, l)| !l.trim().is_empty())
        {
            best = (0, n, line);
        }
    }
    let line = best.2.trim();
    let lower = line.to_lowercase();
    let at = terms
        .iter()
        .filter_map(|t| lower.find(t.as_str()))
        .min()
        .unwrap_or(0);
    let chars: Vec<char> = line.chars().collect();
    let at = lower[..at.min(lower.len())]
        .chars()
        .count()
        .min(chars.len());
    let from = at.saturating_sub(40);
    let to = (from + 160).min(chars.len());
    let mut snippet: String = chars[from..to].iter().collect();
    if from > 0 {
        snippet.insert(0, '…');
    }
    if to < chars.len() {
        snippet.push('…');
    }
    (note.body_line + best.1, snippet)
}

/// How many notes there are and when the newest changed: cheap to compute,
/// so a viewer can poll it and reload only when something moved.
pub fn fingerprint(root: &Path) -> (usize, Option<SystemTime>) {
    let mut files = Vec::new();
    let _ = walk(root, &mut files);
    let newest = files
        .iter()
        .filter_map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())
        .max();
    (files.len(), newest)
}

/// Windows' canonical form `\\?\C:\…` as plain `C:\…`, which editors,
/// Obsidian and URLs understand. Network paths keep their form.
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => path,
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if !SKIP_DIRS.contains(&name.as_ref()) {
                let _ = walk(&path, out);
            }
        } else if kind.is_file() && name.to_lowercase().ends_with(".md") {
            out.push(path);
        }
    }
    Ok(())
}

fn rel_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some(parts.join("/"))
}

fn strip_md(s: &str) -> &str {
    if s.len() >= 3 && s[s.len() - 3..].eq_ignore_ascii_case(".md") {
        &s[..s.len() - 3]
    } else {
        s
    }
}

/// The vault for work in `cwd`: `$HIVE_VAULT` if set, else a folder with
/// `.obsidian/` at or above `cwd` within its repository, else one directly
/// inside the repository root (`vault/`, `docs/`, …).
pub fn locate(cwd: &Path) -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("HIVE_VAULT").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(v));
    }
    find_vault(cwd)
}

/// [`locate`] without `$HIVE_VAULT`: the vault the directory tree holds.
pub fn find_vault(cwd: &Path) -> Option<PathBuf> {
    let root = hive_workspace::discover(cwd)
        .map(|r| r.worktree_root)
        .unwrap_or_else(|| cwd.to_path_buf());
    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    hive_workspace::find_obsidian_vault(&cwd, &root)
}

/// A title as a file name: characters Obsidian and filesystems reject
/// become `-`.
pub fn file_name(title: &str) -> String {
    let name: String = title
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let name = name.trim_matches('.').trim();
    if name.is_empty() {
        "Untitled".into()
    } else {
        name.to_string()
    }
}

/// The path of note `rel` (relative, `.md` added if missing) inside `root`,
/// refusing anything that would leave the vault.
pub fn note_path(root: &Path, rel: &str) -> io::Result<PathBuf> {
    let rel = rel.replace('\\', "/");
    let rel = if rel.to_lowercase().ends_with(".md") {
        rel
    } else {
        format!("{rel}.md")
    };
    let path = Path::new(&rel);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{rel}: notes must be inside the vault"),
        ));
    }
    Ok(root.join(path))
}

/// Appends `text` to note `rel` (created with its folders if missing).
/// Never rewrites what is there.
pub fn append(root: &Path, rel: &str, text: &str) -> io::Result<PathBuf> {
    use std::io::Write as _;
    let path = note_path(root, rel)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let existing = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    // Start on a fresh line after existing text.
    if existing > 0 {
        let ends_newline = std::fs::read(&path)
            .map(|b| b.ends_with(b"\n"))
            .unwrap_or(true);
        if !ends_newline {
            file.write_all(b"\n")?;
        }
    }
    file.write_all(text.as_bytes())?;
    if !text.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(files: &[(&str, &str)]) -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let p = dir.path().join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        let v = Vault::open(dir.path()).unwrap();
        (dir, v)
    }

    #[test]
    fn notes_are_parsed_for_links_tags_and_frontmatter() {
        let text = "---\ntags: [arch, \"daemon\"]\naliases:\n  - Lifecycle\n---\n# Session\nSee [[ADR-0009 Persist#Decision|the ADR]] and ![[diagram.png]].\n\
            Also [old](Old%20Note.md) and https://x.y. #status/done #1\n```\n[[not a link]] #nope\n```\n`[[code]]`";
        let n = Note::parse("Architecture/Session Lifecycle.md", text, None);
        assert_eq!(n.title, "Session Lifecycle");
        assert_eq!(n.body_line, 6);
        assert!(n.body.starts_with("# Session"));
        assert_eq!(
            n.links,
            vec!["ADR-0009 Persist", "diagram.png", "Old Note.md"]
        );
        assert_eq!(n.tags, vec!["arch", "daemon", "status/done"]);
        assert_eq!(n.aliases, vec!["Lifecycle"]);
    }

    #[test]
    fn wikilinks_carry_heading_alias_and_range() {
        let line = "a [[Note#Part|shown]] b ![[img.png]] [[x]]";
        let links = wikilinks(line);
        assert_eq!(links.len(), 3);
        assert_eq!(&line[links[0].start..links[0].end], "[[Note#Part|shown]]");
        assert_eq!(links[0].text(), "shown");
        assert!(links[1].embed && &line[links[1].start..links[1].end] == "![[img.png]]");
        let h = &wikilinks("[[Note#Part]]")[0];
        assert_eq!(h.text(), "Note › Part");
    }

    #[test]
    fn links_resolve_like_obsidian_and_backlinks_follow() {
        let (_d, v) = vault(&[
            (
                "Home.md",
                "Start: [[status]], [[Decisions/ADR-1]], [[Lifecycle]], [[missing]]",
            ),
            ("00-Index/Status.md", "Status. Back to [[home]]."),
            ("Decisions/ADR-1.md", "decision"),
            ("Deep/x/Status.md", "a deeper status"),
            ("Arch.md", "---\naliases: [Lifecycle]\n---\nbody"),
        ]);
        let id = |p: &str| v.notes.iter().position(|n| n.path == p).unwrap();
        // Case-insensitive name; the shallower of two Status notes wins.
        assert_eq!(v.resolve("status"), Some(id("00-Index/Status.md")));
        assert_eq!(
            v.resolve("Decisions/ADR-1.md"),
            Some(id("Decisions/ADR-1.md"))
        );
        assert_eq!(v.resolve("x/Status"), Some(id("Deep/x/Status.md")));
        assert_eq!(v.resolve("Lifecycle"), Some(id("Arch.md")));
        assert_eq!(v.unresolved(id("Home.md")), vec!["missing"]);
        assert_eq!(v.backlinks(id("Home.md")), vec![id("00-Index/Status.md")]);
        assert_eq!(v.links(id("Home.md")).len(), 3);
        assert_eq!(v.find("Deep/x/Status.md"), Some(id("Deep/x/Status.md")));
        // A unique start of a name is enough; an ambiguous one isn't.
        assert_eq!(v.find("adr"), Some(id("Decisions/ADR-1.md")));
        assert_eq!(v.find("Sta"), None);
    }

    #[test]
    fn windows_verbatim_paths_are_made_plain() {
        assert_eq!(plain(PathBuf::from(r"\\?\C:\v")), PathBuf::from(r"C:\v"));
        let unc = PathBuf::from(r"\\?\UNC\host\share");
        assert_eq!(plain(unc.clone()), unc);
        assert_eq!(plain(PathBuf::from("/a/b")), PathBuf::from("/a/b"));
    }

    #[test]
    fn hidden_and_build_folders_are_not_notes() {
        let (_d, v) = vault(&[
            ("a.md", "a"),
            (".obsidian/workspace.md", "x"),
            ("node_modules/pkg/README.md", "x"),
            ("b.txt", "x"),
        ]);
        let paths: Vec<&str> = v.notes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(paths, vec!["a.md"]);
    }

    #[test]
    fn search_ranks_names_tags_and_phrases_and_points_at_the_line() {
        let (_d, v) = vault(&[
            (
                "Worktree rules.md",
                "Branches are kept.\nDirty worktrees are never removed.",
            ),
            (
                "Daemon.md",
                "The daemon owns PTYs.\nIt never removes a worktree by itself.",
            ),
            ("Notes.md", "---\ntags: [worktree]\n---\nunrelated text"),
            ("Other.md", "nothing here"),
        ]);
        let hits = v.search("worktrees", 10);
        let paths: Vec<&str> = hits.iter().map(|h| v.notes[h.note].path.as_str()).collect();
        assert_eq!(paths[0], "Worktree rules.md", "{paths:?}");
        assert!(paths.contains(&"Daemon.md") && paths.contains(&"Notes.md"));
        assert!(!paths.contains(&"Other.md"));
        assert_eq!(hits[0].line, 2);
        assert!(hits[0].snippet.contains("worktrees are never removed"));
        // The exact phrase lifts a note.
        let hits = v.search("owns PTYs", 10);
        assert_eq!(v.notes[hits[0].note].path, "Daemon.md");
        assert_eq!(hits[0].line, 1);
        assert!(v.search("  ", 10).is_empty());
        assert_eq!(v.search("worktree", 1).len(), 1);
    }

    #[test]
    fn appending_creates_and_never_rewrites_or_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let p = append(dir.path(), "Hive/Decisions", "first").unwrap();
        assert_eq!(p, dir.path().join("Hive/Decisions.md"));
        std::fs::write(&p, "first\nno newline").unwrap();
        append(dir.path(), "Hive/Decisions.md", "second").unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "first\nno newline\nsecond\n"
        );
        assert!(append(dir.path(), "../out", "x").is_err());
        assert!(append(dir.path(), "/etc/x", "x").is_err());
        assert_eq!(file_name(" a/b: c? "), "a-b- c-");
        assert_eq!(file_name("..."), "Untitled");
    }

    #[test]
    fn snippets_are_trimmed_on_character_boundaries() {
        let long = format!("{} needle {}", "é".repeat(100), "ü".repeat(200));
        let (_d, v) = vault(&[("long.md", &long)]);
        let hit = &v.search("needle", 1)[0];
        assert!(hit.snippet.starts_with('…') && hit.snippet.ends_with('…'));
        assert!(hit.snippet.contains("needle"));
        assert!(hit.snippet.chars().count() <= 162);
    }
}
