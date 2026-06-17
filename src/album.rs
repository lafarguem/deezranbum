use std::{
    collections::HashSet,
    error::Error,
    io::{self, Write},
};

use crate::{
    QueueBehaviours,
    error::{AppError, AppResult},
    picker, queue, session,
    storage::{Album, ApiAlbum, AppState, UserAlbum, load_state, save_state},
};
use chrono::{Duration, NaiveDate, Utc};
use futures::stream::{self, StreamExt};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Deezer rate-limits to roughly 50 requests / 5s; keep concurrency well under
/// that so a full-library refresh doesn't trip the quota.
const FETCH_CONCURRENCY: usize = 8;

/// Deezer's error `code` for "Quota limit exceeded".
const QUOTA_EXCEEDED_CODE: i64 = 4;

const MAX_RETRIES: u32 = 5;

const BASE_URL: &str = "https://api.deezer.com/";

#[derive(Serialize, Deserialize, Debug, Default)]
struct DeezerResponse {
    data: Vec<UserAlbum>,
}

pub struct AlbumFilters {
    pub min_duration: Option<u64>,
    pub max_duration: Option<u64>,
    pub before: Option<NaiveDate>,
    pub after: Option<NaiveDate>,
    pub genre: Vec<String>,
    pub exclude_genre: Vec<String>,
    pub artist: Vec<String>,
    pub exclude_artist: Vec<String>,
}

pub fn add_album(state: &mut AppState, id: u64) {
    if state.album_ids.insert(id) {
        state.album_order.push(id);
    }
}

pub async fn get_albums(state: &mut AppState, force_fetch: bool) -> AppResult<Vec<Album>> {
    let client = reqwest::Client::new();
    let url = format!("{}/user/{}/albums?limit=1000", BASE_URL, state.user_id);

    let response = client.get(url).send().await?;
    let user_albums: DeezerResponse = response.json().await?;
    let album_data = user_albums.data;
    let now = Utc::now().naive_utc();
    let ff = force_fetch || (now - state.last_redirect_update > Duration::days(30));
    let albums = update_albums(state, album_data, &client, ff).await;
    if ff {
        state.last_redirect_update = now;
    }

    state
        .albums
        .extend(albums.iter().cloned().map(|a| (a.id, a)));

    Ok(albums)
}

#[derive(Deserialize)]
struct DeezerErrorBody {
    error: DeezerError,
}

#[derive(Deserialize)]
struct DeezerError {
    code: i64,
    message: String,
}

async fn update_album(id: &u64, client: &Client) -> std::result::Result<Album, Box<dyn Error>> {
    for attempt in 0..MAX_RETRIES {
        let body = client
            .get(format!("{}/album/{}", BASE_URL, id))
            .send()
            .await?
            .text()
            .await?;

        // Deezer returns 200 with an `{ "error": {...} }` body for failures
        // (quota, unknown album, …) rather than an HTTP error status.
        if let Ok(err) = serde_json::from_str::<DeezerErrorBody>(&body) {
            if err.error.code == QUOTA_EXCEEDED_CODE {
                // Exponential backoff, then retry the same album.
                let delay = 250u64 * 2u64.pow(attempt);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            return Err(format!("deezer error for album {id}: {}", err.error.message).into());
        }

        let api: ApiAlbum = serde_json::from_str(&body)?;
        return Ok(Album::from_api(*id, api));
    }

    Err(format!("album {id}: rate limited, gave up after {MAX_RETRIES} retries").into())
}

async fn update_albums(
    state: &AppState,
    albums: Vec<UserAlbum>,
    client: &Client,
    fetch_all: bool,
) -> Vec<Album> {
    let mut updated_albums: Vec<Album> = Vec::new();
    let mut missing_albums: Vec<UserAlbum> = Vec::new();
    for album in albums {
        if !fetch_all
            && let Some(a) = state.albums.get(&album.id)
            && a.has_metadata()
        {
            updated_albums.push(a.clone());
        } else {
            missing_albums.push(album);
        }
    }

    let resolved: Vec<Album> = stream::iter(missing_albums)
        .map(|album| async move {
            match update_album(&album.id, client).await {
                Ok(album) => album,
                Err(e) => {
                    eprintln!("warning: {e}; using partial metadata");
                    Album::with_user_album(album)
                }
            }
        })
        .buffered(FETCH_CONCURRENCY)
        .collect()
        .await;

    updated_albums.extend(resolved);
    updated_albums
}

fn check_filters(album: &Album, filters: &AlbumFilters) -> bool {
    if let Some(date) = filters.after {
        match album.release_date {
            Some(rd) if rd >= date => {}
            _ => return false,
        }
    }
    if let Some(date) = filters.before {
        match album.release_date {
            Some(rd) if rd <= date => {}
            _ => return false,
        }
    }
    if let Some(duration) = filters.max_duration
        && album.duration > duration
    {
        return false;
    }
    if let Some(duration) = filters.min_duration
        && album.duration < duration
    {
        return false;
    }
    let lowercase_genres: Vec<String> =
        album.genres.iter().map(|g| g.name.to_lowercase()).collect();
    let lowercase_genre_filter: Vec<String> =
        filters.genre.iter().map(|g| g.to_lowercase()).collect();
    if !lowercase_genre_filter.is_empty()
        && !lowercase_genres
            .iter()
            .any(|g| lowercase_genre_filter.contains(g))
    {
        return false;
    }
    let lowercase_exlude_genre_filter: Vec<String> = filters
        .exclude_genre
        .iter()
        .map(|g| g.to_lowercase())
        .collect();
    if !lowercase_exlude_genre_filter.is_empty()
        && lowercase_genres
            .iter()
            .any(|g| lowercase_exlude_genre_filter.contains(g))
    {
        return false;
    }
    let artist = album.artist.name.to_lowercase();
    if !filters.artist.is_empty() && !filters.artist.iter().any(|a| a.to_lowercase() == artist) {
        return false;
    }
    if !filters.exclude_artist.is_empty()
        && filters
            .exclude_artist
            .iter()
            .any(|a| a.to_lowercase() == artist)
    {
        return false;
    }
    true
}

fn choose_albums<'a>(
    albums: &'a [Album],
    state: &mut AppState,
    amount: usize,
    filters: &AlbumFilters,
) -> AppResult<Vec<&'a Album>> {
    // Everything in the library matching the filters, ignoring "seen" status.
    let matching: Vec<&Album> = albums
        .iter()
        .filter(|a| check_filters(a, filters))
        .collect();

    // If the library can't satisfy the request even after a reset, bail out
    // before mutating any state so the session is left untouched.
    if matching.len() < amount {
        return Err(AppError::NotEnoughAlbums {
            found: matching.len(),
            requested: amount,
        });
    }

    let mut candidates: Vec<&Album> = matching
        .iter()
        .copied()
        .filter(|a| !state.album_ids.contains(&a.id))
        .collect();

    // Not enough unseen albums left — wrap around: clear the session and draw
    // from the full matching set. Safe to clear now: we know `matching` is big
    // enough, so this can't fail afterwards.
    if candidates.len() < amount {
        session::clear_state(state);
        candidates = matching;
    }

    let mut rng = rand::thread_rng();
    Ok(candidates
        .choose_multiple(&mut rng, amount)
        .copied()
        .collect())
}

fn prompt_queue(queue: QueueBehaviours) -> bool {
    match queue {
        QueueBehaviours::True => true,
        QueueBehaviours::False => false,
        QueueBehaviours::Ask => {
            print!("Add to Deezer queue? [y/N] ");
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap_or(0);
            input.trim().eq_ignore_ascii_case("y")
        }
    }
}

pub fn handle_queue(album_id: &u64, queue: QueueBehaviours, debug: bool) {
    if !prompt_queue(queue) {
        return;
    }
    match crate::queue::add_to_queue(album_id, debug) {
        Ok(()) => println!("Added to Deezer queue."),
        Err(queue::QueueError::NoDeezerTab) => {
            eprintln!("Warning: no Deezer tab found in Chrome — skipping queue.")
        }
        Err(e) => eprintln!("Warning: could not add to queue: {e}"),
    }
}

pub fn best_match<'a>(query: &str, albums: &'a [Album]) -> Option<&'a Album> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf: Vec<char> = Vec::new();
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    albums
        .iter()
        .filter_map(|a| {
            let hay = format!("{} {}", a.artist.name, a.title);
            pattern
                .score(Utf32Str::new(&hay, &mut buf), &mut matcher)
                .map(|s| (s, a))
        })
        .max_by_key(|(s, _)| *s)
        .map(|(_, a)| a)
}

pub async fn next(
    amount: usize,
    queue: QueueBehaviours,
    debug: bool,
    filters: &AlbumFilters,
) -> AppResult<()> {
    let mut state: AppState = load_state()?;
    let albums = get_albums(&mut state, false).await?;
    let chosen = choose_albums(&albums, &mut state, amount, filters)?;

    for album in chosen {
        println!("{}", album);
        handle_queue(&album.real_id.unwrap_or(album.id), queue, debug);
        add_album(&mut state, album.id);
    }
    save_state(&state)?;
    Ok(())
}

pub async fn pick_albums(
    state: &mut AppState,
    initial_selected: Option<&HashSet<u64>>,
    filters: Option<&AlbumFilters>,
) -> AppResult<Vec<Album>> {
    let albums = get_albums(state, false).await?;

    save_state(state)?;

    let matching: Vec<&Album> = match filters {
        Some(f) => albums.iter().filter(|a| check_filters(a, f)).collect(),
        None => albums.iter().collect(),
    };

    let chosen = picker::pick(matching, initial_selected)?;
    if chosen.is_empty() {
        println!("No albums selected");
    }
    Ok(chosen)
}

pub async fn pick(queue: QueueBehaviours, debug: bool, filters: &AlbumFilters) -> AppResult<()> {
    let mut state: AppState = load_state()?;

    let chosen = pick_albums(&mut state, None, Some(filters)).await?;
    if chosen.is_empty() {
        return Ok(());
    }

    for album in &chosen {
        println!("{}", album);
        handle_queue(&album.real_id.unwrap_or(album.id), queue, debug);
        add_album(&mut state, album.id);
    }

    save_state(&state)?;
    Ok(())
}

pub async fn search(query: &str, queue: QueueBehaviours, debug: bool) -> AppResult<()> {
    let mut state: AppState = load_state()?;
    let albums = get_albums(&mut state, false).await?;

    let Some(album) = best_match(query, &albums) else {
        println!("No album matched '{}'", query);
        return Ok(());
    };

    println!("{}", album);
    handle_queue(&album.real_id.unwrap_or(album.id), queue, debug);
    add_album(&mut state, album.id);
    save_state(&state)?;
    Ok(())
}
