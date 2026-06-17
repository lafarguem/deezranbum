use crate::{
    SessionCommands, album,
    error::AppResult,
    picker,
    storage::{Album, AppState, load_state, save_state},
};

pub fn handle(command: SessionCommands) -> AppResult<()> {
    match command {
        SessionCommands::Clear => clear(),
        SessionCommands::History => history(),
        SessionCommands::Remove { album_name } => {
            if let Err(e) = remove(album_name) {
                println!("Error: {}", e);
            }
            Ok(())
        }
    }
}

pub fn clear_state(state: &mut AppState) {
    state.album_ids.clear();
    state.album_order.clear();
}

pub fn clear() -> AppResult<()> {
    let mut state = load_state()?;
    clear_state(&mut state);
    match save_state(&state) {
        Ok(()) => Ok(()),
        _ => Ok(println!("Error clearing application")),
    }
}

fn history() -> AppResult<()> {
    let state = load_state()?;
    for (index, id) in state.album_order.iter().enumerate() {
        let album = match state.albums.get(id) {
            Some(album) => album,
            None => continue,
        };
        println!("{} : {}", index, album)
    }
    Ok(())
}

pub fn remove_album(state: &mut AppState, id: &u64) {
    state.album_ids.remove(id);
    state.album_order.retain(|x| x != id);
}

fn session_albums(state: &AppState) -> Vec<Album> {
    state
        .album_order
        .iter()
        .filter_map(|id| state.albums.get(id).cloned())
        .collect()
}

fn remove(query: Option<String>) -> AppResult<()> {
    let mut state = load_state()?;
    let albums = session_albums(&state);

    if albums.is_empty() {
        println!("No albums in session");
        return Ok(());
    }

    let to_remove: Vec<Album> = match query {
        None => picker::pick(albums.iter().collect(), None)?,
        Some(q) => match album::best_match(&q, &albums) {
            Some(a) => vec![a.clone()],
            None => {
                println!("No album matched '{}'", q);
                return Ok(());
            }
        },
    };

    if to_remove.is_empty() {
        println!("No albums selected");
        return Ok(());
    }

    for a in &to_remove {
        remove_album(&mut state, &a.id);
        println!("Removed: {}", a);
    }

    save_state(&state)
}
