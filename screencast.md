# Recording the HabitOS screencast

A 60-second asciinema demo for the README. Designed to show the killer flow — install → init → log → TUI → heatmap — in a single take, no editing.

## Why asciinema (not video)

- Embeds inline in the GitHub README and the asciinema player remains crisp at any zoom level.
- 1/100th the file size of a `.mp4`.
- Viewers can pause and copy commands directly from the player.
- Records terminal text, not pixels — `ratatui` colors render natively.

## One-time setup

```bash
brew install asciinema
asciinema auth                       # links to your asciinema.org account
```

Set up a clean recording terminal:

- iTerm or Alacritty, 100×30 size, dark theme.
- Hide your prompt's git branch / hostname clutter. Recommend:

  ```bash
  export PS1='\$ '
  ```

- Use a clean recording profile:

  ```bash
  export HABITOS_HOME=$(mktemp -d)        # fresh data dir for the demo
  unset HABITOS_NUDGED                    # disable the zshrc nudge
  clear
  ```

## The script

Paste the commands one at a time during the recording. Don't pre-type — viewers need to see the cadence.

### Pre-recording sanity check

```bash
which habitos                   # confirm ~/.cargo/bin/habitos
habitos --version
```

### Recording

```bash
asciinema rec habitos-demo.cast \
  --title "HabitOS: local-first AI terminal OS for personal execution" \
  --idle-time-limit 2
```

### The demo (60 seconds)

```bash
# (0–10s) Set up
habitos init
# Walk through:
#   Pick one habit:  DSA
#   Mark done?       y
#   Connect AI?      N
```

```bash
# (10–25s) One-line capture
habitos log "did DSA for 45min"
# Output:
#   ✓ DSA done, 45m focus, journal +1

habitos log "1 hour of reading"
# Output:
#   ✓ already logged, 60m focus, journal +1
#   (or fires a streak milestone if seeded)
```

```bash
# (25–40s) Streak + stats
habitos habit stats
# Show: DSA streak, completion %, freeze indicator if applicable
```

```bash
# (40–55s) The TUI
habitos
# Press `?` to show the help overlay
# Press any key to dismiss
# Press `q` to quit
```

```bash
# (55–60s) Heatmap teaser
habitos heatmap --days 30
```

```bash
# Stop recording
# Press Ctrl-D or Ctrl-C
```

### Upload + embed

```bash
asciinema upload habitos-demo.cast
# Copy the resulting URL, e.g. https://asciinema.org/a/681234
```

In `README.md`, replace the existing ASCII art block at the top with:

```markdown
[![HabitOS demo](https://asciinema.org/a/681234.svg)](https://asciinema.org/a/681234?autoplay=1&speed=1.2)
```

The SVG badge shows a preview frame; clicking opens the player at 1.2× speed.

## Pre-flight checklist before hitting record

- [ ] Terminal at exactly 100×30 (asciinema records the actual dimensions)
- [ ] Dark theme that renders `█▓▒░` cleanly (Solarized Dark or Tokyo Night work well)
- [ ] Font with good box-drawing characters (JetBrains Mono, Iosevka, Berkeley Mono)
- [ ] No background apps that emit notifications during recording
- [ ] `HABITOS_HOME` pointed at a fresh tmpdir so the demo starts from zero
- [ ] Prompt simplified to `$ ` (no git branch, no path, no hostname)
- [ ] Optional: seed a 6-day streak so the demo can show the 7-day milestone trigger:

  ```bash
  for d in $(seq -6 -1); do
    habitos habit done DSA --at "$(date -v"${d}d" +%Y-%m-%d)" > /dev/null
  done
  ```

## Variations worth recording

If you want multiple casts, do these as separate files:

| Cast | Focus | Length |
|---|---|---|
| `habitos-demo.cast` | The 60-second flagship | 60s |
| `habitos-tui.cast` | Just the TUI — open, log a habit with `d`, start focus with `f`, edit journal with `e` | 30s |
| `habitos-ai.cast` | `habitos connect claude ...` → `habitos plan` → `habitos coach` against real Claude output | 90s |
| `habitos-heatmap.cast` | Heatmap with a year of real data | 15s |

The flagship is the one to embed in the README. The others can live on the asciinema page or in a `screenshots/` directory linked from the README.

## After recording

1. Embed the asciinema badge in `README.md` (replace the static ASCII block).
2. Generate a still PNG from the cast for the GitHub social preview:

   ```bash
   asciinema gif habitos-demo.cast habitos-demo.gif --speed 2 --theme tango
   # then export the first frame as PNG for the social card
   ```

3. Add `screencast.md` to `.gitattributes` so it doesn't show up in GitHub's language statistics:

   ```
   *.cast linguist-generated=true
   screencast.md linguist-documentation=true
   ```

4. Tag the release: `git tag v0.1.0 && git push --tags`.
