# Idea

## Goal

Make the current font pipeline obey abolishpdfs' stated native-text integrity contract.

The current architecture is intentionally strict:

* Extractable PDF text must be real DOM text.
* Embedded fonts that are required for faithful reconstruction must become actual processed browser fonts.
* We must not report successful conversion merely because HTML was emitted.
* We must not silently substitute `sans-serif` when a text object depends on an embedded font that failed FontForge processing.

At present, `DocumentModel::prepare_fonts()` records FontForge errors as diagnostics but native text can remain reconstructable, and `HtmlWriter` can subsequently render that text using `sans-serif`.

That violates the intended contract.

## Definitions

**Embedded font:** A `FontSource` with `embedded == Some(true)` and usable source bytes. It is the font resource supplied by the PDF and is the only font category covered by this processing contract.

**Processed font:** A validated `ProcessedFont` returned by the FontForge worker. It includes the browser asset bytes, generated family name, glyph count, and advance metrics required by HTML output.

**Font processing state:** The explicit state of an embedded font preparation job: pending before preparation, ready with a processed font, or failed with the original processing failure. A non-embedded font is not a failed processing state merely because it has no processed asset.

**Native text:** Text represented by real DOM text in the generated HTML. A native-text object must use a ready processed font when its PDF font is an embedded font.

**Text-integrity failure:** A structured conversion failure identifying a page and paint-order object whose extractable text cannot satisfy the native-text contract. It is distinct from a non-fatal diagnostic.

**System-font case:** A text object whose font does not require an embedded processed asset under the existing model. This path may continue to use the browser/system font behavior and is not made fatal by this change.

## Desired outcome

Font preparation records an explicit result for every embedded font. A successful job makes its processed family available to native text. A failed job preserves the FontForge error as a diagnostic and marks every dependent native-text object as a structured text-integrity failure before output files are written. The writer cannot turn that failed dependency into a silent `sans-serif` fallback. Legitimate system-font cases retain their existing behavior.

## Scope

Fix the font-processing state model and failure propagation from `FontForgeWorker` through `DocumentModel` and native-text output. Add behavioral tests for successful embedded processing, failed dependent processing, diagnostic preservation, fallback prevention, and valid system-font behavior.

Do not work on font-resource identity or deduplication, PDF character-code/glyph-ID mapping, CFF/CID/Type1/Type3 support, runtime downloading, browser/Playwright integration, cross-platform packaging, or general layout improvements.

## Constraints

Use the smallest explicit domain model that distinguishes pending, ready, and failed embedded-font processing. Keep `HtmlWriter` focused on serializing already-resolved model state. Validate native-text eligibility before writing any output artifact. Preserve the original FontForge error in diagnostics while returning a structured integrity error for dependent text. Follow `AGENTS.md`, existing model/output boundaries, file-size limits, and repository validation conventions.

## Exclusions

This change does not change FontForge invocation, font identity, mapping proof, PDFium extraction, visibility analysis, glyph reconstruction, browser validation, or the policy for text that already has a non-font reconstruction failure. It does not make every missing processed asset fatal.

## Acceptance criteria

1. An embedded font whose FontForge job succeeds produces native text using the processed family.
2. An embedded font whose FontForge job fails causes conversion to fail if native text depends on it.
3. Such failure cannot silently render as `sans-serif`.
4. A legitimate non-embedded/system-font case follows the intended existing behavior rather than becoming unnecessarily fatal.
5. Diagnostics preserve the original FontForge error.
6. `cargo test` passes.
7. `cargo clippy --all-targets --all-features -- -D warnings` passes if that is the repository's current validation convention.
8. `cargo fmt --check` passes.

## Architecture constraints

Follow `AGENTS.md`.

* Keep files focused.
* Do not introduce speculative abstraction.
* Do not modify unrelated PDFium or layout code.
* Do not exceed the repository's file-size rules.
* Prefer explicit domain names over booleans such as `font_failed`.

## Documentation

Update the relevant current documentation only where necessary to make the new invariant clear. Do not rewrite archived planning documents as though history changed.

At completion, report:

* files changed;
* new invariant;
* tests added;
* any cases that remain intentionally capable of using a system/browser font;
* anything another Wave 1 branch needs to know.
