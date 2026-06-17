use std::collections::HashSet;

use rand::{seq::SliceRandom, thread_rng};

use crate::{
    PlaylistSubcommands, QueueBehaviours,
    album::{add_album, handle_queue, pick_albums},
    error::Result,
    session::remove_album,
    storage::{Album, AppState, load_state, save_state},
};

pub async fn handle(command: PlaylistSubcommands, debug: bool) -> Result<()> {
    match command {
        PlaylistSubcommands::Edit { name } => edit(&name).await,
        PlaylistSubcommands::List { name } => match name {
            Some(name) => list_playlist(&name),
            None => {
                list();
                Ok(())
            }
        },
        PlaylistSubcommands::Delete { name } => delete(&name),
        PlaylistSubcommands::Play {
            name,
            number,
            queue,
        } => play(&name, number, queue, debug).await,
    }
}

pub async fn edit(name: &str) -> Result<()> {
    let mut state: AppState = load_state();

    let existing = state.playlists.get(name).cloned().unwrap_or_default();
    let chosen: HashSet<u64> = pick_albums(&mut state, Some(&existing), None)
        .await?
        .into_iter()
        .map(|album| album.id)
        .collect();

    if chosen.is_empty() {
        return Ok(());
    }

    state.playlists.insert(name.to_string(), chosen);
    save_state(&state)?;
    Ok(())
}

pub async fn play(
    name: &str,
    number: Option<usize>,
    queue: QueueBehaviours,
    debug: bool,
) -> Result<()> {
    let mut state: AppState = load_state();
    let playlist = match state.playlists.get(name) {
        Some(playlist) => playlist.clone(),
        None => {
            println!("No playlist named {}", name);
            return Ok(());
        }
    };

    let mut album_ids: Vec<u64> = playlist
        .iter()
        .filter(|id| !state.album_ids.contains(id))
        .copied()
        .collect();
    if album_ids.is_empty() {
        album_ids = playlist.iter().copied().collect();
        for id in &album_ids {
            remove_album(&mut state, id);
        }
    }

    let mut rng = thread_rng();
    album_ids.shuffle(&mut rng);

    if let Some(n) = number {
        album_ids.truncate(n)
    }

    match album_ids.len() {
        0 => {
            save_state(&state)?;
            println!("No album found");
            return Ok(());
        }
        _ => {
            for id in album_ids {
                let album = match state.albums.get(&id) {
                    Some(album) => album,
                    None => &Album::with_id(id),
                };
                println!("{}", album);
                handle_queue(&album.real_id.unwrap_or(album.id), queue, debug);
                add_album(&mut state, id);
            }
            save_state(&state)?;
        }
    }

    Ok(())
}

pub fn delete(name: &str) -> Result<()> {
    let mut state: AppState = load_state();

    let removed = state.playlists.remove(name);
    match removed {
        Some(_) => println!("Playlist {} successfully removed!", name),
        None => println!("No playlist named {}", name),
    }

    save_state(&state)?;
    Ok(())
}

pub fn list() {
    let state: AppState = load_state();

    for (name, playlist) in state.playlists.iter() {
        println!("{} - {} albums", name, playlist.len());
    }
}

pub fn list_playlist(name: &str) -> Result<()> {
    let state: AppState = load_state();

    let playlist = match state.playlists.get(name) {
        Some(playlist) => playlist,
        None => {
            println!("No playlist named {}", name);
            return Ok(());
        }
    };

    for id in playlist.iter() {
        let album = match state.albums.get(id) {
            Some(album) => album,
            None => &Album::with_id(*id),
        };
        println!("{}", album)
    }
    Ok(())
}
