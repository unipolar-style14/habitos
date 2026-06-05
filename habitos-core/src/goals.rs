use crate::error::CoreError;
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Debug, Clone, FromRow)]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub priority: i64,
    pub deadline: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct GoalMilestone {
    pub id: i64,
    pub goal_id: i64,
    pub name: String,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy)]
pub struct GoalProgress {
    pub completed: u32,
    pub total: u32,
}

impl GoalProgress {
    pub fn percent(&self) -> u32 {
        (self.completed * 100).checked_div(self.total).unwrap_or(0)
    }
}

pub struct GoalRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> GoalRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn add(&self, name: &str) -> Result<Goal, CoreError> {
        let row = sqlx::query_as::<_, Goal>(
            "INSERT INTO goals (name) VALUES (?)
             RETURNING id, name, description, priority, deadline, status, created_at, updated_at",
        )
        .bind(name)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Goal>, CoreError> {
        let row = sqlx::query_as::<_, Goal>(
            "SELECT id, name, description, priority, deadline, status, created_at, updated_at
             FROM goals WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list(&self, include_done: bool) -> Result<Vec<Goal>, CoreError> {
        let sql = if include_done {
            "SELECT id, name, description, priority, deadline, status, created_at, updated_at
             FROM goals ORDER BY priority DESC, created_at"
        } else {
            "SELECT id, name, description, priority, deadline, status, created_at, updated_at
             FROM goals WHERE status = 'open' ORDER BY priority DESC, created_at"
        };
        let rows = sqlx::query_as::<_, Goal>(sql).fetch_all(self.pool).await?;
        Ok(rows)
    }

    pub async fn complete(&self, name: &str) -> Result<bool, CoreError> {
        let res = sqlx::query(
            "UPDATE goals
             SET status = 'complete',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE name = ? AND status = 'open'",
        )
        .bind(name)
        .execute(self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn add_milestone(
        &self,
        goal_id: i64,
        name: &str,
    ) -> Result<GoalMilestone, CoreError> {
        let row = sqlx::query_as::<_, GoalMilestone>(
            "INSERT INTO goal_milestones (goal_id, name) VALUES (?, ?)
             RETURNING id, goal_id, name, completed_at, created_at",
        )
        .bind(goal_id)
        .bind(name)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn complete_milestone(
        &self,
        goal_id: i64,
        milestone_name: &str,
    ) -> Result<bool, CoreError> {
        let res = sqlx::query(
            "UPDATE goal_milestones
             SET completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE goal_id = ? AND name = ? AND completed_at IS NULL",
        )
        .bind(goal_id)
        .bind(milestone_name)
        .execute(self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn milestones(&self, goal_id: i64) -> Result<Vec<GoalMilestone>, CoreError> {
        let rows = sqlx::query_as::<_, GoalMilestone>(
            "SELECT id, goal_id, name, completed_at, created_at FROM goal_milestones
             WHERE goal_id = ? ORDER BY created_at",
        )
        .bind(goal_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn progress(&self, goal_id: i64) -> Result<GoalProgress, CoreError> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT
               COUNT(*) FILTER (WHERE completed_at IS NOT NULL) AS done,
               COUNT(*) AS total
             FROM goal_milestones WHERE goal_id = ?",
        )
        .bind(goal_id)
        .fetch_one(self.pool)
        .await?;
        Ok(GoalProgress {
            completed: row.0.max(0) as u32,
            total: row.1.max(0) as u32,
        })
    }
}
