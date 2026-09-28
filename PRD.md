# Astynex — Product Requirements Document

**Status:** Draft 0.1  
**Date:** 2025-05-02  
**Purpose:** Define a testable initial product direction. This document is intentionally provisional; unresolved product choices are called out rather than silently treated as commitments.

## 1. Executive summary

Astynex is a local-first desktop application, initially developed privately by the project team, that helps developers understand unfamiliar source code by connecting three views of the opened file or selected symbol: a concise AI-generated explanation (**Idea**), an interactive software-rendered representation of its behavior (**Algorithm**), and the original source (**Code**). A local dependency graph may show directly related files and symbols needed to understand the current selection; a whole-project graph is outside the MVP.

Its central hypothesis is that a developer can understand real code faster and with better source traceability using a linked visual explanation than by reading source alone. A configured AI provider analyzes the opened file or selected symbol and returns structured analysis; Astynex validates that result and renders it as native graph elements, not generated images. AI must never modify source files. Schema and source references are validated; unsupported or unmappable claims are rejected or shown as uncertain rather than silently trusted.

The MVP should prove this comprehension loop across a selected catalog of 20 widely used programming languages, with availability in all three validated tiers at MVP release. Coverage depth will differ by language and tier; equal completeness is not promised. Validate the end-to-end comprehension experience first with Python and TypeScript, then extend the shared conformance gates and tiered coverage across the catalog.

## 2. Problem and opportunity

When entering an unfamiliar codebase, developers repeatedly move between source files, call sites, types, and implicit control flow. This imposes navigation and working-memory costs, especially when the code is legacy, unfamiliar, or poorly documented. Conventional editors present source effectively, but do not necessarily provide a linked, progressively explorable explanation of a selected function.

Astynex proposes a visual comprehension layer, not an IDE replacement. Its value must be demonstrated against ordinary source reading, not assumed from the appeal of diagrams or AI summaries.

## 3. Product vision and principles

**Vision:** Help a developer move from project context to a useful mental model of a function, then verify that model against its exact source.

**Core journey:** `Project → File/Function → Idea ↔ Algorithm ↔ Code`

Guiding principles:

1. **Traceability first:** Every graph node and explanation claim should map to source ranges or be explicitly marked uncertain/unmapped.
2. **AI analysis, software visualization:** The AI analyzes source and returns structured data; deterministic application code validates and renders the interactive graph. Do not generate graph images.
3. **Local-first and explicit sharing:** Project browsing and source reading are local. Sending code to a remote model requires clear user choice and disclosure; Ollama is the offline AI option.
4. **Progressive complexity:** Analyze on demand, beginning with the opened file or selected applicable symbol and only directly related files/symbols needed for context; do not eagerly analyze the whole project.
5. **Graceful degradation:** Users can open and read source without an AI provider. Analysis, explanations, and graph features require a configured and available provider.
6. **Honest uncertainty:** Unsupported syntax, parse errors, incomplete context, and AI uncertainty are visible rather than presented as certainty.
7. **User-controlled exploration:** Graph selection, search, navigation, and detail panels should help answer questions without autonomous edits or opaque actions.

## 4. Target users and primary needs

### Primary users
- Developers onboarding to an unfamiliar repository.
- Developers investigating legacy or complex code.
- Learners who benefit from a visual account of code structure and logic.

### Secondary users (later validation)
- Code reviewers exploring behavior and failure paths.
- Maintainers preparing documentation or explaining an implementation.

The product is initially developed privately by the project team; this development arrangement does not narrow the intended user groups.

### Jobs to be done
- “When I open an unfamiliar file or select a function, class, interface, or language-equivalent symbol, help me understand its purpose and logic without losing the ability to verify each claim in source.”
- “When I see a node or branch in the visual view, take me to the relevant code.”
- “When analysis is incomplete or AI is unavailable, show what can still be understood and why the rest is missing.”

## 5. Product scope

### MVP: prove one comprehension loop

1. Open a local project directory.
2. Browse a filtered file tree and open a supported source file.
3. Show readable, syntax-highlighted source with line numbers.
4. Detect supported functions and basic control-flow constructs using a parser.
5. Select a function, class, interface, or applicable language-specific symbol and request AI analysis; the configured provider analyzes the opened source plus explicitly scoped directly related context.
6. Validate AI output against a versioned schema and source ranges before Astynex constructs and displays an interactive graph. Reject or visibly flag invalid, unsupported, uncertain, or unmappable content; never generate graph images.
7. Link graph nodes to source and let users navigate from a node to its source range and, where mapping is unambiguous, from source to the corresponding graph node.
8. Allow source browsing and reading without a configured provider, but make analysis, explanation, and graph-generation unavailable with a clear setup/recovery state.
9. Report parser, provider, validation, and rendering errors without losing the project or source view. AI has no operation capable of modifying project files.

### Explicitly out of MVP scope
- Autonomous agents, code modification, debugging, autocomplete, or a full IDE.
- RAG, vector databases, cloud project storage, collaboration, or account systems.
- Eager analysis of every project file, a whole-project graph, or complete repository semantic indexing.
- Claims of complete call/data-flow analysis across dynamic dispatch, reflection, macros, or unresolved dependencies.
- Equal-depth or compiler-grade analysis across all 20 languages. The MVP includes every selected language with explicitly tiered coverage, documented limitations, and language-specific acceptance gates.
- A large built-in provider catalog or parity with every model vendor in the MVP; the architecture should permit broad provider additions over time.
- Whole-project/cross-project graphs and unrestricted cross-file algorithm graphs; MVP context is limited to directly related files/symbols for the opened file or selection.

## 6. User experience and interface direction

The supplied mockup is a directional reference, not a binding layout specification. It suggests a desktop workspace with:

- **Project explorer** at left.
- **Source editor/viewer** adjacent to the explorer.
- **Algorithm canvas** linked to selected source.
- **Explanation/details panel** with summary, steps, inputs/outputs, and complexity where supportable.
- **Local dependency graph** for directly related files/symbols in the context of the opened file or selected symbol; the mockup's whole-project graph is a later direction, not MVP scope.
- **Project navigation/search** for file paths and supported symbols without eagerly analyzing the full project.

The mockup communicates the product’s differentiator well: source, flow, explanation, and project context are visible together. For the MVP, avoid forcing all panels to remain open at once. Prioritize a resizable layout, clear active selection, keyboard-accessible navigation, and a way to focus/maximize the source or graph. Distinguish facts parsed from source from AI-authored descriptions through labels or provenance cues.

### Core interaction requirements
- Selecting an applicable symbol (function, class, interface, or language equivalent) becomes the shared context for Code, Algorithm, and Idea views.
- Selecting a graph node navigates to and highlights its source range; if no reliable range exists, explain that limitation.
- Selecting source within a supported function can identify its corresponding structural node where mapping is unambiguous.
- Graph pan/zoom and node selection must not interfere with ordinary text selection or source navigation.
- Empty, loading, parse-error, unsupported-language, invalid-analysis, and provider-unavailable states have explicit UI treatment. Provider unavailability blocks analysis/Idea/graph features but never blocks opening and reading source.
- Before remote analysis, users can inspect the source and directly related context that will be sent; Ollama is the supported offline path.

## 7. Functional requirements

### Project and source browsing
- **FR-1:** User can select and reopen a local project directory.
- **FR-2:** Explorer lists directories and supported source files; non-source files can be hidden or de-emphasized.
- **FR-3:** User can open a file and see line-numbered source. File size limits and behavior must be configurable or documented.
- **FR-4:** Search finds file paths and parsed symbols within the project’s supported scope.

### Structural analysis and graph
- **FR-5:** Language adapters parse supported files and provide syntax facts, symbol boundaries, and source spans used to validate AI analysis; the parser does not claim unsupported semantic certainty.
- **FR-6:** On explicit user action, AI analyzes the opened file or selected applicable symbol and returns structured graph/explanation data for the declared language coverage.
- **FR-7:** Astynex validates the AI response schema, node/edge references, source ranges, and file identity before creating/rendering graph elements. Invalid or unverifiable output is rejected or clearly flagged.
- **FR-8:** Graph nodes and edges retain file identity and verifiable source ranges where applicable, and distinguish parser facts from AI-derived interpretations and uncertainty.
- **FR-9:** Graph supports selection, pan, zoom, and source navigation. MVP scope is the opened file/selected symbol and directly related files/symbols for context, not a whole-project graph.
- **FR-10:** The graph is rendered natively by Astynex from validated structured data; the application does not request or generate graph images.
- **FR-11:** AI integration has no source-editing capability; analysis cannot write, patch, or otherwise modify project files.

### AI analysis and explanation
- **FR-12:** AI analysis is a required MVP capability available through a configured, working provider; Ollama provides the offline path when installed and configured.
- **FR-13:** Request context is bounded to the opened file/selected symbol and directly related context needed for understanding; the UI identifies the exact context.
- **FR-14:** Explanation distinguishes summary, logic steps, inputs/outputs where known, and uncertainty. Unsupported complexity claims are omitted or qualified.
- **FR-15:** Without a configured/available provider, users can open and read source, but cannot invoke AI analysis, receive AI explanations, or generate analysis graphs; show a clear configuration/recovery state.

### Privacy, preferences, and errors
- **FR-16:** Before remote analysis, disclose provider/model and the code/context being transmitted; obtain explicit user action. Ollama is the offline local-provider path.
- **FR-17:** API secrets are not written as plaintext in ordinary application settings. Use an OS credential store where supported, with documented fallback behavior.
- **FR-18:** Users can configure/remove providers and choose a model. Provider, protocol, and model concepts remain separate; MVP provider choices are determined by a separate research task.
- **FR-19:** Errors identify the failing stage (file read, parse, schema validation, provider, network, persistence) and preserve source browsing and other unaffected views.

## 8. Non-functional requirements

- **NFR-1 Privacy:** No source is transmitted without an explicit analysis action and clear disclosure; no telemetry containing source code by default.
- **NFR-2 Responsiveness:** Project opening, file browsing, parsing, and graph interaction should remain responsive on a documented reference project. Long work must show progress and be cancellable where practical.
- **NFR-3 Reliability:** Invalid model output and malformed/unsupported source must not crash the application or corrupt project state.
- **NFR-4 Accessibility:** Core navigation is keyboard-operable; selection and status are not communicated by color alone; text and graph contrast remain usable.
- **NFR-5 Portability:** Linux and Windows are the initial target platforms; credential-store and packaging differences are handled/documented.
- **NFR-6 Maintainability:** Parsing, IR, provider/protocol integration, and UI are separable boundaries with testable interfaces.
- **NFR-7 Resource use:** Memory and CPU behavior are measured against a stated project/file-size envelope, not described as “fast” without evidence.
- **NFR-8 Observability:** Application logs are structured, level-based, useful for diagnosing failures, and redacted so source code, secrets, prompts, and sensitive paths are not recorded by default.
- **NFR-9 Error containment:** Failures in parsing, AI requests, graph rendering, persistence, or other services must not crash the application or destroy the user's current source view. Provider failure disables analysis features but not source browsing.
- **NFR-10 Testability:** Core analysis, validation, persistence, and provider behavior must be testable without launching the GUI, accessing external networks, or requiring real credentials; provider contracts use deterministic local mocks.

## 9. Proposed architecture (initial, subject to technical validation)

Prefer a small set of stable boundaries over premature framework breadth:

- **Desktop UI:** Rust with eframe and egui as the native application framework and immediate-mode UI. Use egui_graphs (https://github.com/blitzarx1/egui_graphs) for graph rendering and interaction, subject to confirming its egui version compatibility and validating source selection/navigation integration.
- **Project/source service:** Local file access, ignore rules, on-demand file/symbol navigation, direct-related-context resolution, and project identity.
- **Language adapters:** Tree-sitter grammar/parser integration, syntax facts, symbol/source-span extraction, and language-specific validation support.
- **Analysis core:** AI request/context orchestration, structured-result validation, provenance, cancellation, and diagnostics.
- **Astynex IR:** UI-independent, versioned representation of AI analysis, validated nodes/edges, source spans, uncertainty, and annotations.
- **AI boundary:** Required provider registry, protocol adapters, model capabilities, credential references, request policy, structured-output validation, and Ollama support for offline use.
- **Persistence:** One local SQLite database associated with each project folder, incrementally updated as files are analyzed. Use Rust `rusqlite` with its `bundled` feature and relational node/edge tables; use recursive CTEs for bounded graph traversal. Content invalidation, migration strategy, cache contents, retention, and deletion details remain to be specified before implementation.
- **Visualization adapters:** Render only validated IR as native interactive graph elements; never treat generated images or UI widgets as canonical analysis data.
- **Diagnostics and observability:** Typed errors at subsystem boundaries, user-actionable diagnostics, and structured logs through a centralized logging setup.

### Architecture principles

- Keep the domain/IR independent of egui, Tree-sitter implementation details, persistence technology, and provider SDKs.
- Use explicit boundaries (ports/adapters) for filesystem access, language parsing, AI providers, secure credentials, and persistence; avoid global mutable state and cross-layer shortcuts.
- Prefer cohesive modules and a small number of crates initially. Split into workspace crates when ownership, dependency direction, or independent testing justifies it—not simply to increase crate count.
- Treat all external input (repository files, LSP/provider responses, AI output, configuration) as untrusted and validate it at the boundary.
- Make cancellation, resource limits, partial results, source provenance, provider unavailability, and persistence status part of analysis contracts rather than UI-only behavior.
- Never grant the AI integration file-write tools or source-editing APIs; treat repository content and model output as untrusted input.
- Keep UI state separate from canonical analysis data; version persisted or exchanged schemas.
- Depend on abstractions only where they isolate real variation (language adapters, AI protocols/providers, storage). Avoid speculative plugin systems and generic frameworks before a second implementation proves the need.

The provider architecture should be designed for extensibility: keep Provider, Protocol, and Model as distinct concepts, avoid provider-specific assumptions in analysis/UI layers, and make integrations independently testable. Study established open-source multi-provider architectures, including OpenCode, as engineering references; implement Astynex's Rust provider system as an independently designed implementation. Before incorporating any upstream code or assets, verify the exact repository, version, license, notices, and applicable obligations. Do not assume a license alone settles attribution, trademark, patent, or dependency questions.

The goal is to make adding providers inexpensive from the beginning, not to ship every provider in the MVP. Start with a small representative set that exercises the distinct protocols and local/hosted deployment patterns Astynex commits to support, then expand according to user demand and integration quality. Preserve the distinction in concepts and interfaces without prematurely building a sprawling abstraction framework.

### Initial IR proposal

```text
AnalysisDocument
- schema_version
- project/file identity
- language
- analyzed source range
- completeness/status
- nodes: [Node]
- edges: [Edge]
- annotations: [Annotation]
- diagnostics: [Diagnostic]

Node
- id (stable within document)
- kind (entry, operation, decision, loop, call, return, error, exit, ...)
- label
- source_span (file, start/end byte or parser offsets, start/end line/column)
- provenance (parser | AI | user)
- confidence/status where meaningful
- attributes (typed or namespaced, not arbitrary UI state)

Edge
- id
- from, to
- kind (sequence, true, false, loop-back, error, call, ...)
- label (optional)
- provenance

Annotation
- target node/range
- text/category
- provenance and optional confidence
```

Keep parser-derived facts and AI-generated semantic interpretations distinguishable. Prefer byte/parser offsets as canonical locations and derive line/column for display, because line-only spans are ambiguous and can drift. Validate AI-proposed nodes, relationships, and source spans against parsed source before rendering; do not present unverifiable claims as facts. Persist per-folder analysis incrementally as files are analyzed. Stable cross-edit identities and migration details remain subject to the persistence research task.

## 10. Language and parser strategy

The MVP includes a selected catalog of 20 widely used programming languages, organized into three validated coverage tiers. All 20 must be available in the MVP release, but they are not required to have equal structural-analysis depth. Tier labels describe the supported construct coverage and user-visible limitations; they never relax correctness, source mapping, or the requirement not to fabricate analysis.

Use a dated comparison of credible popularity sources (such as developer surveys and public code-hosting language data) alongside product readiness: grammar maturity, Tree-sitter integration quality, construct coverage, representative test corpora, and target-user demand. Record the snapshot date and selection method, and reassess the catalog at roadmap checkpoints. The following is the initial candidate catalog, not a claim of definitive or permanently ordered popularity:

| Tier | Candidate languages | Intended emphasis |
|---|---|---|
| Validation vertical slices | Python, TypeScript | First end-to-end validation; Python for readable control flow and TypeScript for typed and asynchronous web code. These are validation priorities, not a separate coverage promise. |
| Tier 1 — broad general-purpose coverage | JavaScript, Java, C#, Go | Widely used application and service ecosystems; establish robust core structural analysis. |
| Tier 2 — ecosystem and systems breadth | C, C++, PHP, Ruby, Kotlin, Swift, Rust | Systems, web, and mobile code; document language-specific constructs and limits. |
| Tier 3 — focused coverage | SQL, Bash, Dart, Scala, Lua, Elixir, R | Data/query, scripting, mobile, and selected application ecosystems; scope supported constructs explicitly. |

SQL is included for user value, but its query structure is not represented as function-control-flow parity with general-purpose languages. Each catalog entry must identify whether its grammar covers a programming language, query language, or related source format, and define applicable tests accordingly. Before release, verify the catalog against dated popularity evidence and actual grammar availability/version; Tree-sitter grammar availability alone does not establish product support.

Every language requires a documented grammar and version, a language-specific fixture corpus, declared supported/unsupported constructs, source-mapping checks, and an explicit UI representation of partial analysis. Python and TypeScript are the first vertical slices and must exercise the complete project-to-function-to-graph-to-source journey before the same shared contracts are expanded to the remaining languages.

Tree-sitter supplies syntax nodes and positions, not a complete compiler-grade control-flow or call graph. The common conformance suite must cover, where applicable: language detection and valid/incomplete/malformed parsing; function or language-appropriate unit boundaries; sequence, branches, nested loops, early exits/returns, calls, and async constructs; exact source-span and IR relationships; comments, Unicode, and nesting; and constructs that must be reported as partial or unsupported. SQL and shell fixtures use domain-appropriate expectations rather than forcing inapplicable function semantics.

For every supported fixture, tests assert expected structural nodes/edges and their source spans, including that spans are within the original document and select the intended source. Malformed input must preserve file browsing and must not emit fabricated structure. Unsupported or ambiguous constructs must produce explicit diagnostics/partial results rather than guessed edges. Fixtures should be idiomatic, independently reviewed, deterministic, and include regression examples; agents may help author examples, but expected outputs and acceptance coverage require human review.

Tier gates:
- **Tier 1 — core structural:** Grammar loads reproducibly; detection, parsing, declared function boundaries, sequence, conditionals, loops, calls, and early exits pass applicable fixtures with exact source mapping. Malformed and unsupported cases degrade safely and visibly.
- **Tier 2 — extended structural:** Passes every Tier 1 gate plus the language-specific constructs declared for the language (for example, async/await, exceptions, pattern matching, or language-specific declaration forms). No construct is implied supported unless it has fixtures and verified source mapping.
- **Tier 3 — bounded structural:** Language is detected and browsable; a deliberately narrower set of declared constructs passes fixtures and source mapping. Unsupported constructs are explicitly surfaced; the product does not imply Tier 1/2 parity.

All tiers must pass shared safety and integrity gates: no crash on malformed/unsupported fixtures, no fabricated structural facts, valid in-document spans for every emitted mapping, and clear partial/unsupported status. Tier 3 is still useful support, not a parser-only checkbox. A language may not be marked supported until its tier gate passes. Measure coverage by declared construct/case outcomes; do not substitute an arbitrary aggregate percentage for required-case results.

## 11. AI, providers, and structured output

- AI analyzes the opened file or selected symbol and returns structured semantic/logic analysis for Astynex to validate and render. Semantic fluency does not guarantee correct source mapping or control-flow claims, so maintain language-specific conformance/evaluation fixtures.
- Astynex software—not an image-generation model—constructs and renders the native interactive graph from validated structured AI output and parser-backed source facts.
- Start with a versioned structured response schema and strict validation. Bound output size; reject unknown files, node/edge references, and invalid/out-of-source ranges rather than trusting model-provided coordinates. Unsupported/uncertain claims must remain labeled or be omitted.
- Provider integration should isolate protocol details (for example, OpenAI-compatible HTTP versus a distinct messages protocol) from model metadata and UI. The system should make a new provider or compatible endpoint straightforward to add without modifying parser, IR, or visualization code.
- Use OpenCode and other relevant open-source projects as architecture study references for provider breadth and integration patterns. Rust implementation should be designed for Astynex's requirements; any direct reuse of third-party source is a separate, explicit engineering and license-review decision, not implied by architectural study.
- AI analysis is required for analysis/explanation/graph features. Ollama is the supported offline option and must be installed/configured and available; remote providers require explicit code-context disclosure and user action. Never claim code stays local when a remote endpoint is configured.
- Credentials belong in OS-backed secure storage where available. Configuration files may store non-secret provider settings and secret references, never raw secrets by default.
- Persistence policy must specify what is cached (parse facts, AI analysis, graph-ready IR), per-folder database location, content fingerprint/invalidation, schema migration, retention/deletion, and whether prompts/responses contain source. The selected engine direction is SQLite through Rust `rusqlite` with `bundled`; resolve the remaining policy details before implementation.

## 12. Privacy and security

Threat model includes accidental source disclosure, secret leakage through prompts/logs/cache, malicious repositories, oversized or malformed files, unauthorized source modification, and untrusted model output.

Requirements:
- Remote analysis is opt-in per user action; show provider, endpoint/model, and submitted source/context scope.
- Exclude obvious secret files and configurable ignored paths from context construction; do not treat this as a guarantee that secrets are absent.
- Redact provider credentials from logs/errors and never include them in crash reports.
- Treat repository content and model responses as untrusted data; impose size/time limits and validate schemas and source mappings. The AI subsystem is analysis-only and cannot write to project files.
- Document local cache location and provide a clear-data action before persistent caching is introduced.
- Explain that using a third-party endpoint subjects transmitted code to that service’s policies.

## 13. Testing strategy

Testing is a product quality requirement from the first implementation, not a late-stage hardening task. Strict RED → GREEN → REFACTOR TDD is the selected development workflow. Before implementation, establish and record the exact runner and configuration; presence of a test runner alone is not evidence that strict TDD is active.

### Test layers

1. **Unit tests:** IR invariants, source-span conversions, configuration validation, context filtering, error classification, and provider capability logic.
2. **Language conformance tests:** Per-language fixture corpus covering functions, branches, nested loops, early returns, calls, async syntax where applicable, comments, malformed input, and explicitly unsupported constructs. Assert both expected structure and source spans.
3. **Contract tests:** Every provider/protocol adapter passes shared tests for request construction, response parsing, error normalization, timeouts, cancellation, and secret redaction using deterministic mock HTTP responses; no real API key/network required in CI.
4. **Property-based tests:** Use where invariants have broad input spaces (for example, every edge endpoint resolves to a node and every source span lies within the analyzed document). Randomized failures must be reproducible from a recorded seed.
5. **Integration tests:** Exercise project loading → parse → IR → graph adapter and node/source navigation without requiring an interactive window. Keep network and OS keychain dependencies mocked.
6. **UI smoke/manual tests:** Verify representative interaction and accessibility flows on target operating systems; automated core tests must not depend on pixel-perfect snapshots alone.
7. **End-to-end usability validation:** The comprehension experiment in §14 evaluates whether the product improves understanding, not just whether code paths pass tests.

Tests should be deterministic and isolated. CI must not call paid/remote AI services or require developer credentials. Snapshot tests may be used for stable IR output, but assertions should target meaningful behavior and source mapping rather than brittle formatting alone. Every fixed defect should gain a regression test at the narrowest appropriate layer.

### Strict test-driven implementation

For every implementation work item, follow **RED → GREEN → REFACTOR**: add a focused failing test and observe the failure before implementation; make it pass with the smallest coherent change; then refactor while keeping the test suite green. Record the exact runner and observed results in the ODD task evidence. The runner/configuration must be established before source implementation starts.

## 14. Error handling and logging

### Error handling requirements

- Define subsystem-specific typed errors (for example, project I/O, parser, IR validation, provider/protocol, credential store, persistence) with actionable context and preserved underlying causes.
- Convert low-level errors into user-facing messages at the application boundary. Messages explain what failed, what remains available, and a safe next step; do not expose credentials, raw provider payloads, or stack traces as routine UI copy.
- Distinguish recoverable failures, partial results, cancellation, configuration errors, and internal/unexpected failures. Unsupported syntax is a diagnostic/partial-analysis condition, not automatically a fatal application error.
- Preserve unaffected state: a provider failure must not close the source file; a parse failure must not prevent browsing; one failed language adapter must not crash project exploration.
- Bound retries, response sizes, file sizes, and execution time. Retry only transient operations when safe, with cancellation and backoff; do not retry validation/authentication failures blindly.
- Attach stable error/diagnostic codes and correlation/request identifiers where useful. Do not include source text, API keys, prompts, or full model responses in error metadata by default.

### Logging requirements

Use a centralized Rust structured logging/tracing stack from the beginning (recommended baseline: `tracing` with `tracing-subscriber`). Emit appropriate levels (`error`, `warn`, `info`, `debug`, `trace`) and structured fields for operation, subsystem, duration, outcome, and opaque correlation IDs. Default production logging should be useful but conservative; verbose trace logging is opt-in and must preserve redaction guarantees.

Never log API keys, authorization headers, full source contents, prompts, model responses, or credential-store values. Avoid logging absolute file paths by default; where paths are necessary for local diagnostics, document the policy and provide redaction. Logs should rotate or have bounded size if persisted. Logging configuration failure must not prevent the application from starting; provide a safe fallback and communicate it through diagnostics.

## 15. Success metrics and validation

The north-star validation is comparative comprehension, not number of graph nodes or model integrations.

### MVP experiment
Run a small usability study with developers unfamiliar with selected sample functions. Compare source-only comprehension against Astynex’s linked views, using equivalent tasks and counterbalancing order where feasible.

Measure:
- Time to correctly explain purpose and main branches.
- Accuracy on branch/return/error-path questions.
- Ability to map an explanation claim back to source.
- User-reported confidence calibrated against correctness.
- Task completion and interaction failures.

Set numeric pass thresholds after pilot/baseline data; avoid inventing targets before observing task difficulty. A release decision should require evidence that the visual representation improves speed or accuracy without reducing source traceability or misleading users.

### Product instrumentation (privacy-preserving)
If telemetry is ever added, make it opt-in, document event fields, and never collect source, prompts, file paths, or model output by default. Initial validation can be conducted through consented usability sessions without product telemetry.

## 16. MVP acceptance criteria

A candidate MVP is acceptable when:
1. A user can open a local project, browse to a supported file, and read its source without configuring an AI provider.
2. All 20 catalog languages are available in the MVP, each assigned a coverage tier whose required conformance gate passes.
3. Python and TypeScript pass the end-to-end file/symbol-to-analysis-to-native-graph-to-source validation before the remaining languages are accepted.
4. AI analyzes the opened file or selected applicable symbol plus explicitly scoped directly related context; Astynex validates the structured result and renders an interactive graph with verifiable source mappings.
5. Provider-unavailable states preserve project/file/source browsing but disable analysis, AI explanation, and graph-generation features; Ollama works as the offline AI option when installed/configured.
6. AI cannot modify files; invalid, unmappable, or unsupported output is rejected, omitted, or explicitly qualified.
7. Remote transmission is disclosed before invocation and limited to the stated context.
8. Representative unit, language-conformance, contract, and integration tests cover parser constructs, malformed input, IR validation, provider/schema failure, error containment, and source navigation; CI requires no secrets or paid network calls.
9. Failures surface stable diagnostics and actionable user messages without leaking secrets or source by default.
10. Structured logs support debugging while respecting the privacy/redaction requirements in §14.
11. The comparative comprehension pilot is completed and its findings inform whether to expand scope.

## 17. Risks and mitigations

| Risk | Consequence | Mitigation |
| --- | --- | --- |
| AI diagrams look plausible but misrepresent behavior | Users trust false explanations | Versioned structured output, parser-checked source mappings, provenance, explicit partial/unknown states, conformance fixtures, comprehension tests |
| Tree-sitter syntax is mistaken for semantic analysis | Incorrect call/control-flow claims | State analysis limits; test ambiguous constructs; avoid unsupported edges |
| Graph density overwhelms users | Slower comprehension than source | Function-sized scope, progressive detail, layout prototype, focus mode |
| AI output varies or is malformed | Unreliable UX and broken graphs | Versioned schema, strict validation, deterministic fallback |
| Remote code disclosure surprises users | Privacy harm and loss of trust | Explicit per-action disclosure, directly-related-context preview, Ollama offline option, no source transmission before user action |
| Provider abstraction expands prematurely | Maintenance burden delays core value | Implement the smallest useful integration set; separate concepts without building a marketplace |
| Language scope is too broad | Shallow support and fragile parser behavior | Include 20 languages in the MVP with graduated tier gates; validate Python and TypeScript first, require per-language conformance fixtures, documented limits, and no unsupported claims |
| Provider breadth outpaces quality | Inconsistent errors, capabilities, or privacy behavior across integrations | Stable protocol boundaries, shared contract tests, representative integrations first, and provider-specific capability declarations |
| Studied upstream code is reused without adequate review | Attribution, license, or maintenance obligations are missed | Verify repository/version/license and notices before any code reuse; keep architecture study distinct from source reuse |
| Local model performance is poor | Offline analysis is slow or unusable | Treat Ollama inference as an explicitly tested capability; document hardware/model requirements and retain source browsing when unavailable |

## 18. Roadmap hypothesis

1. **Discovery/prototype:** Validate function-level source ↔ flow interaction and language choice with representative code.
2. **MVP:** Linux/Windows project and source browsing; on-demand AI analysis of an opened file/selected symbol with directly related context; validated native interactive graph and explanation; per-folder incremental persistence; required AI provider with Ollama as offline option; and a small extensible provider set. Source reading remains available without a provider. Include all 20 languages at graduated, tested coverage tiers; validate Python and TypeScript end to end first and satisfy the declared gate for each remaining language.
3. **Hardening:** Usability/accessibility, performance envelope, cache decisions, provider contract tests, and reliability improvements based on evidence across the supported language tiers.
4. **Post-MVP expansion:** Improve construct coverage and tier placement, then consider call-graph and cross-file navigation with explicit completeness limits, dependency/project graphs, data-flow views, and richer local inference where validated demand supports them.

This is a sequence of hypotheses, not a commitment to deliver every phase.

## 19. Open product decisions

These decisions should be resolved before detailed implementation planning:
1. Which dated popularity sources and language-readiness criteria should periodically refresh the 20-language MVP catalog?
2. Which user cohort and sample projects should be prioritized for the comprehension pilot?
3. Which content invalidation/migration strategy, cache contents, retention/deletion policy, and database location best fit the selected per-project-folder SQLite database (`rusqlite`, `bundled`)? The engine choice is made; these implementation details remain open.
4. Which remote AI provider/protocol should join Ollama in the minimum MVP integration set?
5. Which initial file-size/context limits, ignore rules, and scale-test envelope should be used for on-demand analysis?
6. Which measurable pilot criteria should validate product value?
7. What policy should govern direct third-party code reuse versus independently implementing patterns learned from architecture studies?

## 20. Recommended additions to the original concept

The following are important because they reduce product risk, not because they add feature breadth:

1. **Make uncertainty and provenance visible.** Label source/parser facts separately from AI interpretation; expose partial analysis and unsupported constructs.
2. **Preserve source browsing without a provider.** Users can open/read code, while analysis, explanations, and graph generation require a configured provider; Ollama is the offline route.
3. **Add a context preview/privacy gate.** Let users see what code would leave the machine before a remote request, with ignore/exclusion controls.
4. **Add source-range contracts to the IR.** Byte/offset spans, provenance, document completeness, and diagnostics are foundational for dependable bidirectional navigation.
5. **Validate comprehension empirically.** Compare with source-only reading before investing in provider breadth or a whole-project graph.
6. **Define accessibility and graph-scale behavior.** Keyboard interaction, non-color cues, focus mode, and readable large-function behavior are core requirements for a graph-oriented tool.
7. **Threat-model repository and model inputs.** Repositories and LLM output are untrusted; enforce size limits, schema checks, secret-safe logs, and bounded context.
8. **Keep cache and telemetry decisions explicit.** Source-derived artifacts can be sensitive even when stored locally; specify retention and deletion before persistence.
9. **Separate source facts from AI semantic analysis.** Tree-sitter is not a compiler or whole-program analyzer; validate AI-produced graph data against source and communicate what is known, inferred, or uncertain.
10. **Prototype before committing to the entire UI stack.** Verify that the chosen graph widget can support selection, source synchronization, graph scale, and accessible navigation in the target desktop framework.
11. **Ship broad language availability without promising uniform depth.** Include a transparently ranked 20-language catalog in the MVP, validate Python and TypeScript end to end first, and require every language to pass an explicit tier gate and source-mapping fixtures; AI familiarity or grammar availability is not proof of reliable structural analysis.
12. **Make provider extensibility a first-class architecture concern.** Study proven multi-provider systems, keep provider/protocol/model boundaries explicit, and validate integrations with shared contract tests; broad support should not mean unchecked integration count.
13. **Separate architecture learning from code reuse.** Study OpenCode as an engineering reference, but review the exact upstream repository/version/license before any direct source reuse and record provenance for reused components.
14. **Make strict TDD, errors, and observability foundational.** Use RED-GREEN-REFACTOR from the first implementation, define language conformance fixtures, typed subsystem errors, graceful degradation, and privacy-safe structured logging.
15. **Keep architecture scalable through boundaries, not speculation.** Separate UI, domain/IR, parsers, providers, storage, and diagnostics; add abstractions and crate boundaries when real variation justifies them, and enforce dependency direction with tests/review.

---

**Document note:** This PRD consolidates the supplied concept and interface reference into a testable product direction. Confirmed product constraints include Linux and Windows targets, on-demand file/symbol analysis with directly related context, AI-required analysis and native software-rendered graphs, source-read access without a provider, no AI file modification, per-folder incremental persistence, and strict TDD. The 20-language catalog and tiers remain subject to dated popularity/grammar validation; persistence technology, initial remote provider set, scale envelope, and usability thresholds remain to be validated.
