# Persistence Smoke Validation

## Goal
Exercise the current SQLite open and source snapshot APIs together, then obtain one independent implementation analysis and a second independent audit before proceeding to fingerprint composition.

## Scope
- Add a headless integration smoke test combining an on-disk sample source, in-memory SHA-256 snapshot, and project cache open.
- Verify expected digest, unchanged source, SQLite schema version 0, and no raw sample bytes in the DB file.
- First agent analyzes behavior/check coverage; a different second agent audits the test and analysis independently.
- No production code changes, fingerprint composition, provider requests, GUI work, or schema payload tables.

## Acceptance
- Exact integration test passes under `cargo test --workspace --all-targets`.
- First agent reports observed behavior and any limitations, without edits.
- Second agent independently audits test quality, privacy claim, and result; no blockers or explicitly recorded blockers.
- Windows availability is stated honestly.

## Tasks
1. [x] Add an end-to-end headless smoke test (test file only).
   - `smoke_snapshot_and_db_open` writes a unique fixture, hashes a memory snapshot, opens a fresh project DB, verifies unchanged source and `user_version=0`, and checks the DB file does not contain the sample bytes.
   - Focused `cargo test --test persistence_snapshot`: 14 passed; `git diff --check` passed. Parent readback removed one unused test import.
2. [x] Run the focused/full tests and analyze behavior with agent one.
   - `cargo test --test persistence_snapshot`: 14 passed; full `cargo test --workspace --all-targets`: 206 passed, 0 failed.
   - Agent independently verified the smoke payload digest and confirmed the exact source remained unchanged, fresh DB opened with `user_version=0`, and raw payload bytes were absent from that DB file.
   - Explicit limitation: this proves only the current smoke operation; no digest is persisted yet and it says nothing about future DB tables or provider network transmission.
   - Agent noted unused-import warnings in the focused test and a possibly unreliable Windows unreadable-file test branch; Windows was not available.
3. [x] Have a different agent audit the test/results.
   - Audit initially found the Windows unreadable-file test was logically broken, the digest reproduction command/comment needed correction, and the privacy statement should be narrowly scoped.
   - Corrected by compiling the permission test only on Unix, using a reproducible `printf %s` digest command, asserting exact `SnapshotRead` errors, removing unused imports and clippy issues, and limiting the DB assertion to fresh empty-schema open behavior.
   - Final independent audit found no blockers. Known limits: no Windows run (permission-denial case is skipped there), no payload-table writes are in scope, and the test does not make a general claim about future persistence or provider network behavior.
4. [x] Record evidence and next step; only then resume source-snapshot fingerprint implementation.
   - Focused suite: 14 passed on Linux. Full workspace suite: 206 passed, 0 failed. `cargo fmt --check`, `cargo clippy --test persistence_snapshot --no-deps -- -D warnings`, and `git diff --check` passed.
   - Earlier validation attempts caught and fixed compile/format/clippy failures; all listed final checks pass.
   - Native review consent was declined for this candidate; an independent verifier and separate audit completed, and the full checks passed.
   - Work-unit commit: `e58865b test: verify source snapshot and SQLite open together`.

## Current state
- Smoke test + first-agent analysis + separate independent audit are complete on Linux.
- The smoke proves only the exact fresh-empty-DB open operation: expected digest, source unchanged, DB version 0, fixture bytes absent from the resulting DB file.
- Fingerprint composition remains the next task and is not started until this validation work unit is closed.
