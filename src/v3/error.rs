use std::fmt;

/// SeedLink `v3` protocol error.
#[derive(Debug, Clone)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR")
    }
}

impl std::error::Error for Error {}
