use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use habitos_ai::{AiBackendConfig, LlmClient, PromptLoader, build_client};
use habitos_core::clock::{Clock, SystemClock};
use habitos_core::events::EventLog;
use habitos_core::export;
use habitos_core::focus::FocusRepo;
use habitos_core::goals::{Goal, GoalRepo};
use habitos_core::habits::{Habit, HabitLog, HabitRepo, LogOutcome, LogStatus, compute_stats};
use habitos_core::journal::JournalRepo;
use habitos_core::memory::{MemoryRepo, top_k};
use habitos_core::reports;
use habitos_core::reviews::{DailyAnswers, ReviewRepo, month_first_day, week_starting};
use habitos_core::{Config, Db};
use std::io::BufRead as _;
use std::io::Write as _;
use std::path::Path;
use time::Date;
use time::macros::format_description;
use tracing_subscriber::EnvFilter;

const ISO_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");

#[derive(Parser)]
#[command(
    name = "habitos",
    version,
    about = "Local-first AI terminal OS for personal execution"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Habit tracking
    #[command(subcommand)]
    Habit(HabitCmd),
    /// Long-term goal management
    #[command(subcommand)]
    Goal(GoalCmd),
    /// AI-generated daily plan
    Plan,
    /// Deep-work focus sessions
    #[command(subcommand)]
    Focus(FocusCmd),
    /// Daily journal
    #[command(subcommand)]
    Journal(JournalCmd),
    /// Guided end-of-day reflection
    Reflect,
    /// Periodic reviews
    #[command(subcommand)]
    Review(ReviewCmd),
    /// AI coach across habits, goals, focus, journals
    Coach,
    /// Quantitative life insights
    Insights,
    /// Ask a question of your long-term memory
    Ask {
        /// Free-form natural-language query
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
    },
    /// Database management
    #[command(subcommand)]
    Db(DbCmd),
    /// AI backend management
    #[command(subcommand)]
    Ai(AiCmd),
    /// Configuration
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Export data as markdown or CSV
    #[command(subcommand)]
    Export(ExportCmd),
    /// Connect an AI backend (writes config, pulls models)
    #[command(subcommand)]
    Connect(ConnectCmd),
    /// Launch the immersive TUI dashboard
    Tui {
        /// Color theme: system (default, uses your terminal's ANSI palette),
        /// vivid (true-color cyan/green/amber), or mono (no color).
        #[arg(long, default_value = "system")]
        theme: String,
    },
    /// One-line capture: log a habit, focus minutes, and journal note in one shot.
    ///
    /// Examples:
    ///   habitos log "did DSA for 45min"
    ///   habitos log "skipped workout, too tired"
    ///   habitos log "1 hour of reading"
    Log {
        /// Free-form description of what you did
        #[arg(required = true, num_args = 1..)]
        text: Vec<String>,
    },
    /// Year heatmap of daily habit completion (GitHub-contributions style)
    Heatmap {
        /// How many days of history to show (default: 365)
        #[arg(long, default_value = "365")]
        days: u32,
    },
    /// Smart, loss-aversion-framed daily nudge. Prints to stdout; pass
    /// `--notify` to display as a macOS notification instead.
    Nudge {
        /// Display via macOS Notification Center (osascript) instead of stdout
        #[arg(long)]
        notify: bool,
    },
    /// Interactive first-run setup: create a habit, log it, optionally connect AI
    Init,
}

#[derive(Subcommand)]
enum ConnectCmd {
    /// Connect to Anthropic Claude
    Claude {
        /// Anthropic API key (sk-ant-...)
        api_key: String,
        /// Model name (default: claude-sonnet-4-6)
        #[arg(long, default_value = "claude-sonnet-4-6")]
        model: String,
    },
    /// Connect to a local Ollama instance with any model
    Ollama {
        /// Model name (e.g. gemma2:2b, qwen2.5:3b, llama3.2:3b)
        #[arg(default_value = "gemma2:2b")]
        model: String,
        /// Skip `ollama pull` even if model is missing locally
        #[arg(long)]
        no_pull: bool,
    },
    /// Shortcut for Ollama + Gemma
    Gemma {
        #[arg(default_value = "gemma2:2b")]
        model: String,
    },
    /// Shortcut for Ollama + Qwen
    Qwen {
        #[arg(default_value = "qwen2.5:3b")]
        model: String,
    },
    /// Print the currently configured backend
    Status,
    /// Disconnect AI (clear config; commands fall back to deterministic mode)
    Off,
}

#[derive(Subcommand)]
enum ExportCmd {
    /// Single markdown document to stdout
    Markdown,
    /// One CSV per entity into a directory
    Csv {
        /// Target directory (created if missing)
        dir: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
enum HabitCmd {
    /// Create a new habit
    Add { name: String },
    /// Mark a habit as done (today by default)
    Done {
        name: String,
        /// Backfill: log against this YYYY-MM-DD date instead of today
        #[arg(long)]
        at: Option<String>,
        /// Optional note attached to the log entry
        #[arg(long)]
        note: Option<String>,
    },
    /// Mark a habit as skipped — neutral, preserves streak but does not extend it
    Skip {
        name: String,
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// List habits
    List {
        /// Include archived habits
        #[arg(long)]
        archived: bool,
    },
    /// Remove a habit (and all its logs)
    Rm { name: String },
    /// Show streaks, completion rate, and recent history per habit
    Stats,
}

#[derive(Subcommand)]
enum GoalCmd {
    /// Create a new goal
    Add { name: String },
    /// List goals
    List {
        /// Include completed goals
        #[arg(long)]
        all: bool,
    },
    /// Show progress across open goals (uses milestones)
    Progress,
    /// Mark a goal complete
    Complete { name: String },
    /// Manage milestones within a goal
    #[command(subcommand)]
    Milestone(MilestoneCmd),
}

#[derive(Subcommand)]
enum MilestoneCmd {
    /// Add a milestone to a goal
    Add { goal: String, name: String },
    /// Mark a milestone done
    Done { goal: String, name: String },
    /// List milestones in a goal
    List { goal: String },
}

#[derive(Subcommand)]
enum FocusCmd {
    /// Start a focus session
    Start {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Stop the active focus session
    Stop,
    /// Show status of current session
    Status,
}

#[derive(Subcommand)]
enum JournalCmd {
    /// Open $EDITOR for a new entry
    New,
    /// Show today's entry
    Today,
    /// Search journal entries
    Search {
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ReviewCmd {
    /// Daily review
    Day,
    /// Weekly review
    Week,
    /// Monthly review
    Month,
}

#[derive(Subcommand)]
enum DbCmd {
    /// Run pending migrations
    Migrate,
    /// Print the database file path
    Path,
}

#[derive(Subcommand)]
enum AiCmd {
    /// Check the configured AI backend is reachable
    Check,
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Print paths to the data directory and config file
    Path,
}

pub async fn run() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let mut config = Config::load_or_init()?;

    let command = cli.command.unwrap_or(Command::Tui {
        theme: "system".to_string(),
    });
    match command {
        Command::Tui { theme } => {
            let theme_kind = crate::tui::ThemeKind::parse(&theme).unwrap_or_else(|| {
                eprintln!("Unknown theme `{theme}`. Using `system`. Options: system, vivid, mono.");
                crate::tui::ThemeKind::System
            });
            let db = open_and_migrate(&config).await?;
            crate::tui::run(&db, &config, theme_kind).await?;
        }
        Command::Db(DbCmd::Migrate) => {
            tracing::info!(db = %config.db_path().display(), "running migrations");
            let db = Db::open(&config).await?;
            db.migrate().await?;
            println!("Migrations applied to {}", config.db_path().display());
        }
        Command::Db(DbCmd::Path) => {
            println!("{}", config.db_path().display());
        }
        Command::Config(ConfigCmd::Path) => {
            println!("data dir: {}", config.data_dir().display());
            println!("config:   {}", config.config_path().display());
            println!("database: {}", config.db_path().display());
        }
        Command::Habit(sub) => {
            let db = open_and_migrate(&config).await?;
            run_habit(&db, sub).await?;
        }
        Command::Goal(sub) => {
            let db = open_and_migrate(&config).await?;
            run_goal(&db, sub).await?;
        }
        Command::Focus(sub) => {
            let db = open_and_migrate(&config).await?;
            run_focus(&db, sub).await?;
        }
        Command::Journal(sub) => {
            let db = open_and_migrate(&config).await?;
            run_journal(&db, sub).await?;
        }
        Command::Reflect => {
            let db = open_and_migrate(&config).await?;
            run_reflect(&db).await?;
        }
        Command::Review(sub) => {
            let db = open_and_migrate(&config).await?;
            run_review(&db, sub).await?;
        }
        Command::Insights => {
            let db = open_and_migrate(&config).await?;
            run_insights(&db).await?;
        }
        Command::Plan => {
            let db = open_and_migrate(&config).await?;
            run_plan(&db, &config).await?;
        }
        Command::Coach => {
            let db = open_and_migrate(&config).await?;
            run_coach(&db, &config).await?;
        }
        Command::Ai(AiCmd::Check) => {
            run_ai_check(&config).await?;
        }
        Command::Ask { query } => {
            let db = open_and_migrate(&config).await?;
            run_ask(&db, &config, &query.join(" ")).await?;
        }
        Command::Export(sub) => {
            let db = open_and_migrate(&config).await?;
            run_export(&db, sub).await?;
        }
        Command::Connect(sub) => {
            run_connect(&mut config, sub).await?;
        }
        Command::Log { text } => {
            let db = open_and_migrate(&config).await?;
            run_log(&db, &text.join(" ")).await?;
        }
        Command::Heatmap { days } => {
            let db = open_and_migrate(&config).await?;
            run_heatmap(&db, days).await?;
        }
        Command::Nudge { notify } => {
            let db = open_and_migrate(&config).await?;
            run_nudge(&db, notify).await?;
        }
        Command::Init => {
            let db = open_and_migrate(&config).await?;
            run_init(&db, &mut config).await?;
        }
    }
    Ok(())
}

async fn run_init(db: &Db, config: &mut Config) -> Result<()> {
    use std::io::Write as _;

    println!();
    println!("Welcome to HabitOS — local-first habit + focus + journal in your terminal.");
    println!("Data lives at: {}", config.data_dir().display());
    println!();

    // 1. Habit setup (skip if any already exist)
    let habit_repo = HabitRepo::new(db.pool());
    let existing = habit_repo.list(true).await?;

    if existing.is_empty() {
        let name = prompt_line(
            "Pick one habit to start tracking (e.g. Workout, Read, Meditate). Empty to skip.\n> ",
        )?;
        if !name.is_empty() {
            let h = habit_repo.add(&name).await?;
            println!("✓ Added `{}`.", h.name);

            if prompt_yes_no(&format!("Mark `{}` done for today?", h.name), true)? {
                let clock = SystemClock;
                habit_repo
                    .log(h.id, clock.today_local(), LogStatus::Done, None)
                    .await?;
                println!("✓ Logged done for today. Your streak is officially started.");
            }
        }
    } else {
        println!(
            "You already have {} habit(s) — skipping habit setup.",
            existing.len()
        );
    }

    // 2. AI setup (optional)
    println!();
    if prompt_yes_no(
        "Connect an AI backend for plan / coach / ask features? (you can do this later)",
        false,
    )? {
        run_init_ai(config)?;
    } else {
        println!(
            "Skipped. Configure later with `habitos connect claude <key>` or `habitos connect ollama <model>`."
        );
    }

    // 3. Pointers
    println!();
    println!("You're set. Try:");
    println!("  habitos log \"did Workout 30min\"   one-line capture");
    println!("  habitos                            immersive TUI dashboard");
    println!("  habitos heatmap                    year of progress (once you have data)");
    println!("  habitos --help                     full command list");
    let _ = std::io::stdout().flush();
    Ok(())
}

fn run_init_ai(config: &mut Config) -> Result<()> {
    use habitos_core::config::AiConfig;
    use std::io::Write as _;

    println!();
    println!("Pick a backend:");
    println!("  [1] Claude (Anthropic API key required)");
    println!("  [2] Ollama (local model, must be installed)");
    println!("  [Enter] Skip");
    let choice = prompt_line("> ")?;

    match choice.as_str() {
        "1" => {
            let key = prompt_line("Anthropic API key (sk-ant-...): ")?;
            if key.is_empty() {
                println!("No key provided. Skipping.");
                return Ok(());
            }
            config.save_ai(AiConfig {
                backend: Some("anthropic".into()),
                model: Some("claude-sonnet-4-6".into()),
                endpoint: None,
                api_key: Some(key),
                timeout_secs: 30,
            })?;
            println!("✓ Saved Claude config (model = claude-sonnet-4-6).");
        }
        "2" => {
            if !command_exists("ollama") {
                println!("`ollama` not found on PATH.");
                println!("  Install: brew install ollama   (macOS)");
                println!("  Then:    ollama serve          (in another tab)");
                println!("  Then:    habitos connect ollama gemma2:2b");
                return Ok(());
            }
            let raw = prompt_line("Model? [gemma2:2b] ")?;
            let model = if raw.is_empty() {
                "gemma2:2b".to_string()
            } else {
                raw
            };
            println!("Pulling `{model}` via Ollama (cached if already present)...");
            let _ = std::io::stdout().flush();
            let status = std::process::Command::new("ollama")
                .arg("pull")
                .arg(&model)
                .status()?;
            if !status.success() {
                println!("Ollama pull failed. Skipping.");
                return Ok(());
            }
            config.save_ai(AiConfig {
                backend: Some("ollama".into()),
                model: Some(model.clone()),
                endpoint: Some("http://localhost:11434".into()),
                api_key: None,
                timeout_secs: 60,
            })?;
            println!("✓ Saved Ollama config (model = {model}).");
        }
        _ => {
            println!("Skipped. Configure later with `habitos connect ...`.");
        }
    }
    Ok(())
}

fn prompt_line(prompt: &str) -> Result<String> {
    use std::io::Write as _;
    print!("{prompt}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn prompt_yes_no(question: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    let answer = prompt_line(&format!("{question} {suffix} "))?;
    let trimmed = answer.trim().to_lowercase();
    if trimmed.is_empty() {
        Ok(default_yes)
    } else {
        Ok(trimmed.starts_with('y'))
    }
}

async fn run_heatmap(db: &Db, days: u32) -> Result<()> {
    use habitos_core::heatmap;
    use std::collections::HashMap;

    let clock = SystemClock;
    let today = clock.today_local();

    let habit_repo = HabitRepo::new(db.pool());
    let habits = habit_repo.list(true).await?;

    let mut per_day: HashMap<Date, u32> = HashMap::new();
    for h in &habits {
        let logs = habit_repo.logs(h.id).await?;
        for log in logs {
            if log.status == "done"
                && let Ok(d) = Date::parse(&log.on_date, ISO_DATE)
            {
                *per_day.entry(d).or_insert(0) += 1;
            }
        }
    }

    let h = heatmap::build(today, days, &per_day);

    // Render with shaded blocks. Each cell is one terminal char.
    let symbols = [' ', '░', '▒', '▓', '█'];
    let weekday_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    println!();
    println!("  {} — {} ({} days)", h.window_start, h.end, days);

    // Month header row: print month abbrev under the first column of each
    // month-start week.
    let mut month_row = String::from("       ");
    let mut last_month: u8 = 0;
    let mut cursor = h.aligned_start;
    for _ in 0..h.cells.len() {
        let m = cursor.month() as u8;
        if m != last_month {
            let abbr = month_short(m);
            month_row.push_str(&abbr);
            last_month = m;
        } else {
            month_row.push(' ');
        }
        cursor += time::Duration::days(7);
    }
    println!("{month_row}");

    for d in 0..7 {
        print!("  {} ", weekday_labels[d]);
        for week in &h.cells {
            let sym = symbols[week[d] as usize];
            print!("{sym}");
        }
        println!();
    }
    println!();
    println!(
        "  less {} {} {} {} {} more",
        symbols[0], symbols[1], symbols[2], symbols[3], symbols[4]
    );
    Ok(())
}

fn month_short(m: u8) -> String {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "   ",
    }
    .to_string()
}

async fn run_nudge(db: &Db, notify: bool) -> Result<()> {
    let clock = SystemClock;
    let today = clock.today_local();
    let habit_repo = HabitRepo::new(db.pool());
    let habits = habit_repo.list(false).await?;

    let today_str = today.to_string();
    let mut at_risk: Option<(String, u32)> = None;
    let mut total_pending = 0u32;
    for h in habits {
        let logs = habit_repo.logs(h.id).await?;
        let logged_today = logs.iter().any(|l| l.on_date == today_str);
        if logged_today {
            continue;
        }
        total_pending += 1;
        let stats = compute_stats(today, &logs);
        if stats.current_streak > 0 {
            match &at_risk {
                None => at_risk = Some((h.name.clone(), stats.current_streak)),
                Some((_, s)) if stats.current_streak > *s => {
                    at_risk = Some((h.name.clone(), stats.current_streak));
                }
                _ => {}
            }
        }
    }

    let message = match (at_risk, total_pending) {
        (Some((name, streak)), _) => {
            format!("{name} pending — your {streak}-day streak ends if you skip today.")
        }
        (None, 0) => "All habits logged. Nice work.".to_string(),
        (None, n) => format!("{n} habit(s) pending — log them before midnight."),
    };

    if notify {
        send_macos_notification("HabitOS", &message);
    } else {
        println!("{message}");
    }
    Ok(())
}

fn send_macos_notification(title: &str, body: &str) {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        applescript_escape(body),
        applescript_escape(title)
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status();
}

fn applescript_escape(s: &str) -> String {
    s.replace('\\', r"\\").replace('"', r#"\""#)
}

async fn run_log(db: &Db, text: &str) -> Result<()> {
    use habitos_core::capture;
    use std::fmt::Write as _;
    use time::OffsetDateTime;
    use time::UtcOffset;
    use time::format_description::well_known::Iso8601;

    let clock = SystemClock;
    let today = clock.today_local();

    let habit_repo = HabitRepo::new(db.pool());
    let habits = habit_repo.list(false).await?;
    let names: Vec<&str> = habits.iter().map(|h| h.name.as_str()).collect();

    let parsed = capture::parse(text, &names);
    let mut summary: Vec<String> = Vec::new();

    let mut milestone_line: Option<String> = None;
    if let Some(name) = &parsed.habit_match
        && let Some(habit) = habits.iter().find(|h| &h.name == name)
    {
        let outcome = habit_repo
            .log(habit.id, today, parsed.status, Some(text))
            .await?;
        let verb = match parsed.status {
            LogStatus::Done => "done",
            LogStatus::Skipped => "skipped",
        };
        match outcome {
            LogOutcome::Inserted => {
                summary.push(format!("{name} {verb}"));
                let _ = EventLog::new(db.pool())
                    .record("habit", Some(habit.id), verb, Some(&today.to_string()))
                    .await;
                if parsed.status == LogStatus::Done {
                    let logs = habit_repo.logs(habit.id).await?;
                    let stats = compute_stats(today, &logs);
                    if let Some(ms) =
                        habitos_core::milestones::streak_milestone(stats.current_streak)
                    {
                        milestone_line = Some(ms.celebration_line(name));
                    }
                }
            }
            LogOutcome::AlreadyLogged => summary.push(format!("{name} already logged")),
        }
    }

    if let Some(mins) = parsed.duration_minutes
        && mins > 0
    {
        let now = OffsetDateTime::now_utc();
        let start = now - time::Duration::minutes(mins);
        let project = parsed.habit_match.as_deref();
        let start_s = start.format(&Iso8601::DEFAULT)?;
        let end_s = now.format(&Iso8601::DEFAULT)?;
        let focus_repo = FocusRepo::new(db.pool());
        let session = focus_repo
            .log_completed(&start_s, &end_s, project, Some(text))
            .await?;
        summary.push(format!("{mins}m focus"));
        let _ = EventLog::new(db.pool())
            .record("focus", Some(session.id), "log", Some(&format!("{mins}m")))
            .await;
    }

    // Always append the raw text to today's journal with a local-time prefix.
    let journal_repo = JournalRepo::new(db.pool());
    let existing = journal_repo
        .get(today)
        .await?
        .map(|e| e.body)
        .unwrap_or_default();
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let now_local = OffsetDateTime::now_utc().to_offset(local_offset);
    let hh = now_local.hour();
    let mm = now_local.minute();
    let mut new_body = String::with_capacity(existing.len() + text.len() + 16);
    if !existing.is_empty() {
        new_body.push_str(&existing);
        if !existing.ends_with('\n') {
            new_body.push('\n');
        }
    }
    let _ = write!(new_body, "{hh:02}:{mm:02} — {text}");
    let entry = journal_repo.upsert(today, &new_body).await?;
    summary.push("journal +1".into());
    let _ = EventLog::new(db.pool())
        .record("journal", Some(entry.id), "capture", Some(&entry.on_date))
        .await;

    println!("✓ {}", summary.join(", "));
    if let Some(line) = milestone_line {
        println!("{line}");
    }
    Ok(())
}

async fn run_connect(config: &mut Config, sub: ConnectCmd) -> Result<()> {
    use habitos_core::config::AiConfig;
    match sub {
        ConnectCmd::Claude { api_key, model } => {
            config.save_ai(AiConfig {
                backend: Some("anthropic".into()),
                model: Some(model.clone()),
                endpoint: None,
                api_key: Some(api_key),
                timeout_secs: 30,
            })?;
            println!("Saved Claude config (model={model}).");
            verify_backend(config).await;
        }
        ConnectCmd::Ollama { model, no_pull } => {
            connect_ollama(config, &model, no_pull).await?;
        }
        ConnectCmd::Gemma { model } => {
            connect_ollama(config, &model, false).await?;
        }
        ConnectCmd::Qwen { model } => {
            connect_ollama(config, &model, false).await?;
        }
        ConnectCmd::Status => {
            let ai = config.ai();
            println!(
                "Backend:  {}",
                ai.backend.as_deref().unwrap_or("(not configured)")
            );
            println!("Model:    {}", ai.model.as_deref().unwrap_or("(none)"));
            println!(
                "Endpoint: {}",
                ai.endpoint.as_deref().unwrap_or("(default)")
            );
            println!(
                "API key:  {}",
                if ai.api_key.as_deref().map(str::is_empty).unwrap_or(true) {
                    "(unset)"
                } else {
                    "(set)"
                }
            );
            println!("Timeout:  {}s", ai.timeout_secs);
        }
        ConnectCmd::Off => {
            config.save_ai(AiConfig::default())?;
            println!("Disconnected. AI commands will use deterministic fallback.");
        }
    }
    Ok(())
}

async fn connect_ollama(config: &mut Config, model: &str, no_pull: bool) -> Result<()> {
    use habitos_core::config::AiConfig;

    if !command_exists("ollama") {
        eprintln!("`ollama` not found on PATH.");
        eprintln!();
        eprintln!("Install it first:");
        eprintln!("  brew install ollama        (macOS)");
        eprintln!("  https://ollama.com/download (other platforms)");
        eprintln!();
        eprintln!("Then start the server in another terminal: `ollama serve`");
        std::process::exit(1);
    }

    if !no_pull {
        println!("Pulling `{model}` (cached if already present)...");
        let status = std::process::Command::new("ollama")
            .arg("pull")
            .arg(model)
            .status()
            .with_context(|| "failed to spawn `ollama pull`")?;
        if !status.success() {
            anyhow::bail!("`ollama pull {model}` failed (exit {status})");
        }
    }

    config.save_ai(AiConfig {
        backend: Some("ollama".into()),
        model: Some(model.to_string()),
        endpoint: Some("http://localhost:11434".into()),
        api_key: None,
        timeout_secs: 60,
    })?;
    println!("Saved Ollama config (model={model}).");
    verify_backend(config).await;
    Ok(())
}

async fn verify_backend(config: &Config) {
    match build_ai_client(config) {
        None => {
            eprintln!("Config saved but no client could be built — check `habitos connect status`.")
        }
        Some(c) => match c.ping().await {
            Ok(()) => println!("✓ Backend reachable."),
            Err(e) => eprintln!("⚠ Backend not reachable yet: {e}"),
        },
    }
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run_export(db: &Db, sub: ExportCmd) -> Result<()> {
    let snap = export::load(db.pool()).await?;
    match sub {
        ExportCmd::Markdown => {
            print!("{}", export::to_markdown(&snap));
        }
        ExportCmd::Csv { dir } => {
            std::fs::create_dir_all(&dir)?;
            for (name, contents) in export::to_csv_files(&snap) {
                let path = dir.join(name);
                std::fs::write(&path, contents)
                    .with_context(|| format!("writing {}", path.display()))?;
                println!("Wrote {}", path.display());
            }
        }
    }
    Ok(())
}

async fn run_ask(db: &Db, config: &Config, query: &str) -> Result<()> {
    let client = build_ai_client(config).ok_or_else(|| {
        anyhow::anyhow!(
            "`ask` requires an AI backend. Configure [ai] in {}.",
            config.config_path().display()
        )
    })?;

    let model = config
        .ai()
        .model
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Backfill any journal entries not yet indexed.
    let indexed_count = backfill_embeddings(db, client.as_ref(), &model).await?;
    if indexed_count > 0 {
        eprintln!("Indexed {indexed_count} new entries into memory.");
    }

    // Embed the query
    let q_emb = client
        .embed(query)
        .await
        .map_err(|e| anyhow::anyhow!("failed to embed query: {e}"))?;

    let memory = MemoryRepo::new(db.pool());
    let all = memory.all_with_embeddings().await?;
    if all.is_empty() {
        println!("No memories indexed yet. Add journal entries with `habitos journal new`.");
        return Ok(());
    }

    let hits = top_k(&all, &q_emb, 8);

    // Render context block with explicit "Entry: <ref>" lines so the LLM can cite.
    use std::fmt::Write as _;
    let mut context = String::new();
    let _ = writeln!(context, "Question: {query}\n");
    let _ = writeln!(context, "Retrieved entries (most relevant first):");
    for hit in &hits {
        let _ = writeln!(
            context,
            "\n---\nEntry: {} (score {:.3})\n{}",
            hit.source_ref, hit.score, hit.content
        );
    }

    let loader = PromptLoader::new(config.data_dir());
    let prompt = loader
        .render("ask", &context)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    match client.complete(&prompt).await {
        Ok(answer) => println!("{answer}"),
        Err(e) => {
            eprintln!("AI backend unreachable: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Walk the data tables, embed any source rows that don't yet have an
/// ai_memories row, and insert. Returns how many new rows were indexed.
async fn backfill_embeddings(db: &Db, client: &dyn LlmClient, model: &str) -> Result<usize> {
    let memory = MemoryRepo::new(db.pool());
    let journal = JournalRepo::new(db.pool());
    let reviews = ReviewRepo::new(db.pool());

    let already_journal: std::collections::HashSet<String> = memory
        .existing_source_refs("journal")
        .await?
        .into_iter()
        .collect();
    let already_reflection: std::collections::HashSet<String> = memory
        .existing_source_refs("reflection")
        .await?
        .into_iter()
        .collect();

    let mut count = 0usize;

    // Journals
    let entries = journal.recent(10_000).await?;
    for e in entries {
        let source_ref = format!("journal_entries:{}", e.id);
        if already_journal.contains(&source_ref) {
            continue;
        }
        let content = format!("Date: {}\n\n{}", e.on_date, e.body);
        let emb = client
            .embed(&content)
            .await
            .map_err(|err| anyhow::anyhow!("embed failed for {source_ref}: {err}"))?;
        memory
            .upsert("journal", &source_ref, &content, &emb, model)
            .await?;
        count += 1;
    }

    // Reflections (daily reviews — we store the four answers concatenated)
    let dailies = reviews
        .dailies_since(time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap())
        .await?;
    for d in dailies {
        let source_ref = format!("daily_reviews:{}", d.id);
        if already_reflection.contains(&source_ref) {
            continue;
        }
        let body = format!(
            "Went well: {}\nDidn't go well: {}\nLearned: {}\nTomorrow's priority: {}",
            d.went_well.as_deref().unwrap_or("—"),
            d.didnt_go_well.as_deref().unwrap_or("—"),
            d.learned.as_deref().unwrap_or("—"),
            d.tomorrow_priority.as_deref().unwrap_or("—"),
        );
        let content = format!("Date: {}\n\n{}", d.on_date, body);
        let emb = client
            .embed(&content)
            .await
            .map_err(|err| anyhow::anyhow!("embed failed for {source_ref}: {err}"))?;
        memory
            .upsert("reflection", &source_ref, &content, &emb, model)
            .await?;
        count += 1;
    }

    Ok(count)
}

fn build_ai_client(config: &Config) -> Option<Box<dyn LlmClient>> {
    let ai = config.ai();
    let backend = ai.backend.as_deref()?;
    let model = ai.model.as_deref()?;
    // Anthropic has a stable default endpoint; everything else must be explicit.
    let endpoint = ai.endpoint.clone().or_else(|| match backend {
        "anthropic" => Some("https://api.anthropic.com".to_string()),
        _ => None,
    })?;
    let cfg = AiBackendConfig {
        kind: backend.to_string(),
        model: model.to_string(),
        endpoint,
        timeout_secs: ai.timeout_secs,
        api_key: ai.api_key.clone(),
    };
    build_client(&cfg).ok()
}

async fn run_ai_check(config: &Config) -> Result<()> {
    match build_ai_client(config) {
        None => {
            println!(
                "No AI backend configured. Edit {} and set [ai].backend, model, endpoint.",
                config.config_path().display()
            );
        }
        Some(c) => match c.ping().await {
            Ok(()) => println!("AI backend reachable."),
            Err(e) => {
                eprintln!("AI backend unreachable: {e}");
                std::process::exit(1);
            }
        },
    }
    Ok(())
}

async fn run_plan(db: &Db, config: &Config) -> Result<()> {
    let context = gather_planning_context(db).await?;
    match build_ai_client(config) {
        None => {
            eprintln!(
                "Note: no AI backend configured. Showing deterministic context. \
                 Configure [ai] in {} for an AI-generated plan.",
                config.config_path().display()
            );
            println!("{context}");
        }
        Some(c) => {
            let loader = PromptLoader::new(config.data_dir());
            let prompt = loader
                .render("plan", &context)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            match c.complete(&prompt).await {
                Ok(out) => println!("{out}"),
                Err(e) => {
                    eprintln!(
                        "AI backend unreachable ({e}); falling back to deterministic context:"
                    );
                    println!("{context}");
                }
            }
        }
    }
    Ok(())
}

async fn run_coach(db: &Db, config: &Config) -> Result<()> {
    let context = gather_coaching_context(db).await?;
    match build_ai_client(config) {
        None => {
            eprintln!(
                "Note: no AI backend configured. Showing deterministic context. \
                 Configure [ai] in {} for coaching feedback.",
                config.config_path().display()
            );
            println!("{context}");
        }
        Some(c) => {
            let loader = PromptLoader::new(config.data_dir());
            let prompt = loader
                .render("coach", &context)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            match c.complete(&prompt).await {
                Ok(out) => println!("{out}"),
                Err(e) => {
                    eprintln!(
                        "AI backend unreachable ({e}); falling back to deterministic context:"
                    );
                    println!("{context}");
                }
            }
        }
    }
    Ok(())
}

async fn gather_planning_context(db: &Db) -> Result<String> {
    use std::fmt::Write as _;
    let clock = SystemClock;
    let today = clock.today_local();
    let habit_repo = HabitRepo::new(db.pool());
    let focus_repo = FocusRepo::new(db.pool());
    let goal_repo = GoalRepo::new(db.pool());
    let journal_repo = JournalRepo::new(db.pool());

    let mut out = String::new();
    let _ = writeln!(out, "Today is {today}.");
    let _ = writeln!(out);

    // Open goals
    let goals = goal_repo.list(false).await?;
    let _ = writeln!(out, "## Open goals");
    if goals.is_empty() {
        let _ = writeln!(out, "(none)");
    } else {
        for g in &goals {
            let p = goal_repo.progress(g.id).await?;
            if p.total == 0 {
                let _ = writeln!(out, "- {} (no milestones)", g.name);
            } else {
                let _ = writeln!(
                    out,
                    "- {} ({}/{} milestones, {}%)",
                    g.name,
                    p.completed,
                    p.total,
                    p.percent()
                );
            }
        }
    }
    let _ = writeln!(out);

    // Habit status for today
    let habits = habit_repo.list(false).await?;
    let _ = writeln!(out, "## Today's habits");
    if habits.is_empty() {
        let _ = writeln!(out, "(none)");
    } else {
        for h in &habits {
            let logs = habit_repo.logs(h.id).await?;
            let s = compute_stats(today, &logs);
            let today_status = logs
                .iter()
                .find(|l| l.on_date == today.to_string())
                .map(|l| l.status.as_str())
                .unwrap_or("pending");
            let _ = writeln!(
                out,
                "- {} — {} today (streak {}, 30d {}%)",
                h.name, today_status, s.current_streak, s.completion_rate_30d
            );
        }
    }
    let _ = writeln!(out);

    // Recent focus
    let week_start = today - time::Duration::days(7);
    let recent_focus = focus_sessions_since(&focus_repo, week_start).await?;
    let total_min: i64 = recent_focus
        .iter()
        .filter_map(|s| s.duration_minutes())
        .sum();
    let _ = writeln!(
        out,
        "## Focus (last 7 days)\n{} sessions, {}h {}m total",
        recent_focus.len(),
        total_min / 60,
        total_min % 60
    );
    let _ = writeln!(out);

    // Recent journals (titles + snippets)
    let recent_journals = journal_repo.recent(5).await?;
    let _ = writeln!(out, "## Recent journal");
    if recent_journals.is_empty() {
        let _ = writeln!(out, "(no entries)");
    } else {
        for j in recent_journals {
            let snippet: String = j.body.chars().take(160).collect();
            let _ = writeln!(out, "- {} — {}", j.on_date, snippet.replace('\n', " "));
        }
    }

    Ok(out)
}

async fn gather_coaching_context(db: &Db) -> Result<String> {
    use std::fmt::Write as _;
    let clock = SystemClock;
    let today = clock.today_local();
    let habit_repo = HabitRepo::new(db.pool());
    let focus_repo = FocusRepo::new(db.pool());
    let goal_repo = GoalRepo::new(db.pool());
    let journal_repo = JournalRepo::new(db.pool());

    let mut out = String::new();
    let _ = writeln!(out, "Today is {today}. Context covers the last 30 days.");
    let _ = writeln!(out);

    // Goals + progress
    let goals = goal_repo.list(true).await?;
    let _ = writeln!(out, "## All goals");
    for g in &goals {
        let p = goal_repo.progress(g.id).await?;
        let _ = writeln!(
            out,
            "- {} [{}] ({}/{} milestones)",
            g.name, g.status, p.completed, p.total
        );
    }
    let _ = writeln!(out);

    // Habits with 30d stats
    let habits = habit_repo.list(false).await?;
    let _ = writeln!(out, "## Habits (last 30 days)");
    for h in &habits {
        let logs = habit_repo.logs(h.id).await?;
        let s = compute_stats(today, &logs);
        let _ = writeln!(
            out,
            "- {}: current streak {}, best {}, 30d completion {}%, missed {}",
            h.name, s.current_streak, s.longest_streak, s.completion_rate_30d, s.missed_30d
        );
    }
    let _ = writeln!(out);

    // Focus 30d
    let since = today - time::Duration::days(30);
    let focus_30 = focus_sessions_since(&focus_repo, since).await?;
    let total: i64 = focus_30.iter().filter_map(|s| s.duration_minutes()).sum();
    let _ = writeln!(
        out,
        "## Focus (last 30 days)\n{} sessions, {}h {}m total",
        focus_30.len(),
        total / 60,
        total % 60
    );
    let _ = writeln!(out);

    // Journals
    let recent_journals = journals_since(&journal_repo, since).await?;
    let _ = writeln!(out, "## Journal (last 30 days)");
    if recent_journals.is_empty() {
        let _ = writeln!(out, "(no entries)");
    } else {
        for j in recent_journals.iter().take(15) {
            let snippet: String = j.body.chars().take(200).collect();
            let _ = writeln!(out, "- {} — {}", j.on_date, snippet.replace('\n', " "));
        }
    }

    Ok(out)
}

async fn open_and_migrate(config: &Config) -> Result<Db> {
    let db = Db::open(config).await?;
    db.migrate().await?;
    Ok(db)
}

fn parse_date(s: &str) -> Result<Date> {
    Date::parse(s, ISO_DATE).with_context(|| format!("expected YYYY-MM-DD, got `{s}`"))
}

async fn run_habit(db: &Db, sub: HabitCmd) -> Result<()> {
    let repo = HabitRepo::new(db.pool());
    let clock = SystemClock;
    match sub {
        HabitCmd::Add { name } => {
            let h = repo.add(&name).await?;
            println!("Added habit `{}` (id={})", h.name, h.id);
            let _ = EventLog::new(db.pool())
                .record("habit", Some(h.id), "add", Some(&h.name))
                .await;
        }
        HabitCmd::Done { name, at, note } => {
            log_habit(db, &repo, &clock, &name, at, note, LogStatus::Done).await?
        }
        HabitCmd::Skip { name, at, note } => {
            log_habit(db, &repo, &clock, &name, at, note, LogStatus::Skipped).await?
        }
        HabitCmd::List { archived } => {
            let habits = repo.list(archived).await?;
            if habits.is_empty() {
                println!("No habits yet. Create one with `habitos habit add <name>`.");
            } else {
                for h in habits {
                    let tag = if h.archived != 0 { " [archived]" } else { "" };
                    println!("  {}{}", h.name, tag);
                }
            }
        }
        HabitCmd::Rm { name } => {
            let removed = repo.remove(&name).await?;
            if removed {
                println!("Removed `{name}`.");
            } else {
                eprintln!("No habit named `{name}`.");
                std::process::exit(1);
            }
        }
        HabitCmd::Stats => {
            let habits = repo.list(false).await?;
            if habits.is_empty() {
                println!("No habits yet. Create one with `habitos habit add <name>`.");
                return Ok(());
            }
            let today = clock.today_local();
            for h in habits {
                let logs = repo.logs(h.id).await?;
                let s = compute_stats(today, &logs);
                let freeze_tag = if s.freezes_used > 0 {
                    format!(" ❄{}", s.freezes_used)
                } else {
                    String::new()
                };
                println!(
                    "  {:<24} streak {:>3}{}  best {:>3}  30d {:>3}%  missed {:>2}",
                    h.name,
                    s.current_streak,
                    freeze_tag,
                    s.longest_streak,
                    s.completion_rate_30d,
                    s.missed_30d
                );
            }
        }
    }
    Ok(())
}

async fn log_habit(
    db: &Db,
    repo: &HabitRepo<'_>,
    clock: &dyn Clock,
    name: &str,
    at: Option<String>,
    note: Option<String>,
    status: LogStatus,
) -> Result<()> {
    let habit = repo.find_by_name(name).await?.with_context(|| {
        format!("no habit named `{name}` — create it with `habitos habit add {name}`")
    })?;
    let on = match at {
        Some(s) => parse_date(&s)?,
        None => clock.today_local(),
    };
    let outcome = repo.log(habit.id, on, status, note.as_deref()).await?;
    let verb = match status {
        LogStatus::Done => "done",
        LogStatus::Skipped => "skipped",
    };
    match outcome {
        LogOutcome::Inserted => {
            println!("Logged `{name}` {verb} for {on}.");
            let _ = EventLog::new(db.pool())
                .record("habit", Some(habit.id), verb, Some(&on.to_string()))
                .await;
        }
        LogOutcome::AlreadyLogged => println!("`{name}` already logged for {on} — no change."),
    }
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("off"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

async fn run_reflect(db: &Db) -> Result<()> {
    let repo = ReviewRepo::new(db.pool());
    let clock = SystemClock;
    let today = clock.today_local();

    println!("# Reflection for {}\n", today);
    println!("Press Enter on a blank line to skip a question.\n");

    let went_well = prompt_optional("What went well?")?;
    let didnt_go_well = prompt_optional("What didn't?")?;
    let learned = prompt_optional("What did you learn?")?;
    let tomorrow_priority = prompt_optional("What is tomorrow's priority?")?;

    let answers = DailyAnswers {
        went_well,
        didnt_go_well,
        learned,
        tomorrow_priority,
    };
    let saved = repo.save_daily(today, &answers).await?;
    println!("\nSaved reflection for {today}.");
    let _ = EventLog::new(db.pool())
        .record(
            "reflection",
            Some(saved.id),
            "write",
            Some(&today.to_string()),
        )
        .await;
    Ok(())
}

fn prompt_optional(question: &str) -> Result<Option<String>> {
    print!("{question} ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

async fn run_review(db: &Db, sub: ReviewCmd) -> Result<()> {
    let clock = SystemClock;
    let today = clock.today_local();
    let habit_repo = HabitRepo::new(db.pool());
    let focus_repo = FocusRepo::new(db.pool());
    let journal_repo = JournalRepo::new(db.pool());
    let goal_repo = GoalRepo::new(db.pool());
    let review_repo = ReviewRepo::new(db.pool());

    let habits_with_logs = load_habits_with_logs(&habit_repo).await?;

    match sub {
        ReviewCmd::Day => {
            let focus = focus_sessions_since(&focus_repo, today).await?;
            let journal = journal_repo.get(today).await?;
            let reflection = review_repo.get_daily(today).await?;
            let snap = reports::DailySnapshot {
                today,
                habits: &habits_with_logs,
                focus_sessions: &focus,
                journal: journal.as_ref(),
                reflection: reflection.as_ref(),
            };
            print!("{}", reports::daily(&snap));
        }
        ReviewCmd::Week => {
            let week_start = week_starting(today);
            let focus = focus_sessions_since(&focus_repo, week_start).await?;
            let journals = journals_since(&journal_repo, week_start).await?;
            let goals = load_goals_with_progress(&goal_repo).await?;
            let dailies = review_repo.dailies_since(week_start).await?;
            let snap = reports::WeeklySnapshot {
                week_start,
                today,
                habits: &habits_with_logs,
                focus_sessions: &focus,
                journals: &journals,
                goals: &goals,
                dailies: &dailies,
            };
            print!("{}", reports::weekly(&snap));
        }
        ReviewCmd::Month => {
            let month_start = month_first_day(today);
            let focus = focus_sessions_since(&focus_repo, month_start).await?;
            let journals = journals_since(&journal_repo, month_start).await?;
            let goals = load_goals_with_progress(&goal_repo).await?;
            let dailies = review_repo.dailies_since(month_start).await?;
            let snap = reports::MonthlySnapshot {
                month_start,
                today,
                habits: &habits_with_logs,
                focus_sessions: &focus,
                journals: &journals,
                goals: &goals,
                dailies: &dailies,
            };
            print!("{}", reports::monthly(&snap));
        }
    }
    Ok(())
}

async fn run_insights(db: &Db) -> Result<()> {
    let clock = SystemClock;
    let today = clock.today_local();
    let habit_repo = HabitRepo::new(db.pool());
    let focus_repo = FocusRepo::new(db.pool());
    let goal_repo = GoalRepo::new(db.pool());

    let habits_with_logs = load_habits_with_logs(&habit_repo).await?;
    let lookback = today - time::Duration::days(90);
    let focus = focus_sessions_since(&focus_repo, lookback).await?;
    let goals = goal_repo.list(true).await?;
    let mut goal_progress: Vec<(i64, u32)> = Vec::with_capacity(goals.len());
    for g in &goals {
        let p = goal_repo.progress(g.id).await?;
        goal_progress.push((g.id, p.completed));
    }
    let raw = reports::insights(today, &habits_with_logs, &focus, &goals, &goal_progress);
    print!("{}", reports::render_insights(&raw));
    Ok(())
}

async fn load_habits_with_logs(repo: &HabitRepo<'_>) -> Result<Vec<(Habit, Vec<HabitLog>)>> {
    let habits = repo.list(false).await?;
    let mut out = Vec::with_capacity(habits.len());
    for h in habits {
        let logs = repo.logs(h.id).await?;
        out.push((h, logs));
    }
    Ok(out)
}

async fn focus_sessions_since(
    repo: &FocusRepo<'_>,
    since: Date,
) -> Result<Vec<habitos_core::focus::FocusSession>> {
    let s = format!(
        "{}T00:00:00.000Z",
        since
            .format(format_description!("[year]-[month]-[day]"))
            .unwrap()
    );
    Ok(repo.since(&s).await?)
}

async fn journals_since(
    repo: &JournalRepo<'_>,
    since: Date,
) -> Result<Vec<habitos_core::journal::JournalEntry>> {
    // Cheap path: fetch recent N and filter. For V1 this is fine.
    let entries = repo.recent(365).await?;
    let key = since.to_string();
    Ok(entries.into_iter().filter(|e| e.on_date >= key).collect())
}

async fn load_goals_with_progress(repo: &GoalRepo<'_>) -> Result<Vec<(Goal, u32, u32)>> {
    let goals = repo.list(false).await?;
    let mut out = Vec::with_capacity(goals.len());
    for g in goals {
        let p = repo.progress(g.id).await?;
        out.push((g, p.completed, p.total));
    }
    Ok(out)
}

async fn run_goal(db: &Db, sub: GoalCmd) -> Result<()> {
    let repo = GoalRepo::new(db.pool());
    match sub {
        GoalCmd::Add { name } => {
            let g = repo.add(&name).await?;
            println!("Added goal `{}` (id={})", g.name, g.id);
        }
        GoalCmd::List { all } => {
            let goals = repo.list(all).await?;
            if goals.is_empty() {
                println!("No goals yet. Create one with `habitos goal add <name>`.");
            } else {
                for g in goals {
                    let tag = if g.status != "open" {
                        format!(" [{}]", g.status)
                    } else {
                        String::new()
                    };
                    println!("  {}{}", g.name, tag);
                }
            }
        }
        GoalCmd::Progress => {
            let goals = repo.list(false).await?;
            if goals.is_empty() {
                println!("No open goals.");
                return Ok(());
            }
            for g in goals {
                let p = repo.progress(g.id).await?;
                if p.total == 0 {
                    println!("  {:<32} (no milestones)", g.name);
                } else {
                    println!(
                        "  {:<32} {}/{} milestones, {}%",
                        g.name,
                        p.completed,
                        p.total,
                        p.percent()
                    );
                }
            }
        }
        GoalCmd::Complete { name } => {
            if repo.complete(&name).await? {
                println!("Completed `{name}`.");
            } else {
                eprintln!("No open goal named `{name}`.");
                std::process::exit(1);
            }
        }
        GoalCmd::Milestone(MilestoneCmd::Add { goal, name }) => {
            let g = repo
                .find_by_name(&goal)
                .await?
                .with_context(|| format!("no goal named `{goal}`"))?;
            let m = repo.add_milestone(g.id, &name).await?;
            println!("Added milestone `{}` to `{}` (id={})", m.name, g.name, m.id);
        }
        GoalCmd::Milestone(MilestoneCmd::Done { goal, name }) => {
            let g = repo
                .find_by_name(&goal)
                .await?
                .with_context(|| format!("no goal named `{goal}`"))?;
            if repo.complete_milestone(g.id, &name).await? {
                println!("Marked milestone `{name}` done in `{}`.", g.name);
            } else {
                eprintln!("No open milestone `{name}` in `{}`.", g.name);
                std::process::exit(1);
            }
        }
        GoalCmd::Milestone(MilestoneCmd::List { goal }) => {
            let g = repo
                .find_by_name(&goal)
                .await?
                .with_context(|| format!("no goal named `{goal}`"))?;
            let ms = repo.milestones(g.id).await?;
            if ms.is_empty() {
                println!("No milestones in `{}`.", g.name);
            } else {
                for m in ms {
                    let mark = if m.completed_at.is_some() {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    println!("  {} {}", mark, m.name);
                }
            }
        }
    }
    Ok(())
}

async fn run_focus(db: &Db, sub: FocusCmd) -> Result<()> {
    let repo = FocusRepo::new(db.pool());
    match sub {
        FocusCmd::Start { project, note } => {
            let s = repo.start(project.as_deref(), note.as_deref()).await?;
            let tag = s
                .project
                .as_deref()
                .map(|p| format!(" on `{p}`"))
                .unwrap_or_default();
            println!("Started focus session{tag} at {}.", s.start_at);
            let _ = EventLog::new(db.pool())
                .record("focus", Some(s.id), "start", s.project.as_deref())
                .await;
        }
        FocusCmd::Stop => {
            let s = repo.stop().await?;
            let mins = s.duration_minutes().unwrap_or(0);
            println!("Stopped. Duration: {mins} min.");
            let _ = EventLog::new(db.pool())
                .record("focus", Some(s.id), "stop", Some(&format!("{mins}m")))
                .await;
        }
        FocusCmd::Status => match repo.active().await? {
            None => println!("No active focus session."),
            Some(s) => {
                let tag = s.project.as_deref().unwrap_or("(no project)");
                println!("Active: {tag}, started {}", s.start_at);
            }
        },
    }
    Ok(())
}

async fn run_journal(db: &Db, sub: JournalCmd) -> Result<()> {
    let repo = JournalRepo::new(db.pool());
    let clock = SystemClock;
    match sub {
        JournalCmd::New => {
            let today = clock.today_local();
            let existing = repo.get(today).await?.map(|e| e.body).unwrap_or_default();
            let edited = open_in_editor(&existing, &today.to_string())?;
            let trimmed = edited.trim();
            if trimmed.is_empty() || trimmed == existing.trim() {
                println!("No changes. Journal entry not saved.");
                return Ok(());
            }
            let entry = repo.upsert(today, &edited).await?;
            println!("Saved journal entry for {}.", entry.on_date);
            let _ = EventLog::new(db.pool())
                .record("journal", Some(entry.id), "write", Some(&entry.on_date))
                .await;
        }
        JournalCmd::Today => {
            let today = clock.today_local();
            match repo.get(today).await? {
                None => println!("No journal entry for {today} yet."),
                Some(e) => {
                    println!("# {}\n", e.on_date);
                    println!("{}", e.body);
                }
            }
        }
        JournalCmd::Search { query } => {
            let q = query.join(" ");
            let hits = repo.search(&q).await?;
            if hits.is_empty() {
                println!("No matches for `{q}`.");
            } else {
                for e in hits {
                    let snippet: String = e.body.chars().take(120).collect();
                    println!("{}  {}", e.on_date, snippet.replace('\n', " "));
                }
            }
        }
    }
    Ok(())
}

/// Open the user's $VISUAL / $EDITOR (falling back to `vi`) on a temp file,
/// pre-populated with `initial`. Returns the file contents after the editor exits.
fn open_in_editor(initial: &str, suffix_hint: &str) -> Result<String> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let suffix = format!("-{suffix_hint}.md");
    let mut tmp = tempfile::Builder::new()
        .prefix("habitos-journal-")
        .suffix(&suffix)
        .tempfile()?;
    tmp.write_all(initial.as_bytes())?;
    tmp.flush()?;
    let path = tmp.path().to_path_buf();
    let cmd = format!("{editor} {}", shell_quote(&path));
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
        .with_context(|| format!("failed to spawn editor: {editor}"))?;
    if !status.success() {
        return Err(anyhow::anyhow!("editor exited with status {status}"));
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(contents)
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"))
}
