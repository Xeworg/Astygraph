# Snapshot Freshness Revalidation

## Objective
Provide a bounded, headless point-in-time check that re-reads the exact root and context files used for cached parse/analysis results and refuses reuse when the current fingerprint no longer matches.

## Why
Snapshot creation and fingerprint composition exist, but neither currently compares a previously recorded fingerprint with the current file contents. Cache reuse must fail closed when an input changes, disappears, or the ordered context membership changes.

## Scope
- Add parse and analysis revalidation helpers that re-snapshot only the explicit input paths supplied by the caller.
- Return a small typed outcome distinguishing fresh, stale, and missing input; invalid paths/read failures remain typed errors.
- Compare recomputed fingerprints using the existing canonical parse/analysis identity rules.
- Add headless tests for unchanged, changed, missing, reordered, and changed-membership inputs.
- No database reads/writes, cache payload storage, provider calls, UI, watcher, automatic retry, or application integration.

## Constraints
- Strict TDD runner: `cargo test --workspace --all-targets`.
- Validate project-relative identity and symlink policy before reading; use only caller-supplied root/context paths, never scan the project.
- Read source bytes into memory only; never log or persist them.
- A filesystem check is point-in-time, not a lock against external writers; bounded retry remains orchestration-owned.
- A mismatch is stale regardless of whether it came from content, membership/order, provider/model, schema, or settings; no need to expose every dimension in this primitive.
- Missing explicit input returns a missing-input outcome. Invalid/out-of-root/symlink paths and non-NotFound I/O errors return typed errors.

## Tasks
1. [ ] Add path-safe revalidation helpers for parse and analysis fingerprints; re-read only supplied inputs, rebuild current digests/fingerprints, and compare to recorded fingerprints.
2. [ ] Add integration tests for fresh unchanged inputs; changed/missing root; changed/missing context; context reorder/add/remove; verify mismatch blocks freshness and no source bytes are persisted.
3. [ ] Independently verify behavior and all required checks; record platform limits and evidence.
4. [x] Commit the work unit as `a2f4981`; record the implementation commit identity.

## Acceptance
- Unchanged exact bytes, versions/settings, and ordered input membership return Fresh.
- Any changed root/context bytes, missing explicit input, context membership/order change, or output-affecting identity change cannot return Fresh.
- Unsafe paths are rejected before reading; unreadable errors do not become Fresh.
- Only explicit paths are read; no project-wide scan, provider call, retry, or database write occurs.
- `cargo test --workspace --all-targets`, focused freshness tests, `cargo fmt --check`, library and focused-test clippy with `-D warnings`, and `git diff --check` pass.
- Windows status is explicit; do not claim a Windows run unless it occurred. Track Windows VM execution as a future verification task when the VM is available.

## Progress
- [x] Read prior source-snapshot task, PRD §9/cache design, current snapshot/fingerprint API and error boundary; read-only scout confirmed no stale/fresh comparison primitive exists.
- [x] Tasks 1–2 — implemented parse/analysis revalidation and focused tests, including symlink parent/dangling cases.
- [x] Task 3 — independent verification completed; no blockers reported.
- [ ] Task 4 — work-unit commit pending.

## Verification evidence
- `cargo test --test persistence_freshness` — 30 passed.
- `cargo test --workspace --all-targets` — 318 passed, 0 failed.
- `cargo fmt --check` — passed.
- `cargo clippy --lib --no-deps -- -D warnings` — passed.
- `cargo clippy --test persistence_freshness --no-deps -- -D warnings` — passed.
- `git diff --check` — passed.
- Independent Linux verifier confirmed unsafe-path rejection before freshness, NotFound distinction, no DB/provider/app integration, and fingerprint-dimension coverage. Linux: focused freshness 30 passed; workspace 318 passed; fmt, lib clippy, focused-test clippy, and diff-check passed.
- Windows VM: `cargo test --workspace --all-targets` — 298 passed, 0 failed. Unix-only symlink/permission tests are cfg-gated and were not run on Windows. Windows fmt/clippy were not reported.
- [x] Windows VM verification completed: `cargo test --workspace --all-targets` — 298 passed, 0 failed. Unix-only symlink/permission cases were cfg-gated and therefore not run on Windows.

## Commits
- Implementation: `a2f4981 feat: revalidate snapshot freshness`.
- Windows verification evidence: `89a4a55 docs: record Windows freshness verification`.

## Next step
Continue with the next separately scoped persistence feature: design the parser/analysis payload schema. Do not add database payload persistence until that scope is explicitly planned.
