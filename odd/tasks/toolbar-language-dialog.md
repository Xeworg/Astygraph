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
2. [ ] Add the top toolbar and localized language dialog.
   - Acceptance: toolbar is visible across app states; Open Folder invokes the native picker; Language opens a modal with English/Español choices and Apply/Cancel; applying updates all UI strings and cancelling preserves the active locale; dialog/toolbar strings exist in both locales.
   - Keep the existing explorer Close Folder control and folder browsing/viewer behavior.
3. [ ] Independently verify and close the feature.
   - Acceptance: focused and full tests, formatting and diff checks pass; report GUI smoke status; record work-unit commits and evidence here.

## Evidence
- Initial UI exploration and read-only scout: `src/app.rs` currently renders a standalone locale toggle at the top and only renders Open Folder in `Idle`. `AppState::open_folder()` is a no-op from `Loaded`; picker cancellation from Loading returns to Idle.
- User decisions: Open Folder always visible; language dialog requires explicit Apply/Cancel.
- ODD feature task file created before source edits; Engram mirror and visible todo track progress.
