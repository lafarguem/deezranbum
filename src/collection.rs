use std::collections::HashSet;

use rand::{seq::SliceRandom, thread_rng};

use crate::{
    CollectionSubcommands, QueueBehaviours,
    album::{add_album, handle_queue, pick_albums},
    error::AppResult,
    session::remove_album,
    storage::{Album, AppState, load_state, save_state},
};

pub async fn handle(command: CollectionSubcommands, debug: bool) -> AppResult<()> {
    match command {
        CollectionSubcommands::Edit { name } => edit(&name).await,
        CollectionSubcommands::List { name } => match name {
            Some(name) => list_collection(&name),
            None => list(),
        },
        CollectionSubcommands::Delete { name } => delete(&name),
        CollectionSubcommands::Play {
            name,
            number,
            queue,
        } => play(&name, number, queue, debug).await,
    }
}

pub async fn edit(name: &str) -> AppResult<()> {
    let mut state: AppState = load_state()?;

    let existing = state.collections.get(name).cloned().unwrap_or_default();
    let chosen: HashSet<u64> = pick_albums(&mut state, Some(&existing), None)
        .await?
        .into_iter()
        .map(|album| album.id)
        .collect();

    if chosen.is_empty() {
        return Ok(());
    }

    state.collections.insert(name.to_string(), chosen);
    save_state(&state)?;
    Ok(())
}

pub async fn play(
    name: &str,
    number: Option<usize>,
    queue: QueueBehaviours,
    debug: bool,
) -> AppResult<()> {
    let mut state: AppState = load_state()?;
    let collection = match state.collections.get(name) {
        Some(collection) => collection.clone(),
        None => {
            println!("No collection named {}", name);
            return Ok(());
        }
    };

    let mut album_ids: Vec<u64> = collection
        .iter()
        .filter(|id| !state.album_ids.contains(id))
        .copied()
        .collect();
    if album_ids.is_empty() {
        album_ids = collection.iter().copied().collect();
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
                let album = state
                    .albums
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| Album::with_id(id));
                println!("{}", album);
                handle_queue(&album, queue, debug);
                add_album(&mut state, id);
            }
            save_state(&state)?;
        }
    }

    Ok(())
}

pub fn delete(name: &str) -> AppResult<()> {
    let mut state: AppState = load_state()?;

    let removed = state.collections.remove(name);
    match removed {
        Some(_) => println!("Collection {} successfully removed!", name),
        None => println!("No collection named {}", name),
    }

    save_state(&state)?;
    Ok(())
}

pub fn list() -> AppResult<()> {
    let state: AppState = load_state()?;

    for (name, collection) in state.collections.iter() {
        println!("{} - {} albums", name, collection.len());
    }
    Ok(())
}

pub fn list_collection(name: &str) -> AppResult<()> {
    let state: AppState = load_state()?;

    let collection = match state.collections.get(name) {
        Some(collection) => collection,
        None => {
            println!("No collection named {}", name);
            return Ok(());
        }
    };

    for id in collection.iter() {
        let album = match state.albums.get(id) {
            Some(album) => album,
            None => &Album::with_id(*id),
        };
        println!("{}", album)
    }
    Ok(())
}
