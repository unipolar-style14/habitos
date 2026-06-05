use crate::error::CoreError;
use sqlx::{FromRow, Pool, Sqlite};
use time::Date;
use time::macros::format_description;

const ISO_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");
const ISO_MONTH: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]");

#[derive(Debug, Clone, FromRow)]
pub struct DailyReview {
    pub id: i64,
    pub on_date: String,
    pub went_well: Option<String>,
    pub didnt_go_well: Option<String>,
    pub learned: Option<String>,
    pub tomorrow_priority: Option<String>,
    pub ai_summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct WeeklyReview {
    pub id: i64,
    pub week_starting: String,
    pub wins: Option<String>,
    pub failures: Option<String>,
    pub trends: Option<String>,
    pub next_actions: Option<String>,
    pub ai_summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct MonthlyReview {
    pub id: i64,
    pub month: String,
    pub performance: Option<String>,
    pub productivity_score: Option<i64>,
    pub improvement_areas: Option<String>,
    pub ai_summary: Option<String>,
    pub created_at: String,
}

pub struct ReviewRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

#[derive(Debug, Clone, Default)]
pub struct DailyAnswers {
    pub went_well: Option<String>,
    pub didnt_go_well: Option<String>,
    pub learned: Option<String>,
    pub tomorrow_priority: Option<String>,
}

impl<'a> ReviewRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn save_daily(
        &self,
        on: Date,
        answers: &DailyAnswers,
    ) -> Result<DailyReview, CoreError> {
        let date_str = on.format(ISO_DATE).expect("format infallible");
        let row = sqlx::query_as::<_, DailyReview>(
            "INSERT INTO daily_reviews (on_date, went_well, didnt_go_well, learned, tomorrow_priority)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(on_date) DO UPDATE SET
               went_well = excluded.went_well,
               didnt_go_well = excluded.didnt_go_well,
               learned = excluded.learned,
               tomorrow_priority = excluded.tomorrow_priority
             RETURNING id, on_date, went_well, didnt_go_well, learned, tomorrow_priority, ai_summary, created_at",
        )
        .bind(&date_str)
        .bind(answers.went_well.as_deref())
        .bind(answers.didnt_go_well.as_deref())
        .bind(answers.learned.as_deref())
        .bind(answers.tomorrow_priority.as_deref())
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_daily(&self, on: Date) -> Result<Option<DailyReview>, CoreError> {
        let date_str = on.format(ISO_DATE).expect("format infallible");
        let row = sqlx::query_as::<_, DailyReview>(
            "SELECT id, on_date, went_well, didnt_go_well, learned, tomorrow_priority, ai_summary, created_at
             FROM daily_reviews WHERE on_date = ?",
        )
        .bind(date_str)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn dailies_since(&self, since: Date) -> Result<Vec<DailyReview>, CoreError> {
        let date_str = since.format(ISO_DATE).expect("format infallible");
        let rows = sqlx::query_as::<_, DailyReview>(
            "SELECT id, on_date, went_well, didnt_go_well, learned, tomorrow_priority, ai_summary, created_at
             FROM daily_reviews WHERE on_date >= ?
             ORDER BY on_date",
        )
        .bind(date_str)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }
}

/// The Monday of the week containing `d` (Mon=start-of-week convention).
pub fn week_starting(d: Date) -> Date {
    let weekday_idx = d.weekday().number_days_from_monday() as i64;
    d - time::Duration::days(weekday_idx)
}

/// First day of the month for `d`.
pub fn month_first_day(d: Date) -> Date {
    Date::from_calendar_date(d.year(), d.month(), 1).expect("valid date")
}

/// Format a Date as YYYY-MM for month keys.
pub fn month_key(d: Date) -> String {
    d.format(ISO_MONTH).expect("format infallible")
}
