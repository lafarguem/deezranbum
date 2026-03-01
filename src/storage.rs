use directories_next::ProjectDirs;
use std::collections::HashSet;
use std::fmt;
use std::{collections::HashMap, path::PathBuf};
use std::fs::File;
use serde::{Serialize, Deserialize};
use std::io::ErrorKind;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Artist {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Album {
    pub id: u64,
    pub title: String,
    pub link: String,
    pub artist: Artist
}

impl fmt::Display for Album {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} by {} ({})", self.title, self.artist.name, self.link)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppState {
    pub user_id: String,
    pub album_ids: HashSet<u64>,
    pub album_order: Vec<u64>,
    pub albums: HashMap<u64, Album>
}

fn data_file() -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "arugula", "randeezbum")
            .expect("Could not determine directory");

    let dir = proj_dirs.data_dir();
    std::fs::create_dir_all(dir).unwrap();

    dir.join("album.json")
}

pub fn load_state() -> AppState {
    let path = data_file();

    match File::open(path) {
        Ok(file) => serde_json::from_reader(file).unwrap(),

        Err(e) if e.kind() == ErrorKind::NotFound => {
            AppState::default()
        }

        Err(e) => panic!("{}", e),
    }
}

pub fn save_state(state: &AppState) -> std::io::Result<()> {
    let path = data_file();
    let file = File::create(path)?;

    serde_json::to_writer_pretty(file, state).unwrap();

    Ok(())
}

pub fn reset() {
    let path = data_file();

    if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
}
