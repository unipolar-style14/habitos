# HabitOS Roadmap

Derived from `PRD.md`. Ordered by dependency, not by priority within a layer. Each milestone names concrete deliverables so progress is unambiguous.

**Status:** V1.0 shipped. This roadmap now reads as a build log + forward plan.

---

## Guiding cuts (unchanged from V1 planning)

- **AI is optional at runtime, not at architecture.** Every AI-touching feature has a non-AI fallback (raw stats, templated prompts). Preserves the offline-first principle.
- **One feature end-to-end before two features half-built.** Each milestone produces a working slice users could in theory use.
- **No plugin system in V1.** Hardcode the integrations that matter; extract a plugin API only after two integrations exist and rhyme.
- **Single-user only in V1.** The `users` table from the PRD is deferred — the local DB is the user.

---

## Open questions — resolved

| Question | Decision |
|---|---|
| Vector extension | **Skipped sqlite-vec for V1.** Embeddings stored as LE `f32` bytes in BLOB; cosine sim in Rust. Sufficient at V1 scale. Swap point documented in `memory::top_k`. |
| AI as install-time dependency | **Standalone install.** `cargo install habitos` works without any AI backend. AI commands degrade gracefully. |
| Editor for journals | **`$EDITOR` only.** Spawns via `sh -c` so multi-word editor commands work. Inline TUI prompt deferred to V1.1. |
| Time handling | **UTC instants in DB; local calendar day for habit/journal dates.** A reflection logged at 2am still counts for the day it felt like. |
| Secrets storage | **API keys in plain `config.toml` for V1.** OS keychain (`keyring` crate) deferred to V1.1. |

---

## M0 — Foundation ✅

**Goal:** binary compiles, parses subcommands, opens SQLite, runs migrations.

- ✅ `cargo` workspace: `habitos-cli`, `habitos-core`, `habitos-ai`
- ✅ `clap` command tree mirroring every PRD subcommand
- ✅ XDG-style data dir + `HABITOS_HOME` override
- ✅ TOML config with defaults; `habitos config path`
- ✅ `sqlx` + SQLite + embedded migrations; `habitos db migrate`
- ✅ `tracing` for opt-in structured logs
- ✅ `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`

**Exit criteria:** `habitos --help` lists every PRD command; `habitos db migrate` creates the schema. **Met.**

---

## M1 — Habits ✅

- ✅ `habits` + `habit_logs` tables
- ✅ `habit add/list/rm/done/skip/stats`
- ✅ Idempotent `done` (same day twice = no-op)
- ✅ `--at <date>` backfill, `--note` attachment
- ✅ Streak math: current, longest, 30d completion, missed
- ✅ 9 unit tests on streak edge cases

**Exit criteria:** User can run HabitOS daily as a pure habit tracker. Streaks survive across runs. **Met.**

---

## M2 — Goals, Focus, Journal ✅

- ✅ **Goals:** add/list/progress/complete + milestone add/done/list
- ✅ **Focus:** start/stop/status with single-active enforcement
- ✅ **Journal:** new (via `$EDITOR`) / today / search (LIKE-based)

**Exit criteria:** all non-AI commands from PRD §5.1–5.5 work and persist. **Met.**

---

## M3 — Reviews + deterministic reports ✅

- ✅ `daily_reviews`, `weekly_reviews`, `monthly_reviews` tables
- ✅ `reflect` prompts 4 questions, stores answers
- ✅ `review day/week/month` renders templated reports with stub `## AI Summary` section
- ✅ `insights` (deterministic): most-active hour, longest streak, stale goals

**Exit criteria:** a user gets real value from `review week` with zero AI configured. **Met.**

---

## M4 — AI integration ✅

- ✅ `LlmClient` trait + `OllamaClient`, `OpenAiCompatibleClient`
- ✅ Prompts as markdown files under `habitos-ai/prompts/` with disk-override + embedded defaults
- ✅ `ai check` ping
- ✅ Wired AI into `plan`, `coach`
- ✅ Graceful degradation on every AI command

**Added during M4 / post-M4:**

- ✅ `AnthropicClient` (Anthropic Messages API)
- ✅ `habitos connect <backend>` for swap-in configuration

**Exit criteria:** with a backend running, every PRD AI command produces useful output. Without a backend, nothing crashes. **Met.**

---

## M5 — Long-term memory + `ask` ✅

- ✅ Migration `0002` adds `embedding BLOB` + `model TEXT` to `ai_memories`
- ✅ LE `f32` encoding/decoding
- ✅ Cosine similarity + top-k as pure functions (4 unit tests)
- ✅ Auto-backfill on first `ask` call
- ✅ `habitos ask` does embed-query → top-k → LLM synthesis with date citations

**Note:** Anthropic's API has no embeddings endpoint. `ask` requires an Ollama-style backend. Documented in error text and README.

**Exit criteria:** the three example PRD queries return grounded answers with date citations. **Met when paired with Ollama.**

---

## M6 — Polish + ship ✅

- ✅ `event_log` writes on habit/focus/journal/reflection mutations
- ✅ `habitos export markdown` / `habitos export csv <dir>`
- ✅ Performance pass against PRD targets (startup <10ms vs <100ms target; ~10MB RSS vs <150MB target)
- ✅ Binary installable via `cargo install --path habitos-cli`

**Deferred from M6:**

- Goal-mutation event_log writes (pattern is established; flagged in V1.1)
- `settings` k/v table — `config.toml` covers V1 needs
- `install.sh` / brew formula

---

## V1.1 — Engagement layer ✅ (shipped together)

Built after the M0–M6 plan once usage data suggested what drives engagement.

- ✅ **Streak freeze (1 per ISO week)** — removes all-or-nothing cliff. `❄N` indicator in `habitos habit stats`.
- ✅ **Auto-detected milestones** — celebration line on log when crossing 7/30/60/100/365 day streaks.
- ✅ **Smart 9pm nudge** — launchd job calls `habitos nudge --notify` with loss-aversion framing for the at-risk streak.
- ✅ **Year heatmap** — `habitos heatmap` GitHub-contributions-style grid.
- ✅ **Daily score in TUI header** — 0–100 composite, color-graded, with progress bar.
- ✅ **Immersive TUI** — ratatui dashboard (`habitos` with no args). Two-column layout: habits + focus / goals + journal. Themes: `system` / `vivid` / `mono`.
- ✅ **One-line capture (`habitos log`)** — heuristic parser maps free text to habit + focus minutes + journal note in one shot.
- ✅ **`habitos connect`** — switch AI backends from the CLI (Claude / Ollama / Gemma / Qwen shortcuts).

---

## V1.2 — Next batch ⏳

| Item | Why |
|---|---|
| Inline add-habit form in TUI | No more `q` → `habit add` → relaunch |
| Goal-mutation `event_log` writes | Complete the audit trail |
| Anthropic prompt caching | Cheaper / faster `coach` and `ask` syntheses |
| Split AI config: completions + embeddings backends | Run Claude for `coach` and Ollama for `ask` simultaneously |
| OS keychain for API keys | Get the `sk-ant-...` value out of `config.toml` |
| Pomodoro structure on `focus` | 25-min countdown, break alert, auto-stop |
| ASCII-fallback heatmap | For terminals that mangle `░▒▓█` |

---

## V2 — TUI maturity ⚪

- Tabs for goals / reviews / insights inside the TUI
- Streaming AI output (`plan`, `coach`) rendered live inside the TUI
- Calendar sync (read-only first, then bi-directional)
- Mobile companion (read-only first, daily summary)
- Notifications + scheduling beyond a fixed 21:00 launchd job

## V3 — Agentic + voice ⚪

- Voice input for `habitos log` and `habitos reflect`
- Agent workflows that act on stale goals / broken streaks
- Automated task execution from AI plan
- Personal knowledge graph derived from journal embeddings

## V4 — Team + sync ⚪

- Multi-device sync (CRDT-backed; SQLite + event_log makes this tractable)
- Team mode with shared goals
- AI executive assistant with calendar-level autonomy

---

## Build log notes

Decisions made during implementation that diverge from the original M-plan:

1. **Rust edition bumped 2021 → 2024.** Needed for `let-chains` in retrieval / parser code. Stable as of 1.85.
2. **Anthropic added as a third backend** between M4 and M5 because the user wanted Claude integration. The `build_client` factory absorbed it in one match arm.
3. **`habitos connect` shipped as a config helper** because hand-editing TOML kept getting in the way during AI testing. It writes the `[ai]` block + auto-pulls Ollama models.
4. **TUI shipped during V1.1**, not V2, because the dashboard was the highest-leverage engagement primitive once daily-use friction became the bottleneck.
5. **`habitos log` shipped** as the single biggest friction reduction. Replaced multi-step daily logging with one line of natural language.
