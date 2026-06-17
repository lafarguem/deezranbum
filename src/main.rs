mod album;
mod completion;
mod error;
mod picker;
mod playlist;
mod queue;
mod replay;
mod session;
mod stats;
mod storage;
mod user;

use chrono::NaiveDate;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::ArgValueCandidates;

use crate::{album::AlbumFilters, error::AppResult};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Print debug output
    #[arg(long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "lower")]
pub enum QueueBehaviours {
    False,
    Ask,
    True,
}

fn parse_partial_date(s: &str) -> Result<NaiveDate, String> {
    let parts: Vec<&str> = s.split('-').collect();
    let (y, m, d) = match parts.as_slice() {
        [y] => (*y, "1", "1"),
        [y, m] => (*y, *m, "1"),
        [y, m, d] => (*y, *m, *d),
        _ => return Err(format!("invalid date: {s}")),
    };

    let year: i32 = y.parse().map_err(|_| format!("invalid year: {y}"))?;
    let month: u32 = m.parse().map_err(|_| format!("invalid month: {m}"))?;
    let day: u32 = d.parse().map_err(|_| format!("invalid day: {d}"))?;

    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| format!("invalid date: {s}"))
}

fn parse_duration(s: &str) -> Result<u64, String> {
    let mut total = 0u64;
    let mut num = 0u64;
    let mut pending = false; // have digits waiting for a unit?

    for c in s.chars() {
        match c {
            '0'..='9' => {
                num = num * 10 + (c as u64 - '0' as u64);
                pending = true;
            }
            'h' => {
                total += num * 3600;
                num = 0;
                pending = false;
            }
            'm' => {
                total += num * 60;
                num = 0;
                pending = false;
            }
            's' => {
                total += num;
                num = 0;
                pending = false;
            }
            _ => return Err(format!("invalid character '{c}' in duration")),
        }
    }

    if pending {
        return Err(format!("number with no unit in: {s}"));
    }
    Ok(total)
}

#[derive(Subcommand)]
enum Commands {
    /// Session-related commands
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    /// Pick album(s). No arg → interactive picker; number → that many random; string → best fuzzy match.
    Album {
        /// Number of albums (e.g. `3`) or a fuzzy search query (e.g. `michael jackson`). Omit for interactive picker.
        #[arg(add = ArgValueCandidates::new(completion::album_titles))]
        query: Option<String>,

        /// Skip adding album to Deezer queue
        #[arg(long, value_enum, default_value_t = QueueBehaviours::True)]
        queue: QueueBehaviours,

        #[arg(long, value_parser = parse_partial_date)]
        before: Option<NaiveDate>,

        #[arg(long, value_parser = parse_partial_date)]
        after: Option<NaiveDate>,

        #[arg(long, value_parser = parse_duration)]
        min_duration: Option<u64>,

        #[arg(long, value_parser = parse_duration)]
        max_duration: Option<u64>,

        #[arg(long, add = ArgValueCandidates::new(completion::genres))]
        genre: Vec<String>,

        #[arg(long, add = ArgValueCandidates::new(completion::genres))]
        exclude_genre: Vec<String>,

        #[arg(long, add = ArgValueCandidates::new(completion::artists))]
        artist: Vec<String>,

        #[arg(long, add = ArgValueCandidates::new(completion::artists))]
        exclude_artist: Vec<String>,
    },

    /// Replay albums from the session
    Replay {
        /// Starting session index
        from: Option<usize>,

        // Ending session index
        to: Option<usize>,
    },

    /// Set user id
    User {
        user_id: String, // positional argument
    },

    // Manage and play playlists
    Playlist {
        #[command(subcommand)]
        command: PlaylistSubcommands,
    },

    /// Reset everything
    Reset,

    /// Fetch and update all album metadata and redirects
    Fetch,

    /// Show stats
    Stats,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Clear current session
    Clear,

    /// Show session history
    History,

    /// Remove album(s) from session. No arg → interactive picker; string → fuzzy best match.
    Remove {
        /// Fuzzy search query. Omit for the interactive picker.
        album_name: Option<String>,
    },
}

#[derive(Subcommand)]
enum PlaylistSubcommands {
    /// Edit or create a playlist
    Edit {
        /// Name of the playlist
        name: String,
    },

    /// List all playlists or albums in playlist
    List {
        /// Name of the playlist, Omit to list all playlists
        name: Option<String>,
    },

    /// Delete a playlist
    Delete {
        /// Name of the playlist to delete
        name: String,
    },

    /// Play a playlist
    Play {
        /// Name of the playlist to play
        name: String,

        /// Number of albums to play from the playlist. Omit for all albums
        number: Option<usize>,

        /// Skip adding album to Deezer queue
        #[arg(long, value_enum, default_value_t = QueueBehaviours::True)]
        queue: QueueBehaviours,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> AppResult<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();

    match cli.command {
        Commands::Session { command } => session::handle(command)?,
        Commands::Album {
            query,
            queue,
            after,
            before,
            min_duration,
            max_duration,
            genre,
            exclude_genre,
            artist,
            exclude_artist,
        } => {
            let filters = &AlbumFilters {
                after,
                before,
                min_duration,
                max_duration,
                genre,
                exclude_genre,
                artist,
                exclude_artist,
            };
            match query {
                None => album::pick(queue, cli.debug, filters).await,
                Some(q) => match q.parse::<usize>() {
                    Ok(n) => album::next(n, queue, cli.debug, filters).await,
                    Err(_) => album::search(&q, queue, cli.debug).await,
                },
            }?
        }
        Commands::Replay { from, to } => replay::replay(from, to)?,
        Commands::User { user_id } => user::set(user_id)?,
        Commands::Playlist { command } => playlist::handle(command, cli.debug).await?,
        Commands::Reset => storage::reset(),
        Commands::Fetch => {
            let mut state = storage::load_state()?;
            album::get_albums(&mut state, true).await?;
            storage::save_state(&state)?;
            println!("Library refreshed.");
        }
        Commands::Stats => stats::general()?,
    }

    Ok(())
}
