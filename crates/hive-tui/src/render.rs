//! Drawing. Reads `App` only; never mutates state.

use hive_core::brand;
use hive_protocol::{SessionInfo, SessionState};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::{App, Mode, Tone};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    let [header, body, status] = Layout::vertical([
        Constraint::Length(app.header_rows()),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    if header.height > 0 {
        draw_header(frame, header, app, theme);
    }
    match &app.view {
        Some(view) => {
            draw_terminal(frame.buffer_mut(), body, view.parser.screen());
            let screen = view.parser.screen();
            if app.can_type() && !screen.hide_cursor() {
                let (row, col) = screen.cursor_position();
                if row < body.height && col < body.width {
                    frame.set_cursor_position((body.x + col, body.y + row));
                }
            }
        }
        None => draw_empty(frame, body, app, theme),
    }
    draw_status(frame, status, app, theme);
    if app.mode == Mode::Help {
        draw_help(frame, body, theme);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let bar = Style::new().bg(theme.surface).fg(theme.text);
    let mut left = vec![Span::styled(
        format!(" {} {}", brand::MARK, brand::NAME),
        bar.fg(theme.accent).add_modifier(Modifier::BOLD),
    )];
    if let Some(s) = app.current() {
        left.push(Span::styled("  ", bar));
        left.push(Span::styled(
            s.agent.to_uppercase(),
            bar.add_modifier(Modifier::BOLD),
        ));
        if s.label != s.agent {
            left.push(Span::styled(format!(" — {}", s.label), bar));
        }
        left.push(Span::styled(
            format!("  {}", short_path(&s.cwd)),
            bar.fg(theme.muted),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(left)).style(bar), area);
    if app.config.branding.show_tagline {
        let tagline = format!("{} ", brand::TAGLINE);
        frame.render_widget(
            Paragraph::new(Span::styled(tagline, bar.fg(theme.muted))).alignment(Alignment::Right),
            area,
        );
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let bar = Style::new().bg(theme.surface).fg(theme.text);
    let key = bar.fg(theme.accent);
    let muted = bar.fg(theme.muted);

    let left: Vec<Span> = match app.mode {
        Mode::Leader => vec![
            Span::styled(
                " LEADER ",
                Style::new()
                    .bg(theme.accent)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  d", key),
            Span::styled(" detach  ", muted),
            Span::styled("n", key),
            Span::styled(" new  ", muted),
            Span::styled("x", key),
            Span::styled(" kill  ", muted),
            Span::styled("a", key),
            Span::styled(" take control  ", muted),
            Span::styled("?", key),
            Span::styled(" help  ", muted),
            Span::styled("Ctrl+Space", key),
            Span::styled(" send literal", muted),
        ],
        Mode::ConfirmKill(id) => {
            let name = app
                .sessions
                .get(&id)
                .map(|s| s.agent.as_str())
                .unwrap_or("session");
            vec![Span::styled(
                format!(" Terminate {name} (session {id})? y to confirm, any other key cancels "),
                bar.fg(theme.warning).add_modifier(Modifier::BOLD),
            )]
        }
        _ => {
            let mut spans = Vec::new();
            if let Some((tone, msg)) = &app.status {
                let fg = match tone {
                    Tone::Info => theme.text,
                    Tone::Warning => theme.warning,
                    Tone::Error => theme.error,
                };
                spans.push(Span::styled(format!(" {msg}"), bar.fg(fg)));
            } else if let Some(s) = app.current() {
                spans.extend(session_summary(s, app, theme, bar));
            } else {
                spans.push(Span::styled(format!(" {}", counts(app)), muted));
            }
            spans
        }
    };
    frame.render_widget(Paragraph::new(Line::from(left)).style(bar), area);

    if app.mode == Mode::Normal {
        let hint = Line::from(vec![
            Span::styled("Ctrl+Space", key),
            Span::styled(" leader · ", muted),
            Span::styled("?", key),
            Span::styled(" help ", muted),
        ]);
        frame.render_widget(Paragraph::new(hint).alignment(Alignment::Right), area);
    }
}

fn session_summary<'a>(s: &SessionInfo, app: &App, theme: &Theme, bar: Style) -> Vec<Span<'a>> {
    let (dot, color, state) = match &s.state {
        SessionState::Starting => ("○", theme.muted, "starting".to_string()),
        SessionState::Running => ("●", theme.success, "running".to_string()),
        SessionState::Exited { code: Some(c), .. } => ("○", theme.muted, format!("exited ({c})")),
        SessionState::Exited { signal, .. } => (
            "○",
            theme.muted,
            format!("killed ({})", signal.as_deref().unwrap_or("signal")),
        ),
        SessionState::Failed { reason } => ("✕", theme.error, format!("failed: {reason}")),
    };
    let mut spans = vec![
        Span::styled(format!(" {dot} "), bar.fg(color)),
        Span::styled(format!("#{} {}", s.id, s.agent), bar),
        Span::styled(format!(" · {state} · {}", counts(app)), bar.fg(theme.muted)),
    ];
    if app.view.as_ref().is_some_and(|v| !v.controller) {
        spans.push(Span::styled(" · observing", bar.fg(theme.warning)));
    }
    if !s.state.is_alive() {
        spans.push(Span::styled(" · n new · q detach", bar.fg(theme.muted)));
    }
    spans
}

fn counts(app: &App) -> String {
    let active = app.sessions.values().filter(|s| s.state.is_alive()).count();
    let exited = app.sessions.len() - active;
    if exited > 0 {
        format!("{active} active · {exited} exited")
    } else {
        format!("{active} active")
    }
}

fn short_path(p: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && p.starts_with(&home) => format!("~{}", &p[home.len()..]),
        _ => p.to_string(),
    }
}

fn draw_empty(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let agent = &app.config.general.default_agent;
    let lines = vec![
        Line::from(Span::styled(
            format!("{} {}", brand::MARK, brand::NAME),
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled(brand::TAGLINE, Style::new().fg(theme.text))),
        Line::raw(""),
        Line::from(Span::styled(
            "No active sessions.",
            Style::new().fg(theme.muted),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Press ", Style::new().fg(theme.muted)),
            Span::styled("n", Style::new().fg(theme.accent)),
            Span::styled(
                format!(" to launch {agent}, "),
                Style::new().fg(theme.muted),
            ),
            Span::styled("q", Style::new().fg(theme.accent)),
            Span::styled(" to quit.", Style::new().fg(theme.muted)),
        ]),
    ];
    let height = lines.len() as u16;
    let [_, mid, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), mid);
}

fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let rows = [
        ("Ctrl+Space d", "Detach (agents keep running)"),
        ("Ctrl+Space n", "New session (default agent)"),
        ("Ctrl+Space x", "Terminate current session"),
        ("Ctrl+Space a", "Take input control back"),
        ("Ctrl+Space Ctrl+Space", "Send a literal Ctrl+Space"),
        ("Ctrl+Space ?", "This help"),
    ];
    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<22}"), Style::new().fg(theme.accent)),
                Span::styled(d.to_string(), Style::new().fg(theme.text)),
            ])
        })
        .collect();
    let w = 60.min(area.width);
    let h = (lines.len() as u16 + 2).min(area.height);
    let popup = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(theme.accent_dim))
                .title(Span::styled(
                    " Help · any key to close ",
                    Style::new().fg(theme.muted),
                )),
        ),
        popup,
    );
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
