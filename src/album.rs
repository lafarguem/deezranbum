use std::{
    collections::HashSet,
    error::Error,
    io::{self, Write},
};

use crate::{
    QueueBehaviours, picker, queue, session,
    storage::{Album, AppState, load_state, save_state},
};
use futures::stream::{self, StreamExt};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const REDIRECT_CONCURRENCY: usize = 32;

const BASE_URL: &str = "https://api.deezer.com/user/";

#[derive(Serialize, Deserialize, Debug, Default)]

struct DeezerResponse {
    data: Vec<Album>,
}

pub fn add_album(state: &mut AppState, id: u64) {
    if state.album_ids.insert(id) {
        state.album_order.push(id);
    }
}

async fn get_albums(state: &mut AppState) -> Result<Vec<Album>, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}{}/albums?limit=1000", BASE_URL, state.user_id);

    let response = client.get(url).send().await?;
    let albums: DeezerResponse = response.json().await?;
    let mut album_data = albums.data;
    apply_album_redirects(&mut album_data, &client).await;

    state
        .albums
        .extend(album_data.iter().cloned().map(|a| (a.id, a)));

    Ok(album_data)
}

async fn get_album_redirect(id: &u64, client: &Client) -> Result<u64, Box<dyn Error>> {
    let final_url = client
        .head(format!("https://www.deezer.com/album/{}", id))
        .send()
        .await?
        .url()
        .clone();
    let real_id: u64 = final_url
        .path_segments()
        .and_then(|mut segs| segs.rfind(|seg| !seg.is_empty()))
        .ok_or("missing album id in url")?
        .parse()?;
    Ok(real_id)
}

async fn apply_album_redirects(albums: &mut [Album], client: &Client) {
    let ids: Vec<u64> = albums.iter().map(|a| a.id).collect();
    let resolved: Vec<u64> = stream::iter(ids)
        .map(|id| async move { get_album_redirect(&id, client).await.unwrap_or(id) })
        .buffered(REDIRECT_CONCURRENCY)
        .collect()
        .await;

    for (album, new_id) in albums.iter_mut().zip(resolved) {
        album.id = new_id;
    }
}

fn choose_albums<'a>(albums: &'a [Album], state: &mut AppState, amount: usize) -> Vec<&'a Album> {
    let mut chosen: Vec<&Album> = Vec::new();

    let mut candidates: Vec<&Album> = albums
        .iter()
        .filter(|a| !state.album_ids.contains(&a.id))
        .collect();

    if candidates.len() < amount {
        chosen.append(&mut candidates.to_vec());
        session::clear_state(state);

        candidates = albums
            .iter()
            .filter(|a| !state.album_ids.contains(&a.id))
            .collect();
    }

    let remaining = amount - chosen.len();

    let mut rng = rand::thread_rng();
    let mut random: Vec<&Album> = candidates
        .choose_multiple(&mut rng, remaining)
        .cloned()
        .collect();

    chosen.append(&mut random);

    chosen
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

pub async fn next(amount: usize, queue: QueueBehaviours, debug: bool) -> std::io::Result<()> {
    let mut state: AppState = load_state();
    let albums = match get_albums(&mut state).await {
        Ok(albums) => albums,
        Err(e) => {
            panic!("Failed to get albums: {:?}", e);
        }
    };
    let chosen = choose_albums(&albums, &mut state, amount);
    match chosen.len() {
        0 => {
            save_state(&state)?;
            println!("No album found");
            Ok(())
        }
        _ => {
            for album in chosen {
                println!("{}", album);
                handle_queue(&album.id, queue, debug);
                add_album(&mut state, album.id);
            }
            save_state(&state)
        }
    }
}

pub async fn pick_albums(
    state: &mut AppState,
    initial_selected: Option<&HashSet<u64>>,
) -> std::io::Result<Vec<Album>> {
    let albums = match get_albums(state).await {
        Ok(albums) => albums,
        Err(e) => {
            panic!("Failed to get albums: {:?}", e);
        }
    };

    save_state(state)?;

    let chosen = picker::pick(&albums, initial_selected)?;
    if chosen.is_empty() {
        println!("No albums selected");
    }
    Ok(chosen)
}

pub async fn pick(queue: QueueBehaviours, debug: bool) -> std::io::Result<()> {
    let mut state: AppState = load_state();

    let chosen = pick_albums(&mut state, None).await?;
    if chosen.is_empty() {
        return Ok(());
    }

    for album in &chosen {
        println!("{}", album);
        handle_queue(&album.id, queue, debug);
        add_album(&mut state, album.id);
    }

    save_state(&state)
}

pub async fn search(query: &str, queue: QueueBehaviours, debug: bool) -> std::io::Result<()> {
    let mut state: AppState = load_state();
    let albums = match get_albums(&mut state).await {
        Ok(albums) => albums,
        Err(e) => {
            panic!("Failed to get albums: {:?}", e);
        }
    };

    let Some(album) = best_match(query, &albums) else {
        println!("No album matched '{}'", query);
        return Ok(());
    };

    println!("{}", album);
    handle_queue(&album.id, queue, debug);
    add_album(&mut state, album.id);
    save_state(&state)
}
