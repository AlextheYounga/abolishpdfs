# Plan

## Current behavior

`FontForgeWorker::process` in `src/fonts/worker.rs` writes temporary input,
output, and response paths, invokes the configured executable with
`-lang=py -script`, the repository Python script, and job metadata, then polls,
times out, checks the WOFF signature, parses JSON, and validates family name,
glyph count, and metric count.

Its unit tests create a `#!/bin/sh` fixture and use `PermissionsExt::set_mode`.
They cover success, missing executable, invalid output, timeout, and pre-start
mapping rejection, but not non-zero exit or malformed/missing responses. No CI
workflow exists in this worktree.

`tools/fontforge_worker.py` receives string paths and scalar arguments from
FontForge, uses FontForge's file APIs, and writes the response with explicit
UTF-8 encoding. It contains no shell invocation or Unix-only path API.

## Intended behavior

Integration tests obtain the path to a Cargo-built `fontforge_test_helper`
binary. Each test selects an explicit helper mode through the worker's command
configuration. The helper produces valid output or deterministic invalid asset,
missing asset, malformed response, non-zero exit, and delay outcomes.

The worker remains responsible for process lifecycle, timeout termination,
asset signatures, response parsing, and response validation. The helper is not
used by production commands, and normal FontForge invocation remains unchanged.

## Approach

1. Add `src/bin/fontforge_test_helper.rs`, a small Rust binary with fixed modes
   for the required subprocess outcomes.
2. Move behavior tests to `tests/fontforge_worker.rs`, where Cargo exposes the
   helper path through `CARGO_BIN_EXE_fontforge_test_helper`.
3. Extend `WorkerConfig` with optional executable arguments. Tests provide the
   helper mode; production leaves the list empty. Insert these arguments before
   the existing FontForge script arguments.
4. Remove the shell fixture and Unix-only import from `worker.rs` while keeping
   production validation and pre-start checks in their existing module.
5. Leave `tools/fontforge_worker.py` unchanged because repository investigation
   found no concrete path portability defect.
6. Add `.github/workflows/fontforge-tests.yml` with Ubuntu, macOS, and Windows
   jobs running focused and complete Rust tests without FontForge installation.

## Responsibilities and boundaries

* `FontForgeWorker` owns subprocess lifecycle, timeout termination, generated
  asset checks, response parsing, and response validation.
* `WorkerConfig` owns executable path, timeout, output format, and optional
  executable arguments; production callers leave extra arguments empty.
* `src/bin/fontforge_test_helper.rs` owns deterministic fake subprocess
  outcomes and no production font-processing behavior.
* `tests/fontforge_worker.rs` owns cross-platform scenario setup and assertions.
* `tools/fontforge_worker.py` remains the FontForge-native transformation script.
* GitHub Actions owns platform compilation and test execution.

## Affected areas

* `src/fonts/worker.rs`
* `src/bin/fontforge_test_helper.rs`
* `tests/fontforge_worker.rs`
* `.github/workflows/fontforge-tests.yml`
* `tools/fontforge_worker.py` only if a concrete path bug is discovered

## Decisions

* Use a Cargo-built Rust helper instead of per-platform scripts to avoid shell
  and permission semantics.
* Use an integration test so Cargo provides the helper executable path without
  test-only production branches.
* Put helper mode arguments in `WorkerConfig` because executable arguments are a
  real subprocess-boundary concern and preserve a reusable command seam.
* Keep helper output minimal so tests exercise worker checks without duplicating
  FontForge or changing font semantics.
* Add CI rather than installing FontForge because these tests validate the Rust
  boundary and must remain deterministic.

## Risks

* A helper argument-position mismatch could bypass the intended worker path;
  tests must assert returned worker results and errors.
* Incorrect argument insertion could alter real FontForge invocation; the empty
  default must preserve the current command exactly.
* Windows process termination and temporary-file cleanup can differ from Unix;
  CI must run the timeout and cleanup scenarios there.
* Agent 1 may edit nearby font model or worker tests; integration must preserve
  both branches' semantics and resolve overlap intentionally.

## Validation

* Run `cargo fmt --check`.
* Run `cargo clippy --all-targets --all-features -- -D warnings`.
* Run `cargo test --test fontforge_worker` without FontForge installed.
* Run `cargo test --lib` for the library regression suite and
  `cargo test --no-run` to compile every test target.
* Verify Ubuntu, macOS, and Windows CI jobs compile all test targets and execute
  the focused worker and library tests, including timeout termination.
* Review the final diff for Unix imports, shell fixtures, altered default
  FontForge arguments, and accidental Agent 1 model changes.
