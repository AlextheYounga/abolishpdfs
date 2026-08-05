# Tasks

## Implementation

- [x] Define prepared-font state, diagnostics, and deterministic document-local asset identity.
- [x] Add FontForge executable configuration and process error handling.
- [x] Implement one-font worker jobs with temporary inputs, generated output validation, and bounded failure handling.
- [x] Implement initial TrueType/OpenType subsetting, proven Unicode mapping, browser-safe naming, and metric validation.
- [x] Replace raw font asset emission with processed font assets and generated `@font-face` references.
- [x] Add explicit unsupported handling for fonts outside the initial format and mapping scope.

## Validation

- [x] Run worker tests for success and process failures.
- [ ] Inspect generated fonts for mappings, used glyphs, family names, and metrics (requires FontForge).
- [ ] Run output and browser corpus tests with embedded-font fixtures.
- [x] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [x] Document the required headless FontForge toolchain and local setup.
- [x] Review that raw embedded font bytes are not used as processed output.
- [x] Record supported formats, validation results, and deliberately unsupported formats in completion notes.

## Completion notes

Implemented the worker boundary, deterministic processed-font model, CLI executable configuration, native FontForge script, and fallback diagnostics. Rust validation passes. Browser corpus and generated-font inspection remain pending because FontForge and PDFium fixtures are not installed in this environment. The current worker supports embedded TrueType/OpenType inputs and WOFF2 by default, with explicit WOFF configuration; CID, CFF, Type 1, Type 3, unproven mappings, and missing embedded data remain unsupported. Metric data is validated from the worker response; PDF advance correction is deferred until extraction supplies source advances.
