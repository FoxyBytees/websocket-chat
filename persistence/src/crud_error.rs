use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum CRUDError {
    NotFound,
    AlreadyExists
}

impl Display for CRUDError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CRUDError::NotFound => write!(f, "could not find entry"),
            CRUDError::AlreadyExists=> write!(f, "entry already exists"),
        }
    }
}

impl Error for CRUDError {
    /// Provides the underlying error that caused this `CRUDError`.
    /// The `source` method is a key part of the `std::error::Error` trait.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            _ => None,
        }
    }
}