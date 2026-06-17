use std::collections::HashMap;

use crate::{
    error::AppResult,
    storage::{AppState, load_state},
};

type Stat<'a> = Vec<(&'a str, u64)>;

fn seconds_to_duration(seconds: u64) -> String {
    let total_minutes = seconds / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let seconds = seconds % 60;
    format!("{hours}h{minutes}m{seconds}s")
}

pub fn general() -> AppResult<()> {
    let state = load_state()?;
    let (by_artist, by_genre, by_album) = get_stats(&state);
    display_stat(&by_album, "Albums", "plays", None, Some(20usize));
    display_stat(
        &by_artist,
        "Artists",
        "",
        Some(seconds_to_duration),
        Some(20usize),
    );
    display_stat(
        &by_genre,
        "Genres",
        "",
        Some(seconds_to_duration),
        Some(20usize),
    );
    Ok(())
}

fn get_stats<'a>(state: &'a AppState) -> (Stat<'a>, Stat<'a>, Stat<'a>) {
    let mut by_artist: HashMap<&str, u64> = HashMap::new();
    let mut by_genre: HashMap<&str, u64> = HashMap::new();
    let mut by_album: HashMap<&str, u64> = HashMap::new();

    for (album_id, plays) in state.history.iter() {
        let Some(album) = state.albums.get(album_id) else {
            continue;
        };
        *by_album.entry(album.artist.name.as_str()).or_insert(0) += 1;
        let total_duration: u64 = album.duration * plays.len() as u64;
        *by_artist.entry(album.artist.name.as_str()).or_insert(0) += total_duration;
        for genre in &album.genres {
            *by_genre.entry(genre.name.as_str()).or_insert(0) += total_duration;
        }
    }
    let mut result_by_artist: Vec<(&str, u64)> = by_artist.into_iter().collect();
    result_by_artist.sort_by_key(|(_, duration)| std::cmp::Reverse(*duration));

    let mut result_by_genre: Vec<(&str, u64)> = by_genre.into_iter().collect();
    result_by_genre.sort_by_key(|(_, duration)| std::cmp::Reverse(*duration));

    let mut result_by_album: Vec<(&str, u64)> = by_album.into_iter().collect();
    result_by_album.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    (result_by_artist, result_by_genre, result_by_album)
}

fn display_stat(
    stats: &Stat,
    stat_name: &str,
    unit: &str,
    transform: Option<fn(u64) -> String>,
    limit: Option<usize>,
) {
    println!("Top {stat_name}");
    let stats = match limit {
        Some(n) => &stats.iter().take(n).cloned().collect(),
        None => stats,
    };
    for (name, stat) in stats {
        let value = match &transform {
            Some(f) => f(*stat),
            None => stat.to_string(),
        };
        println!("{name}: {value} {unit}")
    }
}
