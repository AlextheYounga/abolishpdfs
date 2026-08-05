# Plan

## Current behavior

`tests/fixtures/manifest.json` classifies generated fixtures, `tests/corpus_manifest.rs` validates manifest coverage, `tools/corpus.py` runs diagnostic conversion, and `tools/browser_corpus.py` opens generated `index.html` files and captures browser output. The browser script does not yet assert text selection, copied content, links, fonts, or defined visual tolerances.

## Intended behavior

The corpus workflow will generate each fixture, inspect its diagnostic classification, open its single-DOM output in a browser, and report failures for missing native text, incorrect copied text, broken page navigation, absent expected assets, or visual differences outside the recorded tolerance.

## Approach

Extend the fixture manifest with observable expectations, then make the Python corpus tools consume those expectations. Keep PDF conversion in the Rust binary and browser observation in Playwright. Store screenshots and failure diagnostics outside tracked source by default, while making the commands and expected results documented and repeatable.

## Responsibilities and boundaries

- `tests/fixtures/manifest.json` owns fixture-level expected capabilities and tolerances.
- `tests/corpus_manifest.rs` owns structural validation that every fixture has a valid classification.
- `tools/corpus.py` owns diagnostic conversion and model-level checks.
- `tools/browser_corpus.py` owns browser DOM, copy, navigation, asset, and screenshot checks.
- Documentation owns the command sequence and interpretation of failures.

## Affected areas

- `tests/fixtures/manifest.json`
- `tests/corpus_manifest.rs`
- `tools/corpus.py`
- `tools/browser_corpus.py`
- `docs/PROJECT.md` or the current project context document
- Browser corpus test fixtures and expected-result data

## Decisions

- Use the existing JSON manifest rather than introducing a second fixture registry.
- Validate native text from the DOM and clipboard content separately because visual correctness does not prove text correctness.
- Capture one representative page per fixture for fast checks and retain full-document checks for navigation and copy behavior.
- Fail on missing expected native text or broken navigation; report screenshot drift using explicit tolerance thresholds.

## Risks

- Clipboard behavior differs across host environments; the harness will use Playwright selection and a documented clipboard fallback.
- Screenshot drift can result from browser or PDFium version changes; tool versions and tolerances must be recorded with failures.
- Fixture expectations can become stale as supported behavior changes; manifest updates must accompany intentional behavior changes.

## Validation

- `cargo test --test corpus_manifest` passes with complete, valid fixture classifications.
- The diagnostic corpus command processes every checked-in fixture and reports the expected feature classes.
- The browser corpus command verifies DOM text, copied text, page count/order, expected navigation, and expected assets.
- Screenshot comparisons pass within recorded tolerances, with failures identifying the fixture and page.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` pass.
