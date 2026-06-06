use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use time::Date;
use time::macros::format_description;

const ISO_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");

/// The on-disk JSON the user authors and points `habitos skill add` at.
/// Intentionally minimal — only `name` and `items` are required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFile {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub items: Vec<SkillItemFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillItemFile {
    /// Stable id from the source (slug, hash, sequence number — anything unique within the file).
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
}

impl SkillFile {
    pub fn from_path(path: &std::path::Path) -> Result<Self, CoreError> {
        let raw = std::fs::read_to_string(path)?;
        let parsed: Self = serde_json::from_str(&raw).map_err(|e| {
            CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid skill JSON: {e}"),
            ))
        })?;
        if parsed.name.trim().is_empty() {
            return Err(CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "skill JSON missing `name`",
            )));
        }
        if parsed.items.is_empty() {
            return Err(CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "skill JSON has no items",
            )));
        }
        // Detect duplicate ids in the source — they'd silently collide on import.
        let mut seen: std::collections::HashSet<&str> =
            std::collections::HashSet::with_capacity(parsed.items.len());
        for item in &parsed.items {
            if !seen.insert(item.id.as_str()) {
                return Err(CoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("skill JSON has duplicate id `{}`", item.id),
                )));
            }
        }
        Ok(parsed)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Skill {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub source_path: Option<String>,
    pub pace: i64,
    pub revisions: i64,
    pub started_at: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Skill {
    pub fn day_number(&self, today: Date) -> i64 {
        match Date::parse(&self.started_at, ISO_DATE) {
            Ok(d) => (today - d).whole_days() + 1,
            Err(_) => 1,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SkillItem {
    pub id: i64,
    pub skill_id: i64,
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub tags: Option<String>,
    pub difficulty: Option<String>,
    pub position: i64,
    pub status: String,
    pub solved_at: Option<String>,
    pub last_revised_at: Option<String>,
    pub created_at: String,
}

impl SkillItem {
    pub fn tag_list(&self) -> Vec<&str> {
        match &self.tags {
            None => Vec::new(),
            Some(t) => t
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SkillProgress {
    pub total: u32,
    pub solved: u32,
    pub skipped: u32,
}

impl SkillProgress {
    pub fn percent(&self) -> u32 {
        (self.solved * 100).checked_div(self.total).unwrap_or(0)
    }
    pub fn pending(&self) -> u32 {
        self.total
            .saturating_sub(self.solved)
            .saturating_sub(self.skipped)
    }
}

pub struct SkillRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> SkillRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn add(
        &self,
        name: &str,
        description: Option<&str>,
        source_path: Option<&str>,
        pace: i64,
        revisions: i64,
        started_at: Date,
    ) -> Result<Skill, CoreError> {
        let date_str = started_at.format(ISO_DATE).expect("format infallible");
        let row = sqlx::query_as::<_, Skill>(
            "INSERT INTO skills (name, description, source_path, pace, revisions, started_at)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id, name, description, source_path, pace, revisions, started_at, created_at, updated_at",
        )
        .bind(name)
        .bind(description)
        .bind(source_path)
        .bind(pace)
        .bind(revisions)
        .bind(date_str)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    /// Insert all items from a SkillFile. Returns (inserted, duplicates_skipped).
    pub async fn import_items(
        &self,
        skill_id: i64,
        items: &[SkillItemFile],
    ) -> Result<(usize, usize), CoreError> {
        let mut inserted = 0usize;
        let mut skipped = 0usize;
        for (idx, item) in items.iter().enumerate() {
            let tags_str = if item.tags.is_empty() {
                None
            } else {
                Some(item.tags.join(","))
            };
            let res = sqlx::query(
                "INSERT INTO skill_items
                   (skill_id, external_id, title, description, url, tags, difficulty, position)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(skill_id, external_id) DO NOTHING",
            )
            .bind(skill_id)
            .bind(&item.id)
            .bind(&item.title)
            .bind(item.description.as_deref())
            .bind(item.url.as_deref())
            .bind(tags_str.as_deref())
            .bind(item.difficulty.as_deref())
            .bind(idx as i64)
            .execute(self.pool)
            .await?;
            if res.rows_affected() == 0 {
                skipped += 1;
            } else {
                inserted += 1;
            }
        }
        Ok((inserted, skipped))
    }

    pub async fn list(&self) -> Result<Vec<Skill>, CoreError> {
        let rows = sqlx::query_as::<_, Skill>(
            "SELECT id, name, description, source_path, pace, revisions, started_at, created_at, updated_at
             FROM skills ORDER BY created_at",
        )
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Skill>, CoreError> {
        let row = sqlx::query_as::<_, Skill>(
            "SELECT id, name, description, source_path, pace, revisions, started_at, created_at, updated_at
             FROM skills WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn remove(&self, name: &str) -> Result<bool, CoreError> {
        let res = sqlx::query("DELETE FROM skills WHERE name = ?")
            .bind(name)
            .execute(self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn items(
        &self,
        skill_id: i64,
        status_filter: Option<&str>,
    ) -> Result<Vec<SkillItem>, CoreError> {
        let rows = if let Some(status) = status_filter {
            sqlx::query_as::<_, SkillItem>(
                "SELECT id, skill_id, external_id, title, description, url, tags, difficulty,
                        position, status, solved_at, last_revised_at, created_at
                 FROM skill_items WHERE skill_id = ? AND status = ?
                 ORDER BY position",
            )
            .bind(skill_id)
            .bind(status)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as::<_, SkillItem>(
                "SELECT id, skill_id, external_id, title, description, url, tags, difficulty,
                        position, status, solved_at, last_revised_at, created_at
                 FROM skill_items WHERE skill_id = ?
                 ORDER BY position",
            )
            .bind(skill_id)
            .fetch_all(self.pool)
            .await?
        };
        Ok(rows)
    }

    pub async fn find_item(
        &self,
        skill_id: i64,
        external_id: &str,
    ) -> Result<Option<SkillItem>, CoreError> {
        let row = sqlx::query_as::<_, SkillItem>(
            "SELECT id, skill_id, external_id, title, description, url, tags, difficulty,
                    position, status, solved_at, last_revised_at, created_at
             FROM skill_items WHERE skill_id = ? AND external_id = ?",
        )
        .bind(skill_id)
        .bind(external_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn pending_items(
        &self,
        skill_id: i64,
        limit: i64,
    ) -> Result<Vec<SkillItem>, CoreError> {
        let rows = sqlx::query_as::<_, SkillItem>(
            "SELECT id, skill_id, external_id, title, description, url, tags, difficulty,
                    position, status, solved_at, last_revised_at, created_at
             FROM skill_items WHERE skill_id = ? AND status = 'pending'
             ORDER BY position LIMIT ?",
        )
        .bind(skill_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Items to revise — solved items, ordered by least-recently touched.
    /// "Touched" = `last_revised_at` if present, else `solved_at`.
    pub async fn revision_candidates(
        &self,
        skill_id: i64,
        limit: i64,
    ) -> Result<Vec<SkillItem>, CoreError> {
        let rows = sqlx::query_as::<_, SkillItem>(
            "SELECT id, skill_id, external_id, title, description, url, tags, difficulty,
                    position, status, solved_at, last_revised_at, created_at
             FROM skill_items WHERE skill_id = ? AND status = 'solved'
             ORDER BY COALESCE(last_revised_at, solved_at) ASC
             LIMIT ?",
        )
        .bind(skill_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn mark_solved(
        &self,
        skill_id: i64,
        external_ids: &[String],
    ) -> Result<usize, CoreError> {
        let mut count = 0;
        for id in external_ids {
            let res = sqlx::query(
                "UPDATE skill_items
                 SET status = 'solved',
                     solved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE skill_id = ? AND external_id = ? AND status != 'solved'",
            )
            .bind(skill_id)
            .bind(id)
            .execute(self.pool)
            .await?;
            if res.rows_affected() > 0 {
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn mark_revised(
        &self,
        skill_id: i64,
        external_ids: &[String],
    ) -> Result<usize, CoreError> {
        let mut count = 0;
        for id in external_ids {
            let res = sqlx::query(
                "UPDATE skill_items
                 SET last_revised_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE skill_id = ? AND external_id = ? AND status = 'solved'",
            )
            .bind(skill_id)
            .bind(id)
            .execute(self.pool)
            .await?;
            if res.rows_affected() > 0 {
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn progress(&self, skill_id: i64) -> Result<SkillProgress, CoreError> {
        let row: (i64, i64, i64) = sqlx::query_as(
            "SELECT
               COUNT(*) AS total,
               COUNT(*) FILTER (WHERE status = 'solved') AS solved,
               COUNT(*) FILTER (WHERE status = 'skipped') AS skipped
             FROM skill_items WHERE skill_id = ?",
        )
        .bind(skill_id)
        .fetch_one(self.pool)
        .await?;
        Ok(SkillProgress {
            total: row.0.max(0) as u32,
            solved: row.1.max(0) as u32,
            skipped: row.2.max(0) as u32,
        })
    }
}

/// The JSON template a new user gets from `habitos skill template`.
pub const SKILL_TEMPLATE: &str = r#"{
  "name": "DSA",
  "description": "Top 90 LeetCode problems for cracking interviews",
  "items": [
    {
      "id": "two-sum",
      "title": "Two Sum",
      "tags": ["array", "hash-map"],
      "difficulty": "easy",
      "url": "https://leetcode.com/problems/two-sum"
    },
    {
      "id": "valid-parentheses",
      "title": "Valid Parentheses",
      "tags": ["string", "stack"],
      "difficulty": "easy",
      "url": "https://leetcode.com/problems/valid-parentheses"
    },
    {
      "id": "reverse-linked-list",
      "title": "Reverse Linked List",
      "tags": ["linked-list"],
      "difficulty": "easy",
      "url": "https://leetcode.com/problems/reverse-linked-list"
    }
  ]
}
"#;

/// Compute pace (items per day) needed to finish `total` items in `days` days.
/// Rounds up so users don't fall behind by 1.
pub fn pace_for_target(total: usize, days: i64) -> i64 {
    if days <= 0 || total == 0 {
        return 1;
    }
    ((total as i64) + days - 1) / days
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pace_calculation_rounds_up() {
        assert_eq!(pace_for_target(90, 45), 2);
        assert_eq!(pace_for_target(91, 45), 3); // 91/45 = 2.02 → 3
        assert_eq!(pace_for_target(100, 30), 4); // 100/30 = 3.33 → 4
        assert_eq!(pace_for_target(5, 30), 1);
    }

    #[test]
    fn pace_handles_zero_safely() {
        assert_eq!(pace_for_target(0, 30), 1);
        assert_eq!(pace_for_target(10, 0), 1);
    }

    #[test]
    fn skill_template_is_valid_json() {
        let parsed: SkillFile = serde_json::from_str(SKILL_TEMPLATE).expect("template must parse");
        assert_eq!(parsed.name, "DSA");
        assert!(parsed.items.len() >= 3);
        for item in &parsed.items {
            assert!(!item.id.is_empty());
            assert!(!item.title.is_empty());
        }
    }

    #[test]
    fn skill_item_tag_split() {
        let item = SkillItem {
            id: 0,
            skill_id: 0,
            external_id: "x".into(),
            title: "X".into(),
            description: None,
            url: None,
            tags: Some("array, hash-map ,  ".into()),
            difficulty: None,
            position: 0,
            status: "pending".into(),
            solved_at: None,
            last_revised_at: None,
            created_at: "".into(),
        };
        assert_eq!(item.tag_list(), vec!["array", "hash-map"]);
    }

    #[test]
    fn skill_progress_percent() {
        let p = SkillProgress {
            total: 90,
            solved: 27,
            skipped: 0,
        };
        assert_eq!(p.percent(), 30);
        assert_eq!(p.pending(), 63);
    }

    #[test]
    fn skill_progress_handles_empty_skill() {
        let p = SkillProgress {
            total: 0,
            solved: 0,
            skipped: 0,
        };
        assert_eq!(p.percent(), 0);
        assert_eq!(p.pending(), 0);
    }
}
