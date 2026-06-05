use crate::error::CoreError;
use sqlx::{FromRow, Pool, Sqlite};
use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;

#[derive(Debug, Clone, FromRow)]
pub struct FocusSession {
    pub id: i64,
    pub start_at: String,
    pub end_at: Option<String>,
    pub project: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

impl FocusSession {
    /// Duration in whole minutes, or None if still active.
    pub fn duration_minutes(&self) -> Option<i64> {
        let start = OffsetDateTime::parse(&self.start_at, &Iso8601::DEFAULT).ok()?;
        let end = OffsetDateTime::parse(self.end_at.as_deref()?, &Iso8601::DEFAULT).ok()?;
        Some((end - start).whole_minutes())
    }
}

pub struct FocusRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> FocusRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn active(&self) -> Result<Option<FocusSession>, CoreError> {
        let row = sqlx::query_as::<_, FocusSession>(
            "SELECT id, start_at, end_at, project, note, created_at
             FROM focus_sessions WHERE end_at IS NULL
             ORDER BY start_at DESC LIMIT 1",
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn start(
        &self,
        project: Option<&str>,
        note: Option<&str>,
    ) -> Result<FocusSession, CoreError> {
        if let Some(existing) = self.active().await? {
            return Err(CoreError::FocusAlreadyActive {
                start_at: existing.start_at,
            });
        }
        let row = sqlx::query_as::<_, FocusSession>(
            "INSERT INTO focus_sessions (start_at, project, note)
             VALUES (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)
             RETURNING id, start_at, end_at, project, note, created_at",
        )
        .bind(project)
        .bind(note)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn stop(&self) -> Result<FocusSession, CoreError> {
        let active = self.active().await?.ok_or(CoreError::NoActiveFocus)?;
        let row = sqlx::query_as::<_, FocusSession>(
            "UPDATE focus_sessions
             SET end_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?
             RETURNING id, start_at, end_at, project, note, created_at",
        )
        .bind(active.id)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    /// All sessions with start_at on/after `since` (inclusive), in start order.
    pub async fn since(&self, since_utc_iso: &str) -> Result<Vec<FocusSession>, CoreError> {
        let rows = sqlx::query_as::<_, FocusSession>(
            "SELECT id, start_at, end_at, project, note, created_at
             FROM focus_sessions WHERE start_at >= ?
             ORDER BY start_at",
        )
        .bind(since_utc_iso)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Insert a pre-completed focus session — used by `habitos log` when you
    /// log time you spent before reaching the terminal.
    pub async fn log_completed(
        &self,
        start_at_iso: &str,
        end_at_iso: &str,
        project: Option<&str>,
        note: Option<&str>,
    ) -> Result<FocusSession, CoreError> {
        let row = sqlx::query_as::<_, FocusSession>(
            "INSERT INTO focus_sessions (start_at, end_at, project, note)
             VALUES (?, ?, ?, ?)
             RETURNING id, start_at, end_at, project, note, created_at",
        )
        .bind(start_at_iso)
        .bind(end_at_iso)
        .bind(project)
        .bind(note)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }
}

/// Total completed-session minutes for sessions that ended within the slice.
pub fn total_minutes(sessions: &[FocusSession]) -> i64 {
    sessions.iter().filter_map(|s| s.duration_minutes()).sum()
}
