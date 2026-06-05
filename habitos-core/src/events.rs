use crate::error::CoreError;
use sqlx::{Pool, Sqlite};

/// Best-effort append-only audit trail. Written from CLI handlers after a
/// successful mutation. If the process dies between the mutation and the log
/// write, the data state is still consistent — the audit trail is allowed to
/// miss events.
pub struct EventLog<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> EventLog<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        entity_type: &str,
        entity_id: Option<i64>,
        action: &str,
        payload: Option<&str>,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "INSERT INTO event_log (entity_type, entity_id, action, payload)
             VALUES (?, ?, ?, ?)",
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(action)
        .bind(payload)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
