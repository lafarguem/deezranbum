use chrono::{NaiveDate, NaiveDateTime};
use clap::ValueEnum;
use directories_next::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::ErrorKind;
use std::{collections::HashMap, path::PathBuf};

use crate::error::{AppError, AppResult};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Artist {
    pub name: String,
}

impl Default for Artist {
    fn default() -> Self {
        Artist {
            name: "Unknown".to_string(),
        }
    }
}

/// Deezer returns release dates as `"YYYY-MM-DD"`, but uses `"0000-00-00"` (and
/// occasionally an empty string) for unknown dates, which are not valid
/// `NaiveDate`s. Treat anything unparseable as a missing date rather than
/// failing the whole response.
fn deserialize_lenient_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()))
}

fn deserialize_lenient_datetime<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.as_deref().and_then(|s| {
        let date = s.split_whitespace().next().unwrap_or(s);
        NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
    }))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Genre {
    pub id: u64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lower")]
pub enum ItemKind {
    #[default]
    Album,
    Playlist,
}

pub const PLAYLIST_KEY_BASE: u64 = 1 << 63;

pub fn playlist_key(playlist_id: u64) -> u64 {
    PLAYLIST_KEY_BASE | playlist_id
}

pub fn kind_of_key(key: u64) -> ItemKind {
    if key & PLAYLIST_KEY_BASE != 0 {
        ItemKind::Playlist
    } else {
        ItemKind::Album
    }
}

pub fn real_id_of_key(key: u64) -> u64 {
    key & !PLAYLIST_KEY_BASE
}

/// Deezer nests list sub-resources (genres, tracks, …) under a `{ "data": [..] }`
/// wrapper rather than returning a bare array. Unwrap it to a plain `Vec`.
fn deserialize_genre_list<'de, D>(deserializer: D) -> Result<Vec<Genre>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct GenreList {
        #[serde(default)]
        data: Vec<Genre>,
    }
    let wrapper: Option<GenreList> = Option::deserialize(deserializer)?;
    Ok(wrapper.map(|w| w.data).unwrap_or_default())
}

/// Response shape of Deezer's `GET /album/{id}` endpoint. Distinct from our
/// stored [`Album`] because the wire format nests `genres` under `data` and the
/// canonical `id` differs from the per-library id we key on.
#[derive(Deserialize, Debug, Clone)]
pub struct ApiAlbum {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub artist: Artist,
    #[serde(default, deserialize_with = "deserialize_genre_list")]
    pub genres: Vec<Genre>,
    #[serde(default, deserialize_with = "deserialize_lenient_date")]
    pub release_date: Option<NaiveDate>,
    #[serde(default)]
    pub duration: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiPlaylist {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub creator: Artist,
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub nb_tracks: u64,
    #[serde(default, deserialize_with = "deserialize_lenient_datetime")]
    pub creation_date: Option<NaiveDate>,
    #[serde(default)]
    pub is_loved_track: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserAlbum {
    pub id: u64,
    pub title: String,
    pub link: String,
    pub artist: Artist,
    #[serde(default, deserialize_with = "deserialize_lenient_date")]
    pub release_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Album {
    pub id: u64,
    pub real_id: Option<u64>,
    #[serde(default)]
    pub kind: ItemKind,
    pub title: String,
    pub link: String,
    pub artist: Artist,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default, deserialize_with = "deserialize_lenient_date")]
    pub release_date: Option<NaiveDate>,
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub nb_tracks: Option<u64>,
}

impl Default for Album {
    fn default() -> Self {
        Album {
            id: 0,
            real_id: None,
            kind: ItemKind::Album,
            title: "Unknown".to_string(),
            link: String::new(),
            artist: Artist::default(),
            genres: Vec::new(),
            release_date: None,
            duration: 0,
            nb_tracks: None,
        }
    }
}

impl Album {
    pub fn with_id(id: u64) -> Self {
        let kind = kind_of_key(id);
        let real_id = real_id_of_key(id);
        let link = match kind {
            ItemKind::Album => format!("https://deezer.com/album/{}", real_id),
            ItemKind::Playlist => format!("https://deezer.com/playlist/{}", real_id),
        };
        Album {
            id,
            real_id: Some(real_id),
            kind,
            link,
            ..Default::default()
        }
    }

    /// Whether full metadata (genres/duration) was successfully fetched.
    /// Fallback albums built from `with_user_album`/`with_id` lack this, so a
    /// cached copy of one should be re-fetched rather than reused.
    pub fn has_metadata(&self) -> bool {
        match self.kind {
            ItemKind::Album => self.duration > 0 || !self.genres.is_empty(),
            ItemKind::Playlist => self.nb_tracks.is_some(),
        }
    }

    pub fn queue_id(&self) -> u64 {
        self.real_id.unwrap_or_else(|| real_id_of_key(self.id))
    }

    /// Build a stored album from a Deezer `/album/{id}` response. Keeps the
    /// per-library `library_id` as `id` (the stable key used for matching and
    /// the "seen" set) and records the canonical id as `real_id` (used when
    /// adding to the Deezer queue).
    pub fn from_api(library_id: u64, api: ApiAlbum) -> Self {
        Album {
            id: library_id,
            real_id: Some(api.id),
            kind: ItemKind::Album,
            title: api.title,
            link: api.link,
            artist: api.artist,
            genres: api.genres,
            release_date: api.release_date,
            duration: api.duration,
            nb_tracks: None,
        }
    }

    pub fn from_playlist(api: ApiPlaylist) -> Self {
        Album {
            id: playlist_key(api.id),
            real_id: Some(api.id),
            kind: ItemKind::Playlist,
            title: api.title,
            link: if api.link.is_empty() {
                format!("https://www.deezer.com/playlist/{}", api.id)
            } else {
                api.link
            },
            artist: api.creator,
            genres: Vec::new(),
            release_date: api.creation_date,
            duration: api.duration,
            nb_tracks: Some(api.nb_tracks),
        }
    }

    pub fn with_user_album(album: UserAlbum) -> Self {
        Album {
            id: album.id,
            real_id: Some(album.id),
            title: album.title,
            link: album.link,
            artist: album.artist,
            release_date: album.release_date,
            ..Default::default()
        }
    }
}

impl fmt::Display for Album {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} by {} ({})", self.title, self.artist.name, self.link)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct AppState {
    pub user_id: String,
    pub last_redirect_update: NaiveDateTime,
    pub album_ids: HashSet<u64>,
    pub album_order: Vec<u64>,
    pub albums: HashMap<u64, Album>,
    #[serde(alias = "playlists")]
    pub collections: HashMap<String, HashSet<u64>>,
    pub playlist_ids: Vec<u64>,
    pub history: HashMap<u64, Vec<NaiveDateTime>>,
}

fn data_file() -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "arugula", "randeezbum").expect("Could not determine directory");

    let dir = proj_dirs.data_dir();
    std::fs::create_dir_all(dir).unwrap();

    dir.join("album.json")
}

pub fn load_state() -> AppResult<AppState> {
    let path = data_file();

    match File::open(path) {
        Ok(file) => Ok(serde_json::from_reader(file).unwrap()),

        Err(e) if e.kind() == ErrorKind::NotFound => Ok(AppState::default()),

        Err(e) => Err(AppError::Io(e)),
    }
}

pub fn save_state(state: &AppState) -> AppResult<()> {
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
