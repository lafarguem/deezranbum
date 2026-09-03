# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`deezranbum` is a CLI tool that fetches a random album from a Deezer user's library, tracking seen
albums across sessions to avoid repeats. Public Deezer playlists can be registered into the same
pool and are then treated exactly like albums.

## Commands

```bash
cargo build              # debug build
cargo build --release    # optimized build
cargo run -- <command>   # run (see CLI below)
cargo test               # run tests
cargo clippy             # lint
cargo fmt                # format
```

## CLI Usage

```bash
cargo run -- album                   # interactive picker over the pool
cargo run -- album 3                 # three random unseen items
cargo run -- album "kendrick"        # best fuzzy match
cargo run -- user <user_id>          # set the Deezer user ID
cargo run -- session clear           # clear current session
cargo run -- session history         # show seen items in order
cargo run -- session remove <title>  # remove item from session
cargo run -- replay <from> <to>      # re-queue a range of the session
cargo run -- collection edit <name>  # create/edit a local collection
cargo run -- collection play <name>  # queue items from a collection
cargo run -- playlist add <url|id>   # register a Deezer playlist into the pool
cargo run -- playlist import         # pick playlists from the user's library
cargo run -- fetch                   # refresh all metadata
cargo run -- stats                   # listening stats
cargo run -- reset                   # delete all persisted state
```

`album` filters: `--kind album|playlist`, `--before`/`--after`, `--min-duration`/`--max-duration`,
repeatable `--genre`/`--artist` and `--exclude-genre`/`--exclude-artist`. `--queue true|ask|false`
controls whether picks reach the Deezer queue.

## Architecture

- **main.rs** – CLI definition (clap) and command routing
- **album.rs** – Deezer API calls, filtering, and random/fuzzy selection; if everything matching
  has been seen, auto-clears the session and retries
- **storage.rs** – `Album` (the pool item, album or playlist), `AppState`, JSON persistence
- **collection.rs** – local named groups of pool items (`collection` subcommands)
- **playlist.rs** – registering Deezer playlists into the pool (`playlist` subcommands)
- **queue.rs** + **js/** – adds an item to the Deezer web player's queue by driving a browser tab
  over Apple Events (JXA → `tab.execute` → blob `<script>` in the page's main world)
- **picker.rs** – ratatui/crossterm fuzzy multi-select TUI
- **session.rs** – session subcommands (clear, history, remove)
- **replay.rs**, **stats.rs**, **user.rs**, **completion.rs**, **error.rs**

## Pool keys

Sessions, history and collections all key off a single `u64` space, but Deezer album ids and
playlist ids are separate namespaces. Playlist keys are therefore tagged with the top bit
(`PLAYLIST_KEY_BASE = 1 << 63`); `kind_of_key` and `real_id_of_key` recover the kind and the real
Deezer id from a bare key. Album keys are untagged, so older state files load unchanged.

## State File Location

| Platform | Path |
|----------|------|
| macOS    | `~/Library/Application Support/com.arugula.randeezbum/album.json` |
| Linux    | `~/.local/share/randeezbum/album.json` |
| Windows  | `%APPDATA%\arugula\randeezbum\data\album.json` |

Written via `directories-next` (`ProjectDirs::from("com", "arugula", "randeezbum")`), so the exact
layout is platform-defined rather than hand-built.

## Key Types (storage.rs)

```rust
enum ItemKind { Album, Playlist }

struct Album {                    // one pool item, album or playlist
    id: u64,                      // pool key (tagged for playlists)
    real_id: Option<u64>,         // canonical Deezer id, used for queueing
    kind: ItemKind,
    title: String,
    link: String,
    artist: Artist,               // playlist creator for playlists
    genres: Vec<Genre>,           // always empty for playlists
    release_date: Option<NaiveDate>, // creation date for playlists
    duration: u64,
    nb_tracks: Option<u64>,       // playlists only; None = never fetched
}

struct AppState {
    user_id: String,
    last_redirect_update: NaiveDateTime,
    album_ids: HashSet<u64>,               // seen keys, for fast lookup
    album_order: Vec<u64>,                 // session order for history/replay
    albums: HashMap<u64, Album>,           // cached metadata by key
    collections: HashMap<String, HashSet<u64>>, // reads legacy `playlists` too
    playlist_ids: Vec<u64>,                // registered Deezer playlist ids
    history: HashMap<u64, Vec<NaiveDateTime>>,
}
```
