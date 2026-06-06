-- Skills system: time-boxed learning curricula loaded from user-authored JSON.
--
-- Use cases the schema accommodates:
--   * DSA — 90 LeetCode problems in 45 days, 2/day + 1 revision
--   * Language vocab — 1000 words in 90 days, 12/day
--   * Music — chord progressions to drill
--   * Books — chapters / theorems / case studies
--
-- The CLI doesn't care what items are; only `external_id` + `title` are required
-- per item, with tags / difficulty / url as optional metadata.

CREATE TABLE skills (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    source_path TEXT,                                -- path to the JSON file the user authored
    pace        INTEGER NOT NULL DEFAULT 2,          -- new items per day
    revisions   INTEGER NOT NULL DEFAULT 1,          -- items to revise per day
    started_at  TEXT NOT NULL,                       -- YYYY-MM-DD local
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE skill_items (
    id              INTEGER PRIMARY KEY,
    skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    external_id     TEXT NOT NULL,                   -- the stable id from the user's JSON
    title           TEXT NOT NULL,
    description     TEXT,
    url             TEXT,
    tags            TEXT,                            -- comma-separated for V1 simplicity
    difficulty      TEXT,
    position        INTEGER NOT NULL,                -- ordering from the JSON; determines daily pick order
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'solved', 'skipped')),
    solved_at       TEXT,
    last_revised_at TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (skill_id, external_id)
);

-- "Give me the next N pending items in position order" — hot path on `skill today`.
CREATE INDEX skill_items_pending_position ON skill_items (skill_id, status, position);

-- "Give me solved items ordered by last-touched" — hot path for revision selection.
CREATE INDEX skill_items_revision_order ON skill_items (skill_id, status, last_revised_at, solved_at);
