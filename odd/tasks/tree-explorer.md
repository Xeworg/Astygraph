# Expandable Project Tree Explorer

## Goal
Replace the current one-directory-at-a-time browser with a familiar expandable project tree while retaining the source viewer and existing filesystem safety policy.

## Decisions
- Opening a project shows the root's direct entries immediately; nested directories start collapsed.
- Load a directory's direct children only when the user expands it; cache those children across collapse/re-expand while the project remains open.
- Apply one global cap of 50,000 discovered entries across the root listing and all cached subtrees. Surface an incomplete/limit-reached state; do not silently exceed or claim completeness.
- Selecting a file shows it in the source viewer without collapsing the tree or changing the expanded state. Expanding/collapsing directories does not clear the selected file.
- Keep symlinks visible but non-expandable and retain the existing default exclusions, binary filtering, and `scan_dir` ignore behavior. Do not add a new symlink policy or nested-ignore policy in this change.
- Keep Close Folder in the Explorer; remove the old Up/parent-directory navigation affordances when tree rendering replaces drill-down navigation.
- Do not recursively pre-scan the entire project, add dependencies, or change graph/parser/AI functionality.
- Strict TDD runner remains `cargo test --workspace --all-targets`.

## Tasks
1. [x] Implement the headless lazy-tree cache and global entry budget.
2. [x] Render the expandable tree and preserve file selection.
3. [x] Independently verify and close the feature.

## Evidence
- User requested a VS Code-like tree and selected root-open/collapsed-child behavior with a 50,000-entry global cap.
- `src/fs/tree.rs` implements a budgeted lazy cache; `src/app_state.rs` integrates cache lifecycle and non-navigational expansion/collapse; `src/app.rs` renders recursive expandable nodes, preserving viewer selection.
- Tests: `cargo test --workspace --all-targets` — 165 passed, 0 failed.
- `cargo fmt --check` and `git diff --check` passed.
- Independent verification completed; GUI smoke test not run.
- Commit: `5109d4e feat: add lazy expandable project tree`.
