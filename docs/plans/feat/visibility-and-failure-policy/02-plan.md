# Plan

## Current behavior

`src/model/page.rs` stores graphics in paint order with optional bounds and activity, `src/pdfium/document.rs` extracts page objects and forms, and `src/pdfium/background.rs` suppresses selected text objects during bitmap rendering. No dedicated analyzer evaluates later-object coverage, clipping, transparency, or ambiguity before reconstruction.

## Intended behavior

An object-level visibility pass walks page paint order and recursive form contents, evaluates supported coverage facts, and assigns a reasoned decision to each text object. Fully proven native text is emitted normally; unsupported or ambiguous cases receive documented fallback or native-text failure treatment according to the integrity policy.

## Approach

Implement a conservative analyzer over the existing owned graphics model. Track paint order and inherited form context, reject unsupported clipping/transparency combinations, and only classify text as covered when supported facts prove complete opaque coverage. Feed the decision into background suppression and text-integrity validation without querying PDFium from the writer.

## Responsibilities and boundaries

- `src/pdfium` extracts object geometry, order, form children, clipping, and paint metadata.
- A focused `src/text` or model visibility module owns visibility decisions.
- `src/model/page.rs` stores decisions and reasons as owned values.
- `src/pdfium/background.rs` consumes selected object identities for raster rendering.
- Conversion validation applies the native-text failure policy.
- `src/output/document.rs` serializes the already-decided result.

## Affected areas

- `src/model/page.rs` and graphics model types
- `src/pdfium/document.rs` and `src/pdfium/background.rs`
- New visibility-analysis module and focused tests
- `src/model/text.rs` reconstruction reasons
- Graphics, clipping, transparency, and form fixtures
- Diagnostic output and browser corpus expectations

## Decisions

- Analyze at PDF object granularity because PDFium exposes reliable object association and background suppression operates at that boundary.
- Require complete supported evidence for an opaque-coverage classification; overlap alone yields ambiguity.
- Preserve paint order and recursive form context in the model so decisions are deterministic and inspectable.
- Route ambiguous text through explicit conservative policy rather than silently declaring native success.

## Risks

- Conservative decisions increase raster usage and can expose gaps in the native-text contract.
- Recursive forms and inherited clipping can make object identity unstable; tests must cover nested paint order.
- Transparency and soft masks may remain unsupported and require explicit diagnostics rather than approximated visibility.

## Validation

- Unit tests cover opaque coverage, partial overlap, clipping, transparency, nested forms, and ambiguous bounds.
- Diagnostic output records each non-native visibility reason.
- Browser corpus checks prove covered and clipped text remains visually correct without duplicate native text.
- Native-text integrity tests prove uncertain extractable text is reported rather than silently hidden.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` pass.
