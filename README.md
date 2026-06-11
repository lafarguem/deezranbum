# deezranbum

Picks a random album from your Deezer library and adds it to your queue, without repeating albums across sessions.

## Quickstart

1. Install via Homebrew:
   ```bash
   brew install lafarguem/tap/deezranbum
   ```
2. Find your Deezer user ID by going to your favorites — the URL will look like `https://www.deezer.com/us/profile/USER_ID`.
3. Set your user ID:
   ```bash
   deezranbum user USER_ID
   ```
4. Open Deezer in your browser and run:
   ```bash
   deezranbum album
   ```

That's it — a random album from your library will be added to your Deezer queue.

## Requirements

### Allow JavaScript from Apple Events

`deezranbum` controls your browser via Apple Events to add albums to your Deezer queue. You need to enable this once in your Chromium-based browser (Chrome, Brave, Arc, etc.):

> **View > Developer > Allow JavaScript from Apple Events**

This only needs to be done once. A Deezer tab must be open in your browser when running `deezranbum album`.

### Find your Deezer user ID

Go to your Deezer profile (Favorites page). The URL will look like:

```
https://www.deezer.com/us/profile/1234567890
```

The number at the end is your user ID.

## Setup

Set your user ID (only needed once, persisted across sessions):

```bash
deezranbum user USER_ID
```

## Commands

| Command | Description |
|---|---|
| `deezranbum album` | Add a random unseen album to your Deezer queue |
| `deezranbum user USER_ID` | Set your Deezer user ID |
| `deezranbum session history` | Show all albums seen so far, in order |
| `deezranbum session remove TITLE` | Remove an album from the seen list |
| `deezranbum session clear` | Clear the current session (all albums become unseen again) |
| `deezranbum reset` | Delete all persisted state (user ID and session) |

Once every album in your library has been seen, the session is automatically cleared and the cycle starts over.

## Releasing (Homebrew deployment)

Distribution is handled by [cargo-dist](https://opensource.axo.dev/cargo-dist/) (configured in `dist-workspace.toml`). Pushing a version tag triggers the `release` GitHub Actions workflow, which builds the macOS binaries (Apple Silicon + Intel), creates a GitHub Release with those artifacts, and **automatically pushes an updated formula to the [`lafarguem/homebrew-tap`](https://github.com/lafarguem/homebrew-tap) repo**. No manual formula editing is required.

### Cut a release

1. Bump the version in `Cargo.toml` (e.g. `version = "0.1.5"`) and refresh `Cargo.lock`:
   ```bash
   cargo build      # rewrites Cargo.lock with the new version
   ```
2. Commit the bump:
   ```bash
   git commit -am "release v0.1.5"
   ```
3. Tag with a matching `vX.Y.Z` tag and push it:
   ```bash
   git tag v0.1.5
   git push origin main --tags
   ```
4. Watch the run; the formula push happens at the end of a green build:
   ```bash
   gh run watch
   ```

Once it finishes, users get the new version with `brew upgrade deezranbum` (or `brew install lafarguem/tap/deezranbum` for a fresh install).

### One-time prerequisites

- The tap repo `lafarguem/homebrew-tap` must exist.
- A Personal Access Token with write access to the tap repo must be saved as the `HOMEBREW_TAP_TOKEN` secret on this repo — the default `GITHUB_TOKEN` cannot push to a different repository.

### Regenerate CI after changing dist config

If you edit `dist-workspace.toml` (targets, installers, etc.), regenerate the workflow so CI stays in sync:

```bash
dist init        # or: dist generate
git commit -am "chore: regenerate dist CI"
```

## License

MIT — see [LICENSE](LICENSE).
