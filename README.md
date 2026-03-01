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

## License

MIT — see [LICENSE](LICENSE).
