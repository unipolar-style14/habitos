use crate::error::CoreError;
use sqlx::{FromRow, Pool, Sqlite};
use time::Date;
use time::macros::format_description;

const ISO_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");

#[derive(Debug, Clone, FromRow)]
pub struct JournalEntry {
    pub id: i64,
    pub on_date: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct JournalRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> JournalRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, on: Date, body: &str) -> Result<JournalEntry, CoreError> {
        let date_str = on.format(ISO_DATE).expect("format infallible");
        let row = sqlx::query_as::<_, JournalEntry>(
            "INSERT INTO journal_entries (on_date, body) VALUES (?, ?)
             ON CONFLICT(on_date) DO UPDATE
               SET body = excluded.body,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             RETURNING id, on_date, body, created_at, updated_at",
        )
        .bind(&date_str)
        .bind(body)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get(&self, on: Date) -> Result<Option<JournalEntry>, CoreError> {
        let date_str = on.format(ISO_DATE).expect("format infallible");
        let row = sqlx::query_as::<_, JournalEntry>(
            "SELECT id, on_date, body, created_at, updated_at
             FROM journal_entries WHERE on_date = ?",
        )
        .bind(date_str)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    /// LIKE-based substring search. Semantic search comes in M5.
    pub async fn search(&self, query: &str) -> Result<Vec<JournalEntry>, CoreError> {
        let pattern = format!("%{}%", query.replace('%', r"\%").replace('_', r"\_"));
        let rows = sqlx::query_as::<_, JournalEntry>(
            "SELECT id, on_date, body, created_at, updated_at
             FROM journal_entries WHERE body LIKE ? ESCAPE '\\'
             ORDER BY on_date DESC",
        )
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn recent(&self, n: i64) -> Result<Vec<JournalEntry>, CoreError> {
        let rows = sqlx::query_as::<_, JournalEntry>(
            "SELECT id, on_date, body, created_at, updated_at
             FROM journal_entries ORDER BY on_date DESC LIMIT ?",
        )
        .bind(n)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }
}
