use std::{fmt, io};

use crate::note::ParseInlineError;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parse(ParseInlineError),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ParseInlineError> for Error {
    fn from(value: ParseInlineError) -> Self {
        Self::Parse(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => e.fmt(f),
            Error::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
