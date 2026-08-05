# Tasks

## Implementation

- [ ] Define prepared-font state, diagnostics, and deterministic document-local asset identity.
- [ ] Add FontForge executable configuration and process error handling.
- [ ] Implement one-font worker jobs with temporary inputs, generated output validation, and bounded failure handling.
- [ ] Implement initial TrueType/OpenType subsetting, proven Unicode mapping, browser-safe naming, and metric correction.
- [ ] Replace raw font asset emission with processed font assets and generated `@font-face` references.
- [ ] Add explicit unsupported handling for fonts outside the initial format and mapping scope.

## Validation

- [ ] Run worker tests for success and process failures.
- [ ] Inspect generated fonts for mappings, used glyphs, family names, and metrics.
- [ ] Run output and browser corpus tests with embedded-font fixtures.
- [ ] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [ ] Document the pinned FontForge toolchain and local setup.
- [ ] Review that raw embedded font bytes are not used as processed output.
- [ ] Record supported formats, validation results, and deliberately unsupported formats in completion notes.

## Completion notes

Not started.
