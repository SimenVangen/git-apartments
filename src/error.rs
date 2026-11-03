//! The crate's error type.

use std::fmt;

/// Everything that can go wrong between a GitHub response and a GIF.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The input was not the JSON (or color) this crate expects.
    Parse(String),
    /// GitHub answered with errors instead of data.
    Api(String),
    /// GitHub had no user by that login.
    NoSuchUser(String),
    /// The GIF could not be encoded.
    Encode(String),
    /// Writing the output failed.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(m) => write!(f, "could not read the activity: {m}"),
            Error::Api(m) => write!(f, "GitHub returned an error: {m}"),
            Error::NoSuchUser(l) => write!(f, "GitHub has no user called {l:?}"),
            Error::Encode(m) => write!(f, "could not encode the GIF: {m}"),
            Error::Io(e) => write!(f, "could not write the GIF: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}
