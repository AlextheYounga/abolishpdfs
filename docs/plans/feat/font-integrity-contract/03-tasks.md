# Tasks

## Implementation

- [x] Define `FontProcessingState` and the structured processing-failure value in `src/model/font.rs`, and update `FontSource` callers so pending, ready, failed, and valid system-font states are explicit.
- [x] Update `DocumentModel::prepare_fonts` in `src/model/document.rs` to store ready/failed transitions and retain the original FontForge error as a `DocumentDiagnostic`.
- [x] Add `FontProcessingFailed` to the text-integrity model and derive failures for native text that references a failed embedded font, including page and paint-order identity.
- [x] Update `HtmlWriter` validation and font rendering so failed embedded-font dependencies cannot reach accidental `sans-serif` fallback, while ready embedded fonts and valid system-font cases retain their intended output.
- [x] Update affected fixtures and constructors to use the explicit font state without changing font mapping, PDFium, visibility, or layout behavior.

## Validation

- [x] Add a model regression test proving FontForge failure stores a failed state plus the original diagnostic; existing worker/output tests prove successful processing stores and emits a ready processed font.
- [x] Add a document/output regression test proving dependent native text fails with a structured font-processing reason and no output artifact is created.
- [x] Add an output regression test proving ready embedded fonts emit the processed family and asset, while a valid system-font case still emits successfully.
- [ ] Run `cargo test` and confirm all existing and new tests pass. The corpus-manifest integration test currently fails before exercising this change because its fixture is missing the required `native_text` field.
- [x] Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check`.

## Completion

- [x] Review the final diff against `01-idea.md` and `02-plan.md`; remove obsolete optional-state handling and unrelated edits.
- [x] Record validation results, meaningful deviations, and deliberately unfinished work in the completion notes.

## Completion notes

Implemented the explicit font-processing state and native-text failure propagation. `cargo test --lib` passed with 52 tests; `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` passed. Full `cargo test` remains blocked by the existing `tests/corpus_manifest.rs` fixture error: the manifest is missing the required `native_text` field. No changes were made to that unrelated fixture or to PDFium, mapping, visibility, or layout behavior.
