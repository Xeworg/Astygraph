# Persistence Payload Schema v1 — ODD Task Mirror

Source of truth: `odd/tasks/persistence-payload-schema-v1.md`

## Tasks
1. [x] Map migration framework and approved schema contract.
2. [x] Implement one atomic schema v1 migration and tests.
3. [x] Independently verify schema behavior and required checks.
4. [x] Commit implementation work unit and record identity.

## Scope decisions
- One v1 migration contains all eight PRD §9 tables and indexes.
- DDL lives in `src/persistence/migrations/v1.sql`, embedded with `include_str!`.
- No public API expansion, runtime payload APIs, or WAL/synchronous changes.

## Verification evidence
- Focused payload schema 29, foundation 11, snapshot 14; workspace 340 passed, 0 failed, rerun after the digest fixture correction.
- fmt, library clippy, focused migration-test clippy, foundation-test clippy, and diff-check passed.
- Independent audit confirmed migration shape/constraints and preserved versioning, rollback, privacy, and connection pragmas.
- Linux only; Windows validation for this new migration is pending.
- Implementation commit is local; no push was requested.
- Native targeted validation remains unavailable because no model is configured for `review-validator`.
- Implementation commit: `2cbd4fc87d9ab4acc049a29e6808e9a442e1fa37` (`feat: add persistence payload schema v1`).
