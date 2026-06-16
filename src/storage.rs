use chrono::{NaiveDate, NaiveDateTime};
use directories_next::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::ErrorKind;
use std::{collections::HashMap, path::PathBuf};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Genre {
    pub id: u64,
    pub name: String,
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
    pub title: String,
    pub link: String,
    pub artist: Artist,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default, deserialize_with = "deserialize_lenient_date")]
    pub release_date: Option<NaiveDate>,
    #[serde(default)]
    pub duration: u64,
}

impl Default for Album {
    fn default() -> Self {
        Album {
            id: 0,
            real_id: None,
            title: "Unknown".to_string(),
            link: String::new(),
            artist: Artist::default(),
            genres: Vec::new(),
            release_date: None,
            duration: 0,
        }
    }
}

impl Album {
    pub fn with_id(id: u64) -> Self {
        Album {
            id,
            real_id: Some(id),
            link: format!("https://deezer.com/album/{}", id),
            ..Default::default()
        }
    }

    /// Whether full metadata (genres/duration) was successfully fetched.
    /// Fallback albums built from `with_user_album`/`with_id` lack this, so a
    /// cached copy of one should be re-fetched rather than reused.
    pub fn has_metadata(&self) -> bool {
        self.duration > 0 || !self.genres.is_empty()
    }

    /// Build a stored album from a Deezer `/album/{id}` response. Keeps the
    /// per-library `library_id` as `id` (the stable key used for matching and
    /// the "seen" set) and records the canonical id as `real_id` (used when
    /// adding to the Deezer queue).
    pub fn from_api(library_id: u64, api: ApiAlbum) -> Self {
        Album {
            id: library_id,
            real_id: Some(api.id),
            title: api.title,
            link: api.link,
            artist: api.artist,
            genres: api.genres,
            release_date: api.release_date,
            duration: api.duration,
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
    pub playlists: HashMap<String, HashSet<u64>>,
}

fn data_file() -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "arugula", "randeezbum").expect("Could not determine directory");

    let dir = proj_dirs.data_dir();
    std::fs::create_dir_all(dir).unwrap();

    dir.join("album.json")
}

pub fn load_state() -> AppState {
    let path = data_file();

    match File::open(path) {
        Ok(file) => serde_json::from_reader(file).unwrap(),

        Err(e) if e.kind() == ErrorKind::NotFound => AppState::default(),

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
