use std::io::{self, Write};

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use crate::{QueueBehaviours, queue, session, storage::{Album, AppState, load_state, save_state}};

const BASE_URL: &str = "https://api.deezer.com/user/";

#[derive(Serialize, Deserialize, Debug, Default)]

struct DeezerResponse {
    data: Vec<Album>
}

fn add_album(state: &mut AppState, album: Album) {
    let id = album.id;

    state.albums.entry(id).or_insert_with(|| {
        state.album_ids.insert(id);
        state.album_order.push(id);
        album
    });
}

async fn get_albums(state: &AppState) -> Result<Vec<Album>, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}{}/albums", BASE_URL, state.user_id);

    let response= client.get(url).send().await?;
    let albums: DeezerResponse  = response.json().await?;

    Ok(albums.data)
}

fn choose_albums<'a>(
    albums: &'a [Album],
    state: &mut AppState,
    amount: usize,
) -> Vec<&'a Album> {
    let mut chosen: Vec<&Album> = Vec::new();

    let mut candidates: Vec<&Album> = albums
        .iter()
        .filter(|a| !state.album_ids.contains(&a.id))
        .collect();

    if candidates.len() < amount {
        chosen.append(&mut candidates.iter().cloned().collect());
        session::clear_state(state);

        candidates = albums
            .iter()
            .filter(|a| !state.album_ids.contains(&a.id))
            .collect();
    }

    let remaining = amount - chosen.len();

    let mut rng = rand::thread_rng();
    let mut random: Vec<&Album> =
        candidates.choose_multiple(&mut rng, remaining).cloned().collect();

    chosen.append(&mut random);

    chosen
}

pub async fn next(amount: usize, queue: QueueBehaviours, debug: bool) -> std::io::Result<()> {
    let mut state: AppState = load_state();
    let albums = match get_albums(&state).await {
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
                let add_to_queue = match queue {
                    QueueBehaviours::True => true,
                    QueueBehaviours::False => false,
                    QueueBehaviours::Ask => {
                        print!("Add to Deezer queue? [y/N] ");
                        io::stdout().flush().ok();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap_or(0);
                        input.trim().eq_ignore_ascii_case("y")
                    }
                };
                if add_to_queue {
                    match queue::add_to_queue(album, debug) {
                        Ok(()) => println!("Added to Deezer queue."),
                        Err(queue::QueueError::NoDeezerTab) =>
                            eprintln!("Warning: no Deezer tab found in Chrome — skipping queue."),
                        Err(e) => eprintln!("Warning: could not add to queue: {e}"),
                    }
                }
                add_album(&mut state, album.clone());
            }
            save_state(&state)
        },
    }
}
