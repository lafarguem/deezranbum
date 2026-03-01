mod album;
mod queue;
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

    /// Get random album
    Album {
        /// Number of albums to fetch (positional, defaults to 1)
        #[arg(default_value_t = 1)]
        amount: usize,

        /// Skip adding album to Deezer queue
        #[arg(long, value_enum, default_value_t = QueueBehaviours::True)]
        queue: QueueBehaviours,
    },

    /// Set user id
    User {
        user_id: String, // positional argument
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

    /// Remove album from session
    Remove {
        album_name: String, // positional argument
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Session { command } => session::handle(command),
        Commands::Album { queue, amount } => match album::next(amount, queue, cli.debug).await {
            Ok(()) => (),
            _ => println!("Error")
        },
        Commands::User { user_id } => match user::set(user_id) {
            Ok(()) => (),
            _ => println!("Error")
        },
        Commands::Reset => storage::reset(),
    }
}
