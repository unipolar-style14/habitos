-- HabitOS initial schema.
-- Conventions:
--   * Timestamps are ISO-8601 UTC TEXT (`strftime('%Y-%m-%dT%H:%M:%fZ', 'now')`).
--   * Dates (`on_date`, `week_starting`, `month`) are stored as the user's local calendar day,
--     since reviews and habits are bound to the day the user perceives, not to UTC midnight.
--   * Booleans are INTEGER (0|1).
--   * Enums are TEXT with explicit CHECK constraints.
--   * `PRAGMA foreign_keys = ON` is set at connection time (see db.rs).

CREATE TABLE habits (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    archived    INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE habit_logs (
    id          INTEGER PRIMARY KEY,
    habit_id    INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    on_date     TEXT NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('done', 'skipped')),
    note        TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (habit_id, on_date)
);
-- Serves streak/completion queries: latest N logs for a habit, newest first.
CREATE INDEX habit_logs_habit_date ON habit_logs (habit_id, on_date DESC);

CREATE TABLE goals (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    deadline    TEXT,
    status      TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'complete', 'abandoned')),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE goal_milestones (
    id           INTEGER PRIMARY KEY,
    goal_id      INTEGER NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    completed_at TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX goal_milestones_goal ON goal_milestones (goal_id);

CREATE TABLE focus_sessions (
    id          INTEGER PRIMARY KEY,
    start_at    TEXT NOT NULL,
    end_at      TEXT,
    project     TEXT,
    note        TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
-- Partial index serves the "is there an open session?" check on every `focus start`.
CREATE INDEX focus_sessions_open ON focus_sessions (end_at) WHERE end_at IS NULL;
-- Serves daily/weekly aggregates.
CREATE INDEX focus_sessions_start ON focus_sessions (start_at DESC);

CREATE TABLE journal_entries (
    id          INTEGER PRIMARY KEY,
    on_date     TEXT NOT NULL UNIQUE,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE daily_reviews (
    id                 INTEGER PRIMARY KEY,
    on_date            TEXT NOT NULL UNIQUE,
    went_well          TEXT,
    didnt_go_well      TEXT,
    learned            TEXT,
    tomorrow_priority  TEXT,
    ai_summary         TEXT,
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE weekly_reviews (
    id            INTEGER PRIMARY KEY,
    week_starting TEXT NOT NULL UNIQUE,
    wins          TEXT,
    failures      TEXT,
    trends        TEXT,
    next_actions  TEXT,
    ai_summary    TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE monthly_reviews (
    id                  INTEGER PRIMARY KEY,
    month               TEXT NOT NULL UNIQUE,
    performance         TEXT,
    productivity_score  INTEGER,
    improvement_areas   TEXT,
    ai_summary          TEXT,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE ai_memories (
    id          INTEGER PRIMARY KEY,
    kind        TEXT NOT NULL
                CHECK (kind IN ('journal', 'review', 'reflection', 'summary')),
    source_ref  TEXT NOT NULL,
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX ai_memories_kind ON ai_memories (kind);

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE event_log (
    id           INTEGER PRIMARY KEY,
    entity_type  TEXT NOT NULL,
    entity_id    INTEGER,
    action       TEXT NOT NULL,
    payload      TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX event_log_entity ON event_log (entity_type, entity_id);
CREATE INDEX event_log_created ON event_log (created_at DESC);
