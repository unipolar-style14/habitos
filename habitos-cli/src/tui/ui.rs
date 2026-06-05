use crate::tui::App;
use crate::tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, root[0], app, theme);
    draw_body(f, root[1], app, theme);
    draw_status(f, root[2], app, theme);

    if app.show_help {
        draw_help(f, f.area(), theme);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let bar = score_bar(app.score);
    let score_style = if app.score >= 80 {
        Style::default().fg(theme.success())
    } else if app.score >= 50 {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.warn())
    };
    let title = Line::from(vec![
        Span::styled("HabitOS", theme.header()),
        Span::raw("  "),
        Span::styled(format!("{}", app.today), theme.dim()),
        Span::raw("    "),
        Span::styled(bar, score_style),
        Span::raw(" "),
        Span::styled(
            format!("{:>3}", app.score),
            score_style.add_modifier(Modifier::BOLD),
        ),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.dim());
    let p = Paragraph::new(title).block(block);
    f.render_widget(p, area);
}

fn score_bar(score: u32) -> String {
    const WIDTH: u32 = 10;
    let filled = (score * WIDTH / 100).min(WIDTH);
    let empty = WIDTH - filled;
    let mut s = String::new();
    for _ in 0..filled {
        s.push('▓');
    }
    for _ in 0..empty {
        s.push('░');
    }
    s
}

fn draw_body(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(cols[0]);
    draw_habits(f, left[0], app, theme);
    draw_focus(f, left[1], app, theme);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Percentage(50)])
        .split(cols[1]);
    draw_goals(f, right[0], app, theme);
    draw_journal(f, right[1], app, theme);
}

fn draw_habits(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .title(Span::styled(" Today ", theme.header()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.dim());
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.habits.is_empty() {
        let p = Paragraph::new("No habits yet. Quit and run `habitos habit add <name>`.")
            .style(theme.dim())
            .wrap(Wrap { trim: true });
        f.render_widget(p, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::with_capacity(app.habits.len());
    for (i, (h, _, stats)) in app.habits.iter().enumerate() {
        let status = app.habit_today_status(i);
        let mark = match status {
            "done" => Span::styled("✔ ", Style::default().fg(theme.success())),
            "skip" => Span::styled("⊘ ", Style::default().fg(theme.warn())),
            _ => Span::styled("○ ", theme.dim()),
        };

        let name = if Some(i) == app.cursor_habit_idx() {
            Span::styled(format!("{:<18}", h.name), theme.cursor())
        } else {
            Span::styled(format!("{:<18}", h.name), theme.fg())
        };

        let streak_str = if stats.current_streak == 0 {
            "  · ".to_string()
        } else if stats.current_streak >= 3 {
            format!(" {}🔥", stats.current_streak)
        } else {
            format!(" {}  ", stats.current_streak)
        };
        let streak = Span::styled(
            streak_str,
            Style::default().fg(theme.streak_color(stats.current_streak)),
        );

        let rate = Span::styled(format!("   {:>3}%", stats.completion_rate_30d), theme.dim());

        lines.push(Line::from(vec![Span::raw(" "), mark, name, streak, rate]));
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn draw_focus(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .title(Span::styled(" Focus ", theme.header()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.dim());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = match (&app.focus, app.focus_duration_minutes()) {
        (Some(s), Some(mins)) => {
            let project = s.project.as_deref().unwrap_or("(no project)");
            vec![
                Line::from(vec![
                    Span::styled(" ● ", Style::default().fg(theme.success())),
                    Span::styled("active  ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(project.to_string(), Style::default().fg(theme.accent())),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(format!("{mins}m elapsed"), theme.dim()),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(" [f] stop session", theme.dim())]),
            ]
        }
        _ => vec![
            Line::from(vec![
                Span::styled(" ○ ", theme.dim()),
                Span::styled("no active session", theme.dim()),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(" [f] start session", theme.dim())]),
        ],
    };
    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn draw_goals(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .title(Span::styled(" Goals ", theme.header()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.dim());
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.goals.is_empty() {
        let p = Paragraph::new("No open goals. Run `habitos goal add <name>`.")
            .style(theme.dim())
            .wrap(Wrap { trim: true });
        f.render_widget(p, inner);
        return;
    }

    let goal_count = app.goals.len();
    let constraints: Vec<Constraint> = (0..goal_count).map(|_| Constraint::Length(2)).collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, (g, done, total)) in app.goals.iter().enumerate() {
        let pct = if *total == 0 { 0 } else { (done * 100) / total };
        let label = if *total == 0 {
            format!(" {} — no milestones", g.name)
        } else {
            format!(" {}  {}/{}  ", g.name, done, total)
        };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(theme.accent()))
            .percent(pct as u16)
            .label(label);
        f.render_widget(gauge, chunks[i]);
    }
}

fn draw_journal(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .title(Span::styled(" Journal ", theme.header()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.dim());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = match &app.journal_today {
        Some(j) => {
            let mut out = vec![Line::from(Span::styled(
                format!(" {}", j.on_date),
                theme.dim(),
            ))];
            for line in j.body.lines().take(8) {
                out.push(Line::from(Span::styled(format!(" {line}"), theme.fg())));
            }
            out
        }
        None => vec![
            Line::from(Span::styled(" no entry today", theme.dim())),
            Line::from(""),
            Line::from(Span::styled(" [e] open in $EDITOR", theme.dim())),
        ],
    };
    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(p, inner);
}

fn draw_status(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let spans: Vec<Span> = if let Some(msg) = app.flash_message() {
        vec![Span::styled(
            format!(" {msg}"),
            Style::default().fg(theme.accent()),
        )]
    } else {
        vec![Span::styled(
            " ↑↓/jk move  d done  s skip  f focus  e journal  r refresh  ? help  q quit",
            theme.dim(),
        )]
    };
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_help(f: &mut Frame, area: Rect, theme: &Theme) {
    let pop = centered_rect(60, 60, area);
    f.render_widget(Clear, pop);
    let block = Block::default()
        .title(Span::styled(" Keys ", theme.header()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.dim());
    let inner = block.inner(pop);
    f.render_widget(block, pop);

    let lines = vec![
        Line::from(""),
        help_line("  ↑ ↓  /  j k", "move cursor through habits", theme),
        help_line("  d", "mark cursor habit done (today)", theme),
        help_line("  s", "mark cursor habit skipped (today)", theme),
        help_line("  f", "start / stop focus session", theme),
        help_line("  e", "open today's journal in $EDITOR", theme),
        help_line("  r", "refresh data", theme),
        help_line("  ?", "toggle this help", theme),
        help_line("  q  /  ^C", "quit", theme),
        Line::from(""),
        Line::from(Span::styled("  press any key to dismiss", theme.dim())),
    ];
    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn help_line(key: &'static str, desc: &'static str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<14}"), Style::default().fg(theme.accent())),
        Span::styled(desc, theme.fg()),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
