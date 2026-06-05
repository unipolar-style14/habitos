//! Plain-text export of every entity. Markdown for human reading, CSV for
//! spreadsheet import. Used by `habitos export` (M6).

use crate::error::CoreError;
use crate::focus::FocusSession;
use crate::goals::{Goal, GoalMilestone};
use crate::habits::{Habit, HabitLog};
use crate::journal::JournalEntry;
use crate::reviews::DailyReview;
use sqlx::{Pool, Sqlite};
use std::fmt::Write as _;

pub struct Snapshot {
    pub habits: Vec<(Habit, Vec<HabitLog>)>,
    pub goals: Vec<(Goal, Vec<GoalMilestone>)>,
    pub focus_sessions: Vec<FocusSession>,
    pub journals: Vec<JournalEntry>,
    pub dailies: Vec<DailyReview>,
}

pub async fn load(pool: &Pool<Sqlite>) -> Result<Snapshot, CoreError> {
    let habits: Vec<Habit> = sqlx::query_as(
        "SELECT id, name, archived, created_at, updated_at FROM habits ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    let mut habits_with_logs = Vec::with_capacity(habits.len());
    for h in habits {
        let logs: Vec<HabitLog> = sqlx::query_as(
            "SELECT id, habit_id, on_date, status, note FROM habit_logs WHERE habit_id = ? ORDER BY on_date",
        )
        .bind(h.id)
        .fetch_all(pool)
        .await?;
        habits_with_logs.push((h, logs));
    }

    let goals: Vec<Goal> = sqlx::query_as(
        "SELECT id, name, description, priority, deadline, status, created_at, updated_at
         FROM goals ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;
    let mut goals_with_ms = Vec::with_capacity(goals.len());
    for g in goals {
        let ms: Vec<GoalMilestone> = sqlx::query_as(
            "SELECT id, goal_id, name, completed_at, created_at FROM goal_milestones
             WHERE goal_id = ? ORDER BY created_at",
        )
        .bind(g.id)
        .fetch_all(pool)
        .await?;
        goals_with_ms.push((g, ms));
    }

    let focus_sessions: Vec<FocusSession> = sqlx::query_as(
        "SELECT id, start_at, end_at, project, note, created_at FROM focus_sessions ORDER BY start_at",
    )
    .fetch_all(pool)
    .await?;

    let journals: Vec<JournalEntry> = sqlx::query_as(
        "SELECT id, on_date, body, created_at, updated_at FROM journal_entries ORDER BY on_date",
    )
    .fetch_all(pool)
    .await?;

    let dailies: Vec<DailyReview> = sqlx::query_as(
        "SELECT id, on_date, went_well, didnt_go_well, learned, tomorrow_priority, ai_summary, created_at
         FROM daily_reviews ORDER BY on_date",
    )
    .fetch_all(pool)
    .await?;

    Ok(Snapshot {
        habits: habits_with_logs,
        goals: goals_with_ms,
        focus_sessions,
        journals,
        dailies,
    })
}

pub fn to_markdown(snap: &Snapshot) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# HabitOS Export\n");

    let _ = writeln!(out, "## Habits\n");
    if snap.habits.is_empty() {
        let _ = writeln!(out, "_None._\n");
    } else {
        for (h, logs) in &snap.habits {
            let arch = if h.archived != 0 { " [archived]" } else { "" };
            let _ = writeln!(out, "### {}{}", h.name, arch);
            let _ = writeln!(out, "Created: {}\n", h.created_at);
            for l in logs {
                let note = l
                    .note
                    .as_deref()
                    .map(|n| format!(" — {n}"))
                    .unwrap_or_default();
                let _ = writeln!(out, "- {} {}{}", l.on_date, l.status, note);
            }
            let _ = writeln!(out);
        }
    }

    let _ = writeln!(out, "## Goals\n");
    if snap.goals.is_empty() {
        let _ = writeln!(out, "_None._\n");
    } else {
        for (g, ms) in &snap.goals {
            let _ = writeln!(out, "### {} [{}]", g.name, g.status);
            if let Some(desc) = &g.description {
                let _ = writeln!(out, "{desc}");
            }
            if !ms.is_empty() {
                let _ = writeln!(out, "Milestones:");
                for m in ms {
                    let mark = if m.completed_at.is_some() {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let _ = writeln!(out, "- {} {}", mark, m.name);
                }
            }
            let _ = writeln!(out);
        }
    }

    let _ = writeln!(out, "## Focus Sessions\n");
    if snap.focus_sessions.is_empty() {
        let _ = writeln!(out, "_None._\n");
    } else {
        for s in &snap.focus_sessions {
            let dur = s
                .duration_minutes()
                .map(|m| format!("{m} min"))
                .unwrap_or_else(|| "active".into());
            let proj = s.project.as_deref().unwrap_or("");
            let _ = writeln!(
                out,
                "- {} → {} ({}){}",
                s.start_at,
                s.end_at.as_deref().unwrap_or("—"),
                dur,
                if proj.is_empty() {
                    String::new()
                } else {
                    format!(" — {proj}")
                }
            );
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Journal\n");
    for j in &snap.journals {
        let _ = writeln!(out, "### {}\n", j.on_date);
        let _ = writeln!(out, "{}\n", j.body);
    }

    let _ = writeln!(out, "## Daily Reviews\n");
    for r in &snap.dailies {
        let _ = writeln!(out, "### {}", r.on_date);
        let _ = writeln!(
            out,
            "- Went well: {}",
            r.went_well.as_deref().unwrap_or("—")
        );
        let _ = writeln!(
            out,
            "- Didn't go well: {}",
            r.didnt_go_well.as_deref().unwrap_or("—")
        );
        let _ = writeln!(out, "- Learned: {}", r.learned.as_deref().unwrap_or("—"));
        let _ = writeln!(
            out,
            "- Tomorrow's priority: {}\n",
            r.tomorrow_priority.as_deref().unwrap_or("—")
        );
    }

    out
}

/// Returns a Vec of (filename, contents) pairs. Caller decides where to write.
pub fn to_csv_files(snap: &Snapshot) -> Vec<(&'static str, String)> {
    let mut habits_csv = String::from("id,name,archived,created_at\n");
    for (h, _) in &snap.habits {
        let _ = writeln!(
            habits_csv,
            "{},{},{},{}",
            h.id,
            csv_quote(&h.name),
            h.archived,
            h.created_at
        );
    }

    let mut habit_logs_csv = String::from("id,habit_id,on_date,status,note\n");
    for (_, logs) in &snap.habits {
        for l in logs {
            let _ = writeln!(
                habit_logs_csv,
                "{},{},{},{},{}",
                l.id,
                l.habit_id,
                l.on_date,
                l.status,
                csv_quote(l.note.as_deref().unwrap_or(""))
            );
        }
    }

    let mut goals_csv = String::from("id,name,status,priority,deadline,created_at\n");
    for (g, _) in &snap.goals {
        let _ = writeln!(
            goals_csv,
            "{},{},{},{},{},{}",
            g.id,
            csv_quote(&g.name),
            g.status,
            g.priority,
            g.deadline.as_deref().unwrap_or(""),
            g.created_at
        );
    }

    let mut focus_csv = String::from("id,start_at,end_at,project,note\n");
    for s in &snap.focus_sessions {
        let _ = writeln!(
            focus_csv,
            "{},{},{},{},{}",
            s.id,
            s.start_at,
            s.end_at.as_deref().unwrap_or(""),
            csv_quote(s.project.as_deref().unwrap_or("")),
            csv_quote(s.note.as_deref().unwrap_or(""))
        );
    }

    let mut journals_csv = String::from("id,on_date,body\n");
    for j in &snap.journals {
        let _ = writeln!(
            journals_csv,
            "{},{},{}",
            j.id,
            j.on_date,
            csv_quote(&j.body)
        );
    }

    let mut reviews_csv =
        String::from("id,on_date,went_well,didnt_go_well,learned,tomorrow_priority\n");
    for r in &snap.dailies {
        let _ = writeln!(
            reviews_csv,
            "{},{},{},{},{},{}",
            r.id,
            r.on_date,
            csv_quote(r.went_well.as_deref().unwrap_or("")),
            csv_quote(r.didnt_go_well.as_deref().unwrap_or("")),
            csv_quote(r.learned.as_deref().unwrap_or("")),
            csv_quote(r.tomorrow_priority.as_deref().unwrap_or(""))
        );
    }

    vec![
        ("habits.csv", habits_csv),
        ("habit_logs.csv", habit_logs_csv),
        ("goals.csv", goals_csv),
        ("focus_sessions.csv", focus_csv),
        ("journal_entries.csv", journals_csv),
        ("daily_reviews.csv", reviews_csv),
    ]
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
