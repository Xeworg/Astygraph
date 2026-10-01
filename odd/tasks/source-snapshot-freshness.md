# Source Snapshot Freshness

## Objective
Provide deterministic in-memory source snapshots and cache fingerprints that detect stale input before derived analysis is reused or published.

## Why
Project files can change during analysis. Persistent derived results must not be represented as current when their exact source/context snapshot changed, and source bytes must never be written to SQLite.

## Scope
- Add SHA-256 source snapshot/fingerprint primitives, revalidation, and focused headless tests.
- Define separate parse and analysis fingerprints, including ordered context membership and output-affecting versions/settings.
- No parser/AI integration, persistent payload tables, provider calls, UI, or source writes.

## Constraints
- Follow `PRD.md` §9 and `odd/tasks/persistence-cache-design.md`.
- Read each file's bytes into memory for hashing; do not persist them.
- Use normalized project-relative identity and validate paths remain within project root; no symlink following.
- Separate parser fingerprint dimensions from semantic-analysis dimensions (provider/model/prompt-template only for semantic cache).
- Revalidate all inputs and context membership before accepting; changed/missing inputs are stale. Bounded retry policy belongs to orchestration and remains outside this primitive.
- Strict TDD runner: `cargo test --workspace --all-targets`.

## Tasks
1. [x] Add deterministic file snapshot/digest representation and focused tests (known vector, same content, changed content, missing/unreadable file).
   - Added SHA-256 snapshots over exact file bytes held in memory; no database writes.
   - Corrected tests to the standard SHA-256(`abc`) vector; empty input retains its distinct correct digest.
   - Added tests for repeated million-byte vector, identical/different bytes, file round-trip, binary bytes, and missing/unreadable paths. Unix-only permission APIs are cfg-gated; Windows test handling is platform-guarded.
   - TDD RED: known-vector and round-trip tests failed because expected digest was incorrectly the empty-input digest.
   - GREEN: `cargo test --workspace --all-targets` — 205 passed, 0 failed; `cargo fmt --check`, `cargo clippy --lib --no-deps -- -D warnings`, and `git diff --check` passed.
   - Independent verification: 205 tests passed; formatting, clippy, and diff checks passed. Native review closed approved; non-blocking suggestions recorded, and one critical suggestion was refuted by the provider refuter.
   - Work-unit commit: `77ba039 feat: add SHA-256 source snapshots`.
2. [ ] Add parse and analysis fingerprint composition with stable ordered context set, path validation, and freshness comparison tests.
3. [ ] Independently verify, run checks, record evidence and work-unit commits.

## Acceptance
- SHA-256 digest is computed over exact bytes used by the caller; identical bytes yield identical digests.
- Parser fingerprint excludes AI provider/model settings; semantic fingerprint includes provider/model, prompt/template/schema/config and ordered root/context file IDs/digests.
- Missing, changed, reordered/changed-membership context inputs are not considered fresh.
- Path identity is normalized relative to project root and rejects escapes/symlinks.
- No code path writes source bytes or snapshots to SQLite.
- Full tests, formatting, targeted clippy, and diff checks pass; Windows status is explicit.

## Progress
- [x] Persistence foundation and cache contract read; `sha2 0.10.9` is available in local Cargo cache.
- [x] Task 1 — complete; agent recovered, fixes applied, and all Linux checks passed.
- [ ] Task 2 — pending authorization to continue.
- [ ] Task 3 — pending.

## Next step
Next after user confirms continuation: compose separate parse and semantic-analysis fingerprints with normalized project-relative input identities and ordered context membership. Do not persist fingerprints yet.
