# Tasks

## Implementation

- [x] Define prepared text-run and layout-break model data.
- [x] Implement glyph compatibility checks for style, transform, baseline, progression, and visibility state.
- [x] Implement run formation while preserving source order and object boundaries.
- [ ] Calculate run-level advance compensation and local discontinuity offsets.
- [x] Update the writer to serialize prepared runs and spacing metadata.
- [ ] Add geometry fixtures for ordinary spacing, mixed styles, transforms, and layout breaks.

## Validation

- [x] Run focused text-layout unit tests.
- [x] Run output serialization tests for run geometry and CSS compensation.
- [ ] Run browser corpus screenshot, selection, and copied-text checks.
- [x] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [ ] Review output size and verify no incompatible glyph styles are merged.
- [ ] Record tolerance choices, validation results, and deliberately unsupported writing modes in completion notes.

## Completion notes

Run preparation now groups compatible native glyphs per source text object, preserves
source order and spaces, and starts new runs for style, baseline, transform, or
large progression discontinuities. Pure translations use first-glyph bounds;
transformed runs retain normalized y-flipped CSS matrices. `letter_spacing` is
serialized and tested, but remains zero until browser/font advance measurement is
available from the font-processing work; generated layout spaces and local offset
compensation remain deliberately unsupported in this increment.
