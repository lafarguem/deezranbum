use crate::storage::{load_state, playlist_key};
use clap_complete::CompletionCandidate;

pub fn genres() -> Vec<CompletionCandidate> {
    let Ok(state) = load_state() else {
        return Vec::new();
    };
    let mut genres: Vec<String> = state
        .albums
        .values()
        .flat_map(|album| album.genres.iter().map(|genre| genre.name.clone()))
        .collect();
    genres.sort();
    genres.dedup();
    genres.into_iter().map(CompletionCandidate::new).collect()
}

pub fn artists() -> Vec<CompletionCandidate> {
    let Ok(state) = load_state() else {
        return Vec::new();
    };
    let mut artists: Vec<String> = state
        .albums
        .values()
        .map(|album| album.artist.name.clone())
        .collect();
    artists.sort();
    artists.dedup();
    artists.into_iter().map(CompletionCandidate::new).collect()
}

pub fn album_titles() -> Vec<CompletionCandidate> {
    let Ok(state) = load_state() else {
        return Vec::new();
    };

    let mut album_titles: Vec<String> = state
        .albums
        .values()
        .map(|album| album.title.clone())
        .collect();

    album_titles.sort();
    album_titles.dedup();
    album_titles
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

pub fn playlist_titles() -> Vec<CompletionCandidate> {
    let Ok(state) = load_state() else {
        return Vec::new();
    };

    let mut titles: Vec<String> = state
        .playlist_ids
        .iter()
        .filter_map(|id| state.albums.get(&playlist_key(*id)))
        .map(|playlist| playlist.title.clone())
        .collect();

    titles.sort();
    titles.dedup();
    titles.into_iter().map(CompletionCandidate::new).collect()
}
