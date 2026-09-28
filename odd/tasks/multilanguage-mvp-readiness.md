# Multilanguage MVP Readiness

## Goal
Define the MVP as a 20-language release with explicit, graduated structural-analysis coverage and per-language conformance gates. Validate the end-to-end comprehension flow first with Python and TypeScript.

## Decisions
- All 20 selected languages are included in the MVP release; tiers communicate coverage depth, not correctness tolerance.
- Python and TypeScript are the initial vertical-slice languages.
- Every tier must preserve source mapping, avoid fabricated structure, and communicate unsupported/partial analysis.
- Each language requires versioned grammar metadata and language-specific fixtures against common conformance expectations.
- Linux and Windows are the initial target platforms; development is initially private to the project team, without narrowing the intended user groups.
- MVP graph scope is the opened file and selected applicable symbol (function, class, interface, or language equivalent), plus its directly related files/symbols for context; a whole-project graph is post-MVP.
- The AI provider is required for AI analysis, explanations, and graph features; with no provider configured/available, users may still open/read source but those AI features are unavailable.
- AI analyzes the opened source; Astynex software creates/renders the interactive graph. Do not generate graph images.
- AI must never modify source files. Its structured output and source references must be validated before rendering.
- Ollama is the offline AI option; offline AI requires Ollama to be installed/configured and available.
- Use strict RED-GREEN-REFACTOR TDD.
- Persistence uses one local SQLite database per project folder, incrementally populated as files are analyzed, accessed from Rust through `rusqlite` with the `bundled` feature. Store graph nodes and edges relationally and use recursive CTEs for bounded traversal. Cache contents, invalidation, migrations, retention, and deletion details remain open until specified before implementation.
- Scale strategy is on-demand analysis of opened/related files, not a full-project eager scan; exact limits remain for measurement.
- The provided UI image is directional; its project-wide graph is not MVP scope.

## Tasks
1. [x] Reconcile MVP scope, exclusions, roadmap, and language strategy in PRD.
2. [x] Specify common conformance suite, tier gates, and 20-language candidate catalog (19 initial candidates plus R as candidate 20).
3. [x] Align acceptance criteria and open decisions with the language MVP.
4. [x] Update PRD with the newly confirmed platform, AI, graph, persistence, and TDD product constraints.
   - Evidence: PRD reflects Linux/Windows; private development without narrowing intended user groups; on-demand analysis of opened file/symbol plus direct context; local dependency graph rather than whole-project graph; AI structured analysis validated and rendered natively; no image generation or AI file modification; source browsing without a provider while analysis/graph require one; Ollama offline option; per-folder incremental persistence pending technology research; strict RED-GREEN-REFACTOR with runner setup pending.
   - Validation: `git diff --check` passed; targeted contradiction scan/readback confirmed the intended provider-unavailable, graph-scope, source-write, and TDD constraints. Independent subagent verification unavailable because worktree registration failed.
5. [x] Research and select the engine direction for scalable per-folder incremental persistence.
   - Decision: SQLite through Rust `rusqlite` with the `bundled` feature; relational node/edge tables and recursive CTEs for bounded MVP graph traversal.
   - Rationale: SQLite officially supports recursive graph/tree queries and WAL; `rusqlite`'s bundled feature includes SQLite and avoids relying on the system SQLite installation. The MVP graph is bounded to opened files/symbols and directly related context.
   - Alternatives reviewed: SQLite graph extensions (no suitable specific extension established); embedded graph stores, including `sqlitegraph` (GPL-3.0-only and single-maintainer posture are material tradeoffs; not selected).
   - Remaining before implementation: define cached payloads, fingerprint/invalidation scope, migration mechanism, retention/deletion and cache location; validate durability/configuration and test behavior on Linux and Windows.
   - Evidence: SQLite recursive CTE graph traversal ([SQLite WITH](https://www.sqlite.org/lang_with.html)); WAL behavior and same-host constraint ([SQLite WAL](https://www.sqlite.org/wal.html)); `rusqlite` bundled feature ([docs.rs](https://docs.rs/crate/rusqlite/latest)).
   - Commit: `928f476 docs: select rusqlite for graph persistence`.
6. [x] Finalize SQLite cache architecture (per-project, per-file context, transactional invalidation, lazy rebuild).
   - Decision: One SQLite database per project at `.astynex/cache.db`; organized by file and directly related context; obsolete dependent data replaced/invalidated transactionally; clear-cache removes derived artifacts only; lazy rebuild on request, no eager scan.
   - Cache scope: validated AI analysis (IR), parser facts, source-range fingerprints, dependency metadata. Not cached: raw source, prompts, raw provider responses, or credentials. Validated IR remains sensitive source-derived data.
   - `.astynex/` excluded from Astynex discovery; Git ignore recommended in docs.
   - Open before implementation: exact fingerprint algorithm, invalidation granularity, bounded-context invalidation scope, migration mechanism.
   - **Payload/retention nuances:** source content is read from disk on demand, not stored; prompts and raw provider responses are not persisted; clear-cache does not touch provider configuration or credentials.
   - Follows from: Task 5 engine decision (`928f476`).
   - Work-unit commit: `0b069fd docs: define file-oriented cache policy`. Documentation-only validation: `git diff --check`; delegated readback confirmed scope and preserved OpenCode reference.
7. [ ] Study OpenCode provider architecture and define initial AI-provider scope without adding OpenCode as dependency or copying code.
8. [ ] Validate the 20-language catalog, grammar availability, and tier assignments with dated evidence.
9. [ ] Define large-project scan limits, exclusions, cancellation, and scale-test envelope.
10. [ ] Establish workspace and strict TDD runner/configuration.

## Evidence
- PRD update is the deliverable; no source implementation or tests are part of this documentation task.
