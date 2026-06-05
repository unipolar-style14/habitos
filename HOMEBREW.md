# Distributing HabitOS via Homebrew

A self-contained playbook to ship `brew install av-feaster/tap/habitos`. Three pieces: a GitHub Release with binaries, a separate `homebrew-tap` repo, and a Formula that points at them.

## Step 1 — Create the tap repo on GitHub

Homebrew taps must be named `homebrew-<name>` and live under your user. Create it once:

```bash
gh auth login                                      # if your token expired
gh repo create av-feaster/homebrew-tap --public --description "Homebrew tap for av-feaster's tools"
```

Clone it locally somewhere outside the HabitOS repo:

```bash
git clone git@github.com:av-feaster/homebrew-tap.git ~/code/homebrew-tap
mkdir -p ~/code/homebrew-tap/Formula
```

## Step 2 — Cut a HabitOS release

The release workflow in this repo (`.github/workflows/release.yml`) triggers on any `v*` tag push and builds release binaries for:

- `aarch64-apple-darwin` (Apple Silicon)
- `x86_64-apple-darwin` (Intel Mac)
- `x86_64-unknown-linux-gnu` (Linux)

Cut v0.1.0:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Watch the workflow:

```bash
gh run watch
```

When it finishes there's a GitHub Release at `https://github.com/av-feaster/habitos/releases/tag/v0.1.0` with three tarballs and their `.sha256` files.

## Step 3 — Grab the sha256 sums

```bash
gh release view v0.1.0 --json assets --jq '.assets[].name'
# Download the .sha256 files:
gh release download v0.1.0 --pattern '*.sha256' --dir /tmp/habitos-sha256
cat /tmp/habitos-sha256/*.sha256
```

You get three checksums like:

```
abc123…  habitos-v0.1.0-aarch64-apple-darwin.tar.gz
def456…  habitos-v0.1.0-x86_64-apple-darwin.tar.gz
789abc…  habitos-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

## Step 4 — Write the Formula

Copy the template below into `~/code/homebrew-tap/Formula/habitos.rb` and replace the three `REPLACE_WITH_SHA256_…` placeholders with the sums from step 3.

```ruby
class Habitos < Formula
  desc "Local-first AI-powered terminal OS for personal execution"
  homepage "https://github.com/av-feaster/habitos"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/av-feaster/habitos/releases/download/v#{version}/habitos-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_AARCH64_APPLE_DARWIN"
    end

    on_intel do
      url "https://github.com/av-feaster/habitos/releases/download/v#{version}/habitos-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/av-feaster/habitos/releases/download/v#{version}/habitos-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_X86_64_UNKNOWN_LINUX_GNU"
    end
  end

  def install
    bin.install "habitos"
  end

  test do
    assert_match "habitos #{version}", shell_output("#{bin}/habitos --version")
  end
end
```

## Step 5 — Push the tap

```bash
cd ~/code/homebrew-tap
git add Formula/habitos.rb
git commit -m "Add habitos 0.1.0"
git push
```

## Step 6 — Verify the install path

On a clean machine (or after `brew uninstall habitos`):

```bash
brew tap av-feaster/tap
brew install habitos
habitos --version           # → habitos 0.1.0
habitos init                # the onboarding wizard
```

Once that works, you can announce `brew install av-feaster/tap/habitos` in the README and on Show HN.

## Bumping for future releases

For v0.1.1 (or any later tag):

1. Tag and push from the HabitOS repo (`git tag v0.1.1 && git push origin v0.1.1`). The workflow rebuilds binaries.
2. Grab the new sha256 sums (`gh release download v0.1.1 --pattern '*.sha256'`).
3. In `homebrew-tap/Formula/habitos.rb`: bump `version`, update all three `sha256` lines, commit, push.

That's it. `brew upgrade habitos` will then pick it up.

## Why a tap (not homebrew-core)

HabitOS at <1k users isn't a fit for `homebrew-core` (which has strict popularity gates). A user-scoped tap is the canonical pre-mainstream distribution path and works identically from the install side: `brew install av-feaster/tap/habitos`.

When usage grows past the homebrew-core inclusion threshold (typically 75+ GitHub stars + active maintenance), you can submit a PR to homebrew-core and drop the tap requirement.
