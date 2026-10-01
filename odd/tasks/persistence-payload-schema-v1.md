# Persistence Payload Schema v1 Implementation

## Objective
Implement the approved proposed SQLite payload schema as the initial v1 migration, preserving the v0 foundation guarantees and adding no runtime payload behavior.

## Scope
- Add one atomic v1 migration containing all eight approved tables and their indexes/constraints from PRD §9.
- Advance fresh databases from `user_version = 0` to `1`; reopen is idempotent.
- Keep foreign-key enforcement and busy timeout unchanged. Do not select WAL/synchronous overrides.
- Add migration integration tests for table/index shape, versioning, idempotence, and representative foreign-key/constraint behavior.
- Keep source bytes, prompts, raw provider responses, and credentials absent from schema.

## Non-goals
- No parser/analysis payload read/write APIs, Rust domain models, application integration, provider calls, UI, watcher, concurrency policy, WAL/synchronous choice, or payload-size policy.
- No additional migration versions or schema redesign beyond the committed PRD §9 contract unless tests uncover a necessary inconsistency; escalate scope-changing findings.

## Implementation decisions
- One migration step to schema v1: this is the first payload schema and all eight related tables form one atomic initial shape; intermediate partial schemas are not shipped.
- Store DDL in a dedicated `src/persistence/migrations/v1.sql` file and embed with `include_str!` for readable review while keeping migration SQL static.
- Keep migration framework and schema version module private; no public API expansion.
- Update the foundation integration expectations from empty v0 fresh DB to the v1 fresh schema; retain lower-level migration rollback/version-transition unit coverage.

## Tasks
1. [x] Map existing migration framework, schema design, and foundation tests.
2. [x] Implement migration v1 and focused migration coverage test-first.
3. [x] Independently verify all required checks and schema integrity.
4. [x] Commit implementation work unit and record identity.

## Acceptance
- Fresh DB is at `user_version = 1` with all eight tables and specified indexes.
- Reopening does not reapply or mutate schema; existing foreign_keys and busy_timeout settings remain active.
- Composite graph-edge FKs prevent edges from joining nodes across different analyses; cascades and range/uniqueness constraints behave as designed.
- Migration is atomic and existing rollback/future-version tests remain valid.
- `cargo test --workspace --all-targets`, focused persistence tests, `cargo fmt --check`, library/focused-test clippy with `-D warnings`, and `git diff --check` pass.
- No schema choice previously deferred in PRD §9 is silently implemented.

## Verification evidence
- `cargo test --test persistence_payload_schema` — 29 passed.
- `cargo test --test persistence_foundation` — 11 passed.
- `cargo test --test persistence_snapshot` — 14 passed.
- `cargo test --workspace --all-targets` — 340 passed, 0 failed (rerun after correction).
- `cargo fmt --check`, `cargo clippy --lib --no-deps -- -D warnings`, `cargo clippy --test persistence_payload_schema --no-deps -- -D warnings`, `cargo clippy --test persistence_foundation --no-deps -- -D warnings`, and `git diff --check` — passed.
- Independent verifier confirmed 8 tables, 9 indexes, FK/cascade/composite-key constraints, migration rollback/idempotence/future-version behavior, privacy exclusions, and no WAL/synchronous override.
- Linux only for this migration; Windows validation is pending. Existing user-reported Windows suite predates the new migration.
- Native targeted validator could not run because no model is configured for `review-validator`. The one-line fixture correction was independently covered by the full rerun, but native review remains unclosed.
- Implementation commit: `2cbd4fc87d9ab4acc049a29e6808e9a442e1fa37` (`feat: add persistence payload schema v1`).

## Next
Implementation work unit committed locally; no push requested. Windows validation and native targeted review remain pending.
