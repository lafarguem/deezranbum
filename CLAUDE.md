# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`deezranbum` is a CLI tool that fetches a random album from a Deezer user's library, tracking seen albums across sessions to avoid repeats.

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
cargo run -- album              # fetch a random unseen album
cargo run -- user <user_id>     # set the Deezer user ID
cargo run -- session clear      # clear current session
cargo run -- session history    # show seen albums in order
cargo run -- session remove <title>  # remove album from session
cargo run -- reset              # delete all persisted state
```

## Architecture

Five focused modules:

- **main.rs** – CLI definition (clap) and command routing
- **album.rs** – Deezer API calls (`https://api.deezer.com/user/{id}/albums`) and random album selection logic; if all albums have been seen, auto-clears session and retries
- **storage.rs** – `AppState` struct (user_id, HashSet of seen IDs, Vec for ordering, HashMap of full album data); persists as JSON to the platform data directory via `directories-next` (app qualifier: `com.arugula.randeezbum`)
- **session.rs** – handlers for session subcommands (clear, history, remove)
- **user.rs** – sets `user_id` in persisted state

## State File Location

| Platform | Path |
|----------|------|
| macOS    | `~/Library/Application Support/com.arugula/randeezbum/album.json` |
| Linux    | `~/.local/share/com.arugula/randeezbum/album.json` |
| Windows  | `%APPDATA%\com.arugula\randeezbum\album.json` |

## Key Types (storage.rs)

```rust
struct AppState {
    user_id: String,
    album_ids: HashSet<u64>,   // set of seen IDs for fast lookup
    album_order: Vec<u64>,     // insertion order for history
    albums: HashMap<u64, Album>, // full metadata
}
```
