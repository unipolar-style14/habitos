use anyhow::Result;
use habitos_core::Db;
use habitos_core::clock::{Clock, SystemClock};
use habitos_core::focus::{FocusRepo, FocusSession, total_minutes};
use habitos_core::goals::{Goal, GoalRepo};
use habitos_core::habits::{Habit, HabitLog, HabitRepo, HabitStats, compute_stats};
use habitos_core::journal::{JournalEntry, JournalRepo};
use habitos_core::reviews::ReviewRepo;
use std::time::{Duration, Instant};
use time::{Date, OffsetDateTime};

pub struct App {
    pub today: Date,
    pub now: OffsetDateTime,
    pub habits: Vec<(Habit, Vec<HabitLog>, HabitStats)>,
    pub goals: Vec<(Goal, u32, u32)>,
    pub focus: Option<FocusSession>,
    pub journal_today: Option<JournalEntry>,
    pub today_focus_minutes: i64,
    pub reflected_today: bool,
    pub score: u32,

    pub cursor: usize,
    pub should_quit: bool,
    pub show_help: bool,

    flash_text: Option<String>,
    flash_until: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        let clock = SystemClock;
        Self {
            today: clock.today_local(),
            now: clock.now_utc(),
            habits: Vec::new(),
            goals: Vec::new(),
            focus: None,
            journal_today: None,
            today_focus_minutes: 0,
            reflected_today: false,
            score: 0,
            cursor: 0,
            should_quit: false,
            show_help: false,
            flash_text: None,
            flash_until: None,
        }
    }

    pub async fn refresh(&mut self, db: &Db) -> Result<()> {
        let clock = SystemClock;
        self.today = clock.today_local();
        self.now = clock.now_utc();

        let habit_repo = HabitRepo::new(db.pool());
        let habits = habit_repo.list(false).await?;
        let mut with_stats = Vec::with_capacity(habits.len());
        for h in habits {
            let logs = habit_repo.logs(h.id).await?;
            let stats = compute_stats(self.today, &logs);
            with_stats.push((h, logs, stats));
        }
        self.habits = with_stats;
        if self.cursor >= self.habits.len() && !self.habits.is_empty() {
            self.cursor = self.habits.len() - 1;
        }

        let goal_repo = GoalRepo::new(db.pool());
        let goals = goal_repo.list(false).await?;
        let mut with_progress = Vec::with_capacity(goals.len());
        for g in goals {
            let p = goal_repo.progress(g.id).await?;
            with_progress.push((g, p.completed, p.total));
        }
        self.goals = with_progress;

        let focus_repo = FocusRepo::new(db.pool());
        self.focus = focus_repo.active().await?;

        let journal_repo = JournalRepo::new(db.pool());
        self.journal_today = journal_repo.get(self.today).await?;

        // Today's focus minutes (closed sessions starting today UTC; good
        // enough for V1).
        let start_iso = format!("{}T00:00:00.000Z", self.today);
        let sessions = focus_repo.since(&start_iso).await?;
        self.today_focus_minutes = total_minutes(&sessions);

        // Did the user reflect today?
        let review_repo = ReviewRepo::new(db.pool());
        self.reflected_today = review_repo.get_daily(self.today).await?.is_some();

        self.score = self.compute_score();
        Ok(())
    }

    fn compute_score(&self) -> u32 {
        // 50% habits (done today / total non-archived habits)
        // 30% focus (capped at 4 hours = 240 min)
        // 10% journal
        // 10% reflection
        let habit_share: f32 = if self.habits.is_empty() {
            0.0
        } else {
            let done_today = (0..self.habits.len())
                .filter(|i| self.habit_today_status(*i) == "done")
                .count();
            (done_today as f32 / self.habits.len() as f32) * 50.0
        };
        let focus_share = ((self.today_focus_minutes as f32) / 240.0).min(1.0) * 30.0;
        let journal_share = if self.journal_today.is_some() {
            10.0
        } else {
            0.0
        };
        let reflection_share = if self.reflected_today { 10.0 } else { 0.0 };
        (habit_share + focus_share + journal_share + reflection_share).round() as u32
    }

    pub fn tick(&mut self) {
        self.now = OffsetDateTime::now_utc();
        if let Some(until) = self.flash_until
            && Instant::now() >= until
        {
            self.flash_text = None;
            self.flash_until = None;
        }
    }

    pub fn flash(&mut self, msg: impl Into<String>) {
        self.flash_text = Some(msg.into());
        self.flash_until = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn flash_message(&self) -> Option<&str> {
        self.flash_text.as_deref()
    }

    pub fn cursor_down(&mut self) {
        if !self.habits.is_empty() {
            self.cursor = (self.cursor + 1) % self.habits.len();
        }
    }

    pub fn cursor_up(&mut self) {
        if !self.habits.is_empty() {
            if self.cursor == 0 {
                self.cursor = self.habits.len() - 1;
            } else {
                self.cursor -= 1;
            }
        }
    }

    pub fn cursor_habit_idx(&self) -> Option<usize> {
        if self.habits.is_empty() {
            None
        } else {
            Some(self.cursor.min(self.habits.len() - 1))
        }
    }

    pub fn habit_today_status(&self, idx: usize) -> &'static str {
        let today_key = self.today.to_string();
        match self.habits.get(idx) {
            Some((_, logs, _)) => match logs.iter().find(|l| l.on_date == today_key) {
                Some(l) if l.status == "done" => "done",
                Some(l) if l.status == "skipped" => "skip",
                _ => "pending",
            },
            None => "—",
        }
    }

    pub fn focus_duration_minutes(&self) -> Option<i64> {
        let s = self.focus.as_ref()?;
        let start = OffsetDateTime::parse(
            &s.start_at,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .ok()?;
        Some((self.now - start).whole_minutes())
    }
}
