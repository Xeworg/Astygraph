-- ─────────────────────────────────────────────────────────────────────────────
-- Astynex Payload Schema v1
--
-- Eight tables for the analysis graph and cache layer, plus nine supporting
-- indexes.  All foreign keys are enforced via `PRAGMA foreign_keys = ON`.
-- One atomic migration step from user_version = 0 → user_version = 1.
-- ─────────────────────────────────────────────────────────────────────────────

-- 1. project_files  ─────────────────────────────────────────────────────────
--    Seen-file registry: only files actually opened, parsed, or reached as
--    direct context.  NOT a complete project inventory.
CREATE TABLE project_files (
    file_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    relative_path     TEXT    NOT NULL UNIQUE,
    last_observed_at  INTEGER NOT NULL
);

-- 2. file_dependencies  ────────────────────────────────────────────────────
--    Parser-observed file-level import/include edges.
--    NOT semantic call edges.  One row per observed edge per parser snapshot.
--    Completeness is parser-reported and may be partial; label it.
CREATE TABLE file_dependencies (
    source_file_id     INTEGER NOT NULL
                          REFERENCES project_files(file_id)
                          ON DELETE CASCADE,
    target_file_id     INTEGER NOT NULL
                          REFERENCES project_files(file_id)
                          ON DELETE CASCADE,
    relation_kind      TEXT    NOT NULL,
    source_digest      BLOB    NOT NULL,  -- 32-byte SHA-256 of source snapshot
    parser_fingerprint BLOB    NOT NULL,  -- 32-byte parse-fingerprint digest
    completeness       TEXT    NOT NULL,  -- "complete" | "partial" | "unsupported"
    PRIMARY KEY (source_file_id, target_file_id, parser_fingerprint)
);
CREATE INDEX ix_file_deps_source
    ON file_dependencies(source_file_id);
CREATE INDEX ix_file_deps_target
    ON file_dependencies(target_file_id);

-- 3. parser_snapshots  ────────────────────────────────────────────────────
--    Minimal structural facts extracted by the Tree-sitter adapter.
CREATE TABLE parser_snapshots (
    snapshot_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id               INTEGER NOT NULL
                            REFERENCES project_files(file_id)
                            ON DELETE CASCADE,
    source_digest         BLOB    NOT NULL,  -- 32-byte SHA-256 of byte snapshot
    parser_fingerprint    BLOB    NOT NULL,  -- 32-byte parse-fingerprint digest
    facts_schema_version TEXT    NOT NULL,
    minimal_facts_payload BLOB    NOT NULL,  -- versioned opaque envelope
    created_at           INTEGER NOT NULL,
    UNIQUE (file_id, source_digest, parser_fingerprint)
);

-- 4. analyses  ─────────────────────────────────────────────────────────────
--    One row per analysis of a root file/symbol query.
CREATE TABLE analyses (
    analysis_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    root_file_id         INTEGER NOT NULL
                            REFERENCES project_files(file_id)
                            ON DELETE CASCADE,
    query_identity       TEXT    NOT NULL,
    context_set_digest   BLOB    NOT NULL,  -- SHA-256 of ordered (file_id, digest) set
    ir_schema_version    TEXT    NOT NULL,
    analysis_fingerprint BLOB    NOT NULL,  -- SHA-256 over ordered inputs + settings
    state                TEXT    NOT NULL,  -- "candidate" | "current" | "stale" | "superseded"
    created_at           INTEGER NOT NULL,
    retired_at           INTEGER           -- NULL while current
);
CREATE INDEX ix_analyses_root
    ON analyses(root_file_id);
CREATE INDEX ix_analyses_query
    ON analyses(query_identity);

-- 5. analysis_inputs  ──────────────────────────────────────────────────────
--    Exact input membership for an analysis: ordered set of (file_id,
--    content_digest, role).
CREATE TABLE analysis_inputs (
    analysis_id    INTEGER NOT NULL
                        REFERENCES analyses(analysis_id)
                        ON DELETE CASCADE,
    file_id        INTEGER NOT NULL
                        REFERENCES project_files(file_id)
                        ON DELETE CASCADE,
    input_order    INTEGER NOT NULL,
    content_digest BLOB    NOT NULL,
    input_role    TEXT    NOT NULL,  -- "root" | "direct_context" | "transitive_dep"
    PRIMARY KEY (analysis_id, input_order),
    UNIQUE (analysis_id, file_id)
);
CREATE INDEX ix_analysis_inputs_file
    ON analysis_inputs(file_id);

-- 6. graph_nodes  ──────────────────────────────────────────────────────────
--    Validated AI-authored nodes scoped to an analysis snapshot.
--    Source spans are file-relative byte offsets validated against parser.
CREATE TABLE graph_nodes (
    node_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id    INTEGER NOT NULL
                        REFERENCES analyses(analysis_id)
                        ON DELETE CASCADE,
    source_file_id INTEGER NOT NULL
                        REFERENCES project_files(file_id)
                        ON DELETE CASCADE,
    kind           TEXT    NOT NULL,
    label          TEXT    NOT NULL,
    start_byte     INTEGER NOT NULL,
    end_byte       INTEGER NOT NULL,
    provenance     TEXT    NOT NULL,
    CHECK (end_byte > start_byte),
    UNIQUE (analysis_id, node_id)
);
CREATE INDEX ix_graph_nodes_source_file
    ON graph_nodes(source_file_id);

-- 7. graph_edges  ──────────────────────────────────────────────────────────
--    Validated edges scoped to an analysis snapshot.  Both endpoints must
--    exist in graph_nodes for the same analysis_id.  Composite FKs enforce
--    that both from_node_id and to_node_id belong to the same analysis.
CREATE TABLE graph_edges (
    edge_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id   INTEGER NOT NULL
                       REFERENCES analyses(analysis_id)
                       ON DELETE CASCADE,
    from_node_id  INTEGER NOT NULL,
    to_node_id    INTEGER NOT NULL,
    kind          TEXT    NOT NULL,
    label         TEXT,
    provenance    TEXT    NOT NULL,
    FOREIGN KEY (analysis_id, from_node_id)
        REFERENCES graph_nodes(analysis_id, node_id)
        ON DELETE CASCADE,
    FOREIGN KEY (analysis_id, to_node_id)
        REFERENCES graph_nodes(analysis_id, node_id)
        ON DELETE CASCADE
);
CREATE INDEX ix_graph_edges_from_node
    ON graph_edges(analysis_id, from_node_id);
CREATE INDEX ix_graph_edges_to_node
    ON graph_edges(analysis_id, to_node_id);

-- 8. diagnostics  ────────────────────────────────────────────────────────────
--    Per-analysis diagnostics.  Severity and code are extensible TEXT;
--    application logic validates and classifies.
CREATE TABLE diagnostics (
    diagnostic_id INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id  INTEGER NOT NULL
                        REFERENCES analyses(analysis_id)
                        ON DELETE CASCADE,
    code         TEXT    NOT NULL,
    message      TEXT    NOT NULL,
    severity     TEXT    NOT NULL
);
CREATE INDEX ix_diagnostics_analysis
    ON diagnostics(analysis_id);
