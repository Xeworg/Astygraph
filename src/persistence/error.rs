//! Typed error boundary for the persistence layer.
//!
//! All errors surfaced by the persistence module are typed, never panics,
//! and carry sufficient context for user-facing diagnostics without leaking
//! credentials, raw source, or internal SQLite surface details to callers.

use std::error::Error as StdError;
use std::fmt::{self, Display};

/// Error returned when the database schema version is newer than the code
/// supports.  This prevents silent corruption from trying to run
/// forward-migrations on a database created by a newer binary.
#[derive(Debug)]
pub struct FutureSchemaVersion {
    /// The version stored in the database.
    pub found: i32,
    /// The maximum version this binary supports.
    pub supported: i32,
}

impl std::fmt::Display for FutureSchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "schema version {} is newer than this binary supports ({}); \
             the database was created by a newer version of astynex",
            self.found, self.supported
        )
    }
}

impl std::error::Error for FutureSchemaVersion {}

/// Typed errors that may escape the persistence module.
///
/// Variants cover the three distinct failure domains required by the MVP
/// foundation: SQLite database errors, filesystem I/O errors, and cache-
/// directory resolution failures.  Internal SQLite details are not re-exported.
#[derive(Debug)]
pub enum Error {
    /// A SQLite operation failed.  The inner error is boxed to keep
    /// `Error` size predictable across compilation targets.
    Rusqlite(Box<rusqlite::Error>),
    /// A filesystem I/O operation failed (open, create, metadata, etc.).
    Io(std::io::Error),
    /// The project cache directory could not be created or resolved.
    CacheDir(String),
    /// The database schema version is newer than the binary supports.
    /// This prevents silent corruption when a newer astynex created the DB.
    FutureSchemaVersion(FutureSchemaVersion),
}

// ─── conversions ──────────────────────────────────────────────────────────────

/// Converts a `rusqlite::Error` into a typed `Error::Rusqlite`.
impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusqlite(Box::new(err))
    }
}

/// Converts a `std::io::Error` into a typed `Error::Io`.
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

/// Converts a `FutureSchemaVersion` into a typed `Error::FutureSchemaVersion`.
impl From<FutureSchemaVersion> for Error {
    fn from(err: FutureSchemaVersion) -> Self {
        Error::FutureSchemaVersion(err)
    }
}

// ─── Display ──────────────────────────────────────────────────────────────────

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Rusqlite(err) => write!(f, "database error: {err}"),
            Error::Io(err) => write!(f, "I/O error: {err}"),
            Error::CacheDir(msg) => write!(f, "cache directory error: {msg}"),
            Error::FutureSchemaVersion(msg) => write!(f, "{msg}"),
        }
    }
}

// ─── std::error::Error ────────────────────────────────────────────────────────

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Rusqlite(err) => Some(err.as_ref() as &(dyn StdError + 'static)),
            Error::Io(err) => Some(err as &(dyn StdError + 'static)),
            Error::CacheDir(_) => None,
            Error::FutureSchemaVersion(err) => Some(err as &(dyn StdError + 'static)),
        }
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

impl Error {
    /// Constructs a `CacheDir` error with a descriptive message.
    #[track_caller]
    pub fn cache_dir(msg: impl Into<String>) -> Self {
        Error::CacheDir(msg.into())
    }
}
