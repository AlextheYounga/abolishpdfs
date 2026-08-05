# Tasks

## Implementation

- [x] Define serializable object-scoped text failure reasons in the model.
- [x] Record native-emission eligibility and semantic text availability during PDFium extraction.
- [x] Add document-level integrity validation before output file creation.
- [x] Propagate structured failures through the CLI with page and object context.
- [x] Update diagnostic JSON and fixture expectations for native text and explicit failures.
- [x] Add regression tests proving raster graphics do not satisfy the native-text requirement.

## Validation

- [x] Run focused model, output, and CLI failure tests.
- [x] Verify successful fixtures contain expected native DOM text.
- [x] Verify unsupported-text fixtures fail visibly and do not leave successful output artifacts.
- [x] Run formatting, all Rust tests, clippy, and diff checks.

## Completion

- [x] Review all fallback paths for silent text success.
- [x] Record fixture behavior and validation results in completion notes.

## Completion notes

Implemented the model, extraction, output, diagnostics, and documentation changes. Non-native text is represented as `TextFailure` with a serializable page/object reason and semantic-text flag; normal HTML output validates discovered failures before creating any output artifact. Raster backgrounds remain available for graphics, but cannot make failed text conversion successful.

Validation completed: `cargo fmt --check`, `cargo test` (31 tests), `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check`.
