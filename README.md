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
| `deezranbum album N` | Add N random unseen albums |
| `deezranbum album QUERY` | Add the best fuzzy match for QUERY |
| `deezranbum user USER_ID` | Set your Deezer user ID |
| `deezranbum session history` | Show all albums seen so far, in order |
| `deezranbum session remove TITLE` | Remove an album from the seen list |
| `deezranbum session clear` | Clear the current session (all albums become unseen again) |
| `deezranbum replay FROM TO` | Re-queue a range of the session |
| `deezranbum collection edit NAME` | Create or edit a local collection of albums |
| `deezranbum collection list [NAME]` | List collections, or the albums in one |
| `deezranbum collection play NAME [N]` | Queue albums from a collection |
| `deezranbum collection delete NAME` | Delete a collection |
| `deezranbum playlist add URL` | Treat a Deezer playlist as an album |
| `deezranbum playlist import` | Pick playlists to add from your Deezer library |
| `deezranbum playlist list` | List the Deezer playlists in the pool |
| `deezranbum playlist remove [TITLE]` | Remove a Deezer playlist from the pool |
| `deezranbum fetch` | Refresh all album and playlist metadata |
| `deezranbum stats` | Show listening stats |
| `deezranbum reset` | Delete all persisted state (user ID and session) |

Once every album in your library has been seen, the session is automatically cleared and the cycle starts over.

`deezranbum album` accepts `--kind album|playlist`, `--before`/`--after` (partial dates like
`2019` or `2019-05` work), `--min-duration`/`--max-duration` (e.g. `1h30m`), and repeatable
`--genre`/`--artist` plus their `--exclude-` counterparts. Pass `--queue false` to print a pick
without touching your Deezer queue, or `--queue ask` to be prompted.

## Playlists as albums

Any public Deezer playlist can be added to the pool and is then treated exactly like an album —
picked at random, tracked as seen, replayed, and addable to a collection:

```bash
deezranbum playlist add https://www.deezer.com/playlist/1313621735
deezranbum playlist import        # pick from your own playlists
deezranbum album --kind playlist  # pick a playlist specifically
```

A playlist's creator stands in for the artist and its creation date for the release date, so
`--artist` and `--before`/`--after` work on them. Playlists carry no genre, so `--genre` never
matches one. Since a playlist's duration is the sum of its tracks, `--max-duration 1h30m` is the
way to keep very long playlists out of a pick.

## Collections

A collection is a local, named group of albums and playlists — it lives only in `deezranbum`
and is never written back to Deezer:

```bash
deezranbum collection edit chill   # interactive picker
deezranbum collection play chill 2
```

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
