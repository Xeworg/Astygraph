# Astynex Desktop Basics

## Goal
Deliver the first usable native desktop slice, prioritizing essential project/source navigation: open a local project folder, browse its files, and view code safely. Prepare clear boundaries for graph, localization, and icon assets without implementing analysis features prematurely.

## Decisions
- Use the selected Rust `eframe`/`egui` desktop stack; validate dependency compatibility during implementation.
- Support Spanish and English UI localization from the start; UI strings must not be hard-coded in widgets.
- Use Heroicons outline SVG icons for the essential UI actions/file types; preserve the upstream MIT license notice.
- Include an accessible graph module/model boundary for later integration, but no graph UI, fake graph data, parser, or AI in this slice.
- Keep filesystem/viewer logic testable without opening a window; the UI delegates to tested components.
- Text browsing is language-agnostic; do not call file extensions semantic language support or claim structural analysis.
- Respect Linux/Windows targets, the 1 MiB provisional file limit and discovery/exclusion policy; preserve the existing `APPLICATION_NAME` test and untracked `.codegraph/`.
- Explicit non-goals: Tree-sitter/parser adapters, AI providers, semantic analysis, rendered graph, persistence/cache, syntax highlighting, and telemetry.
- User explicitly authorized local work-unit commits for this feature; no push or PR is authorized.

## Tasks
1. [x] Complete the native application shell and folder-open flow.
   - Acceptance: native window title is `Astynex`; folder selection is injectable/testable separately from UI; cancellation does not corrupt current state; headless tests cover state transitions.
   - Implementation: eframe/egui shell, native rfd folder picker, and pure `AppState` transitions; retained the existing application identity test.
   - RED: the new app-shell tests failed to compile because `astynex::app_state` did not exist.
   - GREEN: `cargo test --workspace --all-targets` passed with 7 tests (1 identity + 6 app-shell); `cargo fmt --all -- --check` and `git diff --check` passed.
   - Review: independent verifier confirmed cancellation/state transitions; removed unused direct `winit` and speculative `tracing-subscriber` dependencies before closeout. Manual GUI launch was not exercised.
   - Work-unit commit: `dc8a5b6 feat: add native desktop shell and folder picker`.
2. [x] Add safe project-file navigation and a code viewer.
   - Acceptance: browse nested project files and display selected text with line numbers; show visible diagnostic for files over the provisional 1 MiB ceiling; surface invalid UTF-8 and I/O failures without panics; tests cover boundaries and errors. Apply the PRD's default exclusions, symlink policy, and bounded/incomplete discovery behavior.
   - GREEN: `cargo test --workspace --all-targets` passed with 53 tests; `cargo fmt --all -- --check` and `git diff --check` passed.
   - Review: independent verification confirmed nested drill-down, safe parent/sibling/traversal and symlink guards, viewer error/large-file behavior, exclusions, and bounded discovery. Manual GUI smoke and Windows execution were not exercised.
   - Work-unit commit: `dc87558 feat: add safe project navigation and code viewer`.
3. [x] Add Spanish/English localization and Heroicons outline assets.
   - Acceptance: all user-visible shell/browser/viewer strings use translation keys; users can switch between Spanish and English; essential actions and file/folder types use Heroicons outline SVGs; retain the MIT license notice and test locale fallback/key coverage.
   - GREEN: `cargo test --workspace --all-targets` passed with 91 tests; `cargo fmt --all -- --check` and `git diff --check` passed.
   - Implementation: `src/i18n/mod.rs` — `Locale` enum (En/Es), `I18n` struct with binary-search lookup, silent fallback, positional args; 16 keys per locale. `src/icons/mod.rs` — `Icon` enum (Folder, FolderOpen, Document, Link, ArrowUp, XMark) with inline Heroicons outline SVG path data, `parse_path()` converting SVG path strings to `egui::Pos2` for `Shape::Path` rendering, MIT license as doc comment. `src/app_state.rs` — added `locale: Locale` field, propagated through all state transitions. `src/app.rs` — replaced all hardcoded strings with `i18n.t("key")` / `i18n.t_args(...)`, replaced `[DIR]/[LINK]` prefixes with SVG icon rendering via `draw_icon()`, added locale toggle button `[EN]/[ES]`. `src/icons/heroicons_license.txt` — full MIT license text. `Cargo.toml` — added `egui = "0.36"` as direct dependency. `tests/i18n_icons.rs` — 19 headless tests covering locale switching, fallback, key coverage in both locales, icon path validity, path parsing, and locale preservation during folder operations. The picker title is localized and the SVG parser handles absolute/relative arcs and negative coordinates.
   - Independent verification found no blockers; manual GUI smoke and Windows execution were not exercised.
   - Work-unit commits: `be1a144 feat: add localized heroicons desktop UI` (initial feature) and `2b24ece fix: position heroicons within egui layout` (follow-up layout fix authorized by user).
4. [ ] Establish a UI-independent graph-view model boundary.
   - Acceptance: define only the minimal node/edge/document types needed to accept future graph data; no fake data, graph widget, AI, or parser logic; compile-time/unit tests cover basic model invariants without coupling it to egui.
5. [ ] Verify the complete slice and record evidence.
   - Acceptance: `cargo test --workspace --all-targets`, `cargo fmt --all -- --check`, and `git diff --check` pass; report whether manual desktop smoke was possible; every completed implementation task has a separate work-unit commit recorded here.

## Evidence
- Initial exploration found only the Rust workspace bootstrap and an application-name smoke test; no desktop UI or project-browser implementation existed.
- Product direction: essentials first (folder picker, file explorer, text/code viewer); graph UI and AI deferred.
- User decisions: adopt eframe/egui; include Spanish and English UI localization; use Heroicons outline; leave a graph-model boundary only for now; local commits authorized, no push/PR.
- Icon feasibility check: Heroicons upstream states the set is MIT licensed; egui supports SVG through `egui_extras`. Pin compatible versions and preserve license text during implementation.
