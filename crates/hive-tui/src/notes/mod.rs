//! `hive notes`: a lazygit-style app for a markdown vault (ADR-0027). It
//! runs in a pane like any program; the vault's files are its only state,
//! so Obsidian and Hive always show the same notes.
//!
//! [`NotesApp`] is a pure state machine (keys in, [`Action`]s out); [`run`]
//! owns the terminal and carries the actions out.

mod md;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use hive_core::config::Config;
use hive_notes::{Hit, Vault};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Theme;
use md::{Rendered, Target};

/// What a note's render depends on: the note, the width and the
/// highlighted link.
type RenderKey = (usize, u16, Option<usize>);

/// A row of the note tree.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Folder { path: String, depth: usize },
    Note { note: usize, depth: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Tree,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Input {
    None,
    Search(String),
    NewNote(String),
}

/// What the app asks its runner to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    /// Edit this file in the user's editor, then come back.
    Edit(PathBuf),
    /// Hand a URL or file to the system (browser, Obsidian, viewer).
    Open(String),
}

pub struct NotesApp {
    pub vault: Vault,
    collapsed: BTreeSet<String>,
    rows: Vec<Row>,
    selected: usize,
    focus: Focus,
    /// The note shown on the right.
    current: Option<usize>,
    /// Notes left by following links, for Backspace.
    history: Vec<usize>,
    scroll: usize,
    /// The highlighted link of the current note.
    link: Option<usize>,
    input: Input,
    hits: Vec<Hit>,
    /// The note shown when the search began: Esc returns to it, and
    /// opening a hit keeps it in the history.
    before_search: Option<usize>,
    status: Option<String>,
    help: bool,
    /// `[integrations.obsidian] journal_dir`: where daily notes go.
    daily_dir: String,
    /// The last render of the current note, keyed by what it depends on.
    cache: Option<(RenderKey, Rendered)>,
    /// Geometry of the last frame, for scrolling and the mouse.
    tree_area: Rect,
    tree_offset: usize,
    note_area: Rect,
}

impl NotesApp {
    pub fn open(root: &Path, daily_dir: &str) -> std::io::Result<NotesApp> {
        let vault = Vault::open(root)?;
        let mut app = NotesApp {
            vault,
            collapsed: BTreeSet::new(),
            rows: Vec::new(),
            selected: 0,
            focus: Focus::Tree,
            current: None,
            history: Vec::new(),
            scroll: 0,
            link: None,
            input: Input::None,
            hits: Vec::new(),
            before_search: None,
            status: None,
            help: false,
            daily_dir: daily_dir.to_string(),
            cache: None,
            tree_area: Rect::default(),
            tree_offset: 0,
            note_area: Rect::default(),
        };
        app.rebuild();
        // Start on the vault's home note if it has one, else the first.
        let home = ["Home", "README", "Index", "index"]
            .iter()
            .find_map(|n| app.vault.resolve(n));
        let first = home.or_else(|| {
            app.rows.iter().find_map(|r| match r {
                Row::Note { note, .. } => Some(*note),
                Row::Folder { .. } => None,
            })
        });
        if let Some(n) = first {
            app.reveal(n);
            app.current = Some(n);
        }
        Ok(app)
    }

    /// Reads the vault again, keeping the shown note and selection by path.
    pub fn reload(&mut self) {
        let current = self.current.map(|i| self.vault.notes[i].path.clone());
        let selected = self
            .rows
            .get(self.selected)
            .cloned()
            .map(|r| self.row_key(&r));
        let history: Vec<String> = self
            .history
            .iter()
            .map(|&i| self.vault.notes[i].path.clone())
            .collect();
        let Ok(vault) = Vault::open(&self.vault.root) else {
            self.status = Some("Couldn't read the vault.".into());
            return;
        };
        self.vault = vault;
        let by_path = |v: &Vault, p: &str| v.notes.iter().position(|n| n.path == p);
        self.current = current.and_then(|p| by_path(&self.vault, &p));
        self.history = history
            .iter()
            .filter_map(|p| by_path(&self.vault, p))
            .collect();
        self.cache = None;
        self.rebuild();
        if let Some(key) = selected
            && let Some(i) = self.rows.iter().position(|r| self.row_key(r) == key)
        {
            self.selected = i;
        }
        self.selected = self.selected.min(self.rows.len().saturating_sub(1));
        if let Input::Search(q) = &self.input {
            self.hits = self.vault.search(q, 50);
        }
    }

    fn row_key(&self, row: &Row) -> String {
        match row {
            Row::Folder { path, .. } => format!("{path}/"),
            Row::Note { note, .. } => self.vault.notes[*note].path.clone(),
        }
    }

    /// The visible tree: folders first, then notes, each sorted without
    /// regard to case; nothing under a collapsed folder.
    fn rebuild(&mut self) {
        let mut folders = BTreeSet::new();
        for n in &self.vault.notes {
            let mut parts: Vec<&str> = n.path.split('/').collect();
            parts.pop();
            for k in 1..=parts.len() {
                folders.insert(parts[..k].join("/"));
            }
        }
        let mut rows = Vec::new();
        self.walk_tree("", 0, &folders, &mut rows);
        self.rows = rows;
    }

    fn walk_tree(&self, dir: &str, depth: usize, folders: &BTreeSet<String>, rows: &mut Vec<Row>) {
        let parent = |p: &str| {
            p.rsplit_once('/')
                .map(|(d, _)| d.to_string())
                .unwrap_or_default()
        };
        let mut subs: Vec<&String> = folders.iter().filter(|f| parent(f) == dir).collect();
        subs.sort_by_key(|f| f.to_lowercase());
        for f in subs {
            rows.push(Row::Folder {
                path: f.clone(),
                depth,
            });
            if !self.collapsed.contains(f) {
                self.walk_tree(f, depth + 1, folders, rows);
            }
        }
        let mut notes: Vec<usize> = (0..self.vault.notes.len())
            .filter(|&i| parent(&self.vault.notes[i].path) == dir)
            .collect();
        notes.sort_by_key(|&i| self.vault.notes[i].title.to_lowercase());
        rows.extend(notes.into_iter().map(|note| Row::Note { note, depth }));
    }

    /// Opens the folders above `note` and selects its row.
    fn reveal(&mut self, note: usize) {
        let path = self.vault.notes[note].path.clone();
        let mut changed = false;
        let mut dir = String::new();
        for part in path
            .split('/')
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .skip(1)
            .rev()
        {
            dir = if dir.is_empty() {
                part.to_string()
            } else {
                format!("{dir}/{part}")
            };
            changed |= self.collapsed.remove(&dir);
        }
        if changed {
            self.rebuild();
        }
        if let Some(i) = self.rows.iter().position(|r| {
            *r == Row::Note {
                note,
                depth: r_depth(r),
            }
        }) {
            self.selected = i;
        }
    }

    /// Shows `note` on the right; `follow` keeps the way back.
    fn show(&mut self, note: usize, follow: bool) {
        if follow
            && let Some(c) = self.current
            && c != note
        {
            self.history.push(c);
        }
        if self.current != Some(note) {
            self.scroll = 0;
            self.link = None;
        }
        self.current = Some(note);
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Option<Action> {
        self.status = None;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            return Some(Action::Quit);
        }
        if self.help {
            self.help = false;
            return None;
        }
        match self.input.clone() {
            Input::Search(q) => return self.on_search_key(key, q),
            Input::NewNote(name) => return self.on_new_key(key, name),
            Input::None => {}
        }
        match key.code {
            KeyCode::Char('q') => return Some(Action::Quit),
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('/') => {
                self.input = Input::Search(String::new());
                self.before_search = self.current;
                self.hits.clear();
                self.selected = 0;
                self.focus = Focus::Tree;
            }
            KeyCode::Char('n') => self.input = Input::NewNote(String::new()),
            KeyCode::Char('t') => return self.today(),
            KeyCode::Char('r') => {
                self.reload();
                self.status = Some("Reloaded.".into());
            }
            KeyCode::Char('e') => {
                return self
                    .target_note()
                    .map(|n| Action::Edit(self.vault.root.join(&self.vault.notes[n].path)));
            }
            KeyCode::Char('o') => {
                return self.target_note().map(|n| {
                    let path = self.vault.root.join(&self.vault.notes[n].path);
                    Action::Open(format!(
                        "obsidian://open?path={}",
                        percent(&path.display().to_string())
                    ))
                });
            }
            KeyCode::Backspace | KeyCode::Char('b') => self.back(),
            _ => {
                return match self.focus {
                    Focus::Tree => self.on_tree_key(key),
                    Focus::Note => self.on_note_key(key),
                };
            }
        }
        None
    }

    /// The note an action applies to: the shown one, or the selected row's.
    fn target_note(&self) -> Option<usize> {
        match (self.focus, self.rows.get(self.selected)) {
            (Focus::Tree, Some(Row::Note { note, .. })) => Some(*note),
            _ => self.current,
        }
    }

    fn back(&mut self) {
        if let Some(n) = self.history.pop() {
            self.current = Some(n);
            self.scroll = 0;
            self.link = None;
            self.reveal(n);
        } else {
            self.status = Some("Nothing to go back to.".into());
        }
    }

    fn on_tree_key(&mut self, key: KeyEvent) -> Option<Action> {
        let last = self.rows.len().saturating_sub(1);
        let page = (self.tree_area.height.saturating_sub(2) as usize).max(1);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.select(self.selected + 1),
            KeyCode::Up | KeyCode::Char('k') => self.select(self.selected.saturating_sub(1)),
            KeyCode::PageDown => self.select(self.selected + page),
            KeyCode::PageUp => self.select(self.selected.saturating_sub(page)),
            KeyCode::Home | KeyCode::Char('g') => self.select(0),
            KeyCode::End | KeyCode::Char('G') => self.select(last),
            KeyCode::Tab => {
                if self.current.is_some() {
                    self.focus = Focus::Note;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                match self.rows.get(self.selected) {
                    Some(Row::Folder { path, .. }) => {
                        let path = path.clone();
                        let open = !self.collapsed.contains(&path);
                        if key.code == KeyCode::Enter && open {
                            self.collapsed.insert(path);
                        } else {
                            self.collapsed.remove(&path);
                        }
                        self.rebuild();
                    }
                    Some(Row::Note { note, .. }) => {
                        let note = *note;
                        self.show(note, false);
                        self.focus = Focus::Note;
                    }
                    None => {}
                }
            }
            KeyCode::Left | KeyCode::Char('h') => match self.rows.get(self.selected).cloned() {
                Some(Row::Folder { path, .. }) if !self.collapsed.contains(&path) => {
                    self.collapsed.insert(path);
                    self.rebuild();
                }
                Some(row) => {
                    // To the folder above.
                    let key = self.row_key(&row);
                    let key = key.trim_end_matches('/');
                    if let Some((dir, _)) = key.rsplit_once('/') {
                        let dir = dir.to_string();
                        if let Some(i) = self
                            .rows
                            .iter()
                            .position(|r| matches!(r, Row::Folder { path, .. } if *path == dir))
                        {
                            self.selected = i;
                        }
                    }
                }
                None => {}
            },
            _ => {}
        }
        None
    }

    /// Moves the tree selection; the preview follows it, like lazygit.
    fn select(&mut self, i: usize) {
        self.selected = i.min(self.rows.len().saturating_sub(1));
        if let Some(Row::Note { note, .. }) = self.rows.get(self.selected) {
            let note = *note;
            self.show(note, false);
        }
    }

    fn on_note_key(&mut self, key: KeyEvent) -> Option<Action> {
        let height = self.note_area.height.saturating_sub(2) as usize;
        let half = (height / 2).max(1);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.scroll += 1,
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Char('d') if ctrl => self.scroll += half,
            KeyCode::Char('u') if ctrl => self.scroll = self.scroll.saturating_sub(half),
            KeyCode::Char('d') | KeyCode::PageDown | KeyCode::Char(' ') => {
                self.scroll += height.max(1)
            }
            KeyCode::Char('u') | KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(height.max(1))
            }
            KeyCode::Home | KeyCode::Char('g') => self.scroll = 0,
            KeyCode::End | KeyCode::Char('G') => self.scroll = usize::MAX,
            KeyCode::Tab => self.step_link(1),
            KeyCode::BackTab => self.step_link(-1),
            KeyCode::Enter => return self.follow(),
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => self.focus = Focus::Tree,
            _ => {}
        }
        None
    }

    fn step_link(&mut self, delta: isize) {
        let count = self.links().len();
        if count == 0 {
            self.status = Some("No links in this note.".into());
            return;
        }
        let next = match self.link {
            None if delta > 0 => 0,
            None => count - 1,
            Some(i) => (i as isize + delta).rem_euclid(count as isize) as usize,
        };
        self.link = Some(next);
        // Keep it in view.
        let line = self.links()[next].line;
        let height = (self.note_area.height.saturating_sub(2) as usize).max(1);
        if line < self.scroll || line >= self.scroll + height {
            self.scroll = line.saturating_sub(height / 3);
        }
    }

    /// The current note's links, from the last render.
    fn links(&self) -> Vec<md::LinkAt> {
        self.cache
            .as_ref()
            .map(|(_, r)| r.links.clone())
            .unwrap_or_default()
    }

    fn follow(&mut self) -> Option<Action> {
        let Some(link) = self.link.and_then(|i| self.links().get(i).cloned()) else {
            self.status = Some("Tab picks a link first.".into());
            return None;
        };
        match link.target {
            Target::Url(url) => {
                if url.contains(':') {
                    Some(Action::Open(url))
                } else {
                    // A file next to the note (an image, a PDF).
                    let dir = self
                        .current
                        .and_then(|c| {
                            self.vault
                                .root
                                .join(&self.vault.notes[c].path)
                                .parent()
                                .map(Path::to_path_buf)
                        })
                        .unwrap_or_else(|| self.vault.root.clone());
                    Some(Action::Open(
                        dir.join(url.replace("%20", " ")).display().to_string(),
                    ))
                }
            }
            Target::Note(name) => {
                let name = name.trim_start_matches("../").to_string();
                match self.vault.resolve(&name) {
                    Some(n) => {
                        self.show(n, true);
                        self.reveal(n);
                    }
                    None => {
                        // Like Obsidian: following a link to nothing creates it.
                        let rel = hive_notes::file_name(name.trim_end_matches(".md"));
                        match hive_notes::append(&self.vault.root, &rel, &format!("# {rel}\n")) {
                            Ok(_) => {
                                self.status = Some(format!("Created {rel}.md"));
                                self.open_path(&format!("{rel}.md"), true);
                            }
                            Err(e) => self.status = Some(format!("Couldn't create {rel}: {e}")),
                        }
                    }
                }
                None
            }
        }
    }

    /// Reloads and shows the note at vault path `rel`.
    fn open_path(&mut self, rel: &str, follow: bool) {
        self.reload();
        if let Some(n) = self.vault.notes.iter().position(|n| n.path == rel) {
            self.show(n, follow);
            self.reveal(n);
        }
    }

    fn on_search_key(&mut self, key: KeyEvent, mut q: String) -> Option<Action> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                self.input = Input::None;
                self.hits.clear();
                if let Some(c) = self.before_search.take() {
                    self.show(c, false);
                    self.reveal(c);
                }
                return None;
            }
            KeyCode::Enter => {
                if let Some(h) = self.hits.get(self.selected) {
                    let n = h.note;
                    self.input = Input::None;
                    self.hits.clear();
                    if let Some(b) = self.before_search.take()
                        && b != n
                    {
                        self.history.push(b);
                    }
                    self.show(n, false);
                    self.reveal(n);
                    self.focus = Focus::Note;
                }
                return None;
            }
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.hits.len().saturating_sub(1))
            }
            KeyCode::Char('n') if ctrl => {
                self.selected = (self.selected + 1).min(self.hits.len().saturating_sub(1))
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('p') if ctrl => self.selected = self.selected.saturating_sub(1),
            KeyCode::Backspace => {
                q.pop();
            }
            KeyCode::Char(c) if !ctrl => q.push(c),
            _ => {}
        }
        if self.input != Input::Search(q.clone()) {
            self.hits = self.vault.search(&q, 50);
            self.selected = 0;
            self.input = Input::Search(q);
        }
        if let Some(h) = self.hits.get(self.selected) {
            let n = h.note;
            self.show(n, false);
        }
        None
    }

    fn on_new_key(&mut self, key: KeyEvent, mut name: String) -> Option<Action> {
        match key.code {
            KeyCode::Esc => self.input = Input::None,
            KeyCode::Enter => {
                self.input = Input::None;
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                // Into the selected folder unless a folder is given.
                let dir = match self.rows.get(self.selected) {
                    _ if name.contains('/') => String::new(),
                    Some(Row::Folder { path, .. }) => format!("{path}/"),
                    Some(Row::Note { note, .. }) => self.vault.notes[*note]
                        .path
                        .rsplit_once('/')
                        .map(|(d, _)| format!("{d}/"))
                        .unwrap_or_default(),
                    None => String::new(),
                };
                let parts: Vec<String> = name.split('/').map(hive_notes::file_name).collect();
                let title = parts.last().cloned().unwrap_or_default();
                let rel = format!("{dir}{}.md", parts.join("/"));
                let path = match hive_notes::note_path(&self.vault.root, &rel) {
                    Ok(p) => p,
                    Err(e) => {
                        self.status = Some(e.to_string());
                        return None;
                    }
                };
                if !path.exists()
                    && let Err(e) =
                        hive_notes::append(&self.vault.root, &rel, &format!("# {title}\n"))
                {
                    self.status = Some(format!("Couldn't create {rel}: {e}"));
                    return None;
                }
                self.open_path(&rel, true);
                return Some(Action::Edit(path));
            }
            KeyCode::Backspace => {
                name.pop();
                self.input = Input::NewNote(name);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                name.push(c);
                self.input = Input::NewNote(name);
            }
            _ => {}
        }
        None
    }

    /// Today's daily note (created when missing), as the journal writes it.
    fn today(&mut self) -> Option<Action> {
        let date = jiff::Zoned::now().strftime("%Y-%m-%d").to_string();
        let dir = self.daily_dir.trim_matches('/');
        let rel = if dir.is_empty() {
            format!("{date}.md")
        } else {
            format!("{dir}/{date}.md")
        };
        let exists = hive_notes::note_path(&self.vault.root, &rel).is_ok_and(|p| p.exists());
        if !exists
            && let Err(e) = hive_notes::append(&self.vault.root, &rel, &format!("# {date}\n"))
        {
            self.status = Some(format!("Couldn't create {rel}: {e}"));
            return None;
        }
        self.open_path(&rel, true);
        self.focus = Focus::Note;
        None
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        let at = ratatui::layout::Position::new(m.column, m.row);
        let in_tree = self.tree_area.contains(at);
        let in_note = self.note_area.contains(at);
        match m.kind {
            MouseEventKind::ScrollDown if in_note => self.scroll += 3,
            MouseEventKind::ScrollUp if in_note => self.scroll = self.scroll.saturating_sub(3),
            MouseEventKind::ScrollDown if in_tree => self.select(self.selected + 1),
            MouseEventKind::ScrollUp if in_tree => self.select(self.selected.saturating_sub(1)),
            MouseEventKind::Down(MouseButton::Left) if in_tree && self.input == Input::None => {
                let row = (m.row.saturating_sub(self.tree_area.y + 1)) as usize + self.tree_offset;
                if row < self.rows.len() {
                    self.focus = Focus::Tree;
                    if self.selected == row {
                        self.on_tree_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    } else {
                        self.select(row);
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) if in_note && self.current.is_some() => {
                self.focus = Focus::Note
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();
        if area.height < 4 || area.width < 20 {
            return;
        }
        let main = Rect {
            height: area.height - 1,
            ..area
        };
        let bar = Rect {
            y: area.y + area.height - 1,
            height: 1,
            ..area
        };
        // One panel when narrow: the focused one.
        let (tree, note) = if area.width < 60 {
            match self.focus {
                Focus::Tree => (main, Rect::default()),
                Focus::Note => (Rect::default(), main),
            }
        } else {
            let w = (area.width * 30 / 100).clamp(24, 42);
            (
                Rect { width: w, ..main },
                Rect {
                    x: main.x + w,
                    width: main.width - w,
                    ..main
                },
            )
        };
        self.tree_area = tree;
        self.note_area = note;
        if tree.width > 0 {
            self.draw_tree(frame, tree, theme);
        }
        if note.width > 0 {
            self.draw_note(frame, note, theme);
        }
        self.draw_bar(frame, bar, theme);
        if self.help {
            draw_help(frame, area, theme);
        }
    }

    fn panel(&self, title: String, focused: bool, theme: &Theme) -> Block<'static> {
        let color = if focused { theme.accent } else { theme.muted };
        Block::new()
            .borders(Borders::ALL)
            .border_set(theme.glyphs.border)
            .border_style(Style::new().fg(color))
            .title(Span::styled(
                title,
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ))
    }

    fn draw_tree(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == Focus::Tree;
        let name = hive_notes_name(&self.vault.root);
        let title = match &self.input {
            Input::Search(_) => " Search ".to_string(),
            _ => format!(" Notes · {name} "),
        };
        let block = self.panel(title, focused, theme);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let height = inner.height as usize;
        let width = inner.width as usize;
        let mut lines: Vec<Line> = Vec::new();
        if let Input::Search(q) = &self.input {
            lines.push(Line::from(vec![
                Span::styled("/ ", Style::new().fg(theme.accent)),
                Span::styled(q.clone(), Style::new().fg(theme.text)),
                Span::styled(theme.glyphs.caret, Style::new().fg(theme.accent)),
            ]));
            if self.hits.is_empty() {
                let msg = if q.trim().is_empty() {
                    "type to search every note"
                } else {
                    "no matches"
                };
                lines.push(Line::from(Span::styled(msg, Style::new().fg(theme.muted))));
            }
            let room = height.saturating_sub(1);
            let start = (self.selected + 1).saturating_sub(room);
            for (i, h) in self.hits.iter().enumerate().skip(start).take(room) {
                let n = &self.vault.notes[h.note];
                let base = if i == self.selected {
                    theme.cursor_row()
                } else {
                    Style::new()
                };
                let folder = n.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                let text = fit(
                    &n.title,
                    width.saturating_sub(folder.len().min(width / 2) + 1),
                    theme,
                );
                lines.push(
                    Line::from(vec![
                        Span::styled(text, base.fg(theme.text)),
                        Span::styled(format!(" {folder}"), base.fg(theme.muted)),
                    ])
                    .style(base),
                );
            }
            frame.render_widget(Paragraph::new(lines), inner);
            return;
        }
        if self.rows.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No notes yet. n makes one.",
                    Style::new().fg(theme.muted),
                )),
                inner,
            );
            return;
        }
        // Scroll only as far as needed to keep the selection in view.
        if self.selected < self.tree_offset {
            self.tree_offset = self.selected;
        } else if self.selected >= self.tree_offset + height {
            self.tree_offset = self.selected + 1 - height;
        }
        for (i, row) in self
            .rows
            .iter()
            .enumerate()
            .skip(self.tree_offset)
            .take(height)
        {
            let selected = i == self.selected;
            let base = if selected {
                theme.cursor_row()
            } else {
                Style::new()
            };
            let line = match row {
                Row::Folder { path, depth } => {
                    let open = !self.collapsed.contains(path);
                    let name = path.rsplit('/').next().unwrap_or(path);
                    let arrow = if open {
                        theme.glyphs.group
                    } else {
                        theme.glyphs.collapsed
                    };
                    Line::from(vec![
                        Span::styled(
                            format!("{}{arrow} ", "  ".repeat(*depth)),
                            base.fg(theme.muted),
                        ),
                        Span::styled(
                            fit(name, width.saturating_sub(depth * 2 + 2), theme),
                            base.fg(theme.text).add_modifier(Modifier::BOLD),
                        ),
                    ])
                }
                Row::Note { note, depth } => {
                    let shown = self.current == Some(*note);
                    let style = if shown {
                        base.fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        base.fg(theme.text)
                    };
                    Line::from(vec![
                        Span::styled("  ".repeat(*depth + 1), base),
                        Span::styled(
                            fit(
                                &self.vault.notes[*note].title,
                                width.saturating_sub(depth * 2 + 2),
                                theme,
                            ),
                            style,
                        ),
                    ])
                }
            };
            lines.push(line.style(base));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn draw_note(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == Focus::Note;
        let Some(c) = self.current else {
            let block = self.panel(" Note ".into(), focused, theme);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new(Span::styled("Pick a note.", Style::new().fg(theme.muted))),
                inner,
            );
            return;
        };
        let title = format!(" {} ", self.vault.notes[c].title);
        let block = self.panel(title, focused, theme);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let text = Rect {
            x: inner.x + 1,
            width: inner.width.saturating_sub(2),
            ..inner
        };
        let key = (c, text.width, self.link);
        if self.cache.as_ref().map(|(k, _)| *k) != Some(key) {
            let body = self.body_with_backlinks(c);
            self.cache = Some((key, md::render(&body, text.width, theme, self.link)));
        }
        let (_, rendered) = self.cache.as_ref().unwrap();
        let height = text.height as usize;
        let max = rendered.lines.len().saturating_sub(height);
        self.scroll = self.scroll.min(max);
        let lines: Vec<Line> = rendered
            .lines
            .iter()
            .skip(self.scroll)
            .take(height)
            .cloned()
            .collect();
        let count = rendered.links.len();
        frame.render_widget(Paragraph::new(lines), text);
        // Where you are: links and scroll position, on the bottom border.
        let mut info = Vec::new();
        if count > 0 {
            info.push(match self.link {
                Some(i) => format!("link {}/{count}", i + 1),
                None => format!("{count} links"),
            });
        }
        if rendered.lines.len() > height {
            info.push(format!(
                "{}%",
                (self.scroll + height) * 100 / rendered.lines.len().max(1)
            ));
        }
        if !info.is_empty() && area.height > 1 {
            let label = format!(" {} ", info.join(" · "));
            frame.render_widget(
                Paragraph::new(Span::styled(label, Style::new().fg(theme.muted)))
                    .alignment(Alignment::Right),
                Rect {
                    x: area.x + 1,
                    y: area.y + area.height - 1,
                    width: area.width.saturating_sub(2),
                    height: 1,
                },
            );
        }
    }

    /// The note's text, plus a "Linked from" list that Tab can reach.
    fn body_with_backlinks(&self, note: usize) -> String {
        let mut body = self.vault.notes[note].body.clone();
        let back = self.vault.backlinks(note);
        if !back.is_empty() {
            body.push_str("\n\n---\n\n**Linked from**\n\n");
            for b in back {
                let n = &self.vault.notes[b];
                let target = n.path.trim_end_matches(".md");
                body.push_str(&format!("- [[{target}|{}]]\n", n.title));
            }
        }
        body
    }

    fn draw_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let hints: &[(&str, &str)] = match (&self.input, self.focus) {
            (Input::Search(_), _) => &[
                ("type", "search"),
                ("↑↓", "pick"),
                ("⏎", "open"),
                ("esc", "cancel"),
            ],
            (Input::NewNote(_), _) => &[("⏎", "create"), ("esc", "cancel")],
            (Input::None, Focus::Tree) => &[
                ("⏎", "open"),
                ("/", "search"),
                ("n", "new"),
                ("t", "today"),
                ("e", "edit"),
                ("o", "obsidian"),
                ("?", "keys"),
                ("q", "quit"),
            ],
            (Input::None, Focus::Note) => &[
                ("tab", "links"),
                ("⏎", "follow"),
                ("⌫", "back"),
                ("e", "edit"),
                ("o", "obsidian"),
                ("esc", "tree"),
                ("?", "keys"),
            ],
        };
        let mut spans = Vec::new();
        if let Input::NewNote(name) = &self.input {
            spans.push(Span::styled(
                " New note: ",
                Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(name.clone(), Style::new().fg(theme.text)));
            spans.push(Span::styled(
                theme.glyphs.caret,
                Style::new().fg(theme.accent),
            ));
            spans.push(Span::raw("  "));
        }
        for (k, what) in hints {
            spans.push(Span::styled(
                format!(" {k} "),
                Style::new().fg(theme.accent),
            ));
            spans.push(Span::styled(
                format!("{what} "),
                Style::new().fg(theme.muted),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        if let Some(s) = &self.status {
            frame.render_widget(
                Paragraph::new(Span::styled(format!("{s} "), Style::new().fg(theme.text)))
                    .alignment(Alignment::Right),
                area,
            );
        }
    }
}

fn r_depth(r: &Row) -> usize {
    match r {
        Row::Folder { depth, .. } | Row::Note { depth, .. } => *depth,
    }
}

fn hive_notes_name(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

/// `s` cut to `width` columns with an ellipsis.
fn fit(s: &str, width: usize, theme: &Theme) -> String {
    use unicode_width::UnicodeWidthStr;
    if s.width() <= width {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        if out.width() + 2 > width {
            break;
        }
        out.push(c);
    }
    out.push_str(theme.glyphs.ellipsis);
    out
}

/// Percent-encodes everything but unreserved characters and `/`.
fn percent(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let keys: &[(&str, &str)] = &[
        ("j k  ↑ ↓", "move, or scroll the note"),
        ("⏎  l  →", "open a note or folder"),
        ("h  ←  esc", "back to the tree, fold a folder"),
        ("tab  shift+tab", "next / previous link in the note"),
        ("⏎ on a link", "follow it (a missing note is created)"),
        ("⌫  b", "back to the previous note"),
        ("/", "search every note"),
        ("n", "new note (Folder/Name for a folder)"),
        ("t", "today's daily note"),
        ("e", "edit in $EDITOR"),
        ("o", "open in Obsidian"),
        ("r", "reload (it also reloads on its own)"),
        ("d u  space  g G", "page down / up, top / bottom"),
        ("q", "quit"),
    ];
    let width = 56.min(area.width.saturating_sub(4));
    let height = (keys.len() as u16 + 4).min(area.height.saturating_sub(2));
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.accent))
        .title(Span::styled(
            " Keys ",
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let lines: Vec<Line> = keys
        .iter()
        .map(|(k, what)| {
            Line::from(vec![
                Span::styled(format!(" {k:<16}"), Style::new().fg(theme.accent)),
                Span::styled(*what, Style::new().fg(theme.text)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

/// The editor for `e`: `$VISUAL`, `$EDITOR`, else nvim, vim, vi or nano.
fn editor() -> Option<Vec<String>> {
    let from_env = ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.trim().is_empty())
        .map(|v| v.split_whitespace().map(str::to_string).collect::<Vec<_>>());
    from_env.or_else(|| {
        ["nvim", "vim", "vi", "nano"]
            .iter()
            .find(|e| {
                let path = std::env::var_os("PATH").unwrap_or_default();
                hive_core::launch::find_executable(e, &path).is_some()
            })
            .map(|e| vec![e.to_string()])
    })
}

/// Hands a URL or path to the system's opener, without waiting.
fn open_external(target: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(target);
        c
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", target]);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        c
    };
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Runs `hive notes` on `dir` (default: the vault for the current
/// directory, else the directory itself) until the user quits.
pub fn run(dir: Option<PathBuf>, config: &Config) -> anyhow::Result<()> {
    let root = match dir {
        Some(d) => d,
        None => {
            let cwd = std::env::current_dir()?;
            hive_notes::locate(&cwd).unwrap_or(cwd)
        }
    };
    let theme = crate::theme_for(config)?;
    let mut app = NotesApp::open(&root, &config.integrations.obsidian.journal_dir)?;
    let mouse = config.terminal.mouse_support;
    let start = |mouse: bool| -> anyhow::Result<ratatui::DefaultTerminal> {
        let t = ratatui::try_init()?;
        if mouse {
            crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
        }
        Ok(t)
    };
    let stop = || {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        ratatui::restore();
    };
    struct Guard<F: Fn()>(F);
    impl<F: Fn()> Drop for Guard<F> {
        fn drop(&mut self) {
            (self.0)()
        }
    }
    let mut terminal = start(mouse)?;
    let _guard = Guard(stop);
    let mut seen = hive_notes::fingerprint(&app.vault.root);
    let mut checked = Instant::now();
    loop {
        terminal.draw(|f| app.draw(f, &theme))?;
        if event::poll(Duration::from_millis(500))? {
            let action = match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => app.on_key(k),
                Event::Mouse(m) => {
                    app.on_mouse(m);
                    None
                }
                _ => None,
            };
            match action {
                Some(Action::Quit) => return Ok(()),
                Some(Action::Edit(path)) => {
                    let Some(mut argv) = editor() else {
                        app.status = Some("No editor: set $EDITOR.".into());
                        continue;
                    };
                    stop();
                    argv.push(path.display().to_string());
                    let status = std::process::Command::new(&argv[0])
                        .args(&argv[1..])
                        .status();
                    terminal = start(mouse)?;
                    terminal.clear()?;
                    if let Err(e) = status {
                        app.status = Some(format!("Couldn't run {}: {e}", argv[0]));
                    }
                    app.reload();
                }
                Some(Action::Open(target)) => {
                    app.status = Some(match open_external(&target) {
                        Ok(()) if target.starts_with("obsidian:") => "Opening in Obsidian…".into(),
                        Ok(()) => "Opening…".into(),
                        Err(e) => format!("Couldn't open it: {e}"),
                    });
                }
                None => {}
            }
        }
        // Notes edited elsewhere (Obsidian, an agent) show up on their own.
        if checked.elapsed() >= Duration::from_secs(2) {
            checked = Instant::now();
            let now: (usize, Option<SystemTime>) = hive_notes::fingerprint(&app.vault.root);
            if now != seen {
                seen = now;
                app.reload();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ch(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    fn vault() -> (tempfile::TempDir, NotesApp) {
        let dir = tempfile::tempdir().unwrap();
        let files = [
            (
                "Home.md",
                "# Home\n\nSee [[Decisions/ADR-1|the decision]] and [[Status]].\n\nAlso [[Missing note]].",
            ),
            ("00-Index/Status.md", "# Status\n\nAll good. Back [[Home]]."),
            (
                "Decisions/ADR-1.md",
                "# ADR-1\n\nWorktrees are kept when dirty.",
            ),
            ("zeta.md", "last"),
        ];
        for (p, t) in files {
            let path = dir.path().join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, t).unwrap();
        }
        let app = NotesApp::open(dir.path(), "Daily").unwrap();
        (dir, app)
    }

    fn screen(app: &mut NotesApp, w: u16, h: u16) -> String {
        let theme = Theme::hive_dark(true);
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| app.draw(f, &theme)).unwrap();
        let buf = term.backend().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn title(app: &NotesApp) -> &str {
        &app.vault.notes[app.current.unwrap()].title
    }

    #[test]
    fn the_tree_lists_folders_first_and_starts_on_home() {
        let (_d, mut app) = vault();
        let rows: Vec<String> = app.rows.iter().map(|r| app.row_key(r)).collect();
        assert_eq!(
            rows,
            vec![
                "00-Index/",
                "00-Index/Status.md",
                "Decisions/",
                "Decisions/ADR-1.md",
                "Home.md",
                "zeta.md"
            ]
        );
        assert_eq!(title(&app), "Home");
        assert_eq!(
            app.rows[app.selected],
            Row::Note {
                note: app.current.unwrap(),
                depth: 0
            }
        );
        let s = screen(&mut app, 100, 20);
        assert!(s.contains("Notes · "), "{s}");
        assert!(s.contains("See the decision and Status."), "{s}");
        assert!(s.contains("Linked from"), "Status links back to Home:\n{s}");
        // Folding a folder hides its notes; the preview follows the cursor.
        app.on_key(ch('g'));
        app.on_key(key(KeyCode::Enter));
        assert!(
            !app.rows
                .iter()
                .any(|r| app.row_key(r) == "00-Index/Status.md")
        );
        app.on_key(ch('j'));
        app.on_key(ch('j'));
        assert_eq!(title(&app), "ADR-1");
    }

    #[test]
    fn links_are_followed_and_back_returns() {
        let (dir, mut app) = vault();
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.focus, Focus::Note);
        screen(&mut app, 100, 20);
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.link, Some(0));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(title(&app), "ADR-1");
        assert_eq!(
            app.vault.notes[app.current.unwrap()].path,
            "Decisions/ADR-1.md"
        );
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(title(&app), "Home");
        // A link to a missing note creates it, like Obsidian.
        screen(&mut app, 100, 20);
        // Its links: the decision, Status, Missing note, then the
        // "Linked from" list (Status links to Home).
        for _ in 0..3 {
            app.on_key(key(KeyCode::Tab));
        }
        app.on_key(key(KeyCode::Enter));
        assert_eq!(title(&app), "Missing note");
        assert!(dir.path().join("Missing note.md").exists());
    }

    #[test]
    fn search_finds_text_and_opens_the_hit() {
        let (_d, mut app) = vault();
        app.on_key(ch('/'));
        for c in "dirty".chars() {
            app.on_key(ch(c));
        }
        assert_eq!(app.hits.len(), 1);
        assert_eq!(title(&app), "ADR-1", "preview follows the pick");
        let s = screen(&mut app, 100, 20);
        assert!(s.contains("Search") && s.contains("/ dirty"), "{s}");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.input, Input::None);
        assert_eq!(app.focus, Focus::Note);
        assert_eq!(app.history.len(), 1);
    }

    #[test]
    fn new_and_daily_notes_are_created_in_place() {
        let (dir, mut app) = vault();
        // Selected row is Home (root): a new note goes to the root.
        app.on_key(ch('n'));
        for c in "Ideas".chars() {
            app.on_key(ch(c));
        }
        let action = app.on_key(key(KeyCode::Enter));
        let path = dir.path().canonicalize().unwrap().join("Ideas.md");
        assert_eq!(action, Some(Action::Edit(path.clone())));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Ideas\n");
        assert_eq!(title(&app), "Ideas");
        // In a folder when the cursor is in one.
        app.on_key(ch('g'));
        app.on_key(ch('n'));
        app.on_key(ch('X'));
        app.on_key(key(KeyCode::Enter));
        assert!(dir.path().join("00-Index/X.md").exists());
        app.on_key(ch('t'));
        let date = jiff::Zoned::now().strftime("%Y-%m-%d").to_string();
        assert!(dir.path().join(format!("Daily/{date}.md")).exists());
        assert_eq!(title(&app), date);
    }

    #[test]
    fn edit_and_obsidian_name_the_note() {
        let (dir, mut app) = vault();
        let root = dir.path().canonicalize().unwrap();
        assert_eq!(
            app.on_key(ch('e')),
            Some(Action::Edit(root.join("Home.md")))
        );
        let Some(Action::Open(url)) = app.on_key(ch('o')) else {
            panic!()
        };
        assert!(url.starts_with("obsidian://open?path=/"));
        assert!(url.ends_with("/Home.md"));
        assert_eq!(percent("a b/ü"), "a%20b/%C3%BC");
        assert_eq!(app.on_key(ch('q')), Some(Action::Quit));
    }

    #[test]
    fn narrow_panes_show_one_panel() {
        let (_d, mut app) = vault();
        let s = screen(&mut app, 50, 12);
        assert!(s.contains("Notes · ") && !s.contains("See the"), "{s}");
        app.on_key(key(KeyCode::Enter));
        let s = screen(&mut app, 50, 12);
        assert!(s.contains("See the") && !s.contains("Notes · "), "{s}");
    }
}
