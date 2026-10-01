# Persistence Cache Design

## Objective
Specify the MVP's per-project SQLite cache as a privacy-conscious, file-oriented store for direct file dependencies and on-demand parser/analysis results, without persisting source bytes.

## Why
The cache is foundational to project graph correctness and must handle source changes during analysis without returning stale results or retaining source contents.

## Scope
- Distinguish the persistent, partial project-file dependency graph from the query-time semantic graph for an opened file/symbol and its bounded direct context.
- Specify freshness identity, race detection, and transactional replacement.
- Specify the conceptual relational schema, migration policy, recovery, and clear-cache semantics.
- Update PRD and multilanguage ODD record; documentation only, no runtime implementation.

## Decisions
- Persist only file records actually opened/parsed/reached as direct context; no eager project enumeration.
- Persist only parser-observed file-level dependencies such as validated imports/includes. Do not treat semantic call edges as project dependencies. Report partial coverage honestly.
- Read source into an in-memory snapshot; never store raw source bytes, prompts, provider responses, or credentials. Derived names/labels/diagnostics may contain source-derived text and are sensitive. Remote providers may receive submitted context; only a local provider path can avoid that transfer.
- Separate parse-cache identity from semantic-analysis identity. Parsing depends on content/path/language/grammar/parser/parser-facts schema. Semantic cache also depends on stable query identity, ordered context membership and digests, IR schema, provider/model, prompt template, and output-affecting settings.
- Before accepting a result, re-read and re-hash each input and revalidate direct-context membership. On change/missing input, discard candidate and preserve prior accepted snapshot. Retry once automatically; a second mismatch returns stale/cancelled status pending a new request. This is point-in-time freshness, not a lock against external writers. Revalidate every cache reuse.
- Refresh dependency sets only from files examined. Changed edge membership invalidates analyses that recorded the previous context-set digest. Invalidation is direct and lazy, not a transitive whole-project rebuild.
- `project_files` is a seen-file registry, not a claim of complete inventory. Query identity uses parser-derived file/symbol/range identity, not free-form prompt text. Source file identity accompanies byte spans.
- Use SQLite `PRAGMA user_version` for ordered migrations; enable and verify foreign keys on every connection. Migrations and analysis replacement are atomic; failed migration rolls back and never touches source. Incompatible parser/IR snapshots are invalidated for lazy rebuild.
- Keep safe SQLite durability defaults; use bounded busy timeout. Change concurrency/durability settings only with platform evidence.
- Clear-cache removes derived database/cache only, never project source, provider settings, or credentials.

## Conceptual schema
See `PRD.md` §9 for the diagram. Tables represent seen files, observed file dependencies, versioned minimal parser snapshots, analyses, analysis input membership/digests, analysis-scoped graph nodes/edges, and diagnostics. Parser-fact payload shape remains driven by Python/TypeScript vertical-slice conformance tests; the diagram is conceptual, not a universal parser schema.

## Tasks
1. [x] Reconcile graph scope and prior decisions; distinguish file dependency graph from on-demand semantic graph.
2. [x] Specify content fingerprints, race detection, and direct-context invalidation.
3. [x] Specify conceptual SQLite schema, migrations, transactions, retention/clear-cache, and update PRD/ODD records.
4. [x] Independently verify consistency, privacy claims, acceptance checks, and documentation diffs; record evidence and commit.

## Acceptance and checks
- PRD distinguishes persistent file dependency graph from query-time semantic analysis graph.
- No source bytes, prompts, raw provider response, or credentials are stored; remote provider transfer limitation is explicit.
- Analysis result/cache reuse requires matching content digests, parser/config versions, query identity, and ordered context membership.
- File changes or changed direct dependency membership make affected cached analysis stale without eager project-wide rebuild.
- Migration failure rolls back and never modifies source; recovery is explicit and scoped to derived artifacts.
- `git diff --check`; independent readback/verification; no code tests expected for docs-only changes.

## Progress
- [x] Reconciled earlier SQLite/file-cache choices with user-confirmed privacy and freshness requirements.
- [x] Incorporated independent read-only review findings on graph scope, race boundaries, invalidation, schema, and migration behavior.
- [x] Independent verifier reviewed the graph scope, fingerprint/cache validity, unavoidable external-writer race boundary, context membership, privacy, schema, and migrations; no blockers reported.
- [x] `git diff --check` passed. GUI/runtime tests are not applicable to this docs-only change; schema/migration behavior remains an implementation-time verification gate.
- [x] Work-unit commit: `5ec6a25 docs: specify privacy-safe SQLite cache design`.

## Next step
Documentation checks and independent review passed. Design and evidence recorded; the work-unit commit is `5ec6a25`.
