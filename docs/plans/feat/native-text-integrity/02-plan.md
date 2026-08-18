# Plan

## Current behavior

`src/model/text.rs` stores `ReconstructionDecision`, `src/pdfium/background.rs` can suppress selected native text while rendering a PNG, and `src/output/document.rs` omits text marked for background fallback. `src/cli.rs` exposes diagnostic and output modes, but there is no conversion-level failure for a document containing text that was not emitted natively.

## Intended behavior

Model construction records every text object that is not safely native. The conversion coordinator aggregates those records into a structured error containing page and object identifiers, and the normal output command refuses successful output when the document contains extractable text without a native representation. Graphic-only raster backgrounds remain valid.

## Approach

Introduce an owned text-integrity diagnostic at the model/conversion boundary. Classify each text object once, validate that every extractable object selected for successful conversion is rendered by the HTML writer, and return a structured error before writing incomplete output. Extend serialized diagnostics and tests to make the decision observable.

## Responsibilities and boundaries

- PDFium adaptation records extraction facts and object identity.
- `src/model/text.rs` owns reconstruction decisions and failure reasons.
- Conversion orchestration owns document-level integrity validation.
- `src/output/document.rs` owns native text emission and never suppresses an unvalidated text object.
- `src/cli.rs` owns human-readable failure reporting and exit status.

## Affected areas

- `src/model/text.rs` and related model exports
- Conversion orchestration and error types
- `src/output/document.rs` and output tests
- `src/cli.rs`
- Diagnostic serialization tests and fixture expectations
- `docs/agents/archive/003-text-output-contract.md` as the governing source record

## Decisions

- Use object-scoped failures because PDFium currently provides stable text-object association, while per-glyph suppression is not a safe public operation.
- Validate before writing files so failed conversion cannot leave an apparently successful partial artifact.
- Keep failure diagnostics structured and serialize page number, object identity, reason, and semantic text availability.
- Treat graphic fallback and text failure as separate states.

## Risks

- Existing fixtures that intentionally use fallback may become failures and require explicit fixture classification.
- A conservative failure policy reduces successful output coverage until native reconstruction improves.
- Error propagation must preserve useful PDFium diagnostics rather than replacing them with a generic text error.

## Validation

- Unit tests prove each unsupported text reason becomes a page/object diagnostic.
- Output tests prove successful documents contain expected native text and failed documents write no success artifact.
- Diagnostic JSON includes all text-integrity failures.
- CLI integration checks return a non-zero status and actionable error text for failed conversion.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` pass.
