# Tasks

## Implementation

- [ ] Define prepared text-run and layout-break model data.
- [ ] Implement glyph compatibility checks for style, transform, baseline, progression, and visibility state.
- [ ] Implement run formation while preserving source order and object boundaries.
- [ ] Calculate run-level advance compensation and local discontinuity offsets.
- [ ] Update the writer to serialize prepared runs and spacing metadata.
- [ ] Add geometry fixtures for ordinary spacing, mixed styles, transforms, and layout breaks.

## Validation

- [ ] Run focused text-layout unit tests.
- [ ] Run output serialization tests for run geometry and CSS compensation.
- [ ] Run browser corpus screenshot, selection, and copied-text checks.
- [ ] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [ ] Review output size and verify no incompatible glyph styles are merged.
- [ ] Record tolerance choices, validation results, and deliberately unsupported writing modes in completion notes.

## Completion notes

Not started.
