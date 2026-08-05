# Browser Corpus Validation

## Request

Create a repeatable compatibility-corpus gate for high-fidelity PDF-to-HTML output.

## Problem

The repository has fixtures and diagnostic tooling, but conversion correctness is not yet established by a consistent browser-level check. Geometry, native DOM text, copied text, visual rendering, links, fonts, and fallback behavior can regress without one observable acceptance workflow.

## Definitions

**Compatibility corpus:** The checked-in PDFs and expected feature classifications in `tests/fixtures/` used to exercise supported output behavior. It excludes licensed upstream documents that lack recorded redistribution provenance.

**Corpus gate:** The automated and manual checks that must pass before a conversion behavior is considered stable. It covers observable output, not internal implementation details.

## Desired outcome

Running the corpus checks produces deterministic diagnostic results and browser evidence for every fixture, including native DOM text, copied text, page geometry, screenshots, links, embedded fonts, and documented fallback decisions.

## Scope

- Define fixture categories and expected classifications for text, transforms, spacing, fonts, graphics, links, and fallback.
- Extend `tools/corpus.py` and `tools/browser_corpus.py` into one documented validation workflow.
- Add browser assertions for DOM text, selection/copy output, page count/order, navigation, and screenshot capture.
- Record comparison tolerances and failure output suitable for local and CI use.

## Constraints

- Use the existing fixtures, manifest, Rust test framework, and Playwright tooling.
- Treat browser and PDFium antialiasing differences with perceptual tolerances rather than byte-identical screenshots.
- Keep licensed or externally sourced PDFs out of the repository until provenance is recorded.
- This branch adds validation; it does not change conversion behavior to make a fixture pass.

## Exclusions

- New PDF extraction algorithms.
- FontForge processing.
- Release packaging or CI infrastructure beyond the commands needed to run the corpus gate.
