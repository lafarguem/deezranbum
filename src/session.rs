use std::io::Error;

use crate::{SessionCommands, storage::{AppState, load_state, save_state}};

pub fn handle(command: SessionCommands) {
    match command {
        SessionCommands::Clear => clear(),
        SessionCommands::History => history(),
        SessionCommands::Remove { album_name } => match remove(album_name) {
            Ok(()) => (),
            _ => print!("Album not found")
        },
    }
} 

pub fn clear_state(state: &mut AppState) {
    state.album_ids.clear();
    state.album_order.clear();
    state.albums.clear();
}

pub fn clear() {
    let mut state = load_state();
    clear_state(&mut state);
    match save_state(&state) {
        Ok(()) => (),
        _ => println!("Error clearing application")
    }
}

fn history() {
    let state = load_state();
    for (index, id) in state.album_order.iter().enumerate() {
        let album = match state.albums.get(id) {
            Some(album) => album,
            None => continue
        };
        println!("{} : {}", index, album)
    }
}

fn remove(album_title: String) -> Result<(), Error> {
    let mut state = load_state();
    let id = match state.albums.iter().find(|(_, album)| {
        album.title == album_title
    }) {
        Some((id, _)) => id,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Album not found",
            ))
        }
    };
    state.album_ids.remove(id);
    save_state(&state)?;
    Ok(())
}
