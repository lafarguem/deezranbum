mod album;
mod picker;
mod playlist;
mod queue;
mod replay;
mod session;
mod storage;
mod user;

use clap::{Parser, Subcommand, ValueEnum};

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
        query: Option<String>,

        /// Skip adding album to Deezer queue
        #[arg(long, value_enum, default_value_t = QueueBehaviours::True)]
        queue: QueueBehaviours,
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Session { command } => session::handle(command),
        Commands::Album { query, queue } => {
            let result = match query {
                None => album::pick(queue, cli.debug).await,
                Some(q) => match q.parse::<usize>() {
                    Ok(n) => album::next(n, queue, cli.debug).await,
                    Err(_) => album::search(&q, queue, cli.debug).await,
                },
            };
            if result.is_err() {
                println!("Error");
            }
        }
        Commands::Replay { from, to } => replay::replay(from, to),
        Commands::User { user_id } => match user::set(user_id) {
            Ok(()) => (),
            _ => println!("Error"),
        },
        Commands::Playlist { command } => {
            if playlist::handle(command, cli.debug).await.is_err() {
                println!("Error");
            }
        }
        Commands::Reset => storage::reset(),
    }
}
