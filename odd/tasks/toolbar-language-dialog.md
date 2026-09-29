# Toolbar and Language Dialog

## Goal
Organize the desktop shell toolbar so users can open or switch projects from anywhere and choose the UI language through an explicit confirmation dialog.

## Decisions
- Keep **Open Folder** available in the top toolbar whether the app is idle or a project is loaded.
- Opening a replacement folder must preserve the current project if the native picker is cancelled; a confirmed selection replaces it.
- Put the app title and Open Folder action on the left of one top toolbar row; put the Language action on the right.
- The Language action opens a dialog with English and Español choices. A selection is a preview only; **Apply** commits it, **Cancel** discards it.
- Keep Close Folder in the project explorer; it remains contextual to the loaded project.
- Localize all toolbar/dialog copy in English and Spanish. Use a text label for Language; no new icon asset is required.
- Keep the dialog selection state testable without a display. No persistent language preference, settings storage, async picker redesign, or general toolbar framework in this slice.
- Preserve the synchronous native folder picker and existing app architecture; no new dependency.

## Tasks
1. [x] Extend the headless state model for safe project replacement and language-dialog draft/apply/cancel.
   - Acceptance: Open Folder can be requested from Idle or Loaded; picker cancellation preserves the prior state; confirmed selection replaces the open project; language selection stays pending until Apply; Cancel leaves the active locale unchanged.
   - Implementation: `AppState` snapshots the previous `FolderState` before entering `Loading` and restores it if the picker is cancelled; successful selection clears the snapshot. Added headless pending-locale open/draft/apply/cancel transitions without changing the active locale before Apply.
   - RED/GREEN: tests first failed on missing locale draft methods and unavailable folder replacement; after implementation, `cargo test --workspace --all-targets` passed with 119 tests and `cargo fmt --all -- --check` passed.
   - Independent verification: no blockers; confirmed snapshot restore/clear and pending locale behavior across folder operations.
   - Work-unit commit: `96f55ba feat: preserve app state during folder and language selection`.
   - Strict TDD runner: `cargo test --workspace --all-targets`.
2. [x] Add the top toolbar and localized language dialog.
   - Acceptance: toolbar is visible across app states; Open Folder invokes the native picker; Language opens a modal with English/Español choices and Apply/Cancel; applying updates all UI strings and cancelling preserves the active locale; dialog/toolbar strings exist in both locales.
   - Implementation: `src/app.rs` now places title + Open Folder at the left and Language at the right in a persistent toolbar. Language opens `egui::Modal` with the active locale's copy, current locale preselected, English/Español radios, and Apply/Cancel; backdrop/Escape cancel the draft. The modal blocks the underlying toolbar and explorer. Existing Close Folder remains in Explorer.
   - Localization: added six dialog keys for title, Apply, Cancel, English, Español, and current marker; tests cover both locale tables. No icon/dependency changes.
   - RED/GREEN: translation and state tests failed before implementation; `cargo test --workspace --all-targets` passed with 124 tests after implementation; `cargo fmt --all -- --check` passed.
   - Independent verification: no blockers; confirmed modal behavior against egui 0.36, radio/apply event ordering, folder replacement behavior, and translations. `cargo doc` retains two pre-existing redundant-link warnings in `src/app.rs` lines 3–4.
   - Native review: candidate review capture escalated with `native_stop_required`; no reviewer verdict or approval was produced. Do not treat the candidate as approved.
   - Work-unit commit: `b2543a5 feat: add toolbar and language selection dialog`.
3. [x] Independently verify and close the feature.
   - Acceptance: focused and full tests, formatting and diff checks pass; report GUI smoke status; record work-unit commits and evidence here.
   - Final verification: `cargo test --workspace --all-targets` — 124 passed, 0 failed; `cargo fmt --all -- --check` — passed; `git diff --check` — passed. Work-unit commits `96f55ba` and `b2543a5` confirmed in git.
   - Repository status: tracked tree clean; pre-existing `.codegraph/` and `.vscode/` remain untracked and untouched.
   - GUI manual smoke and Windows execution were not exercised.
   - Native review remains escalated with `native_stop_required`; no reviewer verdict or approval was produced. This is not delivery approval.

## Evidence
- Initial UI exploration and read-only scout: `src/app.rs` currently renders a standalone locale toggle at the top and only renders Open Folder in `Idle`. `AppState::open_folder()` is a no-op from `Loaded`; picker cancellation from Loading returns to Idle.
- User decisions: Open Folder always visible; language dialog requires explicit Apply/Cancel.
- ODD feature task file created before source edits; Engram mirror and visible todo track progress.
