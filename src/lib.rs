use std::fmt;

pub mod sqlite;

#[derive(Clone)]
pub struct Feed {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub trait Storage {
    fn list_feeds(&self) -> Result<Vec<Feed>, Error>;
    fn add_feed(&self, url: String) -> Result<Feed, Error>;
    fn get_feed(&self, id: &str) -> Result<Feed, Error>;
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    Io(std::io::Error),
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => write!(f, "not found"),
            Error::Io(err) => write!(f, "{err}"),
            Error::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::Internal(value.to_string())
    }
}
