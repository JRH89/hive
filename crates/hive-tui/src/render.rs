//! Drawing. Reads `App` only; never mutates state.

use hive_core::brand;
use hive_protocol::{SessionInfo, SessionState, WorkspaceId};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{
    Activity, App, Focus, Link, Mode, SIDEBAR_WIDTH, Scope, SidebarHit, Target, Tone,
};
use crate::keymap::Command;
use crate::layout::{Dir, PaneArea};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    // Clickable areas are recorded as they are drawn.
    app.sidebar_hits.borrow_mut().clear();
    let [body, status] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
    // The sidebar runs full height; the tab row sits above the panes only.
    let right = if app.sidebar_visible {
        let [sidebar, right] =
            Layout::horizontal([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(1)]).areas(body);
        draw_sidebar(frame, sidebar, app, theme);
        right
    } else {
        body
    };
    let [header, panel] =
        Layout::vertical([Constraint::Length(app.header_rows()), Constraint::Min(1)]).areas(right);
    if header.height > 0 {
        draw_header(frame, header, app, theme);
    }
    let mut agent_cells = None;
    let (panes, gaps) = app.pane_areas();
    if panes.is_empty() {
        draw_empty(frame, panel, app, theme);
    } else {
        for pane in &panes {
            if let Some(view) = app.views.get(&pane.session) {
                draw_terminal(frame.buffer_mut(), pane.body, view.parser.screen());
            }
        }
        // Before Hive's own pane chrome, so ASCII mode folds only that.
        if theme.glyphs.ascii {
            agent_cells = Some(snapshot(frame.buffer_mut(), panel));
        }
        draw_pane_chrome(frame, app, &panes, &gaps, theme);
        let focused = panes.iter().find(|p| Some(p.session) == app.focused());
        if let (Some(pane), Some(view)) = (focused, app.view()) {
            let screen = view.parser.screen();
            if app.can_type() && !screen.hide_cursor() {
                let (row, col) = screen.cursor_position();
                if row < pane.body.height && col < pane.body.width {
                    frame.set_cursor_position((pane.body.x + col, pane.body.y + row));
                }
            }
        }
    }
    draw_status(frame, status, app, theme);
    if let Link::Reconnecting { attempt, reason } = &app.link {
        draw_reconnecting(frame, body, *attempt, reason, theme);
    } else {
        draw_overlay(frame, body, app, theme);
    }
    if theme.glyphs.ascii {
        asciify(frame.buffer_mut(), panel, agent_cells.as_deref());
    }
}

/// Title rows and separators between split panes. Nothing for a lone pane.
fn draw_pane_chrome(
    frame: &mut Frame,
    app: &App,
    panes: &[PaneArea],
    gaps: &[(Rect, Dir)],
    theme: &Theme,
) {
    let line = Style::new().fg(if theme.mono {
        theme.muted
    } else {
        theme.surface
    });
    for (gap, dir) in gaps {
        let glyph = match dir {
            Dir::Right => theme.glyphs.border.vertical_left,
            Dir::Down => theme.glyphs.border.horizontal_top,
        };
        let buf = frame.buffer_mut();
        for pos in gap.positions() {
            if let Some(cell) = buf.cell_mut(pos) {
                cell.reset();
                cell.set_symbol(glyph).set_style(line);
            }
        }
    }
    for pane in panes.iter().filter(|p| p.title.height > 0) {
        let focused = Some(pane.session) == app.focused();
        let style = if focused {
            theme.highlight()
        } else {
            Style::new().fg(theme.muted)
        };
        let mut text = match app.sessions.get(&pane.session) {
            Some(s) => {
                let (dot, _) = session_dot(s, app, theme);
                format!(" {dot} #{} {}", s.id, s.label)
            }
            None => format!(" #{}", pane.session),
        };
        if let Some(a) = app.activity(pane.session).filter(|a| *a != Activity::Idle) {
            text = format!("{text} · {}", a.word());
        }
        let text = truncate(theme, &text, pane.title.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(text, style)).style(style),
            pane.title,
        );
    }
}

fn snapshot(buf: &Buffer, area: Rect) -> Vec<String> {
    area.positions()
        .map(|p| {
            buf.cell(p)
                .map(|c| c.symbol().to_string())
                .unwrap_or_default()
        })
        .collect()
}

/// ASCII-folds everything Hive drew. Cells of the agent's screen that no
/// overlay changed are left as the agent wrote them.
fn asciify(buf: &mut Buffer, panel: Rect, agent: Option<&[String]>) {
    let area = buf.area;
    for pos in area.positions() {
        if panel.contains(pos)
            && let Some(agent) = agent
        {
            let i = (pos.y - panel.y) as usize * panel.width as usize + (pos.x - panel.x) as usize;
            if buf
                .cell(pos)
                .is_some_and(|c| agent.get(i).map(String::as_str) == Some(c.symbol()))
            {
                continue;
            }
        }
        if let Some(cell) = buf.cell_mut(pos)
            && !cell.symbol().is_ascii()
        {
            let folded = match cell.symbol() {
                "·" => ".",
                "…" => "~",
                "—" | "─" => "-",
                "↑" => "^",
                "↓" => "v",
                "│" => "|",
                "╭" | "╮" | "╰" | "╯" => "+",
                _ => "?",
            };
            cell.set_symbol(folded);
        }
    }
}

fn draw_overlay(frame: &mut Frame, body: Rect, app: &App, theme: &Theme) {
    match app.mode {
        Mode::Help => draw_help(frame, body, app, theme),
        Mode::Palette { selected, .. } => draw_palette(frame, body, app, selected, theme),
        Mode::NewSession {
            selected,
            workspace,
            worktree,
        } => draw_picker(frame, body, app, selected, workspace, worktree, theme),
        Mode::Workspaces { selected } => draw_workspaces(frame, body, app, selected, theme),
        _ => {}
    }
}

/// herdr-style tab row above the panes: tab numbers, `+` for a new
/// terminal tab, and the space's name on the right. Every tab is clickable.
fn draw_header(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let muted = Style::new().fg(theme.muted);
    let mut spans = vec![Span::raw(" ")];
    let mut x = area.x + 1;
    let mut hits = app.sidebar_hits.borrow_mut();
    // This space's tabs only, numbered from 1.
    for (n, i) in app.space_tabs(app.view_space()).into_iter().enumerate() {
        let label = format!("  {}  ", n + 1);
        let width = label.chars().count() as u16;
        hits.push((Rect::new(x, area.y, width, 1), SidebarHit::Tab(i)));
        let style = if app.active_tab().is_some() && i == app.tab {
            theme.highlight()
        } else {
            muted
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw("  "));
        x += width + 2;
    }
    hits.push((Rect::new(x, area.y, 3, 1), SidebarHit::NewTab));
    spans.push(Span::styled(
        " + ",
        Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
    ));
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);

    let name = app
        .shown_space()
        .map(|w| app.workspace_name(w).to_string())
        .unwrap_or_default();
    let room = (area.width as usize).saturating_sub(used + 2);
    if !name.is_empty() && room > 4 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{} ", truncate(theme, &name, room - 1)),
                muted,
            ))
            .alignment(Alignment::Right),
            area,
        );
    }
}

/// Truncates to `max` characters, ending in an ellipsis when cut.
fn truncate(theme: &Theme, s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push_str(theme.glyphs.ellipsis);
        out
    }
}

fn state_dot(state: &SessionState, theme: &Theme) -> (&'static str, Color) {
    let g = &theme.glyphs;
    match state {
        SessionState::Starting => (g.stopped, theme.muted),
        SessionState::Running => (g.running, theme.success),
        SessionState::Exited { .. } => (g.stopped, theme.muted),
        SessionState::Failed { .. } => (g.failed, theme.error),
        SessionState::Lost => (g.lost, theme.muted),
    }
}

fn activity_dot(a: Activity, theme: &Theme) -> (&'static str, Color) {
    let g = &theme.glyphs;
    match a {
        Activity::Idle => (g.running, theme.success),
        Activity::Done => (g.done, theme.success),
        Activity::Working => (g.working, theme.accent),
        Activity::Blocked => (g.blocked, theme.warning),
    }
}

/// The session's dot: agent activity when known, else process state.
fn session_dot(s: &SessionInfo, app: &App, theme: &Theme) -> (&'static str, Color) {
    match app.activity(s.id) {
        Some(a) => activity_dot(a, theme),
        None => state_dot(&s.state, theme),
    }
}

fn dir_name(cwd: &str) -> &str {
    cwd.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("/")
}

/// The branch a space is on: that of a session in its main checkout, else
/// any of its sessions'.
fn space_branch(app: &App, w: WorkspaceId) -> Option<String> {
    let mut in_space = app.sessions.values().filter(|s| s.workspace == w);
    let main = in_space
        .clone()
        .find(|s| s.worktree.is_none() && s.branch.is_some());
    main.or_else(|| in_space.find(|s| s.branch.is_some()))
        .and_then(|s| s.branch.clone())
}

/// herdr-style: SPACES (one entry per workspace), the new/menu buttons,
/// then AGENTS of the selected space. Every entry is clickable.
fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let border = Block::new()
        .borders(Borders::RIGHT)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(if theme.mono {
            theme.muted
        } else {
            theme.surface
        }));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if inner.height < 4 || inner.width < 6 {
        return;
    }
    let heading = Style::new().fg(theme.muted).add_modifier(Modifier::BOLD);
    let browsing = app.focus == Focus::Sidebar || app.sidebar_keys_active();
    let row = |y| Rect {
        y,
        height: 1,
        ..inner
    };
    let width = inner.width as usize;
    let bottom = inner.y + inner.height;
    let mut hits = app.sidebar_hits.borrow_mut();

    // SPACES: up to about half the height, scrolled to the selected one.
    let space = app.shown_space();
    let groups = app.groups();
    frame.render_widget(
        Paragraph::new(Span::styled(" SPACES", heading)),
        row(inner.y),
    );
    let room = ((inner.height.saturating_sub(6)) / 2).max(2) / 2;
    let selected = groups
        .iter()
        .position(|(w, _)| Some(*w) == space)
        .unwrap_or(0);
    let first = selected.saturating_sub(room.saturating_sub(1) as usize);
    let mut y = inner.y + 2;
    for (w, ids) in groups.iter().skip(first).take(room as usize) {
        let chosen = Some(*w) == space;
        let base = if chosen {
            theme.cursor_row()
        } else {
            Style::new()
        };
        let (dot, color) = match app.workspace_activity(*w).filter(|a| *a != Activity::Idle) {
            Some(a) => activity_dot(a, theme),
            None if chosen => (theme.glyphs.running, theme.accent),
            None => ("·", theme.muted),
        };
        let name_style = if chosen {
            base.fg(theme.text).add_modifier(Modifier::BOLD)
        } else {
            base.fg(theme.text)
        };
        let count = if ids.is_empty() {
            String::new()
        } else {
            format!("{} ", ids.len())
        };
        let name = truncate(
            theme,
            app.workspace_name(*w),
            width.saturating_sub(4 + count.len()),
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {dot} "), base.fg(color)),
                Span::styled(name, name_style),
            ]))
            .style(base),
            row(y),
        );
        frame.render_widget(
            Paragraph::new(Span::styled(count, base.fg(theme.muted))).alignment(Alignment::Right),
            row(y),
        );
        let branch = space_branch(app, *w).unwrap_or_default();
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("   {}", truncate(theme, &branch, width.saturating_sub(3))),
                base.fg(theme.muted),
            ))
            .style(base),
            row(y + 1),
        );
        hits.push((
            Rect {
                height: 2,
                ..row(y)
            },
            SidebarHit::Workspace(*w),
        ));
        y += 2;
    }
    if groups.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "   no spaces yet",
                Style::new().fg(theme.muted),
            )),
            row(y),
        );
        y += 1;
    }

    // Buttons, then a rule.
    y += 1;
    if y + 3 > bottom {
        return;
    }
    let half = inner.width / 2;
    let button = Style::new().fg(theme.accent);
    frame.render_widget(
        Paragraph::new(Span::styled(" + new", button)),
        Rect {
            width: half,
            ..row(y)
        },
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{} menu ", theme.glyphs.running),
            button,
        ))
        .alignment(Alignment::Right),
        row(y),
    );
    hits.push((
        Rect {
            width: half,
            ..row(y)
        },
        SidebarHit::New,
    ));
    hits.push((
        Rect {
            x: inner.x + half,
            width: inner.width - half,
            ..row(y)
        },
        SidebarHit::Menu,
    ));
    let rule = theme.glyphs.border.horizontal_top.repeat(width);
    frame.render_widget(
        Paragraph::new(Span::styled(rule, Style::new().fg(theme.surface))),
        row(y + 1),
    );

    // AGENTS of the selected space.
    let top = y + 2;
    frame.render_widget(
        Paragraph::new(Span::styled(
            " AGENTS",
            if browsing {
                heading.fg(theme.accent)
            } else {
                heading
            },
        )),
        row(top),
    );
    let ids = app.space_sessions();
    let mut y = top + 2;
    if ids.is_empty() {
        if y < bottom {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "   none: run one in a tab",
                    Style::new().fg(theme.muted),
                )),
                row(y),
            );
        }
        return;
    }
    let available = bottom.saturating_sub(y) / 2;
    let cursor_idx = ids
        .iter()
        .position(|id| Some(*id) == app.cursor)
        .unwrap_or(0);
    let start = (cursor_idx + 1).saturating_sub(available.max(1) as usize);
    let current = app.focused();
    let shown = app.active_tab().map(|t| t.root.panes()).unwrap_or_default();
    for (position, id) in ids.iter().enumerate().skip(start) {
        if y + 2 > bottom {
            break;
        }
        let Some(s) = app.sessions.get(id) else {
            continue;
        };
        hits.push((
            Rect {
                height: 2,
                ..row(y)
            },
            SidebarHit::Session(*id),
        ));
        draw_sidebar_session(
            frame,
            [row(y), row(y + 1)],
            app,
            s,
            position,
            if current == Some(*id) {
                Shown::Focused
            } else if shown.contains(id) {
                Shown::Pane
            } else {
                Shown::No
            },
            browsing && app.cursor == Some(*id),
            !app.sharing_dir(*id).is_empty(),
            theme,
        );
        y += 2;
    }
}

/// Whether a sidebar session is on screen.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shown {
    No,
    /// In another pane of the active tab.
    Pane,
    Focused,
}

#[allow(clippy::too_many_arguments)]
fn draw_sidebar_session(
    frame: &mut Frame,
    [row1, row2]: [Rect; 2],
    app: &App,
    s: &SessionInfo,
    position: usize,
    shown: Shown,
    is_cursor: bool,
    shared: bool,
    theme: &Theme,
) {
    let width = row1.width as usize;
    // A terminal running an agent is listed as that agent, like herdr.
    let detected = s.presence.as_ref().filter(|_| s.state.is_alive());
    let name = match detected {
        Some(p) if s.agent == "shell" && s.label == s.agent => p.agent.clone(),
        _ => s.label.clone(),
    };
    let base = if is_cursor {
        theme.cursor_row()
    } else {
        Style::new()
    };
    let (dot, dot_color) = session_dot(s, app, theme);
    let label_style = match ((shown == Shown::Focused), s.state.is_alive()) {
        (true, _) => base.fg(theme.accent).add_modifier(Modifier::BOLD),
        (false, true) => base.fg(theme.text),
        (false, false) => base.fg(theme.muted),
    };
    let number = if position < 9 {
        format!("{} ", position + 1)
    } else {
        "  ".into()
    };
    let label_width = width.saturating_sub(4 + number.len() + if shared { 2 } else { 0 });
    let mut line1 = vec![
        Span::styled(
            if shown == Shown::No {
                " "
            } else {
                theme.glyphs.current
            },
            base.fg(if shown == Shown::Focused {
                theme.accent
            } else {
                theme.accent_dim
            }),
        ),
        Span::styled(format!("{dot} "), base.fg(dot_color)),
        Span::styled(truncate(theme, &name, label_width), label_style),
    ];
    if shared {
        // Another running agent uses the same directory.
        line1.push(Span::styled(
            " !",
            base.fg(theme.warning).add_modifier(Modifier::BOLD),
        ));
    }
    // Branch (or directory) first: it tells sessions apart, so it should
    // survive truncation.
    let place = s
        .branch
        .clone()
        .unwrap_or_else(|| dir_name(&s.cwd).to_string());
    let mut detail = Vec::new();
    if s.state == SessionState::Lost {
        detail.push("ended".to_string());
    }
    if let Some(a) = app.activity(s.id).filter(|a| *a != Activity::Idle) {
        detail.push(a.word().to_string());
    }
    detail.push(place);
    // The agent actually running, e.g. Claude Code started from a shell.
    match detected {
        Some(p) if p.label != s.label && name == s.label => detail.push(p.label.clone()),
        Some(_) => {}
        None if s.label != s.agent => detail.push(s.agent.clone()),
        None => {}
    }
    let mut line2 = vec![Span::styled("   ", base)];
    let mut room = width.saturating_sub(4);
    if s.worktree.is_some() {
        line2.push(Span::styled("wt ", base.fg(theme.accent_dim)));
        room = room.saturating_sub(3);
    }
    line2.push(Span::styled(
        truncate(theme, &detail.join(" · "), room),
        base.fg(theme.muted),
    ));
    frame.render_widget(Paragraph::new(Line::from(line1)).style(base), row1);
    frame.render_widget(
        Paragraph::new(Span::styled(number, base.fg(theme.muted))).alignment(Alignment::Right),
        row1,
    );
    frame.render_widget(Paragraph::new(Line::from(line2)).style(base), row2);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let bar = theme.bar();
    let key = bar.fg(theme.accent);
    let muted = bar.fg(theme.muted);
    let hints = |pairs: &[(&str, &str)]| -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for (k, d) in pairs {
            spans.push(Span::styled(format!(" {k}"), key));
            spans.push(Span::styled(format!(" {d} "), muted));
        }
        spans
    };

    if let Link::Reconnecting { attempt, .. } = &app.link {
        let line = Line::from(vec![
            Span::styled(" Reconnecting to hived", bar.fg(theme.warning)),
            Span::styled(format!(" · attempt {attempt}"), muted),
            Span::styled("  q", key),
            Span::styled(" quit", muted),
        ]);
        frame.render_widget(Paragraph::new(line).style(bar), area);
        return;
    }

    let left: Vec<Span> = match &app.mode {
        Mode::Leader => {
            let mut spans = vec![Span::styled(
                " LEADER ",
                theme.highlight().add_modifier(Modifier::BOLD),
            )];
            let pairs: Vec<(String, &str)> = [
                (Command::Palette, "commands"),
                (Command::NextSession, "next"),
                (Command::NewSession, "new"),
                (Command::Workspaces, "workspaces"),
                (Command::SearchSessions, "search"),
                (Command::Kill, "kill"),
                (Command::Detach, "detach"),
                (Command::Help, "help"),
            ]
            .into_iter()
            .filter_map(|(c, d)| app.keymap.hint(c).map(|k| (k, d)))
            .chain([("1-9".to_string(), "jump")])
            .collect();
            let pairs: Vec<(&str, &str)> = pairs.iter().map(|(k, d)| (k.as_str(), *d)).collect();
            spans.extend(hints(&pairs));
            spans
        }
        Mode::Palette { query, scope, .. } => vec![
            Span::styled(
                match scope {
                    Scope::Sessions => " Search sessions: ",
                    Scope::Tasks => " Run task: ",
                    Scope::All => " Command: ",
                },
                bar.fg(theme.accent),
            ),
            Span::styled(query.clone(), bar.add_modifier(Modifier::BOLD)),
            Span::styled(theme.glyphs.caret, key),
            Span::styled("  ↑↓ choose · enter run · esc close", muted),
        ],
        Mode::ConfirmKill(id) => {
            let session = app.sessions.get(id);
            let name = session.map(|s| s.label.as_str()).unwrap_or("session");
            let choices = if session.is_some_and(|s| s.worktree.is_some()) {
                "y terminate · w terminate and remove its worktree · any other key cancels"
            } else {
                "y to confirm, any other key cancels"
            };
            vec![Span::styled(
                format!(" Terminate {name} (session {id})? {choices} "),
                bar.fg(theme.warning).add_modifier(Modifier::BOLD),
            )]
        }
        Mode::AddWorkspace { input } => vec![
            Span::styled(" New space (folder): ", bar.fg(theme.accent)),
            Span::styled(input.clone(), bar.add_modifier(Modifier::BOLD)),
            Span::styled(theme.glyphs.caret, key),
            Span::styled("  tab complete · enter open · esc cancel", muted),
        ],
        Mode::RenameWorkspace { input, .. } => vec![
            Span::styled(" Rename workspace: ", bar.fg(theme.accent)),
            Span::styled(input.clone(), bar.add_modifier(Modifier::BOLD)),
            Span::styled(theme.glyphs.caret, key),
            Span::styled("  enter save · esc cancel · empty resets", muted),
        ],
        Mode::Workspaces { .. } => {
            let mut spans = vec![Span::styled(" Workspaces", bar.fg(theme.accent))];
            spans.extend(hints(&[
                ("enter", "open"),
                ("n", "new session"),
                ("a", "add"),
                ("r", "rename"),
                ("x", "remove"),
                ("esc", "close"),
            ]));
            spans
        }
        Mode::Rename { session, input } => vec![
            Span::styled(format!(" Rename session {session}: "), bar.fg(theme.accent)),
            Span::styled(input.clone(), bar.add_modifier(Modifier::BOLD)),
            Span::styled(theme.glyphs.caret, key),
            Span::styled("  enter save · esc cancel · empty resets", muted),
        ],
        Mode::NewSession { .. } => {
            let mut spans = vec![Span::styled(" New session", bar.fg(theme.accent))];
            spans.extend(hints(&[
                ("↑↓", "choose"),
                ("enter", "launch"),
                ("esc", "cancel"),
            ]));
            if app.picker_can_worktree() {
                spans.extend(hints(&[("w", "worktree")]));
            }
            spans
        }
        Mode::Help => vec![Span::styled(" Help · any key to close", muted)],
        Mode::Normal => {
            if let Some((tone, msg)) = &app.status {
                let fg = match tone {
                    Tone::Info => theme.text,
                    Tone::Warning => theme.warning,
                    Tone::Error => theme.error,
                };
                vec![Span::styled(format!(" {msg}"), bar.fg(fg))]
            } else if app.sidebar_keys_active() && !app.sessions.is_empty() {
                hints(&[
                    ("↑↓", "select"),
                    ("enter", "open"),
                    ("n", "new"),
                    ("w", "workspaces"),
                    ("r", "rename"),
                    ("x", "kill"),
                    ("q", "detach"),
                ])
            } else if let Some(s) = app.current() {
                session_summary(s, app, theme, bar)
            } else {
                vec![Span::styled(format!(" {}", counts(app)), muted)]
            }
        }
    };
    frame.render_widget(Paragraph::new(Line::from(left)).style(bar), area);

    if app.mode == Mode::Normal && app.status.is_none() && !app.sidebar_keys_active() {
        let mut hint = vec![
            Span::styled(app.keymap.leader.to_string(), key),
            Span::styled(" leader", muted),
        ];
        if let Some(p) = app.keymap.hint(Command::Palette) {
            hint.push(Span::styled(" · then ", muted));
            hint.push(Span::styled(p, key));
            hint.push(Span::styled(" commands", muted));
        }
        hint.push(Span::styled(" ", muted));
        let hint = Line::from(hint);
        frame.render_widget(Paragraph::new(hint).alignment(Alignment::Right), area);
    }
}

fn session_summary<'a>(s: &SessionInfo, app: &App, theme: &Theme, bar: Style) -> Vec<Span<'a>> {
    let (dot, color) = session_dot(s, app, theme);
    let state = match &s.state {
        SessionState::Starting => "starting".to_string(),
        SessionState::Running => {
            let mut state = match (&s.presence, app.activity(s.id)) {
                (Some(p), Some(a)) => format!("{} {}", p.label, a.word()),
                (Some(p), None) => p.label.clone(),
                _ => "running".to_string(),
            };
            if let Some(m) = s.presence.as_ref().and_then(|p| p.message.as_ref()) {
                state = format!("{state}: {m}");
            }
            state
        }
        SessionState::Exited { code: Some(c), .. } => format!("exited ({c})"),
        SessionState::Exited { signal, .. } => {
            format!("killed ({})", signal.as_deref().unwrap_or("signal"))
        }
        SessionState::Failed { reason } => format!("failed: {reason}"),
        SessionState::Lost => "ended when hived stopped".to_string(),
    };
    let mut place = app.workspace_name(s.workspace).to_string();
    if let Some(branch) = &s.branch {
        place = format!("{place} · {branch}");
    }
    let mut spans = vec![
        Span::styled(format!(" {dot} "), bar.fg(color)),
        Span::styled(format!("#{} {}", s.id, s.label), bar),
        Span::styled(
            format!(" · {place} · {state} · {}", counts(app)),
            bar.fg(theme.muted),
        ),
    ];
    if app.view().is_some_and(|v| !v.controller) {
        spans.push(Span::styled(" · observing", bar.fg(theme.warning)));
    }
    spans
}

fn counts(app: &App) -> String {
    let active = app.sessions.values().filter(|s| s.state.is_alive()).count();
    let lost = app
        .sessions
        .values()
        .filter(|s| s.state == SessionState::Lost)
        .count();
    let exited = app.sessions.len() - active - lost;
    let mut out = format!("{active} active");
    if exited > 0 {
        out += &format!(" · {exited} exited");
    }
    if lost > 0 {
        out += &format!(" · {lost} ended");
    }
    out
}

fn short_path(p: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && p.starts_with(&home) => format!("~{}", &p[home.len()..]),
        _ => p.to_string(),
    }
}

fn draw_empty(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let muted = Style::new().fg(theme.muted);
    let key = Style::new().fg(theme.accent);
    let mut lines = Vec::new();
    // The hexagon mark, when there is room for it.
    if area.height >= brand::MARK_ART.len() as u16 + 10 {
        lines.extend(
            brand::MARK_ART
                .iter()
                .map(|l| Line::from(Span::styled(*l, Style::new().fg(theme.accent_dim)))),
        );
        lines.push(Line::raw(""));
    }
    lines.extend([
        Line::from(Span::styled(
            format!("{} {}", theme.glyphs.mark, brand::NAME),
            key.add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled(brand::TAGLINE, Style::new().fg(theme.text))),
        Line::raw(""),
    ]);
    if app.sessions.is_empty() {
        lines.push(Line::from(Span::styled("No active sessions.", muted)));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Press ", muted),
            Span::styled("n", key),
            Span::styled(" to launch an agent, ", muted),
            Span::styled("q", key),
            Span::styled(" to quit.", muted),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Select a session with ", muted),
            Span::styled("↑↓", key),
            Span::styled(" and ", muted),
            Span::styled("enter", key),
            Span::styled(", or ", muted),
            Span::styled("n", key),
            Span::styled(" for a new one.", muted),
        ]));
    }
    let height = lines.len() as u16;
    let [_, mid, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), mid);
}

fn popup(
    frame: &mut Frame,
    area: Rect,
    width: u16,
    height: u16,
    title: &str,
    theme: &Theme,
) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let rect = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_set(theme.glyphs.border)
        .border_style(Style::new().fg(theme.accent_dim))
        .title(Span::styled(
            format!(" {title} "),
            Style::new().fg(theme.muted),
        ));
    let inner = block.inner(rect);
    frame.render_widget(Clear, rect);
    frame.render_widget(block, rect);
    inner
}

fn draw_picker(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    selected: usize,
    workspace: Option<WorkspaceId>,
    worktree: bool,
    theme: &Theme,
) {
    let title = match workspace {
        Some(w) => format!("New session in {}", app.workspace_name(w)),
        None => "New session".to_string(),
    };
    let can_worktree = app.picker_can_worktree();
    let extra = if can_worktree { 2 } else { 0 };
    let inner = popup(
        frame,
        area,
        56,
        app.agents.len() as u16 + 2 + extra,
        &title,
        theme,
    );
    let mut lines: Vec<Line> = app
        .agents
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let chosen = i == selected;
            let base = if chosen {
                theme.highlight()
            } else {
                Style::new()
            };
            let name = if a.installed {
                base.fg(if chosen { theme.on_accent } else { theme.text })
            } else {
                base.fg(if chosen { theme.on_accent } else { theme.muted })
            };
            let num = if i < 9 {
                format!(" {} ", i + 1)
            } else {
                "   ".into()
            };
            let what = match a.label {
                Some(label) => format!("{label} · {}", a.command),
                None => a.command.clone(),
            };
            let detail = if a.installed {
                what
            } else {
                format!("{what} (not installed)")
            };
            Line::from(vec![
                Span::styled(
                    num,
                    base.fg(if chosen { theme.on_accent } else { theme.muted }),
                ),
                Span::styled(
                    format!("{:<12} ", a.name),
                    name.add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate(theme, &detail, inner.width.saturating_sub(17) as usize),
                    base.fg(if chosen { theme.on_accent } else { theme.muted }),
                ),
            ])
            .style(base)
        })
        .collect();
    if can_worktree {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(" w ", Style::new().fg(theme.accent)),
            Span::styled("own git worktree: ", Style::new().fg(theme.muted)),
            if worktree {
                Span::styled(
                    "on (new branch)",
                    Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("off (shares the repository)", Style::new().fg(theme.text))
            },
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_workspaces(frame: &mut Frame, area: Rect, app: &App, selected: usize, theme: &Theme) {
    let order = app.workspace_order();
    let inner = popup(
        frame,
        area,
        60,
        order.len().max(1) as u16 + 2,
        "Workspaces",
        theme,
    );
    if order.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " No workspaces yet. a adds one.",
                Style::new().fg(theme.muted),
            )),
            inner,
        );
        return;
    }
    let lines: Vec<Line> = order
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let chosen = i == selected;
            let fg = |c: Color| if chosen { theme.on_accent } else { c };
            let base = if chosen {
                theme.highlight()
            } else {
                Style::new()
            };
            let count = app.sessions.values().filter(|s| s.workspace == *w).count();
            let root = app
                .workspaces
                .get(w)
                .map(|w| short_path(&w.root))
                .unwrap_or_default();
            let num = if i < 9 {
                format!(" {} ", i + 1)
            } else {
                "   ".into()
            };
            Line::from(vec![
                Span::styled(num, base.fg(fg(theme.muted))),
                Span::styled(
                    format!("{:<14} ", truncate(theme, app.workspace_name(*w), 14)),
                    base.fg(fg(theme.text)).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "{count:>2} {:<9}",
                        if count == 1 { "session" } else { "sessions" }
                    ),
                    base.fg(fg(theme.muted)),
                ),
                Span::styled(
                    truncate(theme, &root, inner.width.saturating_sub(31) as usize),
                    base.fg(fg(theme.muted)),
                ),
            ])
            .style(base)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_reconnecting(frame: &mut Frame, area: Rect, attempt: u32, reason: &str, theme: &Theme) {
    let inner = popup(frame, area, 56, 7, "Connection lost", theme);
    let lines = vec![
        Line::from(Span::styled(
            truncate(theme, reason, inner.width.saturating_sub(2) as usize),
            Style::new().fg(theme.muted),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Reconnecting to hived… ", Style::new().fg(theme.text)),
            Span::styled(format!("attempt {attempt}"), Style::new().fg(theme.muted)),
        ]),
        Line::from(Span::styled(
            "Agents keep running unless hived itself died.",
            Style::new().fg(theme.muted),
        )),
        Line::from(vec![
            Span::styled("q", Style::new().fg(theme.accent)),
            Span::styled(" to quit", Style::new().fg(theme.muted)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Generated from the keymap, so it shows the user's own bindings.
fn draw_help(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let leader = app.keymap.leader.to_string();
    let mut rows: Vec<(String, String)> = vec![(
        format!("{leader}, then:"),
        "everything else goes to the agent".into(),
    )];
    for cmd in Command::ALL {
        let keys = app.keymap.keys_for(cmd);
        if keys.is_empty() {
            continue;
        }
        let keys: Vec<String> = keys.iter().map(ToString::to_string).collect();
        rows.push((format!("  {}", keys.join(" ")), cmd.title().to_string()));
    }
    rows.push(("  1-9".into(), "Jump to session".into()));
    rows.push((format!("  {leader}"), format!("Send {leader} to the agent")));
    let mut globals: Vec<(String, String)> = Command::ALL
        .iter()
        .filter_map(|&cmd| {
            let keys = app.keymap.global_keys_for(cmd);
            (!keys.is_empty()).then(|| {
                let keys: Vec<String> = keys.iter().map(ToString::to_string).collect();
                (format!("  {}", keys.join(" ")), cmd.title().to_string())
            })
        })
        .collect();
    if let Some(jump) = app.keymap.jump_hint() {
        globals.insert(0, (format!("  {jump}"), "Jump to session".into()));
    }
    if !globals.is_empty() {
        rows.push(("Without the leader:".into(), "[keys.global]".into()));
        rows.extend(globals);
    }
    rows.push((String::new(), String::new()));
    rows.push((
        "Sidebar".into(),
        "↑↓ select · enter open · n new · x kill · esc".into(),
    ));
    rows.push((
        "Mouse".into(),
        "click a session · wheel scrolls · shift+drag selects".into(),
    ));
    rows.push((
        "New-session picker".into(),
        "w toggles its own git worktree".into(),
    ));
    rows.push((
        "Unbound commands".into(),
        "in the command palette; bind in [keys.bind]".into(),
    ));
    let inner = popup(
        frame,
        area,
        76,
        rows.len() as u16 + 2,
        "Help · any key to close",
        theme,
    );
    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<24}"), Style::new().fg(theme.accent)),
                Span::styled(d.clone(), Style::new().fg(theme.text)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_palette(frame: &mut Frame, area: Rect, app: &App, selected: usize, theme: &Theme) {
    let items = app.palette_items();
    let rows = items.len().clamp(1, 14) as u16;
    let inner = popup(frame, area, 70, rows + 2, "Commands", theme);
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(" No matches", Style::new().fg(theme.muted))),
            inner,
        );
        return;
    }
    let visible = inner.height as usize;
    let offset = selected.saturating_sub(visible.saturating_sub(1));
    let width = inner.width as usize;
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, item)| {
            let chosen = i == selected;
            let base = if chosen {
                theme.highlight()
            } else {
                Style::new()
            };
            let fg = |c: Color| if chosen { theme.on_accent } else { c };
            let marker = match item.target {
                Target::Command(_) => " ",
                Target::Session(id) => app
                    .sessions
                    .get(&id)
                    .map_or(" ", |s| state_dot(&s.state, theme).0),
                Target::Workspace(_) => theme.glyphs.group,
                Target::Task(_) => theme.glyphs.caret,
            };
            let key = item.key.clone().unwrap_or_default();
            let room = width.saturating_sub(4 + key.chars().count() + 1);
            let title_w = item.title.chars().count().min(room);
            let detail_room = room.saturating_sub(title_w + 2);
            let mut spans = vec![
                Span::styled(format!(" {marker} "), base.fg(fg(theme.muted))),
                Span::styled(truncate(theme, &item.title, room), base.fg(fg(theme.text))),
            ];
            if !item.detail.is_empty() && detail_room > 3 {
                spans.push(Span::styled(
                    format!("  {}", truncate(theme, &item.detail, detail_room)),
                    base.fg(fg(theme.muted)),
                ));
            }
            let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            spans.push(Span::styled(
                format!("{:>w$} ", key, w = width.saturating_sub(used + 1)),
                base.fg(fg(theme.accent)),
            ));
            Line::from(spans).style(base)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Copies the emulator's visible screen into `area`, clipping if the PTY is
/// larger than the panel (e.g. while observing another client's session).
pub fn draw_terminal(buf: &mut Buffer, area: Rect, screen: &vt100::Screen) {
    let (rows, cols) = screen.size();
    for row in 0..rows.get().min(area.height) {
        for col in 0..cols.get().min(area.width) {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                // The wide glyph to the left covers this cell.
                continue;
            }
            let mut style = Style::new()
                .fg(color(cell.fgcolor()))
                .bg(color(cell.bgcolor()));
            let mut mods = Modifier::empty();
            mods.set(Modifier::BOLD, cell.bold());
            mods.set(Modifier::DIM, cell.dim());
            mods.set(Modifier::ITALIC, cell.italic());
            mods.set(Modifier::UNDERLINED, cell.underline());
            mods.set(Modifier::REVERSED, cell.inverse());
            style = style.add_modifier(mods);
            let symbol = if cell.has_contents() {
                cell.contents()
            } else {
                " "
            };
            if let Some(target) = buf.cell_mut((area.x + col, area.y + row)) {
                target.set_symbol(symbol).set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU16;

    use crate::app::Start;
    use hive_core::config::Config;
    use hive_protocol::{ServerMessage, TermSize};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn session(id: u64, label: &str, state: SessionState) -> SessionInfo {
        SessionInfo {
            id,
            agent: "claude".into(),
            label: label.into(),
            command: vec!["claude".into()],
            cwd: "/home/u/proj".into(),
            state,
            size: TermSize::DEFAULT,
            created_at: 0,
            attached_clients: 0,
            workspace: 1,
            branch: None,
            worktree: None,
            presence: None,
        }
    }

    fn workspace(id: u64, name: &str) -> hive_protocol::WorkspaceInfo {
        hive_protocol::WorkspaceInfo {
            id,
            name: name.into(),
            root: format!("/home/u/{name}"),
            is_repo: true,
        }
    }

    fn rendered(app: &App, w: u16, h: u16) -> Vec<String> {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, app, &Theme::hive_dark(true)))
            .unwrap();
        let buf = term.backend().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect()
    }

    #[test]
    fn sidebar_lists_sessions_and_marks_the_current_one() {
        let mut app = App::new(Config::default(), Start::Attach(2), (80, 12));
        app.on_connect();
        app.on_server(ServerMessage::WorkspaceList(vec![workspace(1, "proj")]));
        app.on_server(ServerMessage::SessionList(vec![
            session(1, "claude", SessionState::Running),
            session(2, "api", SessionState::Running),
            session(
                3,
                "old",
                SessionState::Exited {
                    code: Some(0),
                    signal: None,
                },
            ),
        ]));
        app.on_server(ServerMessage::Attached {
            session: session(2, "api", SessionState::Running),
            snapshot: b"agent output".to_vec(),
            controller: true,
        });
        let lines = rendered(&app, 80, 18);
        assert!(lines[0].contains("SPACES"));
        assert!(
            lines[2].contains("● proj") && lines[2].contains('3'),
            "{:?}",
            lines[2]
        );
        assert!(lines[5].contains("+ new") && lines[5].contains("menu"));
        assert!(lines[7].contains("AGENTS"));
        assert!(lines[9].contains("● claude") && lines[9].contains('1'));
        // Both running sessions use /home/u/proj: flagged as sharing.
        assert!(lines[9].contains(" !"), "{:?}", lines[9]);
        assert!(lines[11].starts_with("▌● api"), "{:?}", lines[11]);
        assert!(lines[12].contains("proj · claude"));
        assert!(lines[13].contains("○ old") && !lines[13].contains('!'));
        // The agent's screen starts right of the sidebar.
        let panel: String = lines[1]
            .chars()
            .skip(SIDEBAR_WIDTH as usize)
            .take(12)
            .collect();
        assert_eq!(panel, "agent output");
    }

    #[test]
    fn sidebar_groups_by_workspace_with_branches_and_worktrees() {
        let mut app = App::new(Config::default(), Start::Attach(2), (80, 24));
        app.on_connect();
        app.on_server(ServerMessage::WorkspaceList(vec![
            workspace(1, "web"),
            workspace(2, "api"),
            workspace(3, "empty"),
        ]));
        let mut wt = session(2, "codex", SessionState::Running);
        wt.workspace = 2;
        wt.agent = "codex".into();
        wt.cwd = "/data/wt/hive-codex-2".into();
        wt.branch = Some("hive/codex-2".into());
        wt.worktree = Some(wt.cwd.clone());
        let mut main = session(1, "claude", SessionState::Running);
        main.branch = Some("main".into());
        // Session 3 belongs to web but is created last: it still sorts there.
        let mut late = session(3, "later", SessionState::Running);
        late.cwd = "/home/u/other".into();
        app.on_server(ServerMessage::SessionList(vec![main, wt, late]));
        assert_eq!(app.order(), vec![1, 3, 2]);

        // Spaces with their branch (the main checkout's) and agent count;
        // the agents listed are the selected space's.
        let lines = rendered(&app, 80, 24);
        assert!(lines[2].contains("· web") && lines[2].contains('2'));
        assert!(lines[3].contains("main"), "{:?}", lines[3]);
        assert!(lines[4].contains("● api"), "selected: {:?}", lines[4]);
        assert!(lines[5].contains("hive/codex-2"));
        assert!(lines[6].contains("· empty"));
        assert!(lines[13].contains("● codex") && lines[13].contains('1'));
        assert!(lines[14].contains("wt hive/codex-2"), "{:?}", lines[14]);
        assert!(!lines.iter().any(|l| l.contains("later")), "other space");

        // Clicking a space lists its agents, numbered from 1 for Alt+digit.
        let hit = app
            .sidebar_hits
            .borrow()
            .iter()
            .find(|(_, h)| *h == SidebarHit::Workspace(1))
            .map(|(r, _)| *r)
            .unwrap();
        assert_eq!(hit.y, 2);
        app.cursor = Some(1);
        assert_eq!(app.space_sessions(), vec![1, 3]);
        let lines = rendered(&app, 80, 24);
        assert!(lines[15].contains("● later") && lines[15].contains('2'));
    }

    #[test]
    fn ascii_glyphs_and_mono_render_without_unicode_or_colour() {
        let mut app = App::new(Config::default(), Start::Attach(1), (80, 16));
        app.on_connect();
        app.on_server(ServerMessage::WorkspaceList(vec![workspace(1, "proj")]));
        app.on_server(ServerMessage::SessionList(vec![session(
            1,
            "claude",
            SessionState::Running,
        )]));
        app.on_server(ServerMessage::Attached {
            session: session(1, "claude", SessionState::Running),
            snapshot: b"ok".to_vec(),
            controller: true,
        });
        let theme = crate::theme::Theme::from_config(
            "mono",
            &hive_core::config::ThemeConfig {
                glyphs: Some("ascii".into()),
                ..Default::default()
            },
            &crate::theme::Env::default(),
        )
        .unwrap();
        let mut term = Terminal::new(TestBackend::new(80, 16)).unwrap();
        term.draw(|f| draw(f, &app, &theme)).unwrap();
        let buf = term.backend().buffer();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(text.is_ascii(), "non-ASCII glyph drawn: {text:?}");
        assert!(text.contains("SPACES") && text.contains(">* claude"));
        assert!(
            buf.content()
                .iter()
                .all(|c| c.fg == Color::Reset && c.bg == Color::Reset),
            "mono uses no colours"
        );
    }

    #[test]
    fn ascii_mode_leaves_the_agents_own_output_alone() {
        let mut app = App::new(Config::default(), Start::Attach(1), (50, 6));
        app.on_connect();
        app.on_server(ServerMessage::Attached {
            session: session(1, "claude", SessionState::Running),
            snapshot: "agent → 中".as_bytes().to_vec(),
            controller: true,
        });
        let theme = crate::theme::Theme::from_config(
            "hive-dark",
            &hive_core::config::ThemeConfig {
                glyphs: Some("ascii".into()),
                ..Default::default()
            },
            &crate::theme::Env::default(),
        )
        .unwrap();
        let mut term = Terminal::new(TestBackend::new(50, 6)).unwrap();
        term.draw(|f| draw(f, &app, &theme)).unwrap();
        let row: String = (0..50)
            .map(|x| term.backend().buffer()[(x, 1)].symbol().to_string())
            .collect();
        assert!(row.starts_with("agent → 中"), "{row:?}");
    }

    #[test]
    fn narrow_terminal_hides_the_sidebar() {
        let mut app = App::new(Config::default(), Start::Attach(1), (50, 8));
        app.on_connect();
        app.on_server(ServerMessage::Attached {
            session: session(1, "claude", SessionState::Running),
            snapshot: b"agent output".to_vec(),
            controller: true,
        });
        let lines = rendered(&app, 50, 8);
        assert!(lines[1].starts_with("agent output"));
    }

    #[test]
    fn terminal_cells_and_attributes_are_copied() {
        let mut p =
            vt100::Parser::new(NonZeroU16::new(3).unwrap(), NonZeroU16::new(10).unwrap(), 0);
        p.process(b"ab\x1b[1;31mC\x1b[0m\r\n\xe4\xb8\xadx");
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        draw_terminal(&mut buf, area, p.screen());
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(2, 0)].symbol(), "C");
        assert_eq!(buf[(2, 0)].fg, Color::Indexed(1));
        assert!(buf[(2, 0)].modifier.contains(Modifier::BOLD));
        assert_eq!(buf[(0, 1)].symbol(), "中");
        assert_eq!(buf[(2, 1)].symbol(), "x");
    }

    #[test]
    fn larger_screen_is_clipped_to_panel() {
        let mut p =
            vt100::Parser::new(NonZeroU16::new(5).unwrap(), NonZeroU16::new(20).unwrap(), 0);
        p.process(b"0123456789abcdefghij");
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);
        draw_terminal(&mut buf, area, p.screen());
        assert_eq!(buf[(3, 0)].symbol(), "3");
    }
}
