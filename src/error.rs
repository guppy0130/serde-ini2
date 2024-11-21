use std::fmt::{self, Display};

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Parse(Box<dyn std::error::Error>),
    UnsupportedType,
    TrailingCharacters,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Parse(error) => error.fmt(formatter),
            Error::UnsupportedType => formatter.write_str("Cannot serialize type"),
            Error::TrailingCharacters => formatter.write_str("Trailing characters"),
        }
    }
}

impl std::error::Error for Error {}
