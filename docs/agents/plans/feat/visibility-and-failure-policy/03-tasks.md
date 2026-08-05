# Tasks

## Implementation

- [ ] Extend graphics and text model types with visibility reasons and required paint context.
- [ ] Preserve recursive form paint order and supported clipping/transparency metadata during extraction.
- [ ] Implement conservative object-level coverage analysis with explicit ambiguity results.
- [ ] Feed visibility decisions into background suppression and native-text integrity validation.
- [ ] Add fixtures for full coverage, partial overlap, clipping, transparency, and nested forms.
- [ ] Expose visibility reasons in diagnostic output.

## Validation

- [ ] Run focused visibility and model tests for all supported decision classes.
- [ ] Run browser corpus checks for visual fidelity and duplicate-text absence.
- [ ] Verify uncertain extractable text is reported under the native-text policy.
- [ ] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [ ] Review that overlap alone never produces an opaque-coverage decision.
- [ ] Record supported visibility facts, validation results, and deliberate limitations in completion notes.

## Completion notes

Not started.
