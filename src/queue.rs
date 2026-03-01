#[cfg(target_os = "macos")]
#[path = "queue_macos.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "queue_linux.rs"]
mod imp;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod imp {
    use std::fmt;
    use crate::storage::Album;

    #[derive(Debug)]
    pub enum QueueError {
        ScriptError(String),
    }

    impl fmt::Display for QueueError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let QueueError::ScriptError(msg) = self;
            write!(f, "{msg}")
        }
    }

    impl std::error::Error for QueueError {}

    pub async fn add_to_queue(_album: &Album, _debug: bool) -> Result<(), QueueError> {
        Err(QueueError::ScriptError(
            "queue is not supported on this platform".to_string(),
        ))
    }
}

pub use imp::*;
