# HabitOS PRD

## AI-Powered Terminal Operating System for Personal Execution

### Version

2.0

### Status

V1.0 shipped — living document, updated as the product evolves

### Owner

Founder

---

## Document conventions

Sections marked **✅ Shipped** are live in the V1 binary. Sections marked **⏳ In flight** are queued for the next minor release. Sections marked **⚪ Considered** are deferred.

---

# 1. Vision

HabitOS is a local-first AI-powered terminal application that helps users plan, execute, reflect, and improve their lives.

Unlike traditional habit trackers that only record completion, HabitOS acts as an AI Chief of Staff.

It combines:

* Habit tracking
* Goal management
* Daily planning
* Focus tracking
* Journaling
* AI coaching
* Long-term personal memory

into a single terminal-first experience.

The product should feel like:

> "An operating system for running your life."

---

# 2. Problem Statement

Current tools are fragmented:

* Todo apps manage tasks
* Habit apps track streaks
* Calendars manage schedules
* Journals capture thoughts
* AI tools provide isolated advice

Users constantly switch between systems.

There is no unified system that:

* Understands goals
* Tracks habits
* Learns behavior
* Provides accountability
* Operates locally
* Works from the terminal

HabitOS solves this problem.

---

# 3. Target Users

## Primary

Developers, Engineers, Founders, Indie Hackers, Technical Professionals

## Secondary

Students, Researchers, Writers, Knowledge Workers

---

# 4. Product Principles

### Local First ✅

All data remains on the user's machine. SQLite at `~/Library/Application Support/habitos/habitos.db`.

### Offline First ✅

Core functionality (habits, goals, focus, journal, reviews, export) works without internet. AI commands degrade gracefully when no backend is configured or reachable.

### AI Native ✅

AI is embedded into workflows — `plan`, `coach`, `ask`, weekly review summary. Always optional, never required.

### Terminal First ✅

CLI is the primary interface. The immersive ratatui TUI is the default when invoked with no arguments.

### Privacy Focused ✅

No telemetry, no cloud dependency, no third-party tracking. Even AI calls go directly to the user-configured backend (Ollama by default = fully local).

---

# 5. Core Features

## 5.1 Habit Tracking ✅

Users can create habits.

Commands:

```bash
habitos habit add Workout
habitos habit done Workout
habitos habit skip Workout
habitos habit done Workout --at 2026-06-04 --note "morning run"
habitos habit list
habitos habit rm Workout
habitos habit stats
```

Tracks:

* Daily completion
* Current streak (with weekly freeze)
* Longest streak
* 30-day completion rate
* Missed days

**Streak rules:** `done` extends, `skipped` is neutral, missing days are forgiven up to **one per ISO week** (streak freeze).

---

## 5.2 Goal Management ✅

Goals with milestones:

```bash
habitos goal add "Ship HabitOS V1"
habitos goal list
habitos goal milestone add "Ship HabitOS V1" "M0 scaffold"
habitos goal milestone done "Ship HabitOS V1" "M0 scaffold"
habitos goal milestone list "Ship HabitOS V1"
habitos goal progress
habitos goal complete "Ship HabitOS V1"
```

Progress = completed milestones / total milestones.

---

## 5.3 Daily Planning ✅

```bash
habitos plan
```

Inputs: open goals, today's habit status, recent focus, recent journals.

Output: AI-generated agenda when a backend is configured; deterministic context dump otherwise.

---

## 5.4 Focus Sessions ✅

```bash
habitos focus start --project hostops --note "API design"
habitos focus stop
habitos focus status
```

Tracks start, end, duration, project, notes. Refuses to start a second session while one is active.

Metrics: daily focus hours, weekly focus hours, most-active hour (in `insights`).

---

## 5.5 Journaling ✅

```bash
habitos journal new            # opens $EDITOR
habitos journal today
habitos journal search "M0"
```

One entry per day (upsert). Substring search.

---

## 5.6 Reflection System ✅

```bash
habitos reflect
```

Prompts the four PRD questions, saves to `daily_reviews`.

---

## 5.7 Review Engine ✅

```bash
habitos review day
habitos review week
habitos review month
```

Templated deterministic report. When AI is configured, the `## AI Summary` section is populated by Claude / Ollama / etc.

---

## 5.8 One-line Capture ✅ *(added post-PRD)*

```bash
habitos log "did DSA for 45min"
habitos log "skipped workout, too tired"
habitos log "1 hour of reading"
```

A heuristic parser identifies the habit, the action (done/skipped), and the duration, then writes the habit log + a back-dated focus session + a timestamped journal note in one shot.

---

## 5.9 Heatmap ✅ *(added post-PRD)*

```bash
habitos heatmap --days 365
```

GitHub-contributions-style 53-week grid showing daily habit-done density. Intensities `░ ▒ ▓ █`.

---

## 5.10 Smart Nudge ✅ *(added post-PRD)*

```bash
habitos nudge
habitos nudge --notify   # macOS Notification Center
```

Picks the at-risk habit (longest streak that's not yet logged today) and surfaces it with loss-aversion framing. Used by the launchd 21:00 daily reminder.

---

## 5.11 Export ✅

```bash
habitos export markdown
habitos export csv <dir>
```

Single markdown doc or one CSV per entity (habits, habit_logs, goals, focus_sessions, journal_entries, daily_reviews).

---

# 6. AI System

## Overview ✅

AI serves as Coach, Planner, Analyst, Accountability Partner.

---

## Supported Backends ✅

* **Anthropic Claude** (`claude-opus-4-7`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`)
* **Ollama** (any local model: Gemma, Qwen, Llama, Mistral, DeepSeek, …)
* **OpenAI-compatible** (LM Studio, llama.cpp servers, vLLM, etc.)

Switch backends with:

```bash
habitos connect claude sk-ant-...
habitos connect ollama gemma2:2b
habitos connect gemma
habitos connect qwen
habitos connect status
habitos connect off
```

`connect ollama` auto-detects the `ollama` CLI, pulls the model if missing, and runs a ping to verify.

---

# 7. AI Features

## Daily Planner ✅

`habitos plan` — priorities, schedule, risks.

## Coach ✅

`habitos coach` — patterns, recommendation, next step.

## Weekly Review Summary ✅

`habitos review week` — deterministic report + AI synthesis.

## Life Insights ✅

`habitos insights` — most-active hour, longest streak, stale goals, total focus hours. **Currently deterministic only;** AI-augmented insights are V1.1.

---

# 8. Long-Term Memory ✅

Embeddings stored as raw `f32` BLOB in `ai_memories`. In-memory cosine similarity for retrieval (sufficient up to ~10k entries; swap point for `sqlite-vec` is documented in `0002_ai_memory_embeddings.sql`).

```bash
habitos ask "what was I focused on in March?"
habitos ask "how consistent was my workout habit?"
```

Auto-backfills embeddings on first call. Requires an AI backend that exposes an embeddings endpoint (Ollama works; **Anthropic's API does not provide embeddings** — pair Claude with Ollama for `ask`).

---

# 9. Architecture ✅

```
habitos-cli/      Binary. clap subcommand tree + ratatui TUI.
habitos-core/     Domain types, sqlx persistence, pure-function business logic.
                  ├── habits, goals, focus, journal, reviews
                  ├── capture (one-line parser)
                  ├── heatmap, milestones, reports
                  ├── memory (embedding store + cosine retrieval)
                  └── events (audit log)
habitos-ai/       LlmClient trait + Ollama, OpenAI-compatible, Anthropic impls.
                  Prompts loaded from disk (overridable) or embedded defaults.
```

---

# 10. Technical Stack ✅

| Concern | Crate |
|---|---|
| Language | Rust (edition 2024) |
| CLI parsing | `clap` v4 |
| TUI | `ratatui` v0.29 + `crossterm` v0.28 |
| Storage | SQLite via `sqlx` v0.8 |
| Async runtime | `tokio` v1 |
| HTTP (AI) | `reqwest` v0.12 (rustls) |
| Time | `time` v0.3 |
| Errors | `thiserror` (libs) + `anyhow` (binary boundary) |
| Serialization | `serde` + `toml` + `serde_json` |
| XDG paths | `directories` v5 |
| Logging | `tracing` + `tracing-subscriber` |
| Temp files | `tempfile` |
| Embeddings | Native `f32` LE bytes in SQLite BLOB |

---

# 11. Database Schema ✅

Tables (in `0001_init.sql`):

`habits`, `habit_logs`, `goals`, `goal_milestones`, `focus_sessions`, `journal_entries`, `daily_reviews`, `weekly_reviews`, `monthly_reviews`, `ai_memories`, `settings`, `event_log`

Extensions (in `0002_ai_memory_embeddings.sql`):

* `ai_memories.embedding BLOB` (LE-encoded f32 vector)
* `ai_memories.model TEXT` (which model produced the embedding)
* `(kind, source_ref)` unique index for backfill idempotency

The `users` table from the original spec is intentionally deferred — V1 is single-user.

---

# 12. Plugin System ⚪

Deferred to V1.2+. Current `habitos export` covers the markdown + CSV cases without a plugin abstraction. We will design the plugin API after at least two concrete integrations exist (likely Obsidian + Calendar) so the API rhymes with real shapes.

Candidates queued:

* Git Plugin
* Calendar Plugin
* Obsidian Plugin
* Markdown Export ✅ (built-in)
* CSV Export ✅ (built-in)
* Notification Plugin ✅ (built-in via launchd)
* Telegram Plugin
* OpenClaw Plugin

---

# 13. Reporting ✅

**Daily** — habit completion, focus hours, journal preview, reflection summary, optional AI summary

**Weekly** — habit consistency (X/7), focus hours, goal milestone progress, journal count, optional AI summary

**Monthly** — same shape, scaled to the calendar month

---

# 14. Security ✅

* Local-only SQLite database
* No analytics, no telemetry, no third-party tracking, no cloud dependency
* AI API keys stored in `config.toml` (V1.1 will move to OS keychain via the `keyring` crate)

---

# 15. Success Metrics

### Product Metrics ✅

| Target | Actual |
|---|---|
| Startup time < 100ms | **<10ms** measured |
| AI response < 5s | Pass-through from backend; typically <3s on Claude Sonnet |
| SQLite queries < 50ms | All sub-ms at V1 scale |
| Memory usage < 150MB | ~10MB resident |
| Release binary size | 9.3MB |

### User Metrics ⏳

Not yet measured (no telemetry by design). Manual proxy: shell-startup nudge + 9pm notification both designed to drive daily engagement.

---

# 16. Future Roadmap

## V1.1 — Engagement polish ⏳

* Streak freeze ✅
* Auto-detected milestones ✅
* Heatmap ✅
* Daily score in TUI header ✅
* Smart nudge ✅
* One-line capture ✅
* Inline add-habit form in TUI
* Goal-mutation events in `event_log`
* Anthropic prompt caching
* Split AI config: completions backend ≠ embeddings backend
* OS keychain for API keys
* Pomodoro structure on `focus`

## V2 — TUI maturity ⚪

* Tabs for goals / reviews / insights
* Streaming AI output inside the TUI
* Calendar sync
* Mobile companion (read-only first)

## V3 — Multi-device ⚪

* Voice assistant
* Agent workflows
* Automated task execution
* Personal knowledge graph

## V4 — Team mode ⚪

* Multi-device sync
* Team mode
* Shared goals
* AI executive assistant

---

# Final Product Goal

HabitOS should become the terminal equivalent of:

* Notion
* Todoist
* Streaks
* RescueTime
* Rewind
* Motion

combined into a single local-first AI operating system that helps users consistently achieve long-term goals.
