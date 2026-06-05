//! Deterministic, AI-free reports rendered as plain-text/markdown.
//! M4 layers an `## AI Summary` section on top of these.

use crate::focus::{FocusSession, total_minutes};
use crate::goals::Goal;
use crate::habits::{Habit, HabitLog, compute_stats};
use crate::journal::JournalEntry;
use crate::reviews::DailyReview;
use std::fmt::Write as _;
use time::Date;

pub struct DailySnapshot<'a> {
    pub today: Date,
    pub habits: &'a [(Habit, Vec<HabitLog>)],
    pub focus_sessions: &'a [FocusSession],
    pub journal: Option<&'a JournalEntry>,
    pub reflection: Option<&'a DailyReview>,
}

pub struct WeeklySnapshot<'a> {
    pub week_start: Date,
    pub today: Date,
    pub habits: &'a [(Habit, Vec<HabitLog>)],
    pub focus_sessions: &'a [FocusSession],
    pub journals: &'a [JournalEntry],
    pub goals: &'a [(Goal, u32, u32)], // (goal, completed milestones, total)
    pub dailies: &'a [DailyReview],
}

pub struct MonthlySnapshot<'a> {
    pub month_start: Date,
    pub today: Date,
    pub habits: &'a [(Habit, Vec<HabitLog>)],
    pub focus_sessions: &'a [FocusSession],
    pub journals: &'a [JournalEntry],
    pub goals: &'a [(Goal, u32, u32)],
    pub dailies: &'a [DailyReview],
}

pub fn daily(snap: &DailySnapshot<'_>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Daily Review — {}\n", snap.today);

    // Habits today
    let _ = writeln!(out, "## Habits");
    if snap.habits.is_empty() {
        let _ = writeln!(out, "_No habits tracked yet._");
    } else {
        for (h, logs) in snap.habits {
            let s = compute_stats(snap.today, logs);
            let today_status = logs
                .iter()
                .find(|l| l.on_date == snap.today.to_string())
                .map(|l| l.status.as_str())
                .unwrap_or("not logged");
            let _ = writeln!(
                out,
                "- **{}** — {} (streak {}, 30d {}%)",
                h.name, today_status, s.current_streak, s.completion_rate_30d
            );
        }
    }
    let _ = writeln!(out);

    // Focus
    let _ = writeln!(out, "## Focus");
    if snap.focus_sessions.is_empty() {
        let _ = writeln!(out, "_No focus sessions today._");
    } else {
        let total = total_minutes(snap.focus_sessions);
        let _ = writeln!(
            out,
            "{} session(s), {} min total",
            snap.focus_sessions.len(),
            total
        );
        for s in snap.focus_sessions {
            let proj = s.project.as_deref().unwrap_or("(no project)");
            let mins = s.duration_minutes().unwrap_or(0);
            let _ = writeln!(out, "- {} — {} min", proj, mins);
        }
    }
    let _ = writeln!(out);

    // Journal
    let _ = writeln!(out, "## Journal");
    match snap.journal {
        None => {
            let _ = writeln!(out, "_No journal entry yet._");
        }
        Some(j) => {
            let snippet: String = j.body.chars().take(300).collect();
            let _ = writeln!(out, "{}", snippet);
            if j.body.chars().count() > 300 {
                let _ = writeln!(out, "…");
            }
        }
    }
    let _ = writeln!(out);

    // Reflection
    let _ = writeln!(out, "## Reflection");
    match snap.reflection {
        None => {
            let _ = writeln!(out, "_No reflection yet. Run `habitos reflect`._");
        }
        Some(r) => {
            for (label, val) in [
                ("Went well", r.went_well.as_deref()),
                ("Didn't go well", r.didnt_go_well.as_deref()),
                ("Learned", r.learned.as_deref()),
                ("Tomorrow's priority", r.tomorrow_priority.as_deref()),
            ] {
                let _ = writeln!(out, "- **{}:** {}", label, val.unwrap_or("—"));
            }
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## AI Summary");
    let _ = writeln!(
        out,
        "_Add a configured AI backend in `config.toml` to populate this section._"
    );

    out
}

pub fn weekly(snap: &WeeklySnapshot<'_>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Weekly Review — week of {}\n", snap.week_start);

    // Habit consistency
    let _ = writeln!(out, "## Habits");
    if snap.habits.is_empty() {
        let _ = writeln!(out, "_No habits tracked yet._");
    } else {
        for (h, logs) in snap.habits {
            let s = compute_stats(snap.today, logs);
            let done_in_week = logs
                .iter()
                .filter(|l| l.on_date.as_str() >= snap.week_start.to_string().as_str())
                .filter(|l| l.status == "done")
                .count();
            let _ = writeln!(
                out,
                "- **{}** — {}/7 done this week (streak {}, best {})",
                h.name, done_in_week, s.current_streak, s.longest_streak
            );
        }
    }
    let _ = writeln!(out);

    // Focus
    let _ = writeln!(out, "## Focus");
    let total = total_minutes(snap.focus_sessions);
    let hours = total / 60;
    let mins = total % 60;
    let _ = writeln!(
        out,
        "{} sessions, {}h {}m total",
        snap.focus_sessions.len(),
        hours,
        mins
    );
    let _ = writeln!(out);

    // Goals
    let _ = writeln!(out, "## Goals");
    if snap.goals.is_empty() {
        let _ = writeln!(out, "_No open goals._");
    } else {
        for (g, done, total) in snap.goals {
            if *total == 0 {
                let _ = writeln!(out, "- **{}** — no milestones", g.name);
            } else {
                let pct = (done * 100) / total;
                let _ = writeln!(
                    out,
                    "- **{}** — {}/{} milestones ({}%)",
                    g.name, done, total, pct
                );
            }
        }
    }
    let _ = writeln!(out);

    // Journals
    let _ = writeln!(out, "## Journal");
    let _ = writeln!(out, "{} entries this week.", snap.journals.len());
    for j in snap.journals {
        let snippet: String = j.body.chars().take(80).collect();
        let _ = writeln!(out, "- {} — {}", j.on_date, snippet.replace('\n', " "));
    }
    let _ = writeln!(out);

    // Reflections this week
    let _ = writeln!(out, "## Reflections");
    let _ = writeln!(out, "{} daily reflection(s) recorded.", snap.dailies.len());
    let _ = writeln!(out);

    let _ = writeln!(out, "## AI Summary");
    let _ = writeln!(
        out,
        "_Add a configured AI backend in `config.toml` to populate this section._"
    );

    out
}

pub fn monthly(snap: &MonthlySnapshot<'_>) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Monthly Review — {}-{:02}\n",
        snap.month_start.year(),
        snap.month_start.month() as u8
    );

    let _ = writeln!(out, "## Habits");
    if snap.habits.is_empty() {
        let _ = writeln!(out, "_No habits tracked yet._");
    } else {
        for (h, logs) in snap.habits {
            let s = compute_stats(snap.today, logs);
            let done_in_month = logs
                .iter()
                .filter(|l| l.on_date.as_str() >= snap.month_start.to_string().as_str())
                .filter(|l| l.status == "done")
                .count();
            let _ = writeln!(
                out,
                "- **{}** — {} done this month (current {}, best {})",
                h.name, done_in_month, s.current_streak, s.longest_streak
            );
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Focus");
    let total = total_minutes(snap.focus_sessions);
    let _ = writeln!(
        out,
        "{} sessions, {}h {}m total",
        snap.focus_sessions.len(),
        total / 60,
        total % 60
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Goals");
    if snap.goals.is_empty() {
        let _ = writeln!(out, "_No open goals._");
    } else {
        for (g, done, total) in snap.goals {
            if *total == 0 {
                let _ = writeln!(out, "- **{}** — no milestones", g.name);
            } else {
                let pct = (done * 100) / total;
                let _ = writeln!(
                    out,
                    "- **{}** — {}/{} milestones ({}%)",
                    g.name, done, total, pct
                );
            }
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Journal");
    let _ = writeln!(out, "{} entries this month.", snap.journals.len());
    let _ = writeln!(out);

    let _ = writeln!(out, "## Reflections");
    let _ = writeln!(out, "{} daily reflection(s) recorded.", snap.dailies.len());
    let _ = writeln!(out);

    let _ = writeln!(out, "## AI Summary");
    let _ = writeln!(
        out,
        "_Add a configured AI backend in `config.toml` to populate this section._"
    );

    out
}

#[derive(Debug, Clone)]
pub struct Insights {
    pub total_focus_hours: i64,
    pub most_active_hour_utc: Option<u8>,
    pub longest_habit_streak: Option<(String, u32)>,
    pub stale_goals: Vec<String>,
}

/// Compute insights from raw data. Stale = open goal with no milestone activity
/// recorded ever, and created >14 days ago.
pub fn insights(
    today: Date,
    habits: &[(Habit, Vec<HabitLog>)],
    focus_sessions: &[FocusSession],
    goals: &[Goal],
    goal_progress: &[(i64, u32)], // (goal_id, completed_milestone_count)
) -> Insights {
    let total_focus_hours = total_minutes(focus_sessions) / 60;

    let mut hour_counts = [0u32; 24];
    for s in focus_sessions {
        // start_at is ISO-8601 UTC like "2026-06-05T14:30:00.000Z" — hour is at [11..13].
        if let Some(hour_str) = s.start_at.get(11..13)
            && let Ok(hour) = hour_str.parse::<usize>()
            && hour < 24
        {
            hour_counts[hour] += 1;
        }
    }
    let most_active_hour_utc = hour_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| **c)
        .filter(|(_, c)| **c > 0)
        .map(|(i, _)| i as u8);

    let longest_habit_streak = habits
        .iter()
        .map(|(h, logs)| {
            let s = compute_stats(today, logs);
            (h.name.clone(), s.longest_streak)
        })
        .max_by_key(|(_, s)| *s)
        .filter(|(_, s)| *s > 0);

    let stale_threshold_days = 14;
    let mut stale_goals = Vec::new();
    for g in goals {
        if g.status != "open" {
            continue;
        }
        let any_done = goal_progress
            .iter()
            .find(|(gid, _)| *gid == g.id)
            .map(|(_, done)| *done > 0)
            .unwrap_or(false);
        if any_done {
            continue;
        }
        if let Ok(created) = time::OffsetDateTime::parse(
            &g.created_at,
            &time::format_description::well_known::Iso8601::DEFAULT,
        ) {
            let age_days = (today - created.date()).whole_days();
            if age_days > stale_threshold_days {
                stale_goals.push(g.name.clone());
            }
        }
    }

    Insights {
        total_focus_hours,
        most_active_hour_utc,
        longest_habit_streak,
        stale_goals,
    }
}

pub fn render_insights(i: &Insights) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Life Insights\n");
    let _ = writeln!(out, "- Total focus tracked: **{}h**", i.total_focus_hours);
    match i.most_active_hour_utc {
        Some(h) => {
            let _ = writeln!(
                out,
                "- Most active hour (UTC): **{:02}:00** — peak focus window",
                h
            );
        }
        None => {
            let _ = writeln!(out, "- Most active hour: _not enough data_");
        }
    }
    match &i.longest_habit_streak {
        Some((name, n)) => {
            let _ = writeln!(out, "- Longest habit streak: **{}** at {} days", name, n);
        }
        None => {
            let _ = writeln!(out, "- Longest habit streak: _no streaks yet_");
        }
    }
    if i.stale_goals.is_empty() {
        let _ = writeln!(out, "- Stale goals (>14d without progress): _none_");
    } else {
        let _ = writeln!(
            out,
            "- Stale goals (>14d without progress): **{}**",
            i.stale_goals.join(", ")
        );
    }
    out
}
