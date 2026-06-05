# HabitOS Roadmap

Derived from `PRD.md`. Ordered by dependency, not by priority within a layer. Each milestone names concrete deliverables so progress is unambiguous.

---

## Guiding cuts

- **AI is optional at runtime, not at architecture.** Every AI-touching feature has a non-AI fallback (raw stats, templated prompts). This preserves the offline-first principle and lets V1 ship before any LLM glue is wired.
- **One feature end-to-end before two features half-built.** Each milestone produces a working slice users could in theory use.
- **No plugin system in V1.** Hardcode the integrations that matter; extract a plugin API only after two integrations exist and rhyme.
- **Single-user only in V1.** The `users` table from the PRD is deferred — the local DB is the user.

---

## M0 — Foundation (no user-visible features)

**Goal:** a binary that compiles, parses subcommands, opens a SQLite database in a known location, and runs migrations.

- `cargo` workspace: `habitos-cli`, `habitos-core` (domain + persistence), `habitos-ai` (stub).
- `clap` command tree mirroring PRD section 5 (subcommands can be `todo!()`).
- `XDG`-style data dir resolution (`~/.local/share/habitos/` on Linux, `~/Library/Application Support/habitos/` on macOS). Override via `HABITOS_HOME`.
- `toml` config file with defaults; `habitos config path` prints location.
- `sqlx` with SQLite, embedded migrations, `habitos db migrate` command.
- `tracing` for structured logs, off by default.
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.

**Exit criteria:** `habitos --help` lists every PRD command; `habitos db migrate` creates a populated, empty schema.

---

## M1 — Habits (vertical slice)

**Goal:** prove the CLI → domain → SQLite stack on one feature before scaling out.

- Tables: `habits`, `habit_logs`.
- Commands: `habit add`, `habit list`, `habit done`, `habit skip`, `habit rm`, `habit stats`.
- Streak math: current streak, longest streak, miss count, weekly completion rate.
- Idempotent `done` (same day twice = no-op, not duplicate row).
- `--at <date>` flag for backfilling.
- Integration tests against a temp SQLite file.

**Exit criteria:** a user can run HabitOS daily as a pure habit tracker. Streaks survive across runs.

---

## M2 — Goals, Focus, Journal

**Goal:** the remaining "raw data" features. Each follows the M1 pattern.

- **Goals:** `goals`, `goal_milestones` tables. Add/list/progress/complete commands. Percent-complete from milestones.
- **Focus:** `focus_sessions` table. `focus start [--project] [--note]`, `focus stop`, `focus status`, `focus today`. Refuse to start a second session while one is open.
- **Journal:** `journal_entries` table. `journal new` (opens `$EDITOR`), `journal today`, `journal show <date>`, `journal search <query>` (LIKE-based for now — semantic search is M5).

**Exit criteria:** all non-AI commands from PRD sections 5.1–5.5 work and persist.

---

## M3 — Reviews & raw reporting

**Goal:** reflection workflows and deterministic reports, no LLM yet.

- Tables: `daily_reviews`, `weekly_reviews`, `monthly_reviews`. Quarterly can reuse monthly with a range filter — defer dedicated table until needed.
- `reflect` command: prompts the four PRD questions, stores answers.
- `review day|week|month`: renders a templated report from raw data (habit consistency %, focus hours, goal deltas, journal counts, review answers). No AI summary yet — leave a clearly marked `## AI Summary` section as a stub.
- `insights` (deterministic version): most-active hours, longest streaks, stale goals (>14d no progress).

**Exit criteria:** a user gets real value from `review week` with zero AI configured.

---

## M4 — AI integration

**Goal:** swap the templated stubs for LLM output, behind a runtime check.

- `habitos-ai` crate: trait `LlmClient` with `complete(prompt) -> Result<String>` and `embed(text) -> Result<Vec<f32>>`.
- Implementations: Ollama (primary), generic OpenAI-compatible (covers LM Studio, llama.cpp servers).
- Config: model name, endpoint, timeout, optional API key. `habitos ai check` pings the backend.
- Prompt templates as separate `.md` files under `habitos-ai/prompts/`, loaded at runtime so tuning doesn't require recompilation.
- Wire AI into: `plan`, `coach`, `reflect` (summary), `review week` (AI Summary section).
- Graceful degradation: if the backend is unreachable, commands still print the deterministic report and a one-line warning.

**Exit criteria:** with Ollama running, every PRD AI command produces useful output. With Ollama off, nothing crashes.

---

## M5 — Long-term memory & `ask`

**Goal:** semantic search over journals, reviews, and reflections.

- Pick a SQLite vector solution — `sqlite-vec` is the current default; confirm before starting (see open questions).
- Backfill embeddings for existing journal/review/reflection rows on first run.
- Index new entries on write.
- `habitos ask "<query>"`: top-k retrieval → LLM synthesis with citations back to the source rows/dates.
- `ai_memories` table for AI-generated summaries that should be retrievable later.

**Exit criteria:** the three example PRD queries ("What was I focused on in March?", etc.) return grounded answers with date citations.

---

## M6 — Polish for v1.0

**Goal:** ship-ready.

- `event_log` table populated for every mutating command (cheap audit trail; also the foundation for future sync).
- `settings` table for runtime-mutable prefs (separate from on-disk `toml`).
- `habitos export --format md|csv` (covers the markdown + CSV plugins from the PRD without needing a plugin system).
- Performance pass against the PRD targets: startup <100ms, queries <50ms, RSS <150MB.
- `install.sh` + `brew` formula or pre-built binaries for macOS/Linux.
- Docs: README, `habitos --help` examples, a 5-minute getting-started.

**Exit criteria:** a stranger can install HabitOS, run it daily for a week, and not file a usability bug.

---

## Post-1.0 (explicitly out of V1)

| Capability | Earliest milestone |
|---|---|
| Plugin system (dynamic load or subprocess protocol) | V1.1 — needs ≥2 concrete integrations to design against |
| Calendar / Obsidian / Telegram integrations | V1.1, as plugins |
| TUI dashboard | V2 |
| Notifications / scheduling | V2 |
| Voice, agent workflows, multi-device sync, team mode | V3+ |

---

## Open questions to resolve before M0

1. **Vector extension choice.** `sqlite-vec` (newer, simpler) vs `sqlite-vss` (older, requires faiss). Affects build complexity and distribution.
2. **AI as install-time dependency?** Should `cargo install habitos` work standalone, or do we expect the user to install Ollama first? Probably standalone with AI commands as a clearly-gated capability.
3. **Editor for journals.** `$EDITOR` only, or also a built-in TUI prompt? `$EDITOR` is simpler; TUI is friendlier but pushes work into M6.
4. **Time handling.** Local time everywhere (matches "personal" framing) vs UTC in DB with local rendering (correct for travelers). Recommend UTC in DB.
5. **Encryption of "secrets" (PRD §14).** What's actually secret in V1? Likely just the API key for OpenAI-compatible backends — OS keychain is sufficient, no need to encrypt the DB itself yet.

Answering these unblocks M0.
