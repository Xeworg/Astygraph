//! SQLite connection lifecycle.
//!
//! Opens (or creates) the cache database for a project directory and
//! configures per-connection integrity settings.

use super::paths::cache_db_path;
use super::schema;
use super::Error;
use rusqlite::Connection;
use std::path::Path;

/// Bound busy timeout in milliseconds.
///
/// Set to a practical value that prevents SQLite BUSY errors during normal
/// concurrent access while remaining bounded so that genuine deadlocks do not
/// block indefinitely.  5 seconds is well within the 60-second upper bound
/// enforced by the connection test.
const BUSY_TIMEOUT_MS: isize = 5_000;

/// Opens (or creates) the SQLite cache database for `project_dir`.
///
/// The `.astynex/` directory is created lazily by [`cache_db_path`].
///
/// On every successful open this function:
///
/// 1. Creates the DB file if it does not exist.
/// 2. Enables `PRAGMA foreign_keys = ON`.
/// 3. Sets `PRAGMA busy_timeout = 5000`.
/// 4. Runs the ordered migration framework (advances `PRAGMA user_version`
///    atomically, or rolls back on failure).
///
/// Returns a borrowed `Connection` that the caller must manage.  The caller
/// is responsible for closing the connection (or letting it drop) when done.
///
/// # Errors
///
/// - [`Error::CacheDir`] if the project directory does not exist.
/// - [`Error::Rusqlite`] if the database cannot be opened or migrated.
/// - [`Error::Io`] if the `.astynex/` directory cannot be created.
///
pub fn open_cache_db(project_dir: &Path) -> Result<Connection, Error> {
    let db_path = cache_db_path(project_dir)?;

    // Open in default mode (read-write); SQLite creates the file if absent.
    let conn = Connection::open(&db_path)?;

    // ── Per-connection integrity settings ──────────────────────────────────

    // Enable foreign-key enforcement on every connection.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Bounded busy timeout: prevents indefinite blocking on locks while
    // remaining long enough for typical desktop concurrency patterns.
    conn.execute_batch(&format!("PRAGMA busy_timeout = {BUSY_TIMEOUT_MS};"))?;

    // ── Migration framework ────────────────────────────────────────────────
    // Run ordered migrations, advancing user_version atomically or rolling
    // back on failure.  Schema version 0 has no payload tables.
    schema::run_migrations(&conn)?;

    Ok(conn)
}
