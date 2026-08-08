# Tasks

## Implementation

- [x] Extend graphics and text model types with visibility reasons and required paint context.
- [x] Preserve recursive form paint order and supported clipping/transparency metadata during extraction.
- [x] Implement conservative object-level coverage analysis with explicit ambiguity results.
- [x] Feed visibility decisions into background suppression and native-text integrity validation.
- [x] Add focused model fixtures for full coverage, partial overlap, clipping, transparency, and nested forms.
- [x] Expose visibility reasons in diagnostic output.

## Validation

- [x] Run focused visibility and model tests for all supported decision classes.
- [ ] Run browser corpus checks for visual fidelity and duplicate-text absence.
- [x] Verify uncertain extractable text is reported under the native-text policy.
- [x] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [x] Review that overlap alone never produces an opaque-coverage decision.
- [x] Record supported visibility facts, validation results, and deliberate limitations in completion notes.

## Completion notes

Implemented the model-only visibility pass in `src/text/visibility.rs`. It classifies text as visible, covered by complete
opaque containment, or ambiguous. Partial overlap, missing bounds, transparency, and clipping never produce an opaque
coverage result. PDFium extraction records path opacity derived from supported fill/stroke state, clip-path presence,
active state, and recursive form paint order. Ambiguous native text is retained in the raster background and receives an
object-scoped diagnostic; diagnostic JSON includes visibility and paint metadata.

Validation passed: `cargo fmt --all`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`,
`git diff --check`, and `python3 tools/corpus.py validate`. Browser corpus execution remains pending because this
worktree has no PDFium shared library or browser baseline environment.
