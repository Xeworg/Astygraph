# Persistence Payload Schema — ODD Task Mirror

Source of truth: `odd/tasks/persistence-payload-schema.md`

## Tasks
1. [x] Map current PRD, persistence APIs, and existing privacy/freshness contracts.
2. [x] Draft concrete schema, constraints, indexes, state transitions, and transaction/clear semantics in PRD §9.
3. [ ] Independently verify consistency and implementation boundaries; revise per findings.
4. [x] Design commit `8f23491 docs: define persistence payload schema`; identity record follows.

## Human-selected design constraints
- Versioned opaque parser-fact BLOB; explicit schema version; per-language shape deferred.
- Readable extensible TEXT for state, provenance, and relation kind; validate in application, no rigid enum CHECK constraints.
- Clear derived artifacts while preserving seen-file registry; full reset is separate.
- No source/migration code changes in this design task.

## Verification
- `git diff --check` passed.
- Independent read-only audit passed; prior 4 medium and 4 low findings are resolved, with no new findings.
- SQLite in-memory checks validated DDL syntax, FK/cascade behavior, composite endpoint integrity, and declared constraints.
- Docs-only: no Rust tests or migration implementation.
- Proposed schema only; runtime `user_version` remains 0. WAL/synchronous/multi-writer policy remains deferred.
