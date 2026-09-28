# Multilanguage MVP Readiness

## Goal
Define a provisional 19-candidate MVP catalog with explicit, graduated structural-analysis coverage and per-language conformance gates; the release count remains subject to evidence. Validate the end-to-end comprehension flow first with Python and TypeScript.

## Decisions
- Only candidates retained after dated demand/grammar audit and passing their tier gate are included in the MVP release; tiers communicate coverage depth, not correctness tolerance.
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
7. [x] Study OpenCode provider architecture and define initial AI-provider scope without adding OpenCode as dependency or copying code.
   - **Decision: MVP starts with five requested integration families plus the previously required local Ollama option.**
     - Vendor integrations: OpenAI, Anthropic, Google Gemini.
     - Protocol families: OpenAI-compatible endpoints, Anthropic-compatible endpoints.
     - Local provider: Ollama via compatible API (fallback engineering validation required; capability depends on installed model and Ollama version).
   - **Explicit constraints.**
     - No unlimited provider catalog; additional providers post-MVP.
     - Configurable compatible endpoints require trust/privacy disclosure and explicit user action.
     - Credential security via OS-backed stores with documented fallback.
     - Model capability flags: record known structured-output capability per model and probe only where reliable; unknown or unavailable support reported honestly.
     - Strict local schema validation for all responses regardless of provider.
     - No model parity claims; each integration validated independently.
     - No fixed model catalog prescribed in MVP.
     - OpenCode preserved as study-only reference (not dependency or code copy).
   - **Official structured-output documentation consulted in September 2026 (capability remains model/API-version-specific).**
     - OpenAI: JSON Schema structured output — [OpenAI guide](https://platform.openai.com/docs/guides/structured-outputs).
     - Anthropic: JSON output via `output_config.format` — [Anthropic guide](https://docs.anthropic.com/en/docs/build-with-claude/structured-outputs).
     - Google Gemini: JSON Schema structured output with supported subset — [Gemini guide](https://ai.google.dev/gemini-api/docs/structured-output).
     - Ollama: structured output including OpenAI-compatible API; test against installed model/version — [Ollama guide](https://docs.ollama.com/capabilities/structured-outputs).
     - OpenAI-compatible and Anthropic-compatible endpoints: support varies by provider; Astynex validates locally and reports unsupported features honestly.
   - PRD updated with provider family table and explicit scope constraints.
   - Validation: `git diff --check` passed; delegated contradiction scan passed after correcting the family count. Documentation work-unit commit: `a3dd3ac docs: define initial AI provider families`.
8. [ ] Validate the provisional 19-candidate catalog, grammar availability, and tier assignments with dated evidence.
    - **Note:** Catalog is provisional; Swift is deferred pending grammar validation (see Task 11). Final validated language count may be below 19 after grammar/demand audit.
9. [ ] Define large-project scan limits, exclusions, cancellation, and scale-test envelope.
10. [ ] Establish workspace and strict TDD runner/configuration.
11. [x] Scope change: remove Swift from MVP catalog pending grammar validation.
    - **Decision:** Swift is deferred from the 19-language catalog rather than replaced. The old tree-sitter/tree-sitter-swift README states 'Status - Abandoned' and links to alex-pinkus/experimental-tree-sitter-swift as an alternative. A replacement grammar exists (alex-pinkus/tree-sitter-swift) but its Tier 2 Astynex conformance has not been validated.
    - **Rationale:** User explicitly chose to temporarily remove languages with abandoned grammars from MVP rather than replace to maintain count, prioritizing high-demand languages. Do not claim Swift has no maintained grammar—defer it provisionally.
    - **Evidence:** Official tree-sitter/tree-sitter-swift README (https://github.com/tree-sitter/tree-sitter-swift): "Status - Abandoned"; redirect/reference to alex-pinkus/experimental-tree-sitter-swift as the active fork.
    - **Catalog impact:** 20 → 19 provisional languages. Catalog remains provisional; final MVP count may be below 19 after remaining grammar/demand audit.
    - **Documentation updated:** PRD.md (catalog table, acceptance criteria, roadmap, executive summary, open decisions, document note); ODD Goal and Decisions coherently reflect the provisional 19-candidate catalog.
    - **Work-unit commit:** `6155b0e docs: defer Swift from provisional MVP catalog`; documentation verification: `git diff --check` and delegated readback passed. Task 8 remains pending.

## Evidence
- PRD update is the deliverable; no source implementation or tests are part of this documentation task.
