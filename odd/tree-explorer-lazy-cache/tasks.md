# Tree Explorer Lazy Cache — Task 1

## Feature: Headless Lazy Tree Cache

### Goal
Implement a lazily-loaded tree cache with a global 50,000-entry budget for the file tree browser.

### Constraints
- Budget: 50,000 entries total (root direct entries + all cached descendants)
- Root: direct entries visible on project open; child directories start collapsed
- On-demand: directory children load only on expansion
- Cache reuse: collapse/re-expand retains cached children
- Incomplete state: visible cap/limit state, not silent truncation
- Selection preserved: source selection survives expansion/collapse
- Project replacement: clears tree state; picker cancellation preserves it
- Symlinks: visible but non-expandable
- Reuse existing exclusion, binary filtering, scan_dir ignore behavior
- No whole-project eager scan, no nested-ignore policy change, no new dependencies
- Keep legacy navigation state APIs/tests compatible

### Acceptance Tests (headless)
1. Root listing: only direct children visible on open
2. Child-directory expansion: loads direct entries on demand
3. Collapse/re-expand: retains cached children
4. File/symlink/out-of-project paths: do not expand
5. Global budget: bounds total cached entries with testable small budget
6. Incomplete state is observable
7. Picker cancellation preserves tree cache; replacement/close clears it
8. File selection survives expansion/collapse

### Files
- `src/fs/tree.rs`
- `src/app_state.rs`
- `tests/fs_lazy.rs`

### Status
- [x] RED: wrote headless behavior tests
- [x] GREEN: implemented the minimum cache/state behavior
- [x] TRIANGULATE: covered invalid paths, symlinks, caps, replacement/cancellation
- [x] REFACTOR: full suite and formatting remain clean

### Verification and delivery
- `cargo test --workspace --all-targets`: 165 passed, 0 failed
- `cargo fmt --check`: passed
- `git diff --check`: passed
- Independent verifier confirmed state/cache acceptance; GUI smoke test was not run.
- Work-unit commit: pending
