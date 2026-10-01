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
2. [x] Compose parse and analysis fingerprints with stable ordered context set, path validation, and freshness comparison tests.
   - Added `src/persistence/fingerprint.rs` with `ParseFingerprint`, `AnalysisFingerprint`, `AnalysisInput`, and `DigestBytes`; separate 0x01/0x02 domains and canonical 8-byte big-endian length-prefixed encoding.
   - Parse identity includes normalized path, source digest, language, parser/grammar and facts-schema versions; provider/model are excluded by API and encoding.
   - Analysis identity includes root path+digest+role, symbol query, ordered distinct context paths+digests+roles, IR schema, provider/model, prompt-template version, and deterministically sorted output-affecting config; absent config differs from explicit empty config.
   - Path handling normalizes safe `.`/internal `..`, rejects root escapes, absolute Windows/POSIX context identities, symlink components, sibling-prefix escapes, directories, missing files, and non-UTF8 identities. Filesystem checks are point-in-time and not a security boundary.
   - Initial independent audit identified false coverage and implementation gaps; corrected them, added regressions, and removed the vacuous parse/provider test. One bounded worker launch timed out without edits; a retry completed.
   - Final independent verification: `cargo test --workspace --all-targets` — 279 passed, 0 failed; `cargo fmt --check`, `cargo clippy --lib --no-deps -- -D warnings`, `cargo clippy --test persistence_fingerprints --no-deps -- -D warnings`, and `git diff --check` all passed with no warnings. One ignored doc-test remains.
   - Windows was unavailable. Windows drive/UNC/backslash string rejection tests ran on Linux; Windows filesystem/symlink behavior remains unexecuted.
   - Native review outcome for this candidate is unknown (no consent/review closure); ASSESS was unassessable because the new files are untracked. Separate independent verification completed.
3. [ ] Independently verify, run checks, record evidence and work-unit commits.
   - Final independent verification: 279 passed, 0 failed; `cargo fmt --check`, lib clippy (`-D warnings`), fingerprint-test clippy (`-D warnings`), and `git diff --check` passed without warnings. One intentional ignored doc-test; Windows not run.
   - Native consent declined for this candidate. ASSESS returned `unassessable` because selected new source/test files remain untracked; the separate verifier completed the required independent verification. No native review closure is claimed.
   - Work-unit commit: `192dd97 feat: compose source freshness fingerprints`.

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
- [x] Task 2 — complete; independent audit findings corrected and final Linux verification passed (279 tests).
- [x] Task 3 — complete; independent verification recorded and work-unit commit `192dd97` created.

## Next step
Fingerprint composition is committed as `192dd97`; next continue with the separately scoped freshness validation task. Do not persist fingerprints yet.
