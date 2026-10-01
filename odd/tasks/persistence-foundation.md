# SQLite Persistence Foundation

## Objective
Create a headless, project-local SQLite persistence foundation that opens `.astynex/cache.db`, enforces connection integrity settings, and applies ordered transactional schema migrations without coupling storage to UI or source files.

## Why
The approved cache design needs a reliable storage lifecycle before concrete parser/analysis records can be added. This foundation must not persist or mutate project source.

## Scope
- Add `rusqlite` with `bundled` and a typed persistence error boundary.
- Add path resolution, cache-directory creation, SQLite open/configuration, and `PRAGMA user_version` migration framework.
- Add isolated integration tests; no source hashing, parser/analysis schema, cache UI, AI/provider integration, or graph integration yet.
- Keep `.astynex/` excluded from discovery (already true); consider Git ignore as a separately scoped companion only if needed.

## Constraints
- Follow `PRD.md` §9 and `odd/tasks/persistence-cache-design.md`.
- Store no source bytes, prompts, raw provider responses, or credentials.
- Enable and verify foreign keys per connection; bounded busy timeout; keep safe SQLite journaling/durability defaults.
- Migration/open errors must return typed errors, never panic or modify source.
- Linux/Windows are target platforms; tests remain headless. Windows execution may be unavailable in this session and must be reported honestly.
- Strict TDD runner: `cargo test --workspace --all-targets`.

## Tasks
1. [x] Add pinned `rusqlite` dependency and typed persistence error/module surface, tested headlessly.
   - Selected locally cached `rusqlite 0.32.1` with `bundled`; no wildcard dependency range.
   - TDD RED: `cargo test --workspace --all-targets` failed to compile because the public persistence module/error API was absent.
   - GREEN: same runner passed, 164 tests total including 6 persistence error tests; `cargo fmt --check` and `git diff --check` passed.
   - Surface: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/persistence/{mod.rs,error.rs}`, `tests/persistence_errors.rs`.
   - Windows execution not available in this session.
   - Work-unit commit: `0c9fc35 feat: add SQLite persistence error foundation`.
2. [ ] Implement project cache path/bootstrap, open lifecycle, connection pragmas, and atomic versioned migration framework; test fresh/reopen/foreign-key/error/rollback behavior.
3. [ ] Independently verify the implementation, run configured tests/checks, record commits and platform limitations.

## Acceptance
- The library exposes a small `persistence` module with path/open/error APIs, not leaking SQLite into UI/graph layers.
- Opening creates only `<project>/.astynex/cache.db`; repeat open is idempotent; `user_version` reports the current schema version.
- Every returned connection has foreign-key enforcement enabled and a bounded busy timeout.
- Ordered migrations run atomically and only advance `user_version` on success; migration failure rolls back.
- Invalid project/cache paths return errors without panic; no source file is created, modified, or copied into the database.
- `cargo test --workspace --all-targets`, formatting, targeted clippy, and `git diff --check` pass; Windows check is explicitly reported.

## Progress
- [x] Read-only implementation mapping completed; no existing persistence module or rusqlite dependency exists; `.astynex/` already excluded from scanner.
- [x] Task 1 — complete; writer-reported TDD checks passed.
- [ ] Task 2 — in progress.
- [ ] Task 3 — pending.

## Next step
Implement task 2 test-first: path creation, connection setup, and migration behavior. Preserve PRD boundaries; do not add cache payload tables yet.
