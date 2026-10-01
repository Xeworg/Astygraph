//! Integration tests for the persistence foundation.
//!
//! Covers: cache path resolution, SQLite lifecycle, connection settings,
//! migration atomicity, and version tracking — no payload tables yet.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Constructs a fresh temporary project directory (no .astynex/ exists).
fn fresh_project_dir() -> TempDir {
    TempDir::new().expect("TempDir must create")
}

/// Returns the cache DB path for a project directory.
fn cache_db_path(project_dir: &PathBuf) -> PathBuf {
    astynex::persistence::cache_db_path(project_dir).expect("cache_db_path must not panic")
}

/// Opens (creating if absent) the cache DB for a project dir.
fn open_cache(project_dir: &PathBuf) -> Result<rusqlite::Connection, astynex::persistence::Error> {
    astynex::persistence::open_cache_db(project_dir)
}

/// Reads `PRAGMA user_version` from an open connection.
fn read_user_version(conn: &rusqlite::Connection) -> i32 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version query must succeed")
}

// ─── cache_db_path ────────────────────────────────────────────────────────────

/// The cache DB lives at `<project>/.astynex/cache.db`.
#[test]
fn cache_db_path_under_project_dot_astynex() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let db_path = cache_db_path(&project);

    assert_eq!(
        db_path,
        project.join(".astynex").join("cache.db"),
        "cache DB must be at <project>/.astynex/cache.db"
    );
}

/// The .astynex directory is created lazily when the path is derived.
#[test]
fn cache_db_path_creates_dot_astynex() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();
    let dot_astynex = project.join(".astynex");

    assert!(
        !dot_astynex.exists(),
        ".astynex must not exist before resolution"
    );

    let _db_path = cache_db_path(&project);

    assert!(
        dot_astynex.is_dir(),
        ".astynex directory must be created by cache_db_path"
    );
}

/// A non-existent parent path surfaces a CacheDir error.
#[test]
fn parent_path_error() {
    let non_existent = PathBuf::from("/this/path/does/not/exist/anywhere/789xyz");
    let result = astynex::persistence::cache_db_path(&non_existent);
    assert!(
        matches!(result, Err(astynex::persistence::Error::CacheDir(_))),
        "non-existent parent must return Error::CacheDir, got {result:?}"
    );
}

// ─── open_cache_db — fresh / idempotent ───────────────────────────────────────

/// Opening a fresh project creates the DB with schema version 0.
#[test]
fn fresh_database_has_version_zero() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn = open_cache(&project).expect("open_cache_db must succeed on fresh dir");

    assert_eq!(
        read_user_version(&conn),
        0,
        "fresh DB must have user_version = 0"
    );
}

/// Opening the same project twice is idempotent; version stays 0.
#[test]
fn reopen_idempotent_version_stays_zero() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn1 = open_cache(&project).expect("first open must succeed");
    let v1 = read_user_version(&conn1);

    drop(conn1);

    let conn2 = open_cache(&project).expect("second open must succeed");
    let v2 = read_user_version(&conn2);

    assert_eq!(v1, 0, "version after first open must be 0");
    assert_eq!(v2, 0, "version after second open must be 0");
}

/// Reopening does not create additional .astynex directories.
#[test]
fn reopen_idempotent_no_extra_dir() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let _ = open_cache(&project).expect("first open must succeed");
    let _ = open_cache(&project).expect("second open must succeed");

    // Exactly one .astynex entry should exist directly under the project.
    let entries: Vec<_> = fs::read_dir(&project)
        .expect("read_dir must succeed")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .filter(|n| n == ".astynex")
        .collect();

    assert_eq!(
        entries.len(),
        1,
        "exactly one .astynex directory must exist after two opens"
    );
}

// ─── PRAGMA settings ──────────────────────────────────────────────────────────

/// Every returned connection has foreign_keys = ON.
#[test]
fn foreign_keys_enforced() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn = open_cache(&project).expect("open_cache_db must succeed");

    let fk_val: i32 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign_keys query must succeed");

    assert_eq!(fk_val, 1, "foreign_keys must be ON (1), got {fk_val}");
}

/// The busy_timeout pragma is bounded (positive and finite).
#[test]
fn busy_timeout_is_bounded() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn = open_cache(&project).expect("open_cache_db must succeed");

    let timeout_ms: i32 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("busy_timeout query must succeed");

    assert!(
        timeout_ms > 0,
        "busy_timeout must be positive, got {timeout_ms}ms"
    );
    // Bounded — well within practical limits (e.g. ≤ 60 s).
    assert!(
        timeout_ms <= 60_000,
        "busy_timeout must be ≤ 60 000 ms, got {timeout_ms}ms"
    );
}

// ─── migration atomicity ──────────────────────────────────────────────────────

/// Schema version 0 has no payload tables in production.
#[test]
fn schema_version_zero_has_no_payload_tables() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn = open_cache(&project).expect("open_cache_db must succeed");

    let version = read_user_version(&conn);
    assert_eq!(version, 0, "fresh DB schema version must be 0");

    // The DB must be readable — query the sqlite_master table.
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .expect("sqlite_master query must succeed");
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query_map must succeed")
        .filter_map(|r| r.ok())
        .collect();

    // Version 0 must have no Astynex payload tables.
    let payload_tables: Vec<String> = tables
        .into_iter()
        .filter(|t| !t.starts_with("sqlite_"))
        .collect();

    assert!(
        payload_tables.is_empty(),
        "schema version 0 must have no payload tables, found: {payload_tables:?}"
    );
}

/// Attempting to run a migration when the stored version is newer than
/// supported fails clearly rather than silently accepting.
#[test]
fn future_schema_version_fails_clearly() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn = open_cache(&project).expect("open_cache_db must succeed");

    // Simulate a DB created by a newer binary: set version to 99.
    conn.execute_batch("PRAGMA user_version = 99;")
        .expect("setting user_version must succeed");

    // Re-opening should detect the future version and fail clearly.
    let result = astynex::persistence::open_cache_db(&project);
    assert!(
        matches!(result, Err(astynex::persistence::Error::FutureSchemaVersion(ref fsv))
            if fsv.found == 99 && fsv.supported == 0),
        "future version must return FutureSchemaVersion error, got: {result:?}"
    );
}

/// `cache_db_path` does not mutate the filesystem when the project dir
/// already contains .astynex (no-op on existing directory).
#[test]
fn cache_db_path_noop_on_existing_dot_astynex() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    // Pre-create .astynex directory.
    let dot_astynex = project.join(".astynex");
    fs::create_dir(&dot_astynex).expect("pre-create .astynex must succeed");

    // cache_db_path must not fail.
    let db_path = cache_db_path(&project);
    assert_eq!(
        db_path,
        project.join(".astynex").join("cache.db"),
        "cache_db_path must return correct path"
    );

    // .astynex must still be a directory (not a file).
    let meta = fs::metadata(&dot_astynex).expect("metadata must succeed");
    assert!(
        meta.is_dir(),
        ".astynex must remain a directory after cache_db_path"
    );
}
