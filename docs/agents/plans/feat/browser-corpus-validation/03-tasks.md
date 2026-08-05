# Tasks

## Implementation

- [ ] Add explicit native-text, copied-text, navigation, asset, and screenshot expectations to `tests/fixtures/manifest.json`.
- [ ] Extend `tests/corpus_manifest.rs` to reject missing or malformed expectations.
- [ ] Update `tools/corpus.py` to run every fixture and emit structured diagnostic results.
- [ ] Update `tools/browser_corpus.py` to assert DOM text, copied text, page ordering, fragment navigation, expected assets, and screenshot tolerances.
- [ ] Document the corpus commands, dependencies, output locations, and failure interpretation.

## Validation

- [ ] Run `cargo test --test corpus_manifest` and verify every fixture is covered.
- [ ] Run the diagnostic corpus command and verify all fixture classifications match the manifest.
- [ ] Run the browser corpus command and verify expected text, copy, navigation, assets, and screenshots.
- [ ] Run the full Rust formatting, test, clippy, and diff checks.

## Completion

- [ ] Review that validation changes do not alter conversion behavior.
- [ ] Record validation results and any deliberately unsupported fixture behavior in completion notes.

## Completion notes

Not started.
