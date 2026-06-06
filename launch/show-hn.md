# Show HN draft

A pre-launch draft of the HN post for HabitOS v0.1.0. Edit freely before posting.

## Title

```
Show HN: HabitOS – local-first AI habit tracker in your terminal
```

## Body

```
This is a v0.1. I'm posting now to learn what doesn't make sense before
committing more time. If you install it and bounce, an "I bounced because X"
issue is the single most useful thing you can leave behind:
https://github.com/av-feaster/habitos/issues/new?template=feedback.yml

HabitOS is a single Rust binary that runs habits, goals, focus timer, journal,
reflections, and AI coaching in your terminal — and stores everything in a
local SQLite file. No cloud, no telemetry, AI is optional.

I built it because my workflow was spread across 6 tools — Streaks on iOS for
habits, Toggl for focus, Day One for journaling, Notion for goals, Claude.ai
for planning — and none of them talked to each other. So switching tools was
the friction, not the tracking.

What's in V1:

- `habitos log "did DSA for 45min"` — one-line capture that infers the habit,
  parses "45min", logs a focus session, and appends a timestamped journal note
- Habit streaks with a weekly freeze (one missed day per week is forgiven)
- Auto-detected milestones (🌱 7-day, 🔥 30-day, 💯 100-day…)
- Year-style heatmap
- Immersive ratatui TUI with a daily score (0–100) and live focus timer
- AI plan / coach / "ask my journal" via Claude, Ollama, or any
  OpenAI-compatible backend — every command degrades to a deterministic
  fallback if no backend is configured
- Smart 9pm nudge via launchd: "Workout pending — your 5-day streak ends
  if you skip today" (loss-aversion framing)
- Markdown + CSV export

Try it:

  brew install av-feaster/tap/habitos
  habitos init

Honest limits I want to flag:

1. No mobile companion. People log on phones; without that, my market is
   "devs who live at a terminal." Working on a read-only PWA next.
2. AI features (`plan`, `coach`, `ask`) cold-start badly — they need
   months of journal data before they return anything insightful.
3. Single-device by default. The README has a recipe for putting the data
   dir in iCloud/Dropbox/Syncthing, which works fine for one user across
   machines, but it's not real sync.

Stack: Rust 2024 edition, sqlx + SQLite, ratatui + crossterm, reqwest with
rustls. ~37 unit tests. Embeddings stored as raw f32 BLOBs with in-memory
cosine — fast enough up to ~10k entries; sqlite-vec is queued for when
that bites.

Source: https://github.com/av-feaster/habitos
License: MIT

Feedback I'd love:
- Where does the TUI feel wrong on your terminal/font/theme?
- Is the streak freeze a good idea or does it cheapen the streak?
- For the AI cold-start problem — is there a smart way to be useful
  before the user has 60 days of journals?
```

---

## Pre-launch checklist

- [ ] Record the asciinema demo (see `../screencast.md`) and prepend a single-line link at the top of the body: `[Demo: https://asciinema.org/a/...]`. This is the #1 conversion lift.
- [ ] Verify `brew install av-feaster/tap/habitos` works on a clean macOS user — at minimum, run it from a fresh shell with `PATH` pointing at `/opt/homebrew/bin`.
- [ ] Set repo topics for search discoverability:
      `gh repo edit av-feaster/habitos --add-topic habits,cli,rust,tui,terminal,productivity,ai,local-first`
- [ ] Add a GitHub social-preview image (Settings → Social preview). A clean PNG of the TUI works.
- [ ] Decide whether to post under your account or anonymously (HN allows throwaways).
- [ ] Have a `What's next` reply ready — HN's first comment usually asks about the roadmap.

## Timing

- **Best window:** Tuesday or Wednesday, 7–9am Pacific (10am–noon Eastern, ~3–5pm London).
- **Worst window:** Friday afternoon, weekends, and the hour around 12pm Pacific (lunch lull).
- HN front page is essentially won or lost in the first 90 minutes after posting.

## After it goes up

- Stay at the keyboard for 2 hours and answer every comment. HN rewards engaged founders.
- Don't crosspost simultaneously. Wait until the HN trajectory is clear (typically the 2-hour mark), then drop a Lobsters submission and one tweet with the asciinema GIF embedded.
- If it falls off the front page before getting traction, do not repost the same submission. Wait two weeks minimum and reframe the title (e.g., lead with a specific feature rather than the product name).

## Reply templates

A few pre-written responses for common HN comments — copy, edit, and post.

### "How is this different from habitctl / streak / X?"

> `habitctl` is great if you only need habits. HabitOS bundles habits + focus
> timer + journal + AI coaching in one binary so I can do my daily logging in
> one command (`habitos log "did X for Nmin"`). It also stores everything in a
> single SQLite file so I can grep, back up, or sync it however I want. If you
> just want habits in your terminal, `habitctl` is lighter.

### "Why do I want AI for habits?"

> You don't, for habit tracking itself. The AI features are for the
> end-of-week reflection and `habitos ask "when was I focused on X?"` queries
> over months of journal entries. If you don't write journals, ignore the AI
> half — every command works without it.

### "Mobile?"

> No mobile yet, and that's the biggest gap. A read-only PWA that reads the
> SQLite file is the V1.1 plan. For now, the sync section of the README
> shows how to put the data dir in iCloud/Dropbox so at least your laptop +
> desktop stay in sync. iOS app is a real consideration but I'm not there yet.

### "Why Rust?"

> Single static binary, no runtime, <10ms startup, and the embedded SQLite +
> sqlx story is excellent. ratatui is the killer TUI ecosystem. Could have
> been Go; Rust just won my coin flip.
