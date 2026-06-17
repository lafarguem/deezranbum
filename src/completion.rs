use crate::storage::load_state;
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
