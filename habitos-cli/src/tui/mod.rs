use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use habitos_core::clock::{Clock, SystemClock};
use habitos_core::focus::FocusRepo;
use habitos_core::habits::{HabitRepo, LogStatus};
use habitos_core::journal::JournalRepo;
use habitos_core::{Config, Db};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::{Duration, Instant};

mod app;
mod theme;
mod ui;

pub use app::App;
pub use theme::{Theme, ThemeKind};

pub async fn run(db: &Db, _config: &Config, theme_kind: ThemeKind) -> Result<()> {
    let theme = Theme::new(theme_kind);
    let mut terminal = enter()?;
    let mut app = App::new();
    app.refresh(db).await?;

    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = Instant::now();

    let result: Result<()> = loop {
        if let Err(e) = terminal.draw(|f| ui::draw(f, &app, &theme)) {
            break Err(e.into());
        }

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            if let Err(e) = handle_key(&mut app, db, key, &mut terminal).await {
                break Err(e);
            }
            if app.should_quit {
                break Ok(());
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }
    };

    leave(&mut terminal)?;
    result
}

type Term = Terminal<CrosstermBackend<io::Stdout>>;

fn enter() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let term = Terminal::new(backend)?;
    Ok(term)
}

fn leave(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn handle_key(app: &mut App, db: &Db, key: KeyEvent, term: &mut Term) -> Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    if app.show_help {
        app.show_help = false;
        return Ok(());
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if ctrl => app.should_quit = true,

        KeyCode::Char('?') => app.show_help = true,

        KeyCode::Char('r') => {
            app.refresh(db).await?;
            app.flash("refreshed");
        }

        KeyCode::Char('d') => {
            if let Some(idx) = app.cursor_habit_idx()
                && let Some((habit, _, _)) = app.habits.get(idx)
            {
                let id = habit.id;
                let name = habit.name.clone();
                let repo = HabitRepo::new(db.pool());
                let clock = SystemClock;
                repo.log(id, clock.today_local(), LogStatus::Done, None)
                    .await?;
                app.flash(format!("✓ {name} done"));
                app.refresh(db).await?;
            }
        }

        KeyCode::Char('s') => {
            if let Some(idx) = app.cursor_habit_idx()
                && let Some((habit, _, _)) = app.habits.get(idx)
            {
                let id = habit.id;
                let name = habit.name.clone();
                let repo = HabitRepo::new(db.pool());
                let clock = SystemClock;
                repo.log(id, clock.today_local(), LogStatus::Skipped, None)
                    .await?;
                app.flash(format!("⊘ {name} skipped"));
                app.refresh(db).await?;
            }
        }

        KeyCode::Char('f') => {
            let focus_repo = FocusRepo::new(db.pool());
            if focus_repo.active().await?.is_some() {
                let s = focus_repo.stop().await?;
                let mins = s.duration_minutes().unwrap_or(0);
                app.flash(format!("focus stopped — {mins}m"));
            } else {
                focus_repo.start(None, None).await?;
                app.flash("focus started");
            }
            app.refresh(db).await?;
        }

        KeyCode::Char('e') => {
            // Suspend TUI, run $EDITOR on today's journal, then resume.
            leave(term)?;
            let res = edit_journal_inline(db).await;
            *term = enter()?;
            term.clear()?;
            match res {
                Ok(true) => app.flash("journal saved"),
                Ok(false) => app.flash("journal unchanged"),
                Err(e) => app.flash(format!("journal error: {e}")),
            }
            app.refresh(db).await?;
        }

        KeyCode::Down | KeyCode::Char('j') => app.cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => app.cursor_up(),

        _ => {}
    }
    Ok(())
}

async fn edit_journal_inline(db: &Db) -> Result<bool> {
    use std::io::Write as _;
    let clock = SystemClock;
    let today = clock.today_local();
    let repo = JournalRepo::new(db.pool());

    let existing = repo.get(today).await?.map(|e| e.body).unwrap_or_default();

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let mut tmp = tempfile::Builder::new()
        .prefix("habitos-journal-")
        .suffix(".md")
        .tempfile()?;
    tmp.write_all(existing.as_bytes())?;
    tmp.flush()?;
    let path = tmp.path().to_path_buf();

    let cmd = format!("{editor} {}", shell_quote(&path));
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()?;
    if !status.success() {
        anyhow::bail!("editor exited with {status}");
    }
    let contents = std::fs::read_to_string(&path)?;
    let trimmed = contents.trim();
    if trimmed.is_empty() || trimmed == existing.trim() {
        return Ok(false);
    }
    repo.upsert(today, &contents).await?;
    Ok(true)
}

fn shell_quote(p: &std::path::Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"))
}
