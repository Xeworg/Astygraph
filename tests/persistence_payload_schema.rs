//! Integration tests for persistence payload schema v1.
//!
//! Covers: eight-table presence, index set, version bump, open/reopen idempotence,
//! PRAGMA foreign_keys / busy_timeout preservation, composite FK cross-analysis
//! rejection, cascade deletion, and CHECK / UNIQUE constraints.
//!
//! All tests use a fresh in-memory or temp-file DB so they are fully isolated.

use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Opens (or creates) the cache DB for `project_dir`, applying all migrations.
fn open_cache(project_dir: &Path) -> Result<Connection, astynex::persistence::Error> {
    astynex::persistence::open_cache_db(project_dir)
}

/// Constructs a fresh temporary project directory (no .astynex/ exists).
fn fresh_project_dir() -> TempDir {
    TempDir::new().expect("TempDir must create")
}

/// Reads `PRAGMA user_version` from an open connection.
fn read_user_version(conn: &Connection) -> i32 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version query must succeed")
}

/// Collects all table names from sqlite_master (excluding sqlite_* internal).
fn table_names(conn: &Connection) -> Vec<String> {
    conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .expect("prepare must succeed")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query_map must succeed")
        .filter_map(|r| r.ok())
        .filter(|t| !t.starts_with("sqlite_"))
        .collect()
}

/// Collects all index names from sqlite_master (excluding auto-created primary key indexes).
fn index_names(conn: &Connection) -> Vec<String> {
    conn.prepare("SELECT name FROM sqlite_master WHERE type='index'")
        .expect("prepare must succeed")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query_map must succeed")
        .filter_map(|r| r.ok())
        .filter(|t| !t.starts_with("sqlite_autoindex_"))
        .collect()
}

// ─── table and index presence ─────────────────────────────────────────────────

/// The eight approved tables from PRD §9 are present after migration.
#[test]
fn v1_has_eight_tables() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let tables = table_names(&conn);
    let expected = [
        "analyses",
        "analysis_inputs",
        "diagnostics",
        "file_dependencies",
        "graph_edges",
        "graph_nodes",
        "parser_snapshots",
        "project_files",
    ];
    let expected_len = expected.len();
    for name in expected {
        assert!(
            tables.contains(&name.to_string()),
            "table '{name}' must be present in v1 schema"
        );
    }
    assert_eq!(
        tables.len(),
        expected_len,
        "v1 must have exactly {} tables, found {tables:?}",
        expected_len
    );
}

/// The nine approved indexes from PRD §9 are present after migration.
#[test]
fn v1_has_nine_indexes() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let indexes = index_names(&conn);
    let expected = [
        "ix_analyses_query",
        "ix_analyses_root",
        "ix_analysis_inputs_file",
        "ix_diagnostics_analysis",
        "ix_file_deps_source",
        "ix_file_deps_target",
        "ix_graph_edges_from_node",
        "ix_graph_edges_to_node",
        "ix_graph_nodes_source_file",
    ];
    let expected_len = expected.len();
    for name in expected {
        assert!(
            indexes.contains(&name.to_string()),
            "index '{name}' must be present in v1 schema"
        );
    }
    assert_eq!(
        indexes.len(),
        expected_len,
        "v1 must have exactly {} indexes, found {indexes:?}",
        expected_len
    );
}

// ─── version bump ──────────────────────────────────────────────────────────────

/// Fresh database lands at user_version = 1 after migration.
#[test]
fn fresh_db_has_version_one() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    assert_eq!(
        read_user_version(&conn),
        1,
        "fresh DB must have user_version = 1"
    );
}

// ─── open/reopen idempotence ──────────────────────────────────────────────────

/// Reopening an existing v1 DB does not reapply migrations; version stays 1.
#[test]
fn reopen_idempotent_version_stays_one() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn1 = open_cache(&project).expect("first open must succeed");
    let v1 = read_user_version(&conn1);
    drop(conn1);

    let conn2 = open_cache(&project).expect("second open must succeed");
    let v2 = read_user_version(&conn2);

    assert_eq!(v1, 1, "version after first open must be 1");
    assert_eq!(v2, 1, "version after second open must still be 1");
}

/// Reopening does not duplicate tables or indexes.
#[test]
fn reopen_idempotent_no_duplicate_tables() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn1 = open_cache(&project).expect("first open must succeed");
    let tables_once = table_names(&conn1);
    drop(conn1);

    let conn2 = open_cache(&project).expect("second open must succeed");
    let tables_twice = table_names(&conn2);

    assert_eq!(
        tables_once, tables_twice,
        "table set must not change between first and second open"
    );
}

// ─── PRAGMA preservation ───────────────────────────────────────────────────────

/// Every v1 connection has foreign_keys = ON.
#[test]
fn v1_foreign_keys_enforced() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let fk_val: i32 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign_keys query must succeed");

    assert_eq!(fk_val, 1, "foreign_keys must be ON (1), got {fk_val}");
}

/// Every v1 connection has a bounded busy_timeout.
#[test]
fn v1_busy_timeout_is_bounded() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let timeout_ms: i32 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("busy_timeout query must succeed");

    assert!(
        timeout_ms > 0,
        "busy_timeout must be positive, got {timeout_ms}ms"
    );
    assert!(
        timeout_ms <= 60_000,
        "busy_timeout must be ≤ 60 000 ms, got {timeout_ms}ms"
    );
}

/// Reopening preserves existing FK enforcement setting.
#[test]
fn v1_reopen_preserves_foreign_keys() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn1 = open_cache(&project).expect("first open must succeed");
    drop(conn1);

    let conn2 = open_cache(&project).expect("second open must succeed");
    let fk_val: i32 = conn2
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign_keys query must succeed");

    assert_eq!(fk_val, 1, "foreign_keys must remain ON after reopen");
}

/// Reopening preserves existing busy_timeout setting.
#[test]
fn v1_reopen_preserves_busy_timeout() {
    let tmp = fresh_project_dir();
    let project = tmp.path().to_path_buf();

    let conn1 = open_cache(&project).expect("first open must succeed");
    let t1: i32 = conn1
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("busy_timeout query must succeed");
    drop(conn1);

    let conn2 = open_cache(&project).expect("second open must succeed");
    let t2: i32 = conn2
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("busy_timeout query must succeed");

    assert_eq!(t1, t2, "busy_timeout must remain unchanged after reopen");
}

// ─── graph_edges composite FK: cross-analysis rejection ───────────────────────

/// A graph_edge cannot reference a node belonging to a different analysis.
#[test]
fn graph_edge_rejects_cross_analysis_from_node() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    // Create two analyses with their respective nodes.
    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('b.py', 0);

        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q1', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (2, 'q2', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);

        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (1, 1, 'op', 'N1', 0, 4, 'AI');
        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (2, 2, 'op', 'N2', 0, 4, 'AI');
        ",
    )
    .expect("setup must succeed");

    // Attempt to create an edge from analysis-1 node to analysis-2 node.
    // The composite FK (analysis_id, from_node_id) requires from_node_id to
    // belong to the same analysis_id; analysis_id=1 but from_node_id=2 (which
    // belongs to analysis_id=2) must be rejected.
    let result = conn.execute(
        "INSERT INTO graph_edges (analysis_id, from_node_id, to_node_id,
                                  kind, provenance)
             VALUES (1, 2, 1, 'seq', 'AI')",
        [],
    );

    assert!(
        result.is_err(),
        "cross-analysis edge must be rejected by composite FK, got: {result:?}"
    );
}

/// A graph_edge cannot reference a node belonging to a different analysis (to_node).
#[test]
fn graph_edge_rejects_cross_analysis_to_node() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('b.py', 0);

        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q1', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (2, 'q2', X'0000000000000000000000000000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000000000000000000000000000', 'current', 0);

        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (1, 1, 'op', 'N1', 0, 4, 'AI');
        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (2, 2, 'op', 'N2', 0, 4, 'AI');
        ",
    )
    .expect("setup must succeed");

    // Attempt to create an edge where to_node belongs to analysis_id=2 but the
    // edge is declared with analysis_id=1.
    let result = conn.execute(
        "INSERT INTO graph_edges (analysis_id, from_node_id, to_node_id,
                                  kind, provenance)
             VALUES (1, 1, 2, 'seq', 'AI')",
        [],
    );

    assert!(
        result.is_err(),
        "cross-analysis to_node edge must be rejected by composite FK, got: {result:?}"
    );
}

// ─── cascade deletion ──────────────────────────────────────────────────────────

/// Deleting a project_file cascades to file_dependencies.
#[test]
fn file_dependencies_cascade_on_project_file_delete() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('x.py', 0);
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('y.py', 0);
        INSERT INTO file_dependencies
            (source_file_id, target_file_id, relation_kind,
             source_digest, parser_fingerprint, completeness)
            VALUES (1, 2, 'import',
                    X'0000000000000000000000000000000000000000',
                    X'0000000000000000000000000000000000000000',
                    'complete');
        ",
    )
    .expect("setup must succeed");

    let before: i32 = conn
        .query_row("SELECT COUNT(*) FROM file_dependencies", [], |row| {
            row.get(0)
        })
        .unwrap();

    conn.execute("DELETE FROM project_files WHERE file_id = 1", [])
        .expect("delete must succeed");

    let after: i32 = conn
        .query_row("SELECT COUNT(*) FROM file_dependencies", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(before, 1, "should have one edge before delete");
    assert_eq!(
        after, 0,
        "edge must be cascade-deleted when source file is removed"
    );
}

/// Deleting a project_file cascades to parser_snapshots.
#[test]
fn parser_snapshots_cascade_on_project_file_delete() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('x.py', 0);
        INSERT INTO parser_snapshots
            (file_id, source_digest, parser_fingerprint,
             facts_schema_version, minimal_facts_payload, created_at)
            VALUES (1,
                    X'0000000000000000000000000000000000000000',
                    X'0000000000000000000000000000000000000000',
                    '1', X'01', 0);
        ",
    )
    .expect("setup must succeed");

    let before: i32 = conn
        .query_row("SELECT COUNT(*) FROM parser_snapshots", [], |row| {
            row.get(0)
        })
        .unwrap();

    conn.execute("DELETE FROM project_files WHERE file_id = 1", [])
        .expect("delete must succeed");

    let after: i32 = conn
        .query_row("SELECT COUNT(*) FROM parser_snapshots", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(before, 1);
    assert_eq!(
        after, 0,
        "parser_snapshots must cascade-delete with project_file"
    );
}

/// Deleting an analysis cascades to analysis_inputs.
#[test]
fn analysis_inputs_cascade_on_analysis_delete() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO analysis_inputs
            (analysis_id, file_id, input_order, content_digest, input_role)
            VALUES (1, 1, 0, X'0000000000000000000000000000000000000000', 'root');
        ",
    )
    .expect("setup must succeed");

    conn.execute("DELETE FROM analyses WHERE analysis_id = 1", [])
        .expect("delete must succeed");

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM analysis_inputs", [], |row| row.get(0))
        .unwrap();

    assert_eq!(
        count, 0,
        "analysis_inputs must cascade-delete with analysis"
    );
}

/// Deleting an analysis cascades to graph_nodes.
#[test]
fn graph_nodes_cascade_on_analysis_delete() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (1, 1, 'op', 'N', 0, 4, 'AI');
        ",
    )
    .expect("setup must succeed");

    conn.execute("DELETE FROM analyses WHERE analysis_id = 1", [])
        .expect("delete must succeed");

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0, "graph_nodes must cascade-delete with analysis");
}

/// Deleting an analysis cascades to graph_edges.
#[test]
fn graph_edges_cascade_on_analysis_delete() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (1, 1, 'op', 'N1', 0, 4, 'AI');
        INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (1, 1, 'op', 'N2', 4, 8, 'AI');
        INSERT INTO graph_edges (analysis_id, from_node_id, to_node_id,
                                 kind, provenance)
            VALUES (1, 1, 2, 'seq', 'AI');
        ",
    )
    .expect("setup must succeed");

    conn.execute("DELETE FROM analyses WHERE analysis_id = 1", [])
        .expect("delete must succeed");

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM graph_edges", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0, "graph_edges must cascade-delete with analysis");
}

/// Deleting an analysis cascades to diagnostics.
#[test]
fn diagnostics_cascade_on_analysis_delete() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO diagnostics (analysis_id, code, message, severity)
            VALUES (1, 'E001', 'test diagnostic', 'error');
        ",
    )
    .expect("setup must succeed");

    conn.execute("DELETE FROM analyses WHERE analysis_id = 1", [])
        .expect("delete must succeed");

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM diagnostics", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0, "diagnostics must cascade-delete with analysis");
}

// ─── CHECK constraint: end_byte > start_byte ───────────────────────────────────

/// graph_nodes CHECK constraint rejects end_byte ≤ start_byte.
#[test]
fn graph_nodes_rejects_invalid_span() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        ",
    )
    .expect("setup must succeed");

    // end_byte == start_byte is invalid.
    let eq_result = conn.execute(
        "INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                  start_byte, end_byte, provenance)
             VALUES (1, 1, 'op', 'N', 5, 5, 'AI')",
        [],
    );
    assert!(
        eq_result.is_err(),
        "end_byte == start_byte must be rejected by CHECK constraint"
    );

    // end_byte < start_byte is invalid.
    let lt_result = conn.execute(
        "INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                  start_byte, end_byte, provenance)
             VALUES (1, 1, 'op', 'N', 10, 5, 'AI')",
        [],
    );
    assert!(
        lt_result.is_err(),
        "end_byte < start_byte must be rejected by CHECK constraint"
    );

    // end_byte > start_byte is valid.
    let gt_result = conn.execute(
        "INSERT INTO graph_nodes (analysis_id, source_file_id, kind, label,
                                  start_byte, end_byte, provenance)
             VALUES (1, 1, 'op', 'N', 0, 4, 'AI')",
        [],
    );
    assert!(
        gt_result.is_ok(),
        "end_byte > start_byte must be accepted, got: {gt_result:?}"
    );
}

// ─── UNIQUE constraints ───────────────────────────────────────────────────────

/// file_dependencies PK prevents duplicate (source, target, fingerprint) rows.
#[test]
fn file_dependencies_pk_prevents_duplicate() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('b.py', 0);
        INSERT INTO file_dependencies
            (source_file_id, target_file_id, relation_kind,
             source_digest, parser_fingerprint, completeness)
            VALUES (1, 2, 'import',
                    X'0000000000000000000000000000000000000000',
                    X'0000000000000000000000000000000000000000',
                    'complete');
        ",
    )
    .expect("setup must succeed");

    // Attempt to insert a duplicate (same source, target, fingerprint).
    let result = conn.execute(
        "INSERT INTO file_dependencies
             (source_file_id, target_file_id, relation_kind,
              source_digest, parser_fingerprint, completeness)
             VALUES (1, 2, 'import',
                     X'0000000000000000000000000000000000000000',
                     X'0000000000000000000000000000000000000000',
                     'partial')",
        [],
    );

    assert!(
        result.is_err(),
        "duplicate (source, target, fingerprint) must be rejected by PK"
    );
}

/// parser_snapshots UNIQUE prevents duplicate (file, digest, fingerprint).
#[test]
fn parser_snapshots_unique_prevents_duplicate() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO parser_snapshots
            (file_id, source_digest, parser_fingerprint,
             facts_schema_version, minimal_facts_payload, created_at)
            VALUES (1,
                    X'0000000000000000000000000000000000000000',
                    X'0000000000000000000000000000000000000000',
                    '1', X'01', 0);
        ",
    )
    .expect("setup must succeed");

    let result = conn.execute(
        "INSERT INTO parser_snapshots
             (file_id, source_digest, parser_fingerprint,
              facts_schema_version, minimal_facts_payload, created_at)
             VALUES (1,
                     X'0000000000000000000000000000000000000000',
                     X'0000000000000000000000000000000000000000',
                     '1', X'02', 1)",
        [],
    );

    assert!(
        result.is_err(),
        "duplicate (file_id, source_digest, parser_fingerprint) must be rejected"
    );
}

/// analysis_inputs PK prevents duplicate (analysis_id, input_order).
#[test]
fn analysis_inputs_pk_prevents_duplicate_order() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO analysis_inputs
            (analysis_id, file_id, input_order, content_digest, input_role)
            VALUES (1, 1, 0, X'0000000000000000000000000000000000000000', 'root');
        ",
    )
    .expect("setup must succeed");

    let result = conn.execute(
        "INSERT INTO analysis_inputs
             (analysis_id, file_id, input_order, content_digest, input_role)
             VALUES (1, 1, 0, X'0000000000000000000000000000000000000000', 'root')",
        [],
    );

    assert!(
        result.is_err(),
        "duplicate (analysis_id, input_order) must be rejected by PK"
    );
}

/// analysis_inputs UNIQUE prevents duplicate (analysis_id, file_id).
#[test]
fn analysis_inputs_unique_prevents_duplicate_file() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        INSERT INTO analysis_inputs
            (analysis_id, file_id, input_order, content_digest, input_role)
            VALUES (1, 1, 0, X'0000000000000000000000000000000000000000', 'root');
        ",
    )
    .expect("setup must succeed");

    // Same file at a different order is still a duplicate (file_id uniqueness).
    let result = conn.execute(
        "INSERT INTO analysis_inputs
             (analysis_id, file_id, input_order, content_digest, input_role)
             VALUES (1, 1, 1, X'0000000000000000000000000000000000000000', 'root')",
        [],
    );

    assert!(
        result.is_err(),
        "duplicate (analysis_id, file_id) must be rejected by UNIQUE constraint"
    );
}

/// graph_nodes UNIQUE prevents duplicate (analysis_id, node_id) pairs.
#[test]
fn graph_nodes_unique_prevents_duplicate_analysis_node_id() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        -- Manually insert node with explicit node_id to test UNIQUE.
        INSERT INTO graph_nodes (node_id, analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (5, 1, 1, 'op', 'N', 0, 4, 'AI');
        ",
    )
    .expect("setup must succeed");

    let result = conn.execute(
        "INSERT INTO graph_nodes (node_id, analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
             VALUES (5, 1, 1, 'op', 'N2', 4, 8, 'AI')",
        [],
    );

    assert!(
        result.is_err(),
        "duplicate (analysis_id, node_id) must be rejected by UNIQUE"
    );
}

// ─── NOT NULL constraints ─────────────────────────────────────────────────────

/// project_files.relative_path is NOT NULL.
#[test]
fn project_files_relative_path_not_null() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let result = conn.execute(
        "INSERT INTO project_files (relative_path, last_observed_at) VALUES (NULL, 0)",
        [],
    );
    assert!(result.is_err(), "relative_path NOT NULL must be enforced");
}

/// project_files.last_observed_at is NOT NULL.
#[test]
fn project_files_last_observed_at_not_null() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let result = conn.execute(
        "INSERT INTO project_files (relative_path, last_observed_at) VALUES ('a.py', NULL)",
        [],
    );
    assert!(
        result.is_err(),
        "last_observed_at NOT NULL must be enforced"
    );
}

// ─── project_files UNIQUE on relative_path ───────────────────────────────────

/// project_files.relative_path must be unique across rows.
#[test]
fn project_files_relative_path_unique() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute(
        "INSERT INTO project_files (relative_path, last_observed_at) VALUES ('a.py', 0)",
        [],
    )
    .expect("first insert must succeed");

    let result = conn.execute(
        "INSERT INTO project_files (relative_path, last_observed_at) VALUES ('a.py', 1)",
        [],
    );
    assert!(
        result.is_err(),
        "duplicate relative_path must be rejected by UNIQUE"
    );
}

// ─── FK on file_dependencies ──────────────────────────────────────────────────

/// file_dependencies FK rejects a non-existent source_file_id.
#[test]
fn file_dependencies_rejects_nonexistent_source() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let result = conn.execute(
        "INSERT INTO file_dependencies
             (source_file_id, target_file_id, relation_kind,
              source_digest, parser_fingerprint, completeness)
             VALUES (999, 1, 'import',
                     X'0000000000000000000000000000000000000000',
                     X'0000000000000000000000000000000000000000',
                     'complete')",
        [],
    );
    assert!(
        result.is_err(),
        "FK violation on non-existent source_file_id must be rejected"
    );
}

/// file_dependencies FK rejects a non-existent target_file_id.
#[test]
fn file_dependencies_rejects_nonexistent_target() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    let result = conn.execute(
        "INSERT INTO file_dependencies
             (source_file_id, target_file_id, relation_kind,
              source_digest, parser_fingerprint, completeness)
             VALUES (1, 999, 'import',
                     X'0000000000000000000000000000000000000000',
                     X'0000000000000000000000000000000000000000',
                     'complete')",
        [],
    );
    assert!(
        result.is_err(),
        "FK violation on non-existent target_file_id must be rejected"
    );
}

// ─── graph_nodes UNIQUE on (analysis_id, node_id) ────────────────────────────

/// A node with the same analysis_id and node_id cannot be inserted twice.
///
/// Note: `node_id` is `INTEGER PRIMARY KEY AUTOINCREMENT`, so SQLite assigns a
/// globally unique rowid — the composite `UNIQUE (analysis_id, node_id)` from
/// PRD §9 is therefore redundant but harmless. Both constraints enforce the
/// practical no-duplicate guarantee.
#[test]
fn graph_nodes_composite_unique_prevents_duplicate_analysis_node_id() {
    let tmp = fresh_project_dir();
    let conn = open_cache(tmp.path()).expect("open must succeed");

    conn.execute_batch(
        "
        INSERT INTO project_files (relative_path, last_observed_at)
            VALUES ('a.py', 0);
        INSERT INTO analyses (root_file_id, query_identity, context_set_digest,
                              ir_schema_version, analysis_fingerprint, state, created_at)
            VALUES (1, 'q', X'0000000000000000000000000000000000000000',
                    '1', X'0000000000000000000000000000000000000000', 'current', 0);
        -- Insert with explicit node_id.
        INSERT INTO graph_nodes (node_id, analysis_id, source_file_id, kind, label,
                                 start_byte, end_byte, provenance)
            VALUES (1, 1, 1, 'op', 'N1', 0, 4, 'AI');
        ",
    )
    .expect("setup must succeed");

    // Attempting to re-insert the same (node_id=1, analysis_id=1) pair fails
    // because PRIMARY KEY AUTOINCREMENT assigns globally unique rowids.
    let dup_result = conn.execute(
        "INSERT INTO graph_nodes (node_id, analysis_id, source_file_id, kind, label,
                                  start_byte, end_byte, provenance)
             VALUES (1, 1, 1, 'op', 'N1b', 4, 8, 'AI')",
        [],
    );
    assert!(
        dup_result.is_err(),
        "duplicate (analysis_id, node_id) must be rejected"
    );
}
