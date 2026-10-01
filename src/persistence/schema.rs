//! Ordered schema migration framework.
//!
//! Manages the SQLite schema lifecycle via `PRAGMA user_version`.
//! Each migration is atomic: `user_version` is updated only after success;
//! on error the transaction is rolled back and the version stays unchanged.
//!
//! Current schema version: **0** (no payload tables).

use crate::persistence::error::FutureSchemaVersion;
use crate::persistence::Error;
use rusqlite::{Connection, Transaction};

/// Current schema version.
///
/// Version 0 represents the initial empty foundation: SQLite is initialized
/// with integrity pragmas but no Astynex tables exist yet.  Payload tables
/// will be added in later migrations under versioned schema upgrades.
pub const CURRENT_SCHEMA_VERSION: i32 = 0;

/// A single migration step: target schema version and the SQL to apply.
///
/// **Not public API.** Construct only in tests and integration tests
/// via `persistence::run_migrations_with_steps`.
#[derive(Debug, Clone, Copy)]
struct MigrationStep {
    /// The schema version after this step succeeds. Must be strictly greater
    /// than the previous step's target (or `from_version` for the first).
    pub target_version: i32,
    /// SQL statements to execute. May be empty for a no-op version bump.
    pub sql: &'static str,
}

/// Reads the current `PRAGMA user_version` value from `conn`.
fn current_version(conn: &Connection) -> Result<i32, Error> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0))?)
}

/// Sets `PRAGMA user_version` to `version` within the open transaction.
fn set_version(tx: &Transaction, version: i32) -> Result<(), Error> {
    tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
    Ok(())
}

/// Runs an ordered sequence of migration steps atomically.
///
/// The function:
/// 1. Begins a transaction.
/// 2. Computes pending steps from `from_version` to the last step's target.
/// 3. Executes each step's SQL in order, updating `user_version` after each.
/// 4. Commits the transaction.
///
/// If any step fails, the entire transaction rolls back: `user_version`
/// and all schema changes remain at their pre-call state.
///
/// Runs `run_migrations_with_steps` with the given steps, enforcing that
/// each step's `target_version` is exactly `current_version + 1`.
///
/// Private test hook for exercising rollback with injected migration steps; the
/// production API only exposes `run_migrations` through the internal module.
fn run_migrations_with_steps(
    conn: &Connection,
    from_version: i32,
    steps: &[MigrationStep],
) -> Result<(), Error> {
    let target = steps
        .last()
        .map(|s| s.target_version)
        .unwrap_or(from_version);

    if from_version > target {
        return Err(Error::FutureSchemaVersion(FutureSchemaVersion {
            found: from_version,
            supported: target,
        }));
    }

    let tx = conn.unchecked_transaction()?;

    let mut current = from_version;
    for step in steps {
        // Each step must advance by exactly one.
        let expected = current + 1;
        if step.target_version != expected {
            return Err(Error::FutureSchemaVersion(FutureSchemaVersion {
                found: step.target_version,
                supported: expected,
            }));
        }

        if !step.sql.is_empty() {
            tx.execute_batch(step.sql)?;
        }

        current = step.target_version;
        set_version(&tx, current)?;
    }

    tx.commit()?;
    Ok(())
}

// ─── public API ───────────────────────────────────────────────────────────────

/// Runs all pending migrations on `conn` atomically.
///
/// The function:
/// 1. Reads the current `user_version`.
/// 2. Begins a transaction.
/// 3. Computes pending migration steps from `user_version` to
///    `CURRENT_SCHEMA_VERSION`.
/// 4. Executes each step in order, updating `user_version` after each.
/// 5. Commits the transaction.
///
/// If any step fails, the entire transaction rolls back: `user_version`
/// and all schema changes remain at their pre-call state.
///
/// # Errors
///
/// - `FutureSchemaVersion` if the stored version is newer than supported.
/// - `Rusqlite` error wrapped in `Error::Rusqlite` on execution failure.
pub fn run_migrations(conn: &Connection) -> Result<(), Error> {
    let from = current_version(conn)?;

    if from > CURRENT_SCHEMA_VERSION {
        return Err(Error::FutureSchemaVersion(FutureSchemaVersion {
            found: from,
            supported: CURRENT_SCHEMA_VERSION,
        }));
    }

    let steps: [MigrationStep; 0] = [];
    run_migrations_with_steps(conn, from, &steps)
}

// ─── unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        Connection::open_in_memory().expect("in-memory DB must open")
    }

    #[test]
    fn current_version_default_is_zero() {
        let conn = mem_conn();
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

    #[test]
    fn current_version_reflects_user_setting() {
        let conn = mem_conn();
        conn.execute_batch("PRAGMA user_version = 7;").unwrap();
        assert_eq!(current_version(&conn).unwrap(), 7);
    }

    #[test]
    fn run_migrations_is_noop_when_already_at_target() {
        let conn = mem_conn();
        run_migrations(&conn).expect("run_migrations must not error at version 0");
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

    #[test]
    fn run_migrations_rejects_future_version() {
        let conn = mem_conn();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();

        let err = run_migrations(&conn).unwrap_err();
        assert!(
            matches!(err, Error::FutureSchemaVersion(ref fsv) if fsv.found == 99 && fsv.supported == 0),
            "must reject future version, got: {err}"
        );
    }

    #[test]
    fn run_migrations_with_steps_noop_on_empty_steps() {
        let conn = mem_conn();
        conn.execute_batch("PRAGMA user_version = 0;").unwrap();

        run_migrations_with_steps(&conn, 0, &[]).expect("empty steps must be a no-op");
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

    #[test]
    fn run_migrations_with_steps_advances_version() {
        let conn = mem_conn();
        conn.execute_batch("PRAGMA user_version = 0;").unwrap();

        let steps = vec![
            MigrationStep {
                target_version: 1,
                sql: "",
            },
            MigrationStep {
                target_version: 2,
                sql: "",
            },
        ];

        run_migrations_with_steps(&conn, 0, &steps).expect("steps must succeed");
        assert_eq!(current_version(&conn).unwrap(), 2);
    }

    #[test]
    fn run_migrations_with_steps_rollback_ddl_on_failure() {
        let conn = mem_conn();
        conn.execute_batch("PRAGMA user_version = 0;").unwrap();

        let steps = vec![
            MigrationStep {
                target_version: 1,
                sql: "CREATE TABLE IF NOT EXISTS test_rollback (id INTEGER PRIMARY KEY);",
            },
            MigrationStep {
                target_version: 2,
                sql: "INVALID SQL SYNTAX TO FORCE FAILURE",
            },
        ];

        let result = run_migrations_with_steps(&conn, 0, &steps);
        assert!(result.is_err(), "invalid SQL must cause migration to fail");

        // Version must roll back to 0.
        assert_eq!(
            current_version(&conn).unwrap(),
            0,
            "user_version must roll back to 0 after failed migration"
        );

        // Table must not exist (DDL rolled back).
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_rollback'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "DDL must roll back after failed migration");
    }

    #[test]
    fn run_migrations_with_steps_rejects_non_consecutive_version() {
        let conn = mem_conn();
        conn.execute_batch("PRAGMA user_version = 0;").unwrap();

        // Step 1 targets version 2 but current is 0 — must be exactly 1.
        let steps = vec![MigrationStep {
            target_version: 2,
            sql: "",
        }];

        let err = run_migrations_with_steps(&conn, 0, &steps).unwrap_err();
        assert!(
            matches!(err, Error::FutureSchemaVersion(ref fsv) if fsv.found == 2 && fsv.supported == 1),
            "non-consecutive version must be rejected: {err}"
        );
    }
}
