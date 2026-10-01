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
- Parser facts and AI-derived semantic analysis remain separate: Tree-sitter is the syntax/source-location source, while the Astynex IR represents validated analysis. Do not add a second normalized AST tree for MVP; derive only the minimal parser facts needed to validate analysis, and let Python/TypeScript vertical-slice tests determine their shape. This is a design boundary, not a compiler-grade AST, CFG, or call-graph commitment.
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
   - Persistence design follow-up (see `odd/tasks/persistence-cache-design.md` and PRD §9): persist only seen files and parser-observed file dependencies; keep semantic graph query-time and bounded. Separate parse and analysis fingerprints; freshness requires content digests plus ordered context membership and output-affecting versions/configuration. Recheck before acceptance/display, discard changed candidates, and retry once at most. Direct invalidation is lazy; dependency completeness is partial and explicit. Ordered `PRAGMA user_version` migrations and per-connection foreign-key enforcement are required; migrations/analysis replacement are atomic and recovery never alters source. Raw source/prompts/provider responses/credentials are not persisted; remote providers may receive submitted context. Exact digest/config encoding and SQLite platform durability/concurrency remain implementation validation items.
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
8. [x] Close the lightweight catalog/demand decision.
    - **Decision:** Retain all 19 catalog candidates as provisional. Record dated demand basis: Stack Overflow 2025 Developer Survey (developer-reported extensive use, survey question, n=31,771; survey published 2025; https://survey.stackoverflow.co/2025/technology) and GitHub Octoverse 2025 (GitHub monthly contributor counts, August 2025 snapshot; TypeScript ranked #1 by contributor count, ahead of Python/JavaScript; https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/).
    - **Source methodology note:** The two sources use different populations and measurement methods. They inform candidate prioritization, not grammar health or Astynex support. Do not claim all language ranking details are verified from these sources alone.
    - **Grammar gate preserved:** Per-grammar repo/version/license/Rust integration and tier fixtures remain required pre-support implementation gates (Task 12 / PRD §10 grammar gate). Candidate-level evidence is not invented.
    - **Swift deferral preserved:** Task 11 decision stands unchanged.
    - **Task 8 scope:** This task is scoped to the catalog/demand closeout only. The remaining open gate is the grammar evaluation gate (§10) evaluated during implementation for each candidate.
9. [x] Define large-project scan limits, exclusions, cancellation, and scale-test envelope.
    - **Decision:** Scale envelope closed with user-approved provisional numeric caps, all labeled as hypotheses pending benchmark measurement against NFR-7. The feature is on-demand (discovery/index/search responsiveness), not eager parsing of every source file.
    - **Provisional caps (hypothesis pending benchmark; tune after measurement against NFR-7):**
      - **Generated deterministic project fixtures:** 1,000 entries (routine integration), 10,000 entries (large-project benchmark), 50,000 entries (stress/manual or non-blocking benchmark). Entry means filesystem path, not supported source file.
      - **Initial configurable discovery ceiling:** 50,000 entries per project scan; when reached, stop adding work, report incomplete/limit-reached, keep browsing available, allow changing limit/configuration, do not silently claim completeness.
      - **Parsing ceiling:** 1 MiB UTF-8 bytes per file. Larger files openable as text, skip structural parse with visible diagnostic, no fabricated structure.
      - **Context ceiling:** selected file/symbol plus at most 5 directly related files and 128 KiB aggregate UTF-8 source text before provider tokenization; deterministic selection; disclose omitted related files/bytes; never silently truncate a source span or claim complete context.
      - **Exclusions:** Exclude `.git/`, `.astynex/`, binary/non-text files, and ignored paths from discovery/analysis. Honor `.gitignore` with configurable user overrides. Exclude common generated/build/cache directories by configurable defaults. Exclude obvious secrets from provider context. Do not follow symlinks by default (avoids cycles/directory escape); make it an explicit future setting only if warranted.
      - **Cancellation:** Implement for discovery/indexing, parsing, context construction, provider request, and graph construction where practical: stop scheduling work, preserve browsing and already-validated/persisted state, maintain DB transaction atomicity, return explicit cancelled/partial status distinct from failure, report progress for long-running work.
      - **Benchmarks:** Record environment/project fixture, path/file/byte counts, p50/p95 wall time, peak memory, cancellation responsiveness. No latency SLA until measured. CI uses 1k and bounded 10k cases; 50k stress is non-blocking/manual/nightly.
    - **PRD updates:**
      - §7 FR-3: File size behavior with diagnostic for oversized files.
      - §8 NFR-2: Discovery ceiling (50,000 entries) and incomplete/limit-reached behavior.
      - §8 NFR-7: Provisional limits listed explicitly with hypothesis label.
      - §9 Architecture principles: On-demand scope, exclusion rules, cancellation contract.
      - §12 Privacy: Expanded exclusion rules with symlink default and configurable overrides.
      - §13 Testing strategy: Scale-envelope test sizes table (1k/10k/50k fixtures).
      - §14 Error handling: Cancellation contract added.
      - §19 Open Decision #5: Marked resolved with capsule summary; cross-references to NFR-7.
    - **Cross-references:** Long tables are not duplicated; scale-envelope table in §13, provisional limits summarized in NFR-7, full details in Open Decision #5 resolution.
    - **Validation:** `git diff --check` passed (see below).
    - **Acceptance evidence:** `git diff --check` output clean; Task 9 marked done.
10. [x] Establish workspace and strict TDD runner/configuration.
    - **Decision:** Single-package Rust workspace (`astynex`, package name `astynex`, one workspace member). `edition = "2021"` declared; no toolchain pin. One smoke behavior: exported `APPLICATION_NAME` constant asserting `"Astynex"`. No eframe, Tree-sitter, database, provider, or implementation features. `Cargo.lock` auto-generated.
    - **TDD mode:** Strict RED-GREEN-REFACTOR from the prior user-approved PRD decision; exact runner `cargo test --workspace --all-targets` (Cargo 1.95.0).
    - **TDD evidence:**
      - RED: `cargo test --workspace --all-targets` → `error[E0425]: cannot find value APPLICATION_NAME in crate astynex`; test runner exit 101.
      - GREEN: after adding `pub const APPLICATION_NAME: &str = "Astynex";` in `src/lib.rs` → `application_name_exported ... ok; 1 passed; 0 failed`.
      - REFACTOR: not justified — three-file surface (Cargo.toml, src/lib.rs, tests/app_identity.rs) is already lean; brevity IS clarity at this scale.
    - **Validation:**
      - `cargo test --workspace --all-targets` → `test result: ok. 1 passed; 0 failed`.
      - `cargo fmt --all -- --check` → passed (trailing newline normalization applied).
    - **Files created:**
      - `Cargo.toml` — package manifest (`name = "astynex"`, `edition = "2021"`, `path = "src/lib.rs"`).
      - `src/lib.rs` — library root with `APPLICATION_NAME` constant.
      - `tests/app_identity.rs` — smoke integration test asserting `APPLICATION_NAME == "Astynex"`.
      - `Cargo.lock` — auto-generated lockfile.
    - **Work-unit commit:** `b611175 chore: bootstrap minimal Rust workspace` (includes the accumulated Task 8/9 PRD and task-record closeout edits, as explicitly authorized by the user).
    - **Preserved:** untracked `.codegraph/` directory untouched; Cargo `target/` ignored by `.gitignore`.
11. [x] Scope change: remove Swift from MVP catalog pending grammar validation.
    - **Decision:** Swift is deferred from the 19-language catalog rather than replaced. The old tree-sitter/tree-sitter-swift README states 'Status - Abandoned' and links to alex-pinkus/experimental-tree-sitter-swift as an alternative. A replacement grammar exists (alex-pinkus/tree-sitter-swift) but its Tier 2 Astynex conformance has not been validated.
    - **Rationale:** User explicitly chose to temporarily remove languages with abandoned grammars from MVP rather than replace to maintain count, prioritizing high-demand languages. Do not claim Swift has no maintained grammar—defer it provisionally.
    - **Evidence:** Official tree-sitter/tree-sitter-swift README (https://github.com/tree-sitter/tree-sitter-swift): "Status - Abandoned"; redirect/reference to alex-pinkus/experimental-tree-sitter-swift as the active fork.
    - **Catalog impact:** 20 → 19 provisional languages. Catalog remains provisional; final MVP count may be below 19 after remaining grammar/demand audit.
    - **Documentation updated:** PRD.md (catalog table, acceptance criteria, roadmap, executive summary, open decisions, document note); ODD Goal and Decisions coherently reflect the provisional 19-candidate catalog.
    - **Work-unit commit:** `6155b0e docs: defer Swift from provisional MVP catalog`; documentation verification: `git diff --check` and delegated readback passed. Task 8 was subsequently closed for demand-side catalog closeout (see Task 8).
12. [x] Define evidence-based grammar evaluation gate for language implementation decisions.
    - **Decision:** During implementation, require reproducibly pinned Rust-compatible grammar integration, evidence of a viable maintained source or alternative, license/security checks, and Astynex-specific fixtures passing the declared tier gate. If no viable maintained grammar can be established, do not implement or advertise the language as MVP-supported yet; reassess later. No arbitrary activity deadline defines maintenance.
    - **Catalog impact:** 19-candidate catalog preserved; Task 8 closed for demand-side catalog/demand closeout (see Task 8). Implementation-stage acceptance is distinct from Task 8; final validated release count may be below 19.
    - **Distinctions:** Task 8 records the provisional 19-candidate demand basis; it does not audit grammar maintenance or verify per-language rankings from those sources. Language-specific implementation gates (grammar pinning, fixture authoring, tier passage) happen during development. Do not claim fixture work done before it is; do not assert 19 guaranteed release slots.
    - **Stale-signal guidance:** Temporarily outdated releases or a 404 on one URL does not mean no maintained alternative; evaluate holistically before deferring.
    - **Browsing unaffected:** Deferred languages are not advertised as analysis-supported; users can still open/read their source as text. Highlighting requires its own validated path.
    - **Documentation updated:** PRD.md Section 10 with grammar gate criteria, explicit candidate/implementation distinction, stale-signal guidance, and read-only browsing policy.
    - **Validation:** `git diff --check` and delegated readback passed; Swift deferral preserved. Work-unit commit: `14733e5 docs: gate MVP languages on maintained grammars`.

13. [x] Specify the parser-facts and AI-analysis boundary without adding a normalized AST layer.
    - **Decision:** Keep Tree-sitter as the language-specific parser and source of syntax nodes/byte spans. Keep the Astynex IR as the distinct, versioned representation of validated semantic analysis. Do not introduce a second universal normalized AST/tree for MVP.
    - **Constraint:** Extract only the minimal typed parser facts required to validate AI analysis; determine the fact set from Python/TypeScript vertical-slice tests rather than prescribing a comprehensive cross-language schema up front. Preserve parser/AI provenance, canonical byte spans, diagnostics, and explicit partial/unsupported states.
    - **Non-goals:** No compiler-grade AST, complete CFG, or call graph is implied. Revisit richer hierarchical facts only if concrete validation cases require them.
    - **Acceptance:** PRD §§9–10 state the parser-fact/semantic-IR boundary and its non-goals consistently with existing MVP scope; docs-only validation and readback passed.
    - **Route:** Delegated direct writer because the task updates the PRD and this feature task record (multi-file write trigger).
    - **Verification:** `git diff --check` passed.
    - **Evidence:** Independent read-only architecture review by `gentle-ai-explore`; parent synthesis accepted the recommendation with the qualification that tests determine the minimal fact set.
    - **PRD §9 (Initial IR proposal)** updated to: introduce the two-layer model (parser facts / validated semantic IR) explicitly; state the non-goals (no compiler-grade AST, complete CFG, or call graph; no second universal normalized AST); and note that line/column are derived from canonical byte/offset spans.
    - **PRD §10 (Language and parser strategy)** updated to add a new `#### Parser facts and semantic IR boundary` subsection between the tier table and the Tree-sitter conformance paragraph. The subsection includes: a definition of each layer; a three-column property table (`Property`, `Parser facts`, `Validated semantic IR`); an explicit non-goals block matching §9; a minimal-fact-set policy stating that Python/TypeScript vertical-slice tests determine the concrete fact shape; and a statement that diagnostics, byte-span provenance, and partial/unsupported states are first-class concerns at both boundaries.
    - **Work-unit commit:** `cee45d8 docs: define parser fact and analysis IR boundary`.

## Evidence
- PRD update is the deliverable; no source implementation or tests are part of this documentation task.
