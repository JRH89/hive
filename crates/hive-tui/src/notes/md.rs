//! Markdown to wrapped terminal lines for `hive notes`. Wrapping is done
//! here, not by ratatui, so every link's line is known: Tab moves between
//! links and scrolls them into view.

use pulldown_cmark::{Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::Theme;

/// Where a link points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A note by link name, `[[name]]` or `[text](name.md)`.
    Note(String),
    /// Anything else: a web page, a file, `mailto:`.
    Url(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAt {
    pub target: Target,
    /// The wrapped line it starts on.
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Rendered {
    pub lines: Vec<Line<'static>>,
    pub links: Vec<LinkAt>,
}

/// A run of text in one style, maybe part of link `link`.
#[derive(Clone)]
struct Chunk {
    text: String,
    style: Style,
    link: Option<usize>,
}

/// A paragraph-like unit before wrapping: a prefix for its first line, one
/// for the lines after (indents, bullets, quote bars), and its text.
struct Block {
    first: Vec<Chunk>,
    rest: Vec<Chunk>,
    chunks: Vec<Chunk>,
    /// Code: lines are kept as written (wrapped only when too wide).
    verbatim: bool,
}

/// Renders `body` (frontmatter already removed) at `width` columns.
/// `selected` is the index of the link to highlight.
pub fn render(body: &str, width: u16, theme: &Theme, selected: Option<usize>) -> Rendered {
    let mut r = Renderer::new(theme, selected);
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_WIKILINKS
        | Options::ENABLE_GFM;
    for event in Parser::new_ext(body, options) {
        r.event(event);
    }
    r.flush();
    r.finish(width.max(10) as usize)
}

struct Renderer<'t> {
    theme: &'t Theme,
    selected: Option<usize>,
    blocks: Vec<Block>,
    /// The block being filled.
    current: Option<Block>,
    styles: Vec<Style>,
    /// Open lists: the next number, or `None` for bullets.
    lists: Vec<Option<u64>>,
    quote: usize,
    /// Index of the link being filled.
    link: Option<usize>,
    targets: Vec<Target>,
    code: bool,
    table: Option<Table>,
    image: bool,
}

#[derive(Default)]
struct Table {
    rows: Vec<Vec<Vec<Chunk>>>,
    head_rows: usize,
    in_head: bool,
}

impl<'t> Renderer<'t> {
    fn new(theme: &'t Theme, selected: Option<usize>) -> Self {
        Renderer {
            theme,
            selected,
            blocks: Vec::new(),
            current: None,
            styles: vec![Style::new().fg(theme.text)],
            lists: Vec::new(),
            quote: 0,
            link: None,
            targets: Vec::new(),
            code: false,
            table: None,
            image: false,
        }
    }

    fn style(&self) -> Style {
        *self.styles.last().unwrap()
    }

    fn push_style(&mut self, f: impl Fn(Style) -> Style) {
        let s = f(self.style());
        self.styles.push(s);
    }

    fn pop_style(&mut self) {
        if self.styles.len() > 1 {
            self.styles.pop();
        }
    }

    fn glyph(&self, unicode: &'static str, ascii: &'static str) -> &'static str {
        if self.theme.glyphs.ascii {
            ascii
        } else {
            unicode
        }
    }

    /// The prefix every line of a new block gets from enclosing quotes and
    /// lists: (first line, later lines).
    fn prefixes(&self) -> (Vec<Chunk>, Vec<Chunk>) {
        let muted = Style::new().fg(self.theme.muted);
        let bar = self.glyph("▎ ", "| ");
        let mut base = Vec::new();
        for _ in 0..self.quote {
            base.push(chunk(bar, muted));
        }
        let indent = "  ".repeat(self.lists.len().saturating_sub(1));
        (base.clone(), {
            let mut rest = base;
            if !self.lists.is_empty() {
                rest.push(chunk(&format!("{indent}  "), muted));
            }
            rest
        })
    }

    fn begin(&mut self) {
        if self.current.is_none() {
            let (first, rest) = self.prefixes();
            let first = if self.lists.is_empty() {
                first
            } else {
                rest.clone()
            };
            self.current = Some(Block {
                first,
                rest,
                chunks: Vec::new(),
                verbatim: self.code,
            });
        }
    }

    fn flush(&mut self) {
        if let Some(b) = self.current.take()
            && !b.chunks.is_empty()
        {
            self.blocks.push(b);
        }
    }

    fn blank(&mut self) {
        self.flush();
        if self
            .blocks
            .last()
            .is_some_and(|b| !(b.chunks.is_empty() && b.first.is_empty()))
        {
            self.blocks.push(Block {
                first: Vec::new(),
                rest: Vec::new(),
                chunks: Vec::new(),
                verbatim: false,
            });
        }
    }

    fn text(&mut self, text: &str, style: Style) {
        if let Some(t) = &mut self.table {
            let cell = t.rows.last_mut().and_then(|r| r.last_mut());
            if let Some(cell) = cell {
                cell.push(Chunk {
                    text: text.to_string(),
                    style,
                    link: self.link,
                });
            }
            return;
        }
        self.begin();
        let link = self.link;
        let style = match link {
            Some(i) if Some(i) == self.selected => style.add_modifier(Modifier::REVERSED),
            _ => style,
        };
        self.current.as_mut().unwrap().chunks.push(Chunk {
            text: text.to_string(),
            style,
            link,
        });
    }

    fn event(&mut self, event: Event) {
        let theme = self.theme;
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(end) => self.end(end),
            Event::Text(t) => {
                if self.image {
                    return;
                }
                let style = self.style();
                if self.code {
                    // One block per line of code, kept verbatim.
                    for (k, line) in t.split('\n').enumerate() {
                        if k > 0 {
                            self.flush();
                        }
                        if !line.is_empty() || k == 0 {
                            self.text(line, style);
                        }
                    }
                } else {
                    self.text(&t, style);
                }
            }
            Event::Code(t) => {
                let style = self.style().fg(theme.accent_dim);
                self.text(&t, style);
            }
            Event::InlineMath(t) | Event::DisplayMath(t) => {
                let style = self.style().fg(theme.accent_dim);
                self.text(&t, style);
            }
            Event::Html(t) | Event::InlineHtml(t) => {
                let style = Style::new().fg(theme.muted);
                for (k, line) in t.trim_end_matches('\n').split('\n').enumerate() {
                    if k > 0 {
                        self.flush();
                    }
                    self.text(line, style);
                }
            }
            Event::FootnoteReference(t) => {
                let style = self.style().fg(theme.muted);
                self.text(&format!("[^{t}]"), style);
            }
            Event::SoftBreak => {
                let style = self.style();
                self.text(" ", style);
            }
            Event::HardBreak => self.flush(),
            Event::Rule => {
                self.blank();
                self.blocks.push(Block {
                    first: Vec::new(),
                    rest: Vec::new(),
                    chunks: vec![chunk("\u{0}rule", Style::new().fg(theme.surface))],
                    verbatim: true,
                });
                self.blank();
            }
            Event::TaskListMarker(done) => {
                let (mark, style) = if done {
                    (self.glyph("☑ ", "[x] "), Style::new().fg(theme.success))
                } else {
                    (self.glyph("☐ ", "[ ] "), Style::new().fg(theme.muted))
                };
                self.text(mark, style);
            }
        }
    }

    fn start(&mut self, tag: Tag) {
        let theme = self.theme;
        match tag {
            Tag::Paragraph => self.flush(),
            Tag::Heading { level, .. } => {
                self.blank();
                let style = match level {
                    HeadingLevel::H1 => Style::new()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    HeadingLevel::H2 => Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
                    _ => Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
                };
                self.styles.push(style);
                let marks = match level {
                    HeadingLevel::H1 | HeadingLevel::H2 => String::new(),
                    l => format!("{} ", "#".repeat(l as usize)),
                };
                if !marks.is_empty() {
                    self.text(&marks, Style::new().fg(theme.muted));
                }
            }
            Tag::BlockQuote(kind) => {
                self.flush();
                self.quote += 1;
                if let Some(kind) = kind {
                    let label = format!("{kind:?}").to_uppercase();
                    self.text(
                        &label,
                        Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
                    );
                    self.flush();
                }
            }
            Tag::CodeBlock(_) => {
                self.blank();
                self.code = true;
                self.push_style(|_| Style::new().fg(theme.accent_dim));
                self.quote += 1;
            }
            Tag::HtmlBlock => self.flush(),
            Tag::List(start) => {
                self.flush();
                if self.lists.is_empty() {
                    self.blank();
                }
                self.lists.push(start);
            }
            Tag::Item => {
                self.flush();
                let depth = self.lists.len().saturating_sub(1);
                let bullet = match self.lists.last_mut() {
                    Some(Some(n)) => {
                        let b = format!("{n}. ");
                        *n += 1;
                        b
                    }
                    _ => self.glyph("• ", "* ").to_string(),
                };
                let (first, _) = self.prefixes();
                let mut first = first;
                first.push(chunk(&"  ".repeat(depth), Style::new()));
                first.push(chunk(&bullet, Style::new().fg(theme.accent)));
                let rest_indent = " ".repeat(bullet.width());
                let (_, mut rest) = self.prefixes();
                rest.push(chunk(&rest_indent, Style::new()));
                self.current = Some(Block {
                    first,
                    rest,
                    chunks: Vec::new(),
                    verbatim: false,
                });
            }
            Tag::Table(_) => {
                self.blank();
                self.table = Some(Table::default());
            }
            Tag::TableHead => {
                if let Some(t) = &mut self.table {
                    t.in_head = true;
                    t.rows.push(Vec::new());
                }
            }
            Tag::TableRow => {
                if let Some(t) = &mut self.table {
                    t.rows.push(Vec::new());
                }
            }
            Tag::TableCell => {
                if let Some(t) = &mut self.table
                    && let Some(row) = t.rows.last_mut()
                {
                    row.push(Vec::new());
                }
            }
            Tag::Emphasis => self.push_style(|s| s.add_modifier(Modifier::ITALIC)),
            Tag::Strong => self.push_style(|s| s.add_modifier(Modifier::BOLD)),
            Tag::Strikethrough => self.push_style(|s| s.add_modifier(Modifier::CROSSED_OUT)),
            Tag::Superscript | Tag::Subscript => self.push_style(|s| s),
            Tag::Link {
                link_type,
                dest_url,
                ..
            } => {
                let target = match link_type {
                    LinkType::WikiLink { .. } => {
                        Target::Note(dest_url.split('#').next().unwrap_or("").to_string())
                    }
                    _ if !dest_url.contains("://")
                        && !dest_url.starts_with("mailto:")
                        && dest_url
                            .split('#')
                            .next()
                            .unwrap_or("")
                            .to_lowercase()
                            .ends_with(".md") =>
                    {
                        Target::Note(dest_url.split('#').next().unwrap_or("").replace("%20", " "))
                    }
                    _ => Target::Url(dest_url.to_string()),
                };
                self.link = Some(self.targets.len());
                self.targets.push(target);
                self.push_style(|s| s.fg(theme.accent).add_modifier(Modifier::UNDERLINED));
            }
            Tag::Image { dest_url, .. } => {
                let name = dest_url.rsplit('/').next().unwrap_or(&dest_url).to_string();
                self.text(
                    &format!("[{} {name}]", self.glyph("▣", "img")),
                    Style::new().fg(theme.muted),
                );
                self.image = true;
            }
            Tag::FootnoteDefinition(label) => {
                self.flush();
                self.text(&format!("[^{label}] "), Style::new().fg(theme.muted));
            }
            Tag::DefinitionList | Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {
                self.flush()
            }
            Tag::MetadataBlock(_) => self.image = true,
        }
    }

    fn end(&mut self, end: TagEnd) {
        match end {
            TagEnd::Paragraph => self.blank(),
            TagEnd::Heading(_) => {
                self.pop_style();
                self.blank();
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_sub(1);
                self.blank();
            }
            TagEnd::CodeBlock => {
                self.flush();
                self.code = false;
                self.pop_style();
                self.quote = self.quote.saturating_sub(1);
                self.blank();
            }
            TagEnd::HtmlBlock => self.blank(),
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
                if self.lists.is_empty() {
                    self.blank();
                }
            }
            TagEnd::Item => self.flush(),
            TagEnd::Table => {
                if let Some(t) = self.table.take() {
                    self.table_blocks(t);
                }
                self.blank();
            }
            TagEnd::TableHead => {
                if let Some(t) = &mut self.table {
                    t.in_head = false;
                    t.head_rows = t.rows.len();
                }
            }
            TagEnd::TableRow | TagEnd::TableCell => {}
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript => self.pop_style(),
            TagEnd::Link => {
                self.link = None;
                self.pop_style();
            }
            TagEnd::Image => self.image = false,
            TagEnd::FootnoteDefinition => self.blank(),
            TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition => self.flush(),
            TagEnd::MetadataBlock(_) => self.image = false,
        }
    }

    /// Columns padded to their widest cell, `│` between, a rule under the
    /// header. Rows wider than the pane wrap like code.
    fn table_blocks(&mut self, t: Table) {
        let theme = self.theme;
        let cols = t.rows.iter().map(Vec::len).max().unwrap_or(0);
        let width = |cell: &Vec<Chunk>| cell.iter().map(|c| c.text.width()).sum::<usize>();
        let widths: Vec<usize> = (0..cols)
            .map(|c| {
                t.rows
                    .iter()
                    .filter_map(|r| r.get(c))
                    .map(width)
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let sep = Style::new().fg(theme.muted);
        let bar = self.glyph(" │ ", " | ");
        for (r, row) in t.rows.iter().enumerate() {
            let mut chunks = Vec::new();
            for (c, w) in widths.iter().enumerate() {
                if c > 0 {
                    chunks.push(chunk(bar, sep));
                }
                let cell = row.get(c).cloned().unwrap_or_default();
                let pad = w - width(&cell);
                for mut ch in cell {
                    if r < t.head_rows {
                        ch.style = ch.style.add_modifier(Modifier::BOLD);
                    }
                    if ch.link.is_some() && ch.link == self.selected {
                        ch.style = ch.style.add_modifier(Modifier::REVERSED);
                    }
                    chunks.push(ch);
                }
                chunks.push(chunk(&" ".repeat(pad), Style::new()));
            }
            let (first, rest) = self.prefixes();
            self.blocks.push(Block {
                first,
                rest,
                chunks,
                verbatim: true,
            });
            if r + 1 == t.head_rows {
                let line = widths
                    .iter()
                    .map(|w| self.glyph("─", "-").repeat(*w))
                    .collect::<Vec<_>>()
                    .join(self.glyph("─┼─", "-+-"));
                let (first, rest) = self.prefixes();
                self.blocks.push(Block {
                    first,
                    rest,
                    chunks: vec![chunk(&line, sep)],
                    verbatim: true,
                });
            }
        }
    }

    fn finish(mut self, width: usize) -> Rendered {
        while self
            .blocks
            .last()
            .is_some_and(|b| b.chunks.is_empty() && b.first.is_empty())
        {
            self.blocks.pop();
        }
        let mut out = Rendered::default();
        let mut placed = vec![false; self.targets.len()];
        let rule = self.glyph("─", "-");
        for block in self.blocks {
            if block.chunks.len() == 1 && block.chunks[0].text == "\u{0}rule" {
                out.lines.push(Line::from(Span::styled(
                    rule.repeat(width),
                    block.chunks[0].style,
                )));
                continue;
            }
            for (spans, links) in wrap(&block, width) {
                for l in links {
                    if !placed[l] {
                        placed[l] = true;
                        out.links.push(LinkAt {
                            target: self.targets[l].clone(),
                            line: out.lines.len(),
                        });
                    }
                }
                out.lines.push(Line::from(spans));
            }
        }
        // Links in document order (a table can place them out of order).
        out.links.sort_by_key(|l| l.line);
        out
    }
}

fn chunk(text: &str, style: Style) -> Chunk {
    Chunk {
        text: text.to_string(),
        style,
        link: None,
    }
}

/// Greedy word wrap of a block into lines of spans, with the links each
/// line contains. Words wider than the room are broken.
fn wrap(block: &Block, width: usize) -> Vec<(Vec<Span<'static>>, Vec<usize>)> {
    let prefix_width = |p: &[Chunk]| p.iter().map(|c| c.text.width()).sum::<usize>();
    let to_spans = |p: &[Chunk]| -> Vec<Span<'static>> {
        p.iter()
            .map(|c| Span::styled(c.text.clone(), c.style))
            .collect()
    };
    if block.chunks.is_empty() {
        return vec![(to_spans(&block.first), Vec::new())];
    }
    // Pieces: words and the spaces between them, each with its chunk's
    // style and link.
    let mut pieces: Vec<(String, Style, Option<usize>)> = Vec::new();
    for c in &block.chunks {
        let mut word = String::new();
        for ch in c.text.chars() {
            let space = ch == ' ' && !block.verbatim;
            if space {
                if !word.is_empty() {
                    pieces.push((std::mem::take(&mut word), c.style, c.link));
                }
                pieces.push((" ".into(), c.style, c.link));
            } else {
                word.push(ch);
            }
        }
        if !word.is_empty() {
            pieces.push((word, c.style, c.link));
        }
    }
    let mut lines = Vec::new();
    let mut spans = to_spans(&block.first);
    let mut room = width.saturating_sub(prefix_width(&block.first)).max(1);
    let mut used = 0;
    let mut links: Vec<usize> = Vec::new();
    let mut fresh = true;
    let new_line = |spans: &mut Vec<Span<'static>>,
                    links: &mut Vec<usize>,
                    lines: &mut Vec<(Vec<Span<'static>>, Vec<usize>)>,
                    room: &mut usize,
                    used: &mut usize| {
        lines.push((std::mem::take(spans), std::mem::take(links)));
        *spans = to_spans(&block.rest);
        *room = width.saturating_sub(prefix_width(&block.rest)).max(1);
        *used = 0;
    };
    for (text, style, link) in pieces {
        let w = text.width();
        if text == " " {
            if fresh || used + 1 > room {
                continue;
            }
            spans.push(Span::styled(text, style));
            used += 1;
            continue;
        }
        if used + w > room && used > 0 {
            // Drop a trailing space before breaking.
            if spans.last().is_some_and(|s| s.content == " ") {
                spans.pop();
            }
            new_line(&mut spans, &mut links, &mut lines, &mut room, &mut used);
        }
        fresh = false;
        if let Some(l) = link
            && !links.contains(&l)
        {
            links.push(l);
        }
        if w <= room - used {
            spans.push(Span::styled(text, style));
            used += w;
            continue;
        }
        // Too wide even for an empty line: break it by characters.
        let mut part = String::new();
        for ch in text.chars() {
            let cw = ch.width().unwrap_or(0);
            if used + cw > room && used > 0 {
                spans.push(Span::styled(std::mem::take(&mut part), style));
                new_line(&mut spans, &mut links, &mut lines, &mut room, &mut used);
                if let Some(l) = link {
                    links.push(l);
                }
            }
            part.push(ch);
            used += cw;
        }
        if !part.is_empty() {
            spans.push(Span::styled(part, style));
        }
    }
    lines.push((spans, links));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(r: &Rendered) -> Vec<String> {
        r.lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    fn theme() -> Theme {
        Theme::hive_dark(true)
    }

    #[test]
    fn markdown_renders_blocks_lists_and_tables() {
        let md = "# Title\n\nSome *text* with `code`.\n\n## Part\n\n- one\n- two\n  1. nested\n- [x] done\n- [ ] todo\n\n> quoted\n\n```rust\nfn main() {}\n```\n\n| a | bb |\n|---|---|\n| ccc | d |\n\n---\n\nend";
        let r = render(md, 40, &theme(), None);
        let lines = text(&r);
        assert_eq!(lines[0], "Title");
        assert!(
            lines.contains(&"Some text with code.".to_string()),
            "{lines:#?}"
        );
        assert!(lines.contains(&"• one".to_string()));
        assert!(lines.iter().any(|l| l == "  1. nested"), "{lines:#?}");
        assert!(lines.iter().any(|l| l.starts_with("• ☑ done")));
        assert!(lines.iter().any(|l| l.starts_with("• ☐ todo")));
        assert!(lines.contains(&"▎ quoted".to_string()));
        assert!(lines.contains(&"▎ fn main() {}".to_string()));
        assert!(lines.contains(&"a   │ bb".to_string()), "{lines:#?}");
        assert!(lines.contains(&"────┼───".to_string()));
        assert!(lines.contains(&"ccc │ d ".to_string()));
        assert!(lines.contains(&"─".repeat(40)));
        assert_eq!(lines.last().unwrap(), "end");
        // One blank line between blocks, never two.
        assert!(!lines.windows(2).any(|w| w[0].is_empty() && w[1].is_empty()));
    }

    #[test]
    fn links_are_found_in_order_with_their_lines() {
        let md = "Intro [[ADR-0012 Worktrees#Decision|the ADR]] and [site](https://x.y).\n\nLater: [old](Old%20Note.md) then [[Home]].";
        let r = render(md, 20, &theme(), Some(0));
        let targets: Vec<&Target> = r.links.iter().map(|l| &l.target).collect();
        assert_eq!(
            targets,
            vec![
                &Target::Note("ADR-0012 Worktrees".into()),
                &Target::Url("https://x.y".into()),
                &Target::Note("Old Note.md".into()),
                &Target::Note("Home".into()),
            ]
        );
        let lines = text(&r);
        for l in &r.links {
            assert!(l.line < lines.len());
        }
        assert!(r.links[3].line > r.links[0].line);
        // The selected link is highlighted.
        let first = r.lines[r.links[0].line]
            .spans
            .iter()
            .find(|s| s.content.contains("the") || s.content.contains("ADR"))
            .unwrap();
        assert!(first.style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn long_text_wraps_inside_the_width_and_keeps_list_indents() {
        let md = "- a list item that is long enough to wrap onto the next line\n\nsupercalifragilisticexpialidocious";
        let r = render(md, 16, &theme(), None);
        let lines = text(&r);
        assert!(lines.iter().all(|l| l.width() <= 16), "{lines:#?}");
        assert!(lines[0].starts_with("• a list"));
        assert!(
            lines[1].starts_with("  "),
            "continuation indented: {lines:#?}"
        );
        assert!(lines.iter().any(|l| l.starts_with("supercal")));
    }

    #[test]
    fn ascii_glyphs_are_used_when_asked() {
        let mut t = theme();
        t.glyphs = crate::theme::Glyphs::ASCII;
        let r = render("- [x] a\n- b\n\n> q", 30, &t, None);
        let lines = text(&r);
        assert!(lines.contains(&"* [x] a".to_string()), "{lines:#?}");
        assert!(lines.contains(&"| q".to_string()));
    }
}
