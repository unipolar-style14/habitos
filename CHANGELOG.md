# Changelog

All notable changes to HabitOS. Versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Added
- `habitos init` — interactive first-run wizard. Creates a starter habit, logs today, optionally connects an AI backend.

---

## [0.1.0] — 2026-06-06

The first public release. Everything below shipped together as V1.0.

### Foundation
- Cargo workspace: `habitos-cli` (binary), `habitos-core` (domain + persistence), `habitos-ai` (LLM clients).
- SQLite via `sqlx` with embedded forward-only migrations.
- XDG-style data directory; `HABITOS_HOME` override for testing.
- TOML configuration at `~/Library/Application Support/habitos/config.toml`.
- `tracing` for opt-in structured logging.
- Single statically-linked binary (~9 MB) installable via `cargo install`.

### Habits
- `habit add / list / rm / done / skip / stats`.
- Idempotent same-day logging.
- `--at YYYY-MM-DD` backfill, `--note` attachment.
- **Streak freeze** — one missed day per ISO week is forgiven; second miss breaks. `❄N` indicator in stats.
- 30-day completion rate and missed-day count.

### Goals
- `goal add / list / progress / complete`.
- Milestones via `goal milestone add / done / list`.
- Progress = completed milestones / total.

### Focus sessions
- `focus start --project X --note Y / stop / status`.
- Single-active enforcement.
- Daily and weekly totals.

### Journal
- `journal new` (opens `$EDITOR`), `today`, `search`.
- One entry per day (upsert).
- Substring search.

### Reflections + reviews
- `reflect` — 4-question prompt, saves to `daily_reviews`.
- `review day / week / month` — templated reports with stub `## AI Summary` section.

### Reports + insights
- `insights` — most-active hour, longest streak across habits, stale goals (>14d without progress).

### AI integration
- `LlmClient` trait + three backends: **Ollama**, **OpenAI-compatible**, **Anthropic Claude**.
- `habitos connect claude <key> / ollama <model> / gemma / qwen / status / off`.
- Auto-pulls Ollama models on first connect; pings backend to verify.
- Prompts as markdown files (`habitos-ai/prompts/`) — overrideable via the data dir without recompiling.
- Every AI command has a deterministic fallback when no backend is configured.
- `habitos plan` and `habitos coach` use the configured backend or print deterministic context.
- `ai check` does a 1-token completion probe.

### Long-term memory
- `0002_ai_memory_embeddings.sql` adds `embedding BLOB` + `model TEXT` to `ai_memories`.
- Embeddings stored as raw little-endian `f32` bytes; cosine similarity computed in Rust.
- Auto-backfill on first `habitos ask` call.
- `habitos ask "<query>"` does embed → top-k retrieval → LLM synthesis with date citations.
- Requires an embeddings-capable backend (Ollama works; Anthropic does not provide embeddings).

### One-line capture
- `habitos log "did DSA for 45min"` — heuristic parser identifies habit, action (done/skipped), and duration. Writes habit log + back-dated focus session + timestamped journal note in one shot.

### Heatmap
- `habitos heatmap [--days N]` — GitHub-contributions-style 53-week grid with shaded blocks.

### Smart nudge
- `habitos nudge` — picks the at-risk habit (longest streak not yet logged today) and frames it as a loss.
- `habitos nudge --notify` — macOS Notification Center via osascript. Used by a launchd job at 21:00.

### Auto-detected milestones
- Celebration line printed on `habitos log` when a streak crosses 7 / 30 / 60 / 100 / 365 days.

### Immersive TUI
- `habitos` with no args opens a `ratatui` dashboard.
- Two-column layout: today's habits + focus session / open goals + journal preview.
- **Daily score** (0–100) in the header, colored by tier.
- Live focus-duration ticker.
- Themes: `system` (default, uses terminal palette), `vivid` (true-color), `mono` (no color).
- Keys: `↑↓` / `jk` move, `d` done, `s` skip, `f` focus toggle, `e` journal in `$EDITOR`, `r` refresh, `?` help, `q` quit.

### Export
- `habitos export markdown` — single doc to stdout.
- `habitos export csv <dir>` — one CSV per entity (habits, habit_logs, goals, focus_sessions, journal_entries, daily_reviews).

### Audit trail
- `event_log` populated on habit / focus / journal / reflection mutations.

### Quality
- 36 unit tests covering streak math, capture parser, heatmap intensity, milestone thresholds, cosine + top-k retrieval, config round-trip, migration completeness.
- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` all clean.
- Startup time under 10 ms (target was <100 ms).
