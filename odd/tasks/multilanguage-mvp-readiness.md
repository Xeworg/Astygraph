# Multilanguage MVP Readiness

## Goal
Define the MVP as a 20-language release with explicit, graduated structural-analysis coverage and per-language conformance gates. Validate the end-to-end comprehension flow first with Python and TypeScript.

## Decisions
- All 20 selected languages are included in the MVP release; tiers communicate coverage depth, not correctness tolerance.
- Python and TypeScript are the initial vertical-slice languages.
- Every tier must preserve source mapping, avoid fabricated structure, and communicate unsupported/partial analysis.
- Each language requires versioned grammar metadata and language-specific fixtures against common conformance expectations.

## Tasks
1. [x] Reconcile MVP scope, exclusions, roadmap, and language strategy in PRD.
2. [x] Specify common conformance suite, tier gates, and 20-language candidate catalog (19 initial candidates plus R as candidate 20).
3. [x] Align acceptance criteria and open decisions with the language MVP.

## Evidence
- PRD update is the deliverable; no source implementation or tests are part of this documentation task.
