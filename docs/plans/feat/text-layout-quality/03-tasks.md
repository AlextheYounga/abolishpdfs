# Tasks

## Implementation

- [x] Define prepared text-run and layout-break model data.
- [x] Implement glyph compatibility checks for style, transform, baseline, progression, and visibility state.
- [x] Implement run formation while preserving source order and object boundaries.
- [x] Calculate run-level advance compensation and local discontinuity offsets.
- [x] Update the writer to serialize prepared runs and spacing metadata.
- [x] Add geometry fixtures for ordinary spacing, mixed styles, transforms, and layout breaks.

## Validation

- [x] Run focused text-layout unit tests.
- [x] Run output serialization tests for run geometry and CSS compensation.
- [ ] Run browser corpus screenshot, selection, and copied-text checks.
- [x] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [x] Review output size and verify no incompatible glyph styles are merged.
- [x] Record tolerance choices, validation results, and deliberately unsupported writing modes in completion notes.

## Completion notes

Run preparation now groups compatible native glyphs per source text object, preserves
source order and spaces, and starts new runs for style, baseline, transform, or
large progression discontinuities. Pure translations use first-glyph bounds;
transformed runs retain normalized y-flipped CSS matrices. `letter_spacing` is
computed from parseable embedded-font metrics, with residual per-glyph differences
represented as local inline offsets. Fallback fonts retain zero compensation
until browser/font advance measurement is available. Generated layout spaces
remain deliberately unsupported; source and PDFium-generated spaces are kept.

Run-formation tolerances: font-size compatibility is 5% relative, horizontal
baseline and transformed cross-track tolerance is 0.5 PDF units, matrix
component tolerance is 0.001, compensation residual tolerance is 0.05 PDF
units, and a progression gap above 2em starts a new run. Vertical writing is
unsupported in this increment. Browser screenshot, selection, and clipboard
checks remain pending until a PDFium runtime and browser harness are configured.
