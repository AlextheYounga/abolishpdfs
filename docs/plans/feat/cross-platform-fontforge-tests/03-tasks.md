# Tasks

## Implementation

- [x] Add `src/bin/fontforge_test_helper.rs` with explicit success, invalid asset, missing asset, malformed response, non-zero exit, and delay modes matching the worker argument layout.
- [x] Add optional executable arguments and a builder to `WorkerConfig`, preserving the current production command when empty.
- [x] Add `tests/fontforge_worker.rs` covering success, missing executable, process failure, malformed response, missing/invalid asset, timeout, and family/glyph/metric validation.
- [x] Remove the Unix shell fixture and `PermissionsExt` dependency from `src/fonts/worker.rs` without changing production lifecycle or validation behavior.
- [x] Review `tools/fontforge_worker.py` for OS-specific path handling and leave it unchanged because no concrete defect is present.
- [x] Add `.github/workflows/fontforge-tests.yml` with Ubuntu, macOS, and Windows jobs that run focused and library tests and compile all test targets without FontForge.

## Validation

- [x] Run `cargo fmt --check` and resolve formatting differences.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings` and resolve introduced warnings.
- [x] Run `cargo test --test fontforge_worker` without FontForge installed and verify every required outcome.
- [x] Run `cargo test --lib` and verify the library suite remains green.
- [x] Run `cargo test --no-run` and verify every test target compiles.
- [ ] Verify Ubuntu, macOS, and Windows CI jobs compile all test targets and run the helper, including timeout termination.

## Completion

- [x] Review the final diff for unconditional Unix APIs, shell fixtures, altered default FontForge arguments, and unnecessary Agent 1 model changes.
- [x] Record platforms tested, helper mechanism, remaining production portability concerns, validation results, and intentional deviations in the completion notes.

## Completion notes

Implementation uses the Cargo-built Rust fake worker selected in the plan. The
focused worker suite passed 8 tests, library tests passed 44 tests, Clippy and
format checks passed, and `cargo test --no-run` compiled every test target.

The full `cargo test` command was also attempted but remains blocked by the
pre-existing `mixed-styles` entry in `tests/fixtures/manifest.json`, which is
missing required `native_text`, page-size, fallback, and related fields. That
unrelated fixture was not changed. Remote Ubuntu, macOS, and Windows execution
remains to be verified by the added workflow.

The Python worker has no remaining path portability concern identified in this
change. Real FontForge runtime behavior remains unvalidated. Files most likely
to overlap Agent 1 are `src/fonts/worker.rs`, the new worker integration test,
and the shared font model only if that agent changes its test fixtures.
