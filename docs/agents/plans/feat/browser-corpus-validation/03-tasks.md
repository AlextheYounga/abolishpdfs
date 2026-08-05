# Tasks

## Implementation

- [x] Add explicit native-text, copied-text, navigation, asset, and screenshot expectations to `tests/fixtures/manifest.json`.
- [x] Extend `tests/corpus_manifest.rs` to reject missing or malformed expectations.
- [x] Update `tools/corpus.py` to run every fixture and emit structured diagnostic results.
- [x] Update `tools/browser_corpus.py` to assert DOM text, copied text, page ordering, fragment navigation, expected assets, and screenshot tolerances.
- [x] Document the corpus commands, dependencies, output locations, and failure interpretation.

## Validation

- [x] Run `cargo test --test corpus_manifest` and verify every fixture is covered.
- [ ] Run the diagnostic corpus command and verify all fixture classifications match the manifest.
- [ ] Run the browser corpus command and verify expected text, copy, navigation, assets, and screenshots.
- [x] Run the full Rust formatting, test, clippy, and diff checks.

## Completion

- [x] Review that validation changes do not alter conversion behavior.
- [x] Record validation results and any deliberately unsupported fixture behavior in completion notes.

## Completion notes

The manifest now records observable browser expectations for all six generated fixtures.
Diagnostic and browser commands remain separate from conversion: the diagnostic command
checks the owned model, while Playwright checks generated HTML and assets. Screenshot
baselines are deliberately not checked in yet; all fixtures use capture-only status with
explicit perceptual tolerances. `cargo test`, `cargo fmt --check`, clippy with warnings
denied, `cargo test --test corpus_manifest`, Python manifest validation, and
`git diff --check` passed. The diagnostic and browser corpus runs were not executed
because this environment has no configured PDFium library path and no Python Playwright
installation.
