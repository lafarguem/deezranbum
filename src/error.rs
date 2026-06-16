use std::fmt;

/// Unified error type for the application's command handlers.
#[derive(Debug)]
pub enum AppError {
    /// A request to the Deezer API failed.
    Network(reqwest::Error),
    /// Reading/writing local state or terminal I/O failed.
    Io(std::io::Error),
    /// Not enough albums in the library match the requested criteria.
    NotEnoughAlbums { found: usize, requested: usize },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Network(e) => write!(f, "could not reach Deezer: {e}"),
            AppError::Io(e) => write!(f, "i/o error: {e}"),
            AppError::NotEnoughAlbums { found, requested } => write!(
                f,
                "only {found} album(s) match the given criteria, but {requested} were requested"
            ),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Network(e) => Some(e),
            AppError::Io(e) => Some(e),
            AppError::NotEnoughAlbums { .. } => None,
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
