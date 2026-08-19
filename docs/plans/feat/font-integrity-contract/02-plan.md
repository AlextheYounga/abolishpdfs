# Plan

## Current behavior

`FontSource` in `src/model/font.rs` stores embedded source metadata and represents processing with `processed: Option<ProcessedFont>`. That option is `None` both before preparation and after a failed or unnecessary job. `FontForgeWorker::process` in `src/fonts/worker.rs` returns a typed `FontJobError` for missing data, unsupported formats, unproven mappings, process failures, timeouts, and invalid output.

`DocumentModel::prepare_fonts` in `src/model/document.rs` runs the worker once per catalog font. Success stores the processed font; failure appends a `DocumentDiagnostic` and leaves `processed` empty. Existing native-text decisions in `src/model/text.rs` do not include font-processing failure as a reason.

`src/text/runs.rs` prepares runs only for objects already marked `NativeText`. `HtmlWriter::render_run` in `src/output/document.rs` uses the processed family when one exists and otherwise emits `sans-serif`. `HtmlWriter::write_to` rejects only failures returned by `DocumentModel::discovered_text_failures`, so an object marked `NativeText` can currently pass output validation despite depending on a failed embedded font.

## Intended behavior

Every embedded `FontSource` has an explicit processing state. Successful FontForge processing produces `Ready(ProcessedFont)`; a failed job produces `Failed` containing the original processing failure; preparation begins from `Pending`. Fonts that do not require embedded processing remain an explicit valid system-font case rather than becoming failed.

Before native runs are serialized, document/model validation examines each `NativeText` object. If its font is an embedded font in the failed state, validation adds a `TextIntegrityFailure` with a font-processing reason and the page/object identity. The existing FontForge diagnostic remains in `DocumentModel::diagnostics`. `HtmlWriter::write_to` returns the structured integrity error before creating output files, and serialization does not use `sans-serif` to conceal a failed embedded-font dependency. A ready embedded font emits its processed `@font-face` asset and family. A valid system-font case retains the current browser/system-font path.

## Approach

1. Replace the ambiguous optional processed-font field with a focused `FontProcessingState` and a structured failure value that preserves the worker error.
2. Update font preparation to transition each catalog entry to ready or failed, retaining a diagnostic for every failed worker job.
3. Add a font-processing failure variant to the text-integrity model and derive dependent failures from `NativeText` objects that reference failed embedded fonts.
4. Make output validation consume those derived failures before filesystem writes. Keep writer rendering limited to ready embedded fonts and the existing system-font case; remove the accidental fallback path for failed embedded dependencies.
5. Add focused model/output tests using the existing fake FontForge worker boundary and test fixtures.

## Responsibilities and boundaries

* `src/fonts/worker.rs` owns FontForge execution and remains the source of typed processing errors.
* `src/model/font.rs` owns the embedded-font processing state and failure value.
* `src/model/document.rs` owns state transitions, diagnostic retention, and document-level dependency validation.
* `src/model/text.rs` owns the text-integrity reason and page/object failure data.
* `src/text/runs.rs` prepares only text objects that have already passed reconstruction eligibility; it does not decide font-processing policy.
* `src/output/document.rs` refuses invalid documents before writing and serializes ready assets without deciding whether a font should have been processed.
* `src/output/document_tests.rs` owns observable HTML, asset, and no-partial-output coverage.

## Affected areas

* `src/model/font.rs`: explicit processing state and structured failure representation.
* `src/model/document.rs`: preparation transitions, diagnostics, and native-text dependency validation.
* `src/model/text.rs`: font-processing text-failure reason and exports.
* `src/output/document.rs`: validation/rendering behavior that cannot hide failed embedded fonts.
* `src/output/document_tests.rs` and relevant model tests: success, failure, diagnostics, fallback prevention, and system-font regressions.

## Decisions

* Keep the state on `FontSource` because document text references catalog font IDs and the catalog is the existing owner of font preparation data.
* Represent worker failure as a domain value that retains the original `FontJobError` text, so diagnostics remain actionable without coupling text objects to the worker process API.
* Derive font-dependent text failures at the document boundary rather than in `HtmlWriter`; this keeps policy out of serialization and gives `write_to` one pre-write integrity gate.
* Require a ready processed font only for native text that references an embedded font. Do not infer failure from `processed == None`, because that would incorrectly reject valid system-font cases.
* Preserve existing non-font reconstruction failures and aggregate them with font-processing failures in the same structured output error.

## Risks

* A native text object may be associated with a font ID that is absent from the catalog; validation must report that as an integrity failure rather than allowing an implicit browser fallback.
* Repeated preparation must not erase an earlier failed state or duplicate diagnostics unexpectedly; state transitions must remain deterministic for one document.
* Existing fixtures that intentionally omit a font need explicit system-font setup so they do not accidentally exercise the failed embedded-font path.
* Output must remain artifact-free when a font-dependent integrity failure is discovered.

## Validation

* Model tests prove embedded fonts transition to ready on worker success and failed on worker error while preserving the diagnostic message.
* Document tests prove a `NativeText` object depending on a failed embedded font becomes a structured font-processing text failure with page and paint-order identity.
* Output tests prove ready embedded fonts emit their processed family and asset, failed embedded fonts return `OutputError::TextIntegrity` without writing artifacts, and failed dependencies never produce `font-family:sans-serif` output.
* A regression test proves a legitimate non-embedded/system-font object retains existing successful output behavior.
* `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` pass.
* The final diff contains only the planned model, output, test, and necessary documentation changes.
