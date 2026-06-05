use crate::{config::Config, error::CoreError};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use sqlx::{Pool, Sqlite};

pub struct Db {
    pool: Pool<Sqlite>,
}

impl Db {
    pub async fn open(config: &Config) -> Result<Self, CoreError> {
        let opts = SqliteConnectOptions::new()
            .filename(config.db_path())
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePool::connect_with(opts).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), CoreError> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrate_creates_expected_tables() {
        let tmp = tempfile::tempdir().unwrap();
        let config = Config::load_or_init_in(tmp.path().to_path_buf()).unwrap();
        let db = Db::open(&config).await.unwrap();
        db.migrate().await.unwrap();

        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlx_%' ORDER BY name",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();
        let names: Vec<String> = tables.into_iter().map(|(n,)| n).collect();

        for required in [
            "habits",
            "habit_logs",
            "goals",
            "goal_milestones",
            "focus_sessions",
            "journal_entries",
            "daily_reviews",
            "weekly_reviews",
            "monthly_reviews",
            "ai_memories",
            "settings",
            "event_log",
        ] {
            assert!(
                names.iter().any(|n| n == required),
                "missing table: {required}"
            );
        }
    }
}
