# Idea

## Request

Make the FontForge worker test infrastructure portable across Linux, macOS,
and Windows. Fix the test harness boundary, not the production font-processing
architecture.

## Problem

The tests in `src/fonts/worker.rs` create a shell script and make it executable
with `std::os::unix::fs::PermissionsExt`. They cannot compile on Windows and
test shell behavior rather than the worker's cross-platform subprocess boundary.
The repository also has no platform CI job that catches this regression.

## Definitions

**FontForge worker:** The Rust `FontForgeWorker` subprocess adapter. It writes
an input font and job metadata, starts the configured executable, enforces a
timeout, then validates the generated web-font asset and JSON response.

**Fake worker:** A small Rust executable built by Cargo only to exercise the
FontForge worker tests. It does not implement FontForge or test production font
semantics; its explicit modes produce the subprocess outcomes required by the
tests.

**Worker response:** The JSON file emitted by the subprocess. It contains the
requested family name, glyph count, and advance widths, all of which the Rust
worker validates.

**Portable test:** A test that uses Rust and platform-neutral process/file APIs
and does not require `/bin/sh`, Unix permission bits, or an installed FontForge.

## Desired outcome

`cargo test` compiles the FontForge worker tests on Linux, macOS, and Windows.
The tests exercise successful output, missing executables, non-zero exit,
missing or invalid generated assets, malformed responses, timeout termination,
and valid family/glyph/metric responses without requiring a real FontForge
installation.

## Scope

This change includes replacing shell fixtures with a Cargo-built Rust fake
worker, moving worker behavior tests to an integration-test boundary, adding
the smallest reusable command-argument seam needed by the fake worker,
reviewing the Python worker for concrete path portability defects, and adding
GitHub Actions coverage for Ubuntu, macOS, and Windows.

## Constraints

* Follow `AGENTS.md` and repository conventions.
* Use a small Rust-native fake worker rather than `.sh`, `.cmd`, and PowerShell
  fixtures.
* Do not introduce a general-purpose subprocess testing framework.
* Keep production `FontForgeWorker` a normal subprocess wrapper.
* Do not add `#[cfg(test)]` branches throughout production execution logic.
* Do not require a real FontForge installation for these tests.

## Exclusions

Do not change FontForge/native-text failure policy, `FontSource` identity, font
mapping, PDFium, browser tests, runtime download/bootstrap behavior, advanced
font formats, or font semantics in `tools/fontforge_worker.py`. Agent 1 owns
font integrity semantics; avoid unnecessarily touching those model structures.
Real FontForge runtime validation remains runtime/bootstrap and later
integration work.

## Acceptance criteria

Tests cover worker success, missing executable, non-zero exit, malformed or
missing generated asset, malformed response, timeout and process termination,
and family/glyph/metric response validation. No test imports Unix-only APIs
unconditionally. `cargo test` compiles on Windows as well as Unix targets.

The platform matrix runs the Rust tests on Ubuntu, macOS, and Windows without
installing or invoking a real FontForge binary.

At completion report the old portability problem, chosen fake-worker mechanism,
platforms compiled/tested, remaining production FontForge portability concerns,
and files likely to conflict with Agent 1.
