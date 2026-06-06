# HabitOS

> A local-first, AI-optional terminal OS for running your life — habits, goals, focus, journal, reflection, and long-term memory in one immersive CLI.

[![asciicast](https://asciinema.org/a/yXRwfVD2KTqNLdqI.svg)](https://asciinema.org/a/yXRwfVD2KTqNLdqI?autoplay=1)

HabitOS lives where developers already are: the terminal. Everything is one binary, one SQLite file, and zero cloud calls. AI is opt-in — every feature has a deterministic fallback that works offline.

> **This is v0.1.** I'm shipping early on purpose to find out what's broken before committing more time. If you try it and it doesn't stick, [tell me why](https://github.com/av-feaster/habitos/issues/new?template=feedback.yml) — "I bounced because X" is the most valuable thing I can hear right now.

---

## Why HabitOS

Most personal-execution tools fall into one of two camps:

- **Web/mobile apps** (Notion, Todoist, Streaks): great UX, but data lives in someone else's cloud and you context-switch out of your editor to use them.
- **CLI tools** (jrnl, habitctl, taskwarrior): local-first, but each does one thing — you end up gluing five of them together.

HabitOS is the missing combination: **terminal-first, local-first, single-binary, AI-augmented when you want it**.

- **🔒 Local-first** — SQLite database in `~/Library/Application Support/habitos/`. Your data never leaves your machine.
- **⚡ Fast** — `<10ms` startup. The TUI redraws at 60fps.
- **🧠 AI-optional** — Configure Claude, Ollama, or any OpenAI-compatible backend in 1 line. Or don't, and use the deterministic mode.
- **🎯 Sticky core** — Habit streaks, focus timer, and friction-free journaling are the things you actually use daily. Everything else supports those.
- **📦 One binary** — `cargo install habitos` and you're done.

---

## Install

### Homebrew (macOS Apple Silicon / Intel, Linux x86_64)

```bash
brew install av-feaster/tap/habitos
habitos --version
```

### From source (any platform with Rust ≥ 1.85)

```bash
git clone git@github.com:av-feaster/habitos.git
cd habitos
cargo install --path habitos-cli
habitos --version
```

If `habitos` isn't found after a cargo install, ensure `~/.cargo/bin` is on your `PATH`:

```bash
echo '. "$HOME/.cargo/env"' >> ~/.zshrc
source ~/.zshrc
```

### Pre-built binaries

Tarballs for each platform are attached to every [GitHub Release](https://github.com/av-feaster/habitos/releases). Download, extract, and drop `habitos` into a directory on your `PATH`.

---

## Quick start

```bash
habitos init     # 30-second interactive setup
```

That walks you through creating a habit, logging it, and optionally connecting an AI backend. Or skip the wizard:

```bash
habitos habit add DSA
habitos habit add Workout

# Log today in one line — infers habit, duration, and saves a journal note
habitos log "did DSA for 45min"
# ✓ DSA done, 45m focus, journal +1
# 🌱 7-day streak on DSA — keep going.

# Launch the immersive dashboard
habitos
```

That's it. Add the alias `alias l='habitos log'` and your daily flow is `l "..."`.

---

## Core commands

| Command | What it does |
|---|---|
| `habitos` | Open the immersive TUI dashboard |
| `habitos init` | Interactive first-run setup |
| `habitos log "did X for Nmin"` | One-line capture: habit + focus + journal in one shot |
| `habitos habit add <name>` | Create a habit |
| `habitos habit done <name>` | Mark done today |
| `habitos habit stats` | Streaks, completion rate, freezes used |
| `habitos goal add <name>` | Create a goal |
| `habitos goal milestone add <goal> <name>` | Add a milestone |
| `habitos focus start --project X` | Start a focus session |
| `habitos focus stop` | End the session, record duration |
| `habitos journal new` | Open `$EDITOR` for today's entry |
| `habitos reflect` | Guided 4-question end-of-day reflection |
| `habitos review day\|week\|month` | Templated report from raw data |
| `habitos insights` | Quantitative patterns (most-active hour, stale goals, …) |
| `habitos heatmap` | GitHub-contributions-style year heatmap |
| `habitos nudge` | Smart, loss-aversion-framed pending reminder |
| `habitos ask "<question>"` | Semantic search over your journal (AI required) |
| `habitos plan` | AI-generated daily plan (Claude / Ollama) |
| `habitos coach` | AI coach: patterns, recommendation, next step |
| `habitos export markdown` | Single markdown doc to stdout |
| `habitos export csv <dir>` | One CSV per entity |

---

## The TUI

`habitos` (no args) opens a live two-column dashboard:

- **Left** — today's habits with streak counters (🔥 emoji from 3+ days, color-graded by length) and active focus session with live ticker
- **Right** — open goals with progress bars and today's journal preview
- **Header** — a 0–100 daily score (50% habits done + 30% focus hours capped at 4h + 10% journal + 10% reflection)
- **Footer** — keybinding hints

### Keybindings

| Key | Action |
|---|---|
| `↑` `↓` / `j` `k` | Move cursor through habits |
| `d` | Mark habit done |
| `s` | Mark habit skipped |
| `f` | Start / stop focus session |
| `e` | Open today's journal in `$EDITOR` (TUI suspends, resumes after save) |
| `r` | Refresh |
| `?` | Help overlay |
| `q` / `^C` | Quit |

### Themes

```bash
habitos --theme system    # default — uses your terminal's ANSI palette
habitos --theme vivid     # true-color cyan/green/amber
habitos --theme mono      # no color, only bold/dim
```

---

## AI integration

HabitOS supports three backends. Configure with one command:

```bash
# Anthropic Claude
habitos connect claude sk-ant-...

# Local Ollama (auto-pulls the model)
habitos connect ollama gemma2:2b
habitos connect ollama qwen2.5:3b
habitos connect ollama llama3.2:3b

# Shortcuts
habitos connect gemma         # default: gemma2:2b
habitos connect qwen          # default: qwen2.5:3b

# Inspect / clear
habitos connect status
habitos connect off
```

After connecting:

```bash
habitos ai check    # 1-token probe
habitos plan        # AI-generated plan
habitos coach       # AI coaching
habitos ask "what was I focused on in May?"
```

If no backend is configured, every command falls back to a deterministic version. `plan` prints raw context. `coach` shows the same. `ask` returns a clear error (needs embeddings).

**Note:** Anthropic's API doesn't provide embeddings, so `ask` requires Ollama. The other AI commands work fine on Claude.

---

## The engagement layer

A few features designed for daily stickiness:

### Streak freeze (1 per ISO week)
A single missed day per week is forgiven — no all-or-nothing cliff. The `❄` indicator in `habitos habit stats` shows when a freeze was consumed.

### Auto-detected milestones
At 7 / 30 / 60 / 100 / 365 day streaks, the next log triggers a celebration:

```
✓ DSA done, journal +1
🌱 7-day streak on DSA — keep going.
```

### Smart 9pm nudge (macOS)
A launchd job runs `habitos nudge --notify` at 21:00. Instead of a generic "log your habits", it picks the most at-risk streak:

> Workout pending — your 5-day streak ends if you skip today.

### Year heatmap

```bash
$ habitos heatmap
       Jul  Aug   Sep   Oct  Nov   Dec   Jan  Feb   Mar   Apr  May  Jun
  Mon  ░▒▓▓██▓▒░░░▒▒▓▓███▓▓▒░░▒▒▓████▓▓▒▒▒▓▓██▓▓▒▒░░▒▓██▒░░▒▓██▓▒░▓█▓░
  Tue  ▒▒▓▓██▓▒░▒▒▓▓███▓▓▒░░▒▒▓████▓▓▒▒▒▓▓██▓▓▒▒░░▒▓██▒░░▒▓██▓▒░▓█▓░░▒
  ...
  less  ░ ▒ ▓ █  more
```

### Terminal nudge on shell open

Add to `~/.zshrc`:

```bash
HABITOS_NUDGE_FILE="$HOME/.habitos_nudge"
habitos-nudge() { case "$1" in on|off) echo "$1" > "$HABITOS_NUDGE_FILE";; *) cat "$HABITOS_NUDGE_FILE" 2>/dev/null || echo on;; esac }
if [ -z "$HABITOS_NUDGED" ] && [ "$(cat "$HABITOS_NUDGE_FILE" 2>/dev/null || echo on)" = "on" ] && command -v habitos >/dev/null; then
  export HABITOS_NUDGED=1
  habitos habit stats
fi
```

Every new shell shows your streaks at a glance.

---

## Architecture

A 3-crate Rust workspace:

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

**Conventions:**

- All timestamps stored as ISO-8601 UTC. Calendar dates (habit logs, journal entries) stored as the user's local day — a reflection logged at 2am still counts for the day it felt like.
- `thiserror` in libraries, `anyhow` at the CLI boundary. No `unwrap()` outside tests.
- Migrations are forward-only.
- The `LlmClient` trait is depended on by every AI call site; concrete clients are constructed via `build_client(&AiBackendConfig)` so adding a new backend is one match arm.
- Embeddings stored as raw `f32` bytes in SQLite BLOB (cosine sim in Rust). At V1 scale this is faster than loading a vector extension. The retrieval function is the only swap point if you need `sqlite-vec`.

---

## Configuration

Config lives at `~/Library/Application Support/habitos/config.toml`. Show paths with:

```bash
habitos config path
```

Example:

```toml
[ai]
backend     = "anthropic"            # "ollama" | "openai-compatible" | "anthropic"
model       = "claude-sonnet-4-6"
endpoint    = "https://api.anthropic.com"
api_key     = "sk-ant-..."
timeout_secs = 30
```

Override the data directory with `HABITOS_HOME=/path` (useful for testing).

### Sync across machines (without code)

HabitOS is single-file SQLite. To use it across a laptop + desktop, point the data dir at a synced folder:

```bash
# iCloud Drive (macOS)
export HABITOS_HOME="$HOME/Library/Mobile Documents/com~apple~CloudDocs/habitos"

# Dropbox
export HABITOS_HOME="$HOME/Dropbox/habitos"

# Syncthing or any directory under it
export HABITOS_HOME="$HOME/Sync/habitos"
```

Add the export to `~/.zshrc` for persistence. SQLite handles concurrent reads cleanly; single-user writes won't collide as long as you're not running `habitos` on two machines at the exact same instant.

### Prompt overrides

AI prompts ship as embedded defaults but can be overridden without recompiling — drop a file into `~/Library/Application Support/habitos/prompts/`:

```
prompts/plan.md
prompts/coach.md
prompts/reflect_summary.md
prompts/review_week.md
prompts/ask.md
```

---

## A daily workflow that actually works

After using it for a while, this is the loop that sticks:

```bash
# Morning
habitos              # see streaks + score; plan the day
habitos plan         # if AI is on, get a Claude-generated agenda

# During the day
l "did DSA 45min"            # one-line capture, repeat as you go
habitos focus start --project hostops
# … deep work …
habitos focus stop

# Evening
habitos reflect      # 4 prompts, two minutes
habitos review day   # see the whole day at once
```

**Recommended:** track **2–3 habits max**. People who track ten habits stop tracking entirely.

---

## Roadmap

Shipped (V1):

- ✅ Habit / goal / focus / journal / review primitives
- ✅ Deterministic reports + insights
- ✅ AI integration (Claude / Ollama / OpenAI-compatible)
- ✅ Long-term memory + `ask`
- ✅ Export (markdown + CSV)
- ✅ Immersive ratatui TUI
- ✅ One-line capture (`log`)
- ✅ Smart nudge + auto-detected milestones
- ✅ Heatmap + streak freezes + daily score

In flight (V1.1):

- ⏳ Inline add-habit form in TUI
- ⏳ Split AI config: Claude for completions + Ollama for embeddings
- ⏳ Prompt caching for Anthropic
- ⏳ Goal-mutation `event_log` writes
- ⏳ Pomodoro structure on focus sessions

Considered (V2+):

- TUI tabs for goals/reviews/insights
- Live AI streaming inside the TUI
- Mobile companion + multi-device sync
- Plugin system (Obsidian, Calendar, Telegram)

---

## Tests

```bash
cargo test --workspace        # 37 unit tests
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```

Coverage focuses on the pure-function layers: streak math, capture parser, heatmap intensity, milestone thresholds, cosine + top-k retrieval, config round-trip. Persistence is exercised via an in-memory SQLite migration test.

---

## License

MIT.
