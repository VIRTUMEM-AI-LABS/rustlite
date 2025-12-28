//! Error types for RustLite.

use std::fmt;

/// The main error type for RustLite operations.
#[derive(Debug)]
pub enum Error {
    /// A lock was poisoned (internal error)
    LockPoisoned,

    /// I/O error
    Io(std::io::Error),

    /// Serialization/deserialization error
    Serialization(String),

    /// Storage engine error
    Storage(String),

    /// Transaction error
    Transaction(String),

    /// Invalid operation
    InvalidOperation(String),

    /// Not found
    NotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::LockPoisoned => write!(f, "Lock poisoned"),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Error::Storage(msg) => write!(f, "Storage error: {}", msg),
            Error::Transaction(msg) => write!(f, "Transaction error: {}", msg),
            Error::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Error::NotFound => write!(f, "Not found"),
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
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

/// A specialized `Result` type for RustLite operations.
pub type Result<T> = std::result::Result<T, Error>;
