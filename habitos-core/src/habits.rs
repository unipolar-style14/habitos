use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use time::Date;
use time::macros::format_description;

const ISO_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");

/// Status of a single day's log entry for a habit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStatus {
    Done,
    Skipped,
}

impl LogStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Habit {
    pub id: i64,
    pub name: String,
    pub archived: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct HabitLog {
    pub id: i64,
    pub habit_id: i64,
    pub on_date: String,
    pub status: String,
    pub note: Option<String>,
}

/// Outcome of `log()`: was this a new write, or did one already exist for the day?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutcome {
    Inserted,
    AlreadyLogged,
}

pub struct HabitRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> HabitRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn add(&self, name: &str) -> Result<Habit, CoreError> {
        let row = sqlx::query_as::<_, Habit>(
            "INSERT INTO habits (name) VALUES (?)
             RETURNING id, name, archived, created_at, updated_at",
        )
        .bind(name)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Habit>, CoreError> {
        let row = sqlx::query_as::<_, Habit>(
            "SELECT id, name, archived, created_at, updated_at FROM habits WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list(&self, include_archived: bool) -> Result<Vec<Habit>, CoreError> {
        let sql = if include_archived {
            "SELECT id, name, archived, created_at, updated_at FROM habits ORDER BY name"
        } else {
            "SELECT id, name, archived, created_at, updated_at FROM habits WHERE archived = 0 ORDER BY name"
        };
        let rows = sqlx::query_as::<_, Habit>(sql).fetch_all(self.pool).await?;
        Ok(rows)
    }

    pub async fn remove(&self, name: &str) -> Result<bool, CoreError> {
        let res = sqlx::query("DELETE FROM habits WHERE name = ?")
            .bind(name)
            .execute(self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Idempotent. Same (habit, day) inserted twice = `AlreadyLogged`, no duplicate row.
    pub async fn log(
        &self,
        habit_id: i64,
        on: Date,
        status: LogStatus,
        note: Option<&str>,
    ) -> Result<LogOutcome, CoreError> {
        let date_str = on
            .format(ISO_DATE)
            .expect("format YYYY-MM-DD is infallible");
        let res = sqlx::query(
            "INSERT INTO habit_logs (habit_id, on_date, status, note) VALUES (?, ?, ?, ?)
             ON CONFLICT(habit_id, on_date) DO NOTHING",
        )
        .bind(habit_id)
        .bind(&date_str)
        .bind(status.as_str())
        .bind(note)
        .execute(self.pool)
        .await?;
        Ok(if res.rows_affected() == 0 {
            LogOutcome::AlreadyLogged
        } else {
            LogOutcome::Inserted
        })
    }

    /// All logs for a habit, newest first.
    pub async fn logs(&self, habit_id: i64) -> Result<Vec<HabitLog>, CoreError> {
        let rows = sqlx::query_as::<_, HabitLog>(
            "SELECT id, habit_id, on_date, status, note FROM habit_logs
             WHERE habit_id = ?
             ORDER BY on_date DESC",
        )
        .bind(habit_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }
}

/// Per-habit stats summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitStats {
    pub current_streak: u32,
    pub longest_streak: u32,
    /// Done days / 30 in the last 30 calendar days, as a percentage 0..=100.
    pub completion_rate_30d: u32,
    /// Days in the last 30 with no log at all (excluding today if today has no log).
    pub missed_30d: u32,
    /// Weekly freezes consumed inside `current_streak`. 1 freeze per ISO week.
    pub freezes_used: u32,
}

/// Streak/stats computation. Pure function over (today, logs) for easy testing.
///
/// Rules:
/// * `done` extends the current streak.
/// * `skipped` is neutral — preserves but does not extend the streak.
/// * Missing days are forgiven up to **one per ISO week** ("streak freeze").
/// * Once a week's freeze is consumed, a second missing day in that week
///   breaks the streak.
/// * `today` having no log is allowed (the day isn't over) — we walk back
///   from yesterday and don't charge a freeze for today.
pub fn compute_stats(today: Date, logs: &[HabitLog]) -> HabitStats {
    let mut by_date: std::collections::BTreeMap<Date, LogStatus> =
        std::collections::BTreeMap::new();
    for log in logs {
        if let Ok(d) = Date::parse(&log.on_date, ISO_DATE) {
            let status = match log.status.as_str() {
                "done" => LogStatus::Done,
                "skipped" => LogStatus::Skipped,
                _ => continue,
            };
            by_date.insert(d, status);
        }
    }

    let (current_streak, freezes_used) = current_streak(today, &by_date);
    let longest_streak = longest_streak(&by_date);
    let (done_30, missed_30) = window_30d(today, &by_date);
    let completion_rate_30d = (done_30 * 100) / 30;

    HabitStats {
        current_streak,
        longest_streak,
        completion_rate_30d,
        missed_30d: missed_30,
        freezes_used,
    }
}

fn current_streak(
    today: Date,
    by_date: &std::collections::BTreeMap<Date, LogStatus>,
) -> (u32, u32) {
    let mut cursor = if by_date.contains_key(&today) {
        today
    } else {
        match today.previous_day() {
            Some(d) => d,
            None => return (0, 0),
        }
    };
    let mut count: u32 = 0;
    let mut freezes_used: u32 = 0;
    let mut freezes_per_week: std::collections::BTreeMap<(i32, u8), u32> =
        std::collections::BTreeMap::new();
    loop {
        let iso = cursor.to_iso_week_date();
        let week_key = (iso.0, iso.1);
        let remaining = freezes_per_week.entry(week_key).or_insert(1);
        match by_date.get(&cursor) {
            Some(LogStatus::Done) => count += 1,
            Some(LogStatus::Skipped) => {} // neutral
            None => {
                if *remaining > 0 {
                    *remaining -= 1;
                    freezes_used += 1;
                } else {
                    break;
                }
            }
        }
        match cursor.previous_day() {
            Some(prev) => cursor = prev,
            None => break,
        }
    }
    (count, freezes_used)
}

fn longest_streak(by_date: &std::collections::BTreeMap<Date, LogStatus>) -> u32 {
    let mut best: u32 = 0;
    let mut run: u32 = 0;
    let mut prev_date: Option<Date> = None;
    // BTreeMap iterates in date order.
    for (&date, &status) in by_date {
        let contiguous = matches!(prev_date, Some(p) if p.next_day() == Some(date));
        if !contiguous {
            // Reset on first entry or gap.
            run = 0;
        }
        match status {
            LogStatus::Done => run += 1,
            LogStatus::Skipped => {} // neutral, doesn't reset
        }
        best = best.max(run);
        prev_date = Some(date);
    }
    best
}

fn window_30d(today: Date, by_date: &std::collections::BTreeMap<Date, LogStatus>) -> (u32, u32) {
    let mut done = 0u32;
    let mut missed = 0u32;
    let mut cursor = today;
    for i in 0..30 {
        match by_date.get(&cursor) {
            Some(LogStatus::Done) => done += 1,
            Some(LogStatus::Skipped) => {}
            None => {
                // Today not yet logged is "pending" not "missed".
                if i != 0 {
                    missed += 1;
                }
            }
        }
        match cursor.previous_day() {
            Some(prev) => cursor = prev,
            None => break,
        }
    }
    (done, missed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    fn log(on: Date, status: LogStatus) -> HabitLog {
        HabitLog {
            id: 0,
            habit_id: 1,
            on_date: on.format(ISO_DATE).unwrap(),
            status: status.as_str().to_string(),
            note: None,
        }
    }

    #[test]
    fn current_streak_counts_back_from_today() {
        let today = date!(2026 - 06 - 05);
        let logs = vec![
            log(date!(2026 - 06 - 05), LogStatus::Done),
            log(date!(2026 - 06 - 04), LogStatus::Done),
            log(date!(2026 - 06 - 03), LogStatus::Done),
        ];
        assert_eq!(compute_stats(today, &logs).current_streak, 3);
    }

    #[test]
    fn skipped_is_neutral_in_current_streak() {
        let today = date!(2026 - 06 - 05);
        let logs = vec![
            log(date!(2026 - 06 - 05), LogStatus::Done),
            log(date!(2026 - 06 - 04), LogStatus::Skipped),
            log(date!(2026 - 06 - 03), LogStatus::Done),
            log(date!(2026 - 06 - 02), LogStatus::Done),
        ];
        assert_eq!(compute_stats(today, &logs).current_streak, 3);
    }

    #[test]
    fn one_missing_day_uses_freeze_and_streak_continues() {
        let today = date!(2026 - 06 - 05);
        let logs = vec![
            log(date!(2026 - 06 - 05), LogStatus::Done),
            // 2026-06-04 missing — week 23 has 1 freeze, used
            log(date!(2026 - 06 - 03), LogStatus::Done),
        ];
        let s = compute_stats(today, &logs);
        assert_eq!(s.current_streak, 2);
        assert_eq!(s.freezes_used, 1);
    }

    #[test]
    fn second_missing_day_in_same_week_breaks_streak() {
        let today = date!(2026 - 06 - 05);
        let logs = vec![
            log(date!(2026 - 06 - 05), LogStatus::Done),
            // 06-04 missing → freeze
            // 06-03 missing → break
            log(date!(2026 - 06 - 02), LogStatus::Done),
        ];
        let s = compute_stats(today, &logs);
        assert_eq!(s.current_streak, 1);
        assert_eq!(s.freezes_used, 1);
    }

    #[test]
    fn three_done_with_one_missing_in_between() {
        let today = date!(2026 - 06 - 05);
        let logs = vec![
            log(date!(2026 - 06 - 02), LogStatus::Done),
            log(date!(2026 - 06 - 03), LogStatus::Done),
            log(date!(2026 - 06 - 05), LogStatus::Done),
        ];
        let s = compute_stats(today, &logs);
        assert_eq!(s.current_streak, 3, "freeze covers 06-04");
        assert_eq!(s.freezes_used, 1);
    }

    #[test]
    fn unlogged_today_is_grace_period() {
        let today = date!(2026 - 06 - 05);
        // Last 30 days: today + 29 prior. Two of the prior days are done; the
        // other 27 are genuinely missed. Today itself must NOT count as missed.
        let logs = vec![
            log(date!(2026 - 06 - 04), LogStatus::Done),
            log(date!(2026 - 06 - 03), LogStatus::Done),
        ];
        let stats = compute_stats(today, &logs);
        assert_eq!(stats.current_streak, 2, "yesterday counts; today is grace");
        assert_eq!(
            stats.missed_30d, 27,
            "today is grace, not counted as missed"
        );
    }

    #[test]
    fn longest_streak_finds_best_run_with_gaps() {
        let today = date!(2026 - 06 - 05);
        let logs = vec![
            // Old run of 4
            log(date!(2026 - 05 - 01), LogStatus::Done),
            log(date!(2026 - 05 - 02), LogStatus::Done),
            log(date!(2026 - 05 - 03), LogStatus::Done),
            log(date!(2026 - 05 - 04), LogStatus::Done),
            // Gap
            // Recent run of 2
            log(date!(2026 - 06 - 04), LogStatus::Done),
            log(date!(2026 - 06 - 05), LogStatus::Done),
        ];
        assert_eq!(compute_stats(today, &logs).longest_streak, 4);
    }

    #[test]
    fn empty_history_is_all_zeros() {
        let today = date!(2026 - 06 - 05);
        let stats = compute_stats(today, &[]);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.longest_streak, 0);
        assert_eq!(stats.completion_rate_30d, 0);
    }

    #[test]
    fn completion_rate_uses_30_day_window() {
        let today = date!(2026 - 06 - 30);
        // 15 done in the last 30 days = 50%
        let mut logs = Vec::new();
        let mut d = today;
        for _ in 0..15 {
            logs.push(log(d, LogStatus::Done));
            d = d.previous_day().unwrap();
            d = d.previous_day().unwrap(); // every other day
        }
        assert_eq!(compute_stats(today, &logs).completion_rate_30d, 50);
    }
}
