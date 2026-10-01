# Persistence Payload Schema Design

## Objective
Turn the conceptual persistence model in PRD §9 into a reviewable, concrete SQLite payload-schema design without implementing migrations or writing payload tables.

## User decisions
- Parser-fact payload envelope: versioned opaque `BLOB`; keep `facts_schema_version` explicit. Concrete per-language fact shape remains deferred to vertical-slice tests.
- `state` and `provenance`: readable `TEXT`, validated by application logic; avoid rigid SQLite enum CHECK constraints that force a schema migration for every new value.
- `relation_kind`: extensible non-empty `TEXT`; adapter vocabulary can evolve without a closed cross-language enum.
- Clear cache: delete derived artifacts while preserving the seen-file registry; a full reset/forget operation is separate.

## Scope
- Specify concrete columns and SQLite affinities, nullability, keys, constraints, foreign-key delete behavior, and indexes for the conceptual tables in PRD §9.
- Specify the parser payload envelope, fingerprint storage, query identity, and closed-vs-extensible field validation.
- Define analysis-state semantics against Fresh/Stale/MissingInput revalidation outcomes.
- Define candidate replacement transaction ordering and row-level clear-cache behavior.
- Record platform-deferred durability/concurrency decisions and their Linux/Windows evidence gate.
- Update PRD §9 and keep this task record as the implementation handoff.

## Non-goals
- No Rust source changes, migration DDL, schema-version bump, DB payload tables, parser/analysis execution, UI, watcher, provider integration, or automatic retry.
- Do not define a universal cross-language parser-fact structure; only its versioned storage envelope is in scope.

## Tasks
1. [x] Map current PRD, persistence APIs, and existing privacy/freshness contracts.
2. [x] Specify concrete table schema, constraints, indexes, state transitions, and transaction/clear semantics in PRD §9.
3. [ ] Independently verify internal consistency and implementation boundaries; revise based on findings.
4. [x] Docs design committed as `8f23491 docs: define persistence payload schema`; evidence-record commit follows.

## Acceptance
- Every conceptual table has columns/types/nullability/keys/FKs and explicit delete behavior.
- Fingerprints remain separate for parse and analysis, stored as raw 32-byte values; source bytes, prompts, raw provider responses, and credentials remain excluded.
- Payload envelope is versioned BLOB; facts schema contents remain deferred.
- Extensible values remain readable TEXT with application validation, not closed CHECK enums.
- Fresh/Stale/MissingInput and candidate/current/stale/superseded semantics are consistent.
- Filesystem reads and hashing happen before short write transactions; replacement and clear operations are atomic and bounded.
- WAL/journal mode, synchronous level, and multi-writer recovery remain explicitly deferred pending Linux/Windows validation.
- No source or migration implementation is added in this design slice.

## Progress
- [x] Read-only mapping identified schema gaps and candidate documentation surfaces.
- [x] Human selected extensible storage choices listed above.
- [x] Concrete design and PRD §9 update drafted.
- [x] Independent verification completed; prior 4 medium and 4 low findings are resolved, with no new findings.
- [ ] Documentation work-unit commit pending.

## Design decisions recorded in PRD §9

| Topic | Decision |
|---|---|
| Tables | 8 proposed tables: `project_files`, `file_dependencies`, `parser_snapshots`, `analyses`, `analysis_inputs`, `graph_nodes`, `graph_edges`, `diagnostics` |
| Parser payload | Versioned opaque BLOB; explicit `facts_schema_version`; per-language payload shape deferred |
| Extensible values | Readable TEXT validated by application; no closed enum CHECK constraints |
| State | `candidate`, `current`, `stale`, `superseded`; runtime Fresh/Stale/MissingInput remain distinct |
| Clear-cache | Delete derived tables while preserving `project_files`; full database reset is separate |
| Platform | WAL/synchronous and multi-writer policy remain unselected pending Linux+Windows validation |

## Verification evidence
- `git diff --check` — passed.
- Independent read-only audit — PASS; confirmed prior 4 medium and 4 low issues resolved and no new findings.
- SQLite in-memory parse/insertion checks — valid DDL; FK cascades, composite endpoint integrity, CHECK/UNIQUE constraints, and extensible TEXT behavior confirmed.
- No Rust tests run: this is docs-only and no migration/schema code changed.
- Platform durability/concurrency choices remain deferred; this is a proposed schema, not the current runtime schema (current `user_version` remains 0).

## Commits
- Design: `8f23491 docs: define persistence payload schema`.
- Commit identity record: pending.

## Next step
The next implementation must be separately scoped and authorized; this design does not authorize migration implementation.
