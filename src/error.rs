use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    Abseil(abseil::Error),
    Config(String),
    Glob(glob::PatternError),
    Message(String),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<abseil::Error> for Error {
    fn from(value: abseil::Error) -> Self {
        Self::Abseil(value)
    }
}

impl From<glob::PatternError> for Error {
    fn from(value: glob::PatternError) -> Self {
        Self::Glob(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Json(e) => e.fmt(f),
            Error::Abseil(e) => e.fmt(f),
            Error::Config(e) => write!(f, "{e}"),
            Error::Glob(e) => e.fmt(f),
            Error::Message(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}
