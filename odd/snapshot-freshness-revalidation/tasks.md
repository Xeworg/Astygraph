# Snapshot Freshness Revalidation — ODD Task Mirror

Source of truth: `odd/tasks/snapshot-freshness-revalidation.md`

## Tasks
1. [x] Add path-safe revalidation helpers for parse and analysis fingerprints.
2. [x] Add headless tests for fresh, stale, missing, unsafe paths, context membership/order, and output identity changes.
3. [x] Independently verify behavior and required checks.
4. [x] Implementation commit `a2f4981`; Windows evidence close-out commit `89a4a55 docs: record Windows freshness verification`.

## Evidence
- Focused tests: `cargo test --test persistence_freshness` — 30 passed.
- Workspace tests: `cargo test --workspace --all-targets` — 318 passed, 0 failed.
- `cargo fmt --check` — passed.
- `cargo clippy --lib --no-deps -- -D warnings` — passed.
- `cargo clippy --test persistence_freshness --no-deps -- -D warnings` — passed.
- `git diff --check` — passed.
- Independent verifier found no blockers. Linux only; Windows not run. Unix-only permission/symlink tests are cfg-gated.
- [x] Windows VM: `cargo test --workspace --all-targets` — 298 passed, 0 failed. Unix-only symlink/permission tests were cfg-gated and not run. Windows fmt/clippy not reported.
- No database/provider/application integration; source bytes are read in memory for digest recomputation only.
