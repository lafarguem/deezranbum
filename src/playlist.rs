use std::collections::HashSet;

use crate::{
    PlaylistSubcommands,
    album::{best_match, fetch_playlist, fetch_user_playlists},
    error::AppResult,
    picker,
    storage::{Album, AppState, load_state, playlist_key, save_state},
};

pub async fn handle(command: PlaylistSubcommands) -> AppResult<()> {
    match command {
        PlaylistSubcommands::Add { playlist } => add(&playlist).await,
        PlaylistSubcommands::Remove { playlist } => remove(playlist),
        PlaylistSubcommands::Import => import().await,
        PlaylistSubcommands::List => list(),
    }
}

fn parse_id(input: &str) -> Option<u64> {
    let input = input.trim();
    if let Ok(id) = input.parse::<u64>() {
        return Some(id);
    }
    let tail = input.split("playlist/").nth(1)?;
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn describe(playlist: &Album) -> String {
    match playlist.nb_tracks {
        Some(n) => format!("{} - {} tracks", playlist, n),
        None => format!("{}", playlist),
    }
}

fn registered(state: &AppState) -> Vec<Album> {
    state
        .playlist_ids
        .iter()
        .map(|id| {
            let key = playlist_key(*id);
            state
                .albums
                .get(&key)
                .cloned()
                .unwrap_or_else(|| Album::with_id(key))
        })
        .collect()
}

async fn add(input: &str) -> AppResult<()> {
    let Some(id) = parse_id(input) else {
        println!("Could not read a playlist id from '{}'", input);
        return Ok(());
    };

    let mut state: AppState = load_state()?;
    let client = reqwest::Client::new();

    let playlist = match fetch_playlist(id, &client).await {
        Ok(playlist) => playlist,
        Err(e) => {
            println!("Could not add playlist {}: {}", id, e);
            return Ok(());
        }
    };

    if state.playlist_ids.contains(&id) {
        println!("Already added: {}", describe(&playlist));
    } else {
        state.playlist_ids.push(id);
        println!("Added: {}", describe(&playlist));
    }

    state.albums.insert(playlist.id, playlist);
    save_state(&state)?;
    Ok(())
}

fn remove(query: Option<String>) -> AppResult<()> {
    let mut state: AppState = load_state()?;
    let playlists = registered(&state);

    if playlists.is_empty() {
        println!("No playlists added");
        return Ok(());
    }

    let to_remove: Vec<Album> = match query {
        None => picker::pick(playlists.iter().collect(), None)?,
        Some(q) => match best_match(&q, &playlists) {
            Some(playlist) => vec![playlist.clone()],
            None => {
                println!("No playlist matched '{}'", q);
                return Ok(());
            }
        },
    };

    if to_remove.is_empty() {
        println!("No playlists selected");
        return Ok(());
    }

    for playlist in &to_remove {
        let id = playlist.queue_id();
        state.playlist_ids.retain(|registered| *registered != id);
        println!("Removed: {}", playlist);
    }

    save_state(&state)?;
    Ok(())
}

async fn import() -> AppResult<()> {
    let mut state: AppState = load_state()?;

    if state.user_id.is_empty() {
        println!("No user id set. Run `deezranbum user <user_id>` first.");
        return Ok(());
    }

    let client = reqwest::Client::new();
    let fetched = match fetch_user_playlists(&state.user_id, &client).await {
        Ok(fetched) => fetched,
        Err(e) => {
            println!("Could not fetch playlists: {}", e);
            return Ok(());
        }
    };

    let candidates: Vec<Album> = fetched
        .into_iter()
        .filter(|playlist| !playlist.is_loved_track)
        .map(Album::from_playlist)
        .collect();

    if candidates.is_empty() {
        println!("No playlists found for user {}", state.user_id);
        return Ok(());
    }

    let already: HashSet<u64> = state
        .playlist_ids
        .iter()
        .map(|id| playlist_key(*id))
        .collect();
    let chosen = picker::pick(candidates.iter().collect(), Some(&already))?;

    if chosen.is_empty() {
        println!("No playlists selected");
        return Ok(());
    }

    let keep: HashSet<u64> = chosen.iter().map(|playlist| playlist.queue_id()).collect();
    let dropped: Vec<Album> = candidates
        .iter()
        .filter(|playlist| {
            let id = playlist.queue_id();
            state.playlist_ids.contains(&id) && !keep.contains(&id)
        })
        .cloned()
        .collect();

    let dropped_ids: HashSet<u64> = dropped.iter().map(|playlist| playlist.queue_id()).collect();
    state.playlist_ids.retain(|id| !dropped_ids.contains(id));

    for playlist in &dropped {
        println!("Removed: {}", playlist);
    }

    let mut added = 0;
    for playlist in chosen {
        let id = playlist.queue_id();
        if !state.playlist_ids.contains(&id) {
            state.playlist_ids.push(id);
            added += 1;
            println!("Added: {}", describe(&playlist));
        }
        state.albums.insert(playlist.id, playlist);
    }

    if added == 0 && dropped.is_empty() {
        println!("No changes");
    }

    save_state(&state)?;
    Ok(())
}

fn list() -> AppResult<()> {
    let state: AppState = load_state()?;
    let playlists = registered(&state);

    if playlists.is_empty() {
        println!("No playlists added");
        return Ok(());
    }

    for playlist in &playlists {
        println!("{}", describe(playlist));
    }
    Ok(())
}
